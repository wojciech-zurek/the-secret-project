# The secret project Nekark

### Usage help
```
 cargo run -- --help
 csv-cli-analyzer 0.1.0
Wojciech Zurek <zurek.wojciech2@gmail.com>
Simple CSV reader for transaction analyze

USAGE:
    csv-cli-analyzer <file_path>

ARGS:
    <file_path>    File path where csv file is located

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version informatio
```

### How to run

```fish
$ cargo run -- transactions.csv > accounts.csv
```

### How to test
```fish
cargo test --workspace
# or
cd core
cargo test
```
### Test coverage (Tarpaulin)
```
 Coverage Results:
|| Tested/Total Lines:
|| cli/src/cli.rs: 0/4 +0.00%
|| cli/src/error.rs: 0/18 +0.00%
|| cli/src/main.rs: 0/3 +0.00%
|| cli/src/process.rs: 0/14 +0.00%
|| cli/src/reader.rs: 0/7 +0.00%
|| cli/src/write.rs: 0/10 +0.00%
|| core/src/account/basic.rs: 134/146 +0.00%
|| core/src/account/wrap.rs: 105/109 +0.00%
|| core/src/processor/basic_processor.rs: 276/289 +0.00%
|| core/src/processor/wrap_processor.rs: 302/313 +0.00%
|| core/src/repository/basic_account_repository.rs: 6/8 +0.00%
|| core/src/repository/transaction_repository.rs: 10/10 +0.00%
|| core/src/repository/wrap_account_repository.rs: 7/7 +0.00%
|| core/src/transaction.rs: 9/9 +0.00%
||
89.65% coverage, 849/947 lines covered, +0% change in coverage
```

### Assumptions and some info
1. Custom types:
   pub type Client = u16; Clients are represented by u16 integers
   pub type TxId = u32; The tx is a valid u32 transaction ID

2. The client has a single asset account. All transactions are to and from this single asset account;
3. There are multiple clients. Transactions reference clients. If a client doesn't exist create a new record;
4. The amount is a decimal value with a precision of up to four places past the decimal.
5. Input is read from file (csv), with fields:
- transaction type,
- client (id),
- tx (tx id),
- amount (optional).
6. Output write to stdout, as csv data type, with fields:
- available - The total funds that are available for trading, staking, withdrawal, etc. This should be equal to the total - held amounts
- held - The total funds that are held for dispute. This should be equal to total - available amounts,
- total - The total funds that are available or held. This should be equal to available + held,
- locked - Whether the account is locked. An account is locked if a chargeback occurs.
8. For amount values has been used `rust_decimal` crate.
9. For parsing command argument has been used `clap` crate.
10. For csv reads/writes are used: `serde` and `csv` crate.
11. Because client id / tx id  are primitive types `nohash-hasher` crate has been used for HashMap key hasher for maximum speed lookup. If we need more secure solution we can use `hashbrown` or `ahash` or use default `SipHash` for DOS resistance.
12. This application contains core library and the cli frontend.
13. Core library can be easily used for different purpose: web server, etl, web assembly (?), etc...
14. Only core library is tested by units test.
15. Core library contains 2 processor (also as example if we need different strategy)
- BasicTransactionProcessor (BasicProcessor) -  contains client/account state repository , transaction repository, dispute repository as separate fields. This processor does not contains any lock system.
- WrapTransactionProcessor(WrapProcessor) - contains only client/account state repository. WrapAccount struct contains transaction repository and dispute repository.
  Based on distribution of data (number of clients, transaction count, how many disputes per client) those processors may produce different performance.
16. In a multithreaded environment, we must use `Arc<Mutex<...>>` or `Arc<RwLock<..>>` on those processors.
17. Each transaction may produce different error.
18. By default, those errors are silenced.
19. Transaction amount money must be >= 0.
20. Money account state must be >= 0. No negative state is allowed.
21. For Withdrawal transaction available amount must be greater than transaction amount.
22. For Deposit dispute transaction available amount must be greater than transaction amount.
23. For Resolve transaction held amount must be greater than transaction amount.
24. For Chargeback transaction held amount must be greater than transaction amount.
25. By default, for Decimal value serialization has been used `rust_decimal::serde::str`. Alternative solution is custom serializer `four_place_decimal_serializer`
26. The document is a bit unclear about what kind of transactions can be disputed. From one side dispute should be for Withdrawal, for example because of fraud, but then why we double decrease of available funds when Dispute occurs. From the other side dispute can be also for Deposit, for example deposit amount is less than client expected or some kind of purchase return. So there are 2 disputes (one for Deposit and one for Withdrawal(experimental)).
27. Dispute, Resolve and Chargeback transactions must have same client id as the original transaction.
28. Main bank account/state not exist in this example (credit or debit side). There is no transaction between 2 accounts.
29. When Dispute occurs only a Resolve or a Chargeback is allowed.
30. No transaction on account is allowed when account is locked.
31. No re-dispute transaction allowed.
32. For core crate there is a features `dlq = []` as example od data structure to collect transactions with error.