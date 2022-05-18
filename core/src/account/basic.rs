use rust_decimal::Decimal;
use crate::client::Client;
use serde::Serialize;
use crate::ProcessError;
use crate::ProcessError::{DecimalAmountOverflow, NegativeAmount, NotSufficientAvailableFunds, NotSufficientHeldFunds};

/// As alternative we can use custom serializer for Decimal type.
/// This serializer will format as four places past the decimal.
#[allow(dead_code)]
fn four_place_decimal_serializer<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
    where S: serde::Serializer,
{
    let str = format!("{:.4}", value);
    serializer.serialize_str(&str)
}

/// Basic account contains only state and balance for client/id.
/// The client has a single asset account. All transactions are to and from this single asset account;
/// There are multiple clients. Transactions reference clients.
/// Clients are represented by u16 integers. No names, addresses, or complex client profile info;
/// If a chargeback occurs the client's account should be immediately frozen.
#[derive(Debug, Default, Serialize)]
pub struct BasicAccount {
    client: Client,

    // The total funds that are available for trading, staking, withdrawal, etc. This
    // should be equal to the total - held amounts
    // as alternative we can use custom serializer
    // #[serde(serialize_with = "four_place_decimal_serializer")]
    #[serde(with = "rust_decimal::serde::str")]
    available: Decimal,

    // The total funds that are held for dispute. This should be equal to total -
    // available amounts
    // as alternative we can use custom serializer
    // #[serde(serialize_with = "four_place_decimal_serializer")]
    #[serde(with = "rust_decimal::serde::str")]
    held: Decimal,

    // The total funds that are available or held. This should be equal to available +
    // held
    // as alternative we can use custom serializer
    // #[serde(serialize_with = "four_place_decimal_serializer")]
    #[serde(with = "rust_decimal::serde::str")]
    total: Decimal,

    // Whether the account is locked. An account is locked if a charge back occurs
    locked: bool,
}

impl BasicAccount {
    pub fn new(client: Client) -> Self {
        BasicAccount {
            client,
            ..Default::default()
        }
    }
    pub fn available(&self) -> &Decimal {
        &self.available
    }
    pub fn held(&self) -> &Decimal {
        &self.held
    }
    pub fn total(&self) -> &Decimal {
        &self.total
    }
    pub fn locked(&self) -> bool {
        self.locked
    }
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// A deposit is a credit to the client's asset account, meaning it should increase the available and
    /// total funds of the client account
    pub fn deposit(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        if amount.is_sign_negative() {
            return Err(NegativeAmount);
        }

        // check for overflow
        let available = self.available.checked_add(*amount).ok_or(DecimalAmountOverflow)?;
        let total = available.checked_add(self.held).ok_or(DecimalAmountOverflow)?;

        //no overflow, we can update values
        self.available = available;
        self.total = total;

        Ok(())
    }

    /// A withdraw is a debit to the client's asset account, meaning it should decrease the available and
    /// total funds of the client account
    pub fn withdrawal(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        if amount.is_sign_negative() {
            return Err(NegativeAmount);
        }

        if self.available < *amount {
            return Err(NotSufficientAvailableFunds);
        }

        // check for overflow
        let available = self.available.checked_sub(*amount).ok_or(DecimalAmountOverflow)?;
        let total = available.checked_add(self.held).ok_or(DecimalAmountOverflow)?;

        //no overflow, we can update values
        self.available = available;
        self.total = total;

        Ok(())
    }

    /// A dispute for deposit represents a client's claim that a transaction was erroneous and should be reversed.
    /// The transaction shouldn't be reversed yet but the associated funds should be held. This means
    /// that the clients available funds should decrease by the amount disputed, their held funds should
    /// increase by the amount disputed, while their total funds should remain the same.
    pub fn dispute_deposit(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        if amount.is_sign_negative() {
            return Err(NegativeAmount);
        }

        // should we allow or forbid if available < amount ?
        if self.available < *amount {
            return Err(NotSufficientAvailableFunds);
        }

        // check for overflow
        let available = self.available.checked_sub(*amount).ok_or(DecimalAmountOverflow)?;
        let held = self.held.checked_add(*amount).ok_or(DecimalAmountOverflow)?;
        let total = available.checked_add(held).ok_or(DecimalAmountOverflow)?; //total should remain the same.

        //no overflow, we can update values
        self.available = available;
        self.held = held;
        self.total = total;

        Ok(())
    }

    /// A dispute for withdrawal represents a client's claim that a transaction was erroneous and should be reversed.
    /// The transaction shouldn't be reversed yet but the associated funds should be held. This means
    /// that the clients held funds should increase by the amount disputed and their total funds should also increase.
    /// Dangerous: This method is unstable, may produce bugs in calculation and must be tested with external resources.
    pub fn dispute_withdrawal(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        if amount.is_sign_negative() {
            return Err(NegativeAmount);
        }

        // check for overflow
        let held = self.held.checked_add(*amount).ok_or(DecimalAmountOverflow)?;
        let total = self.available.checked_add(held).ok_or(DecimalAmountOverflow)?;

        //no overflow, we can update values
        self.held = held;
        self.total = total;

        Ok(())
    }

