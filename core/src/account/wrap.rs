use rust_decimal::Decimal;
use crate::account::basic::BasicAccount;
use crate::client::Client;
use crate::ProcessError;
use crate::TransactionRepository;

/// WrapAccount contains BasicAccount and transaction/dispute repository
pub struct WrapAccount {
    // account state and balance
    basic_account: BasicAccount,
    // repository to store withdraw transactions
    tx_repository: TransactionRepository,

    // repository to store dispute transactions
    //as alternative solution we can store this in HashSet<TxId> if transaction details not needed
    dispute_tx_repository: TransactionRepository,
}

impl WrapAccount {
    pub fn new(client: Client) -> Self {
        WrapAccount {
            basic_account: BasicAccount::new(client),
            tx_repository: TransactionRepository::new(),
            dispute_tx_repository: TransactionRepository::new(),
        }
    }

    #[allow(dead_code)]
    pub fn available(&self) -> &Decimal {
        self.basic_account.available()
    }

    #[allow(dead_code)]
    pub fn held(&self) -> &Decimal {
        self.basic_account.held()
    }

    #[allow(dead_code)]
    pub fn total(&self) -> &Decimal {
        self.basic_account.total()
    }

    #[allow(dead_code)]
    pub fn locked(&self) -> bool {
        self.basic_account.locked()
    }

    #[allow(dead_code)]
    pub fn client(&self) -> &Client {
        self.basic_account.client()
    }

    pub fn deposit(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        self.basic_account.deposit(amount)
    }

    pub fn withdrawal(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        self.basic_account.withdrawal(amount)
    }

    pub fn dispute_deposit(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        self.basic_account.dispute_deposit(amount)
    }

    pub fn dispute_withdrawal(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        self.basic_account.dispute_withdrawal(amount)
    }

    pub fn resolve(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        self.basic_account.resolve(amount)
    }

    pub fn chargeback(&mut self, amount: &Decimal) -> Result<(), ProcessError> {
        self.basic_account.chargeback(amount)
    }

    pub fn tx_repository(&self) -> &TransactionRepository {
        &self.tx_repository
    }

    pub fn mut_tx_repository(&mut self) -> &mut TransactionRepository {
        &mut self.tx_repository
    }

    pub fn dispute_tx_repository(&self) -> &TransactionRepository {
        &self.dispute_tx_repository
    }

    pub fn mut_dispute_tx_repository(&mut self) -> &mut TransactionRepository {
        &mut self.dispute_tx_repository
    }

    #[allow(dead_code)]
    pub fn account(&self) -> &BasicAccount {
        &self.basic_account
    }
    pub fn into_account(self) -> BasicAccount {
        self.basic_account
    }
}

#[cfg(test)]
mod tests {
    use rust_decimal::Decimal;
    use crate::account::wrap::WrapAccount;

    #[test]
    fn deposit_then_withdrawal() {
        let mut account = WrapAccount::new(1);

        assert!(account.deposit(&Decimal::from(100_u64)).is_ok());
        assert!(account.deposit(&Decimal::from(50_u64)).is_ok());
        assert!(account.withdrawal(&Decimal::from(50_u64)).is_ok());

        assert_eq!(account.total(), &Decimal::from(100_u64));
        assert_eq!(account.held(), &Decimal::from(0_u64));
        assert_eq!(account.available(), &Decimal::from(100_u64));
    }

    #[test]
    fn deposit_withdrawal_then_chargeback() {
        let mut account = WrapAccount::new(1);

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
        let mut account = WrapAccount::new(1);

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
        let mut account = WrapAccount::new(1);

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
        let mut account = WrapAccount::new(1);

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
        let mut account = WrapAccount::new(1);

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
        let mut account = WrapAccount::new(1);
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