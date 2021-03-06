use nohash_hasher::{BuildNoHashHasher, IntMap};
use crate::Transaction;
use crate::transaction::TxId;

/// Repository to store transaction (withdraw, dispute, or other transaction types if needed)
/// This repository is using HashMap/BuildNoHashHasher as hash implementation
/// TxId is a valid u32 transaction ID
pub struct TransactionMemoryRepository {
    inner: IntMap<TxId, Transaction>,
}

impl Default for TransactionMemoryRepository {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionMemoryRepository {
    pub fn new() -> Self {
        TransactionMemoryRepository {
            inner: IntMap::default()
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        TransactionMemoryRepository {
            inner: IntMap::with_capacity_and_hasher(capacity,  BuildNoHashHasher::default())
        }
    }

    pub fn find_by_tx_id(&self, tx_id: &TxId) -> Option<&Transaction> {
        self.inner.get(tx_id)
    }

    pub fn exist_by_tx_id(&self, tx_id: &TxId) -> bool {
        self.inner.contains_key(tx_id)
    }

    pub fn insert(&mut self, tx_id: TxId, transaction: Transaction) {
        self.inner.insert(tx_id, transaction);
    }

    pub fn delete_by_id(&mut self, tx_id: &TxId) {
        self.inner.remove(tx_id);
    }
}