    /// A resolve represents a resolution to a dispute, releasing the associated held funds. Funds that
    /// were previously disputed are no longer disputed. This means that the clients held funds should
    /// decrease by the amount no longer disputed, their available funds should increase by the
    /// amount no longer disputed, and their total funds should remain the same.
    pub fn resolve(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        if amount.is_sign_negative() {
            return Err(NegativeAmount);
        }

        // should we allow or forbid if if held < amount?
        if self.held < *amount {
            return Err(NotSufficientHeldFunds);
        }

        // check for overflow
        let available = self.available.checked_add(*amount).ok_or(DecimalAmountOverflow)?;
        let held = self.held.checked_sub(*amount).ok_or(DecimalAmountOverflow)?;
        let total = available.checked_add(held).ok_or(DecimalAmountOverflow)?; //total should remain the same.\

        //no overflow, we can update values
        self.available = available;
        self.held = held;
        self.total = total;

        Ok(())
    }

    /// A chargeback is the final state of a dispute and represents the client reversing a transaction.
    /// Funds that were held have now been withdrawn. This means that the clients held funds and
    /// total funds should decrease by the amount previously disputed. If a chargeback occurs the
    /// client's account should be immediately frozen.
    pub fn chargeback(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        if amount.is_sign_negative() {
            return Err(NegativeAmount);
        }

        // should we allow or forbid if if held < amount?
        if self.held < *amount {
            return Err(NotSufficientHeldFunds);
        }

        // check for overflow
        let held = self.held.checked_sub(*amount).ok_or(DecimalAmountOverflow)?;
        let total = self.available.checked_add(held).ok_or(DecimalAmountOverflow)?;

        //no overflow, we can update values
        self.held = held;
        self.total = total;

        // account must be locked
        self.locked = true;

        Ok(())
    }
}


#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use crate::account::basic::BasicAccount;

    #[test]
    fn deposit_then_withdrawal() {
        let mut account = BasicAccount::new(1);

        assert!(account.deposit(&Decimal::from(100_u64)).is_ok());
        assert!(account.deposit(&Decimal::from(50_u64)).is_ok());
        assert!(account.withdrawal(&Decimal::from(50_u64)).is_ok());

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(100_u64));
    }

    #[test]
    fn deposit_withdrawal_then_chargeback() {
        let mut account = BasicAccount::new(1);

        assert!(account.deposit(&Decimal::from(100_u64)).is_ok());
        assert!(account.deposit(&Decimal::from(50_u64)).is_ok());
        assert!(account.withdrawal(&Decimal::from(50_u64)).is_ok());
        assert!(account.dispute_deposit(&Decimal::from(50_u64)).is_ok());

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(50_u64));
        assert_eq!(account.available(), &Decimal::from(50_u64));
        assert!(!account.locked());

        assert!(account.chargeback(&Decimal::from(50_u64)).is_ok());

        assert_eq!(account.total(), &Decimal::from(50_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(50_u64));
        assert!(account.locked());
    }

    #[test]
    fn deposit_withdrawal_insufficient_founds() {
        let mut account = BasicAccount::new(1);

        assert!(account.deposit(&Decimal::from(100_u64)).is_ok());

        //this must return error
        assert!(account.withdrawal(&Decimal::from(200_u64)).is_err());

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));

        assert!(!account.locked());
    }

    #[test]
    fn deposit_then_dispute() {
        let mut account = BasicAccount::new(1);

        assert!(account.deposit(&Decimal::from(200_u64)).is_ok());
        assert!(account.deposit(&Decimal::from(300_u64)).is_ok());

        assert!(account.dispute_deposit(&Decimal::from(300_u64)).is_ok());

        assert_eq!(account.total(), &Decimal::from(500_u64));
        assert_eq!(account.available(), &Decimal::from(200_u64));
        assert_eq!(account.held(), &Decimal::from(300_u64));

        assert!(!account.locked());
    }

    #[test]
    fn deposit_dispute_then_resolve() {
        let mut account = BasicAccount::new(1);

        assert!(account.deposit(&Decimal::from(200_u64)).is_ok());
        assert!(account.deposit(&Decimal::from(300_u64)).is_ok());

        assert!(account.dispute_deposit(&Decimal::from(200_u64)).is_ok());
        assert!(account.resolve(&Decimal::from(200_u64)).is_ok());


        assert_eq!(account.total(), &Decimal::from(500_u64));
        assert_eq!(account.available(), &Decimal::from(500_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));

        assert!(!account.locked());
    }

    #[test]
    fn deposit_withdrawal_then_dispute_deposit() {
        let mut account = BasicAccount::new(1);

        assert!(account.deposit(&Decimal::from(300_u64)).is_ok());
        assert!(account.withdrawal(&Decimal::from(300_u64)).is_ok());

        // must be error
        assert!(account.dispute_deposit(&Decimal::from(100_u64)).is_err());

        assert_eq!(account.total(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(0_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));

        assert!(!account.locked());
    }

    #[test]
    fn one_million_deposit_then_withdrawal() {
        let mut account = BasicAccount::new(1);
        for i in 0..1_000_001_u64 {
            assert!(account.deposit(&Decimal::from(i)).is_ok());
        }

        assert_eq!(account.total(), &Decimal::from(500000500000_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(500000500000_u64));
        assert!(!account.locked());

        for i in 0..1_000_001_u64 {
            assert!(account.withdrawal(&Decimal::from(i)).is_ok());
        }

        assert_eq!(account.total(), &Decimal::from(0_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(0_u64));
        assert!(!account.locked());
    }
}