//! Core library for calculating states and balance for client account.
//! All calculation is based on the TransactionProcessor trait and on the implementation of this trait.
//! This library contains two processors: BasicProcessor and WrapProcessor

extern crate core;

use crate::processor::wrap_processor::WrapTransactionProcessor;
use crate::transaction::Transaction;
use crate::error::ProcessError;
#[allow(unused_imports)]
use crate::processor::basic_processor::BasicTransactionProcessor;
#[allow(unused_imports)]
use crate::repository::basic_account_repository::BasicAccountMemoryRepository;
use crate::repository::wrap_account_repository::WrapAccountMemoryRepository;
use crate::repository::transaction_repository::TransactionMemoryRepository;

pub mod client;
pub mod transaction;
pub mod transaction_type;
pub (crate) mod repository;
pub mod error;
pub mod processor;
mod account;

pub type BasicProcessor = BasicTransactionProcessor;
pub type WrapProcessor = WrapTransactionProcessor;

type WrapAccountRepository = WrapAccountMemoryRepository;
type BasicAccountRepository = BasicAccountMemoryRepository;
type TransactionRepository = TransactionMemoryRepository;


/// Transaction processor trait is abstraction about process of transaction.
/// We can easily build own transaction process if default processors are not enough.
pub trait TransactionProcessor: IntoIterator {
    fn process(&mut self, transaction: Transaction) -> Result<(), ProcessError>;
}
