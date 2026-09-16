pub mod store;
pub mod verify;

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub use store::SqliteStore;
pub use verify::verify_chain;

pub const GENESIS: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub seq: u64,
    pub timestamp: DateTime<Utc>,
    pub action: String,
    pub payload_hash: String,
    pub prev_hash: String,
    pub hash: String,
    pub signature: Option<String>,
}

impl Receipt {
    pub fn digest(
        seq: u64,
        ts: DateTime<Utc>,
        action: &str,
        payload_hash: &str,
        prev_hash: &str,
    ) -> String {
        let mut h = Sha256::new();
        h.update(b"SVR1");
        h.update(seq.to_be_bytes());
        h.update(ts.timestamp_millis().to_be_bytes());
        h.update((action.len() as u64).to_be_bytes());
        h.update(action.as_bytes());
        h.update(hex::decode(payload_hash).unwrap_or_default());
        h.update(prev_hash.as_bytes());
        hex::encode(h.finalize())
    }
}

pub struct Ledger {
    inner: parking_lot::Mutex<Inner>,
}

struct Inner {
    seq: u64,
    last_hash: String,
    store: Box<dyn store::LedgerStore + Send>,
}

impl Ledger {
    pub fn open(store: Box<dyn store::LedgerStore + Send>) -> Result<Self> {
        let (seq, last_hash) = match store.last()? {
            Some(r) => (r.seq + 1, r.hash),
            None => (0, GENESIS.to_string()),
        };
        Ok(Self {
            inner: parking_lot::Mutex::new(Inner { seq, last_hash, store }),
        })
    }

    pub fn append(&self, action: &str, payload: &[u8]) -> Result<Receipt> {
        let mut g = self.inner.lock();
        let ts = Utc::now();
        let payload_hash = hex::encode(Sha256::digest(payload));
        let hash = Receipt::digest(g.seq, ts, action, &payload_hash, &g.last_hash);
        let receipt = Receipt {
            seq: g.seq,
            timestamp: ts,
            action: action.to_string(),
            payload_hash,
            prev_hash: g.last_hash.clone(),
            hash: hash.clone(),
            signature: None,
        };
        g.store.append(&receipt)?;
        g.seq += 1;
        g.last_hash = hash;
        Ok(receipt)
    }

    pub fn head(&self) -> (u64, String) {
        let g = self.inner.lock();
        (g.seq, g.last_hash.clone())
    }

    pub fn all(&self) -> Result<Vec<Receipt>> {
        let g = self.inner.lock();
        g.store.all()
    }
}
