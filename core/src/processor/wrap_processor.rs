use crate::client::Client;
use crate::ProcessError::{AccountLocked, AmountNotFound, DisputedTransactionNotFound, InvalidTransactionTypeOrAmount, MismatchClientId, OrgTransactionNotFound, TransactionExists, TransactionUnderDispute};
use crate::{WrapAccountRepository, ProcessError, Transaction, TransactionProcessor};
use crate::account::wrap::WrapAccount;
use crate::account::basic::BasicAccount;
use crate::transaction_type::TransactionType::{Chargeback, Deposit, Dispute, Resolve, Withdrawal};

/// WrapTransactionProcessor contains only wrap-account repository.
/// Wrap account structure contains rest of data.
/// WrapTransactionProcessor is using WrapAccountRepository and WrapAccount.
pub struct WrapTransactionProcessor {
    client_repository: WrapAccountRepository,

    //for future use if we want to store transaction with all kind errors
    //dead letter queue
    //_dlq_repository: DlqRepository,
}

impl Default for WrapTransactionProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl WrapTransactionProcessor {
    pub fn new() -> Self {
        WrapTransactionProcessor {
            client_repository: WrapAccountRepository::new(),
            //_dlq_repository: None,
        }
    }

    fn account(&mut self, client: Client) -> Result<&mut WrapAccount, ProcessError> {
        let account = self.client_repository.find_by_client(client);

        // Whether the account is locked. An account is locked if a charge back occurs
        if account.locked() {
            return Err(AccountLocked);
        }

        Ok(account)
    }

    /// A withdraw is a debit to the client's asset account, meaning it should decrease the available and
    /// total funds of the client account
    fn withdrawal(&mut self, transaction: Transaction) -> Result<(), ProcessError> {
        let amount = &transaction.amount().ok_or(AmountNotFound)?;

        let account = self.account(transaction.client())?;

        match account.tx_repository().exist_by_tx_id(&transaction.tx_id()) {
            true => Err(TransactionExists),
            false => {
                let _ = account.withdrawal(amount)?;
                // The document is a bit unclear about what kind of transactions can be disputed, so we must save withdrawal transactions
                account.mut_tx_repository().insert(transaction.tx_id(), transaction);
                Ok(())
            }
        }
    }

    /// A deposit is a credit to the client's asset account, meaning it should increase the available and
    /// total funds of the client account
    fn deposit(&mut self, transaction: Transaction) -> Result<(), ProcessError> {
        let amount = &transaction.amount().ok_or(AmountNotFound)?;

        let account = self.account(transaction.client())?;

        match account.tx_repository().exist_by_tx_id(&transaction.tx_id()) {
            true => Err(TransactionExists),
            false => {
                let _ = account.deposit(amount)?;
                // The document is a bit unclear about what kind of transactions can be disputed, so we must save deposit transactions
                account.mut_tx_repository().insert(transaction.tx_id(), transaction);
                Ok(())
            }
        }
    }

    /// A dispute represents a client's claim that a transaction was erroneous and should be reversed.
    /// The transaction shouldn't be reversed yet but the associated funds should be held. This means
    /// that the clients available funds should decrease by the amount disputed, their held funds should
    /// increase by the amount disputed, while their total funds should remain the same.
    fn dispute(&mut self, transaction: Transaction) -> Result<(), ProcessError> {
        let account = self.account(transaction.client())?;

        if account.dispute_tx_repository().exist_by_tx_id(&transaction.tx_id()) {
            return Err(TransactionUnderDispute);
        }

        let org_tx = account.tx_repository().find_by_tx_id(&transaction.tx_id()).ok_or(OrgTransactionNotFound)?;

        if org_tx.client() != transaction.client() {
            return Err(MismatchClientId);
        }

        // The document is a bit unclear about what kind of transactions can be disputed
        match (org_tx.r#type(), org_tx.amount()) {
            (Deposit, Some(amount)) => {
                let _ = account.dispute_deposit(&amount)?;
                account.mut_dispute_tx_repository().insert(transaction.tx_id(), transaction);
                Ok(())
            }
            (Withdrawal, Some(amount)) => {
                let _ = account.dispute_withdrawal(&amount)?;
                account.mut_dispute_tx_repository().insert(transaction.tx_id(), transaction);
                Ok(())
            }
            _ => Err(InvalidTransactionTypeOrAmount)
        }
    }

