use super::{Receipt, GENESIS};
use anyhow::{bail, Result};

pub fn verify_chain(receipts: &[Receipt]) -> Result<()> {
    let mut prev = GENESIS.to_string();
    for (i, r) in receipts.iter().enumerate() {
        if r.seq != i as u64 {
            bail!("seq mismatch at index {i}: expected {i}, got {}", r.seq);
        }
        if r.prev_hash != prev {
            bail!("prev_hash mismatch at index {i}");
        }
        let expect = Receipt::digest(r.seq, r.timestamp, &r.action, &r.payload_hash, &r.prev_hash);
        if expect != r.hash {
            bail!("hash mismatch at index {i}: recomputed {expect}, stored {}", r.hash);
        }
        prev = r.hash.clone();
    }
    Ok(())
}
