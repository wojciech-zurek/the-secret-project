#[cfg(feature = "dlq")]
use crate::ProcessError;
#[cfg(feature = "dlq")]
use crate::Transaction;

#[cfg(feature = "dlq")]
pub struct NaiveDlqMemoryRepository {
    inner: Vec<(Transaction, ProcessError)>,
}

#[cfg(feature = "dlq")]
impl NaiveDlqMemoryRepository {
    pub fn new() -> Self {
        NaiveDlqMemoryRepository {
            inner: Vec::new()
        }
    }

    pub fn insert(&mut self, transaction: Transaction, error: ProcessError) {
        self.inner.push((transaction, error))
    }

    pub fn get_all(&self) -> impl Iterator<Item=&(Transaction, ProcessError)> {
        self.inner.iter()
    }
}