use super::Receipt;
use anyhow::{Context, Result};
use chrono::DateTime;
use rusqlite::{params, Connection};
use std::path::Path;

pub trait LedgerStore: Send {
    fn append(&mut self, r: &Receipt) -> Result<()>;
    fn last(&self) -> Result<Option<Receipt>>;
    fn all(&self) -> Result<Vec<Receipt>>;
}

pub struct SqliteStore {
    conn: Connection,
}

impl SqliteStore {
    pub fn open(path: impl AsRef<Path>) -> Result<Self> {
        let conn = Connection::open(path).context("open ledger db")?;
        conn.pragma_update(None, "journal_mode", "WAL")?;
        conn.pragma_update(None, "synchronous", "FULL")?;
        conn.execute_batch(
            r#"CREATE TABLE IF NOT EXISTS receipts (
                 seq          INTEGER PRIMARY KEY,
                 ts           TEXT NOT NULL,
                 action       TEXT NOT NULL,
                 payload_hash TEXT NOT NULL,
                 prev_hash    TEXT NOT NULL,
                 hash         TEXT NOT NULL UNIQUE,
                 signature    TEXT
               );"#,
        )?;
        Ok(Self { conn })
    }
}

impl LedgerStore for SqliteStore {
    fn append(&mut self, r: &Receipt) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT INTO receipts (seq, ts, action, payload_hash, prev_hash, hash, signature)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                r.seq as i64,
                r.timestamp.to_rfc3339(),
                r.action,
                r.payload_hash,
                r.prev_hash,
                r.hash,
                r.signature,
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    fn last(&self) -> Result<Option<Receipt>> {
        let mut stmt = self.conn.prepare(
            "SELECT seq, ts, action, payload_hash, prev_hash, hash, signature
             FROM receipts ORDER BY seq DESC LIMIT 1",
        )?;
        let mut rows = stmt.query([])?;
        if let Some(row) = rows.next()? {
            return Ok(Some(row_to_receipt(row)?));
        }
        Ok(None)
    }

    fn all(&self) -> Result<Vec<Receipt>> {
        let mut stmt = self.conn.prepare(
            "SELECT seq, ts, action, payload_hash, prev_hash, hash, signature
             FROM receipts ORDER BY seq ASC",
        )?;
        let rows = stmt.query_map([], |row| Ok(row_to_receipt(row).unwrap()))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

fn row_to_receipt(row: &rusqlite::Row) -> Result<Receipt> {
    Ok(Receipt {
        seq: row.get::<_, i64>(0)? as u64,
        timestamp: DateTime::parse_from_rfc3339(&row.get::<_, String>(1)?)?
            .with_timezone(&chrono::Utc),
        action: row.get(2)?,
        payload_hash: row.get(3)?,
        prev_hash: row.get(4)?,
        hash: row.get(5)?,
        signature: row.get(6)?,
    })
}
