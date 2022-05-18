use std::error::Error;
use std::fmt::{Display, Formatter};

#[derive(Debug)]
pub enum ProcessError {
    // Can't process tx: Transaction details not contains amount value
    AmountNotFound,

    // Can't process tx: Value overflow after transaction
    DecimalAmountOverflow,

    // Can't process tx: Expected amount >= 0;
    NegativeAmount,

    // Can't process tx:: Available money not sufficient for transaction
    NotSufficientAvailableFunds,

    // Can't process tx:: Held money not sufficient for transaction
    NotSufficientHeldFunds,

    // Can't process tx: Account locked after chargeback
    AccountLocked,

    // Can't process tx: A transaction already exists in the repository
    TransactionExists,

    // Can't process tx: Original transaction not exist in tx repository
    OrgTransactionNotFound,

    // Can't process tx: Dispute process not started
    DisputedTransactionNotFound,

    // Can't process tx: Transaction dispute process already started.
    TransactionUnderDispute,

    // Can't process tx: Invalid transaction type or amount not exist
    // Expected different original transaction type or amount
    InvalidTransactionTypeOrAmount,

    // Can't process tx: Original Client Id != Actual Client Id
    // For example dispute transaction has different client id than original transaction
    MismatchClientId,

    // Can't process tx: Acquiring a Mutex lock or RwLock unsuccessful.
    MutexLockError,

    // Can't process tx: Unexpected error
    UnknownOrUnexpectedError,
}

impl Display for ProcessError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ProcessError {}