    /// A resolve represents a resolution to a dispute, releasing the associated held funds. Funds that
    /// were previously disputed are no longer disputed. This means that the clients held funds should
    /// decrease by the amount no longer disputed, their available funds should increase by the
    /// amount no longer disputed, and their total funds should remain the same.
    fn resolve(&mut self, transaction: Transaction) -> Result<(), ProcessError> {
        let account = self.account(transaction.client())?;

        let dispute_tx = account.dispute_tx_repository().find_by_tx_id(&transaction.tx_id()).ok_or(DisputedTransactionNotFound)?;
        let org_tx = account.tx_repository().find_by_tx_id(&dispute_tx.tx_id()).ok_or(OrgTransactionNotFound)?;

        if org_tx.client() != transaction.client() {
            return Err(MismatchClientId);
        }

        // can we use resolve only for withdrawal?
        match (org_tx.r#type(), org_tx.amount()) {
            (Withdrawal | Deposit, Some(amount)) => {
                let _ = account.resolve(&amount)?;
                account.mut_dispute_tx_repository().delete_by_id(&transaction.tx_id());
                // no re-dispute allowed
                account.mut_tx_repository().delete_by_id(&transaction.tx_id());
                Ok(())
            }
            _ => Err(InvalidTransactionTypeOrAmount)
        }
    }

    /// A chargeback is the final state of a dispute and represents the client reversing a transaction.
    /// Funds that were held have now been withdrawn. This means that the clients held funds and
    /// total funds should decrease by the amount previously disputed. If a chargeback occurs the
    /// client's account should be immediately frozen.
    fn charge_back(&mut self, transaction: Transaction) -> Result<(), ProcessError> {
        let account = self.account(transaction.client())?;

        let dispute_tx = account.dispute_tx_repository().find_by_tx_id(&transaction.tx_id()).ok_or(DisputedTransactionNotFound)?;
        let org_tx = account.tx_repository().find_by_tx_id(&dispute_tx.tx_id()).ok_or(OrgTransactionNotFound)?;

        if org_tx.client() != transaction.client() {
            return Err(MismatchClientId);
        }

        // can we use chargeback only for withdrawal?
        match (org_tx.r#type(), org_tx.amount()) {
            (Withdrawal | Deposit, Some(amount)) => {
                let _ = account.chargeback(&amount)?;
                account.mut_dispute_tx_repository().delete_by_id(&transaction.tx_id());
                // no re-dispute allowed
                account.mut_tx_repository().delete_by_id(&transaction.tx_id());
                Ok(())
            }
            _ => Err(InvalidTransactionTypeOrAmount)
        }
    }
}

impl TransactionProcessor for WrapTransactionProcessor {
    fn process(&mut self, transaction: Transaction) -> Result<(), ProcessError> {
        // we can here match result and write transaction with errors to dlq repository
        // but by default we do nothing
        match &transaction.r#type() {
            Withdrawal => self.withdrawal(transaction),
            Deposit => self.deposit(transaction),
            Dispute => self.dispute(transaction),
            Resolve => self.resolve(transaction),
            Chargeback => self.charge_back(transaction),
        }
    }
}

impl IntoIterator for WrapTransactionProcessor {
    type Item = BasicAccount;
    type IntoIter = Box<dyn Iterator<Item=Self::Item>>; //impl Iterator in traits is unstable :(

