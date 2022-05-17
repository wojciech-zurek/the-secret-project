use rust_decimal::Decimal;
use serde::Deserialize;
use crate::client::Client;
use crate::transaction_type::TransactionType;

pub type TxId = u32;

#[derive(Debug, Deserialize)]
pub struct Transaction {
    r#type: TransactionType,
    client: Client,
    tx: TxId,
    amount: Option<Decimal>,
}

impl Transaction {
    pub fn new(r#type: TransactionType, client: Client, tx: TxId, amount: Option<Decimal>) -> Self {
        Transaction {
            r#type,
            client,
            tx,
            amount,
        }
    }

    pub fn r#type(&self) -> &TransactionType {
        &self.r#type
    }
    pub fn client(&self) -> Client {
        self.client
    }
    pub fn tx_id(&self) -> TxId {
        self.tx
    }
    pub fn amount(&self) -> Option<Decimal> {
        self.amount
    }
}

