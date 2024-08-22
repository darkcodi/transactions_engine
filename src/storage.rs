use thiserror::Error;

use crate::account::Account;
use crate::decimal::Decimal4;
use crate::transaction::Transaction;

pub enum Operation {
    Deposit { account_id: u16, tx_id: u32, amount: Decimal4 },
    Withdraw { account_id: u16, tx_id: u32, amount: Decimal4 },
    Dispute { account_id: u16, tx_id: u32 },
    Resolve { account_id: u16, tx_id: u32 },
    Chargeback { account_id: u16, tx_id: u32 },
}

pub trait Storage {
    type DbTx;

    async fn get_tx(&self, db_tx: &Self::DbTx, tx_id: u32) -> Result<Option<Transaction>, DbError>;
    async fn insert_tx(&self, db_tx: &Self::DbTx, tx: &Transaction) -> Result<(), DbError>;
    async fn update_tx(&self, db_tx: &Self::DbTx, tx: &Transaction, prev_token: u16) -> Result<(), DbError>;

    async fn get_account(&self, db_tx: &Self::DbTx, account_id: u16) -> Result<Option<Account>, DbError>;
    async fn insert_account(&self, db_tx: &Self::DbTx, account: &Account) -> Result<(), DbError>;
    async fn update_account(&self, db_tx: &Self::DbTx, account: &Account, prev_token: u16) -> Result<(), DbError>;

    // methods for idempotency
    async fn is_operation_processed(&self, db_tx: &Self::DbTx, op: &Operation) -> Result<bool, DbError>;
    async fn insert_operation(&self, db_tx: &Self::DbTx, op: &Operation) -> Result<(), DbError>;

    // methods for consistency
    async fn start_db_tx(&mut self) -> Result<Self::DbTx, DbError>;
    async fn commit_db_tx(&mut self, db_tx: Self::DbTx) -> Result<(), DbError>;
}

#[derive(Error, Debug)]
pub enum DbError {
    #[error("insertion failed because entity already exists")]
    EntityAlreadyExists,

    #[error("update failed because of concurrency token mismatch")]
    ConcurrentModification,

    #[error("database error: {0}")]
    DatabaseError(String),
}

pub struct EchoDbStorage {

}

impl EchoDbStorage {
    pub fn new() -> Self {
        // let a = echodb::new();
        Self {
        }
    }
}