    fn into_iter(self) -> Self::IntoIter {
        Box::new(self.client_repository.get_all_account_into_iter())
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::FromPrimitive;
    use crate::{WrapTransactionProcessor, Transaction, TransactionProcessor};
    use crate::transaction_type::TransactionType::{Chargeback, Deposit, Dispute, Resolve, Withdrawal};

    #[test]
    fn deposit_into_iter() {
        let mut processor = WrapTransactionProcessor::new();
        let transaction = Transaction::new(Deposit, 1, 1, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert!(!account.locked());
    }

    #[test]
    fn deposit_find_by_client() {
        let mut processor = WrapTransactionProcessor::new();
        let transaction = Transaction::new(Deposit, 1, 1, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let account = processor.client_repository.find_by_client(1);

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert!(!account.locked());
    }

    #[test]
    fn deposit_then_withdrawal() {
        let mut processor = WrapTransactionProcessor::new();
        let transaction = Transaction::new(Deposit, 1, 1, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Withdrawal, 1, 2, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.total(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(0_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert!(!account.locked());
    }

    #[test]
    fn two_clients_deposit_then_withdrawal() {
        let mut processor = WrapTransactionProcessor::new();
        let transaction = Transaction::new(Deposit, 1, 1, Some(Decimal::from_f32(1.0).unwrap()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Deposit, 2, 2, Some(Decimal::from_f32(2.0).unwrap()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Deposit, 1, 3, Some(Decimal::from_f32(2.0).unwrap()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Withdrawal, 1, 4, Some(Decimal::from_f32(1.5).unwrap()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Withdrawal, 2, 5, Some(Decimal::from_f32(3.0).unwrap()));
        //mus be error, insufficient founds
        assert!(processor.process(transaction).is_err());

        let mut iter = processor.into_iter();

        let account = iter.next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.available(), &Decimal::from_f32(1.5).unwrap());
        assert_eq!(account.held(), &Decimal::from_f32(0.0).unwrap());
        assert_eq!(account.total(), &Decimal::from_f32(1.5).unwrap());
        assert!(!account.locked());

        let account = iter.next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.available(), &Decimal::from_f32(2.0).unwrap());
        assert_eq!(account.held(), &Decimal::from_f32(0.0).unwrap());
        assert_eq!(account.total(), &Decimal::from_f32(2.0).unwrap());
        assert!(!account.locked());
    }

    #[test]
    fn deposit_withdrawal_dispute_then_resolve() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(200_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Withdrawal, 1, 2, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Dispute, 1, 2, None);
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Resolve, 1, 2, None);
        assert!(processor.process(transaction).is_ok());

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.total(), &Decimal::from(200_u64));
        assert_eq!(account.available(), &Decimal::from(200_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));

        //account not locked
        assert!(!account.locked());
    }

    #[test]
    fn deposit_withdrawal_then_invalid_resolve() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(200_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Withdrawal, 1, 2, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Resolve, 1, 2, None);
        assert!(processor.process(transaction).is_err());

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));

        //account not locked
        assert!(!account.locked());
    }

    #[test]
    fn deposit_dispute_then_valid_chargeback() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(200_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Withdrawal, 1, 2, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Dispute, 1, 2, None);
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Chargeback, 1, 2, None);
        assert!(processor.process(transaction).is_ok());

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();
        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));

        //account locked
        assert!(account.locked());
    }

    #[test]
    fn deposit_withdrawal_then_invalid_chargeback() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Withdrawal, 1, 2, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Chargeback, 1, 2, None);
        //error - dispute not started
        assert!(processor.process(transaction).is_err());

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        //no chargeback
        assert_eq!(account.total(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(0_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        //account not locked
        assert!(!account.locked());
    }

    #[test]
    fn deposit_dispute_then_resolve() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Dispute, 1, 1, None);
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Resolve, 1, 1, None);
        assert!(processor.process(transaction).is_ok());

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        //account not locked
        assert!(!account.locked());
    }

    #[test]
    fn deposit_dispute_then_invalid_dispute() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Dispute, 1, 1, None);
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Dispute, 1, 1, None);
        assert!(processor.process(transaction).is_err());

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(0_u64));
        assert_eq!(account.held(), &Decimal::from(100_u64));
        //account not locked
        assert!(!account.locked());
    }

    #[test]
    fn deposit_then_invalid_dispute_tx() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Dispute, 1, 2, None);
        assert!(processor.process(transaction).is_err());


        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        //account not locked
        assert!(!account.locked());
    }

    #[test]
    fn deposit_dispute_then_invalid_resolve_tx() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Dispute, 1, 1, None);
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Resolve, 1, 2, None);
        // must be error
        assert!(processor.process(transaction).is_err());


        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(0_u64));
        assert_eq!(account.held(), &Decimal::from(100_u64));
        //account not locked
        assert!(!account.locked());
    }

    #[test]
    fn deposit_dispute_then_invalid_chargeback_tx() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(100_u64.into()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Dispute, 1, 1, None);
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Chargeback, 1, 2, None);
        // must be error
        assert!(processor.process(transaction).is_err());


        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.available(), &Decimal::from(0_u64));
        assert_eq!(account.held(), &Decimal::from(100_u64));
        //account not locked
        assert!(!account.locked());
    }


    #[test]
    fn deposit_withdrawal_dispute_resolve_then_chargeback() {
        let mut processor = WrapTransactionProcessor::new();

        let transaction = Transaction::new(Deposit, 1, 1, Some(Decimal::from_f32(200.00).unwrap()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Withdrawal, 1, 2, Some(Decimal::from_f32(100.00).unwrap()));
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Dispute, 1, 2, None);
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Resolve, 1, 2, None);
        assert!(processor.process(transaction).is_ok());

        let transaction = Transaction::new(Chargeback, 1, 2, None);
        // must be error
        assert!(processor.process(transaction).is_err());

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.total(), &Decimal::from_f32(200.00).unwrap());
        assert_eq!(account.available(), &Decimal::from_f32(200.00).unwrap());
        assert_eq!(account.held(), &Decimal::from_f32(0.00).unwrap());
        //account not locked
        assert!(!account.locked());
    }

    #[test]
    fn one_million_deposit_then_withdrawal() {
        let mut processor = WrapTransactionProcessor::new();

        for i in 0..1_000_001_u32 {
            let transaction = Transaction::new(Deposit, 1, i, Some(i.into()));
            assert!(processor.process(transaction).is_ok());
        }

        let account = processor.client_repository.find_by_client(1);

        assert_eq!(account.total(), &Decimal::from(500000500000_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(500000500000_u64));
        assert!(!account.locked());


        for i in 0..1_000_001_u32 {
            let transaction = Transaction::new(Withdrawal, 1, i + 1_000_002, Some(i.into()));
            assert!(processor.process(transaction).is_ok());
        }

        let account = processor.into_iter().next();
        assert!(account.is_some());
        let account = account.unwrap();

        assert_eq!(account.total(), &Decimal::from(0_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(0_u64));
        assert!(!account.locked());
    }

    #[test]
    fn max_client_deposit_then_withdrawal() {
        let mut processor = WrapTransactionProcessor::new();

        let per_client_tx = 1000_u32;
        for client_id in 0..u16::MAX {
            let from = client_id as u32 * per_client_tx;
            for tx_id in from..(from + per_client_tx) {
                let transaction = Transaction::new(Deposit, client_id, tx_id, Some(client_id.into()));
                let r = processor.process(transaction);
                assert!(r.is_ok());
            }
        }

        let account = processor.client_repository.find_by_client(1);
        //
        assert_eq!(account.total(), &Decimal::from(1000_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(1000_u64));
        assert!(!account.locked());

        let shift= per_client_tx * u16::MAX as u32;
        for client_id in 0..u16::MAX {
            let from = shift + client_id as u32 * per_client_tx;
            for tx_id in from..(from + per_client_tx) {
                let transaction = Transaction::new(Withdrawal, client_id, tx_id, Some(client_id.into()));
                let r = processor.process(transaction);
                assert!(r.is_ok());
            }
        }

        let account = processor.client_repository.find_by_client(1);

        assert_eq!(account.total(), &Decimal::from(0_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(0_u64));
        assert!(!account.locked());
    }

}
