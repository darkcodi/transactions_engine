use echodb::Error;
use thiserror::Error;

use crate::account::Account;
use crate::transaction::Transaction;

pub trait Storage {
    type DbTx;

    async fn get_tx(&self, db_tx: &mut Self::DbTx, tx_id: u32) -> Result<Option<Transaction>, DbError>;
    async fn insert_tx(&self, db_tx: &mut Self::DbTx, tx: &Transaction) -> Result<(), DbError>;
    async fn update_tx(&self, db_tx: &mut Self::DbTx, old_tx: &Transaction, new_tx: &Transaction) -> Result<(), DbError>;

    async fn get_account(&self, db_tx: &mut Self::DbTx, acc_id: u16) -> Result<Option<Account>, DbError>;
    async fn insert_account(&self, db_tx: &mut Self::DbTx, acc: &Account) -> Result<(), DbError>;
    async fn update_account(&self, db_tx: &mut Self::DbTx, old_acc: &Account, new_acc: &Account) -> Result<(), DbError>;

    // methods for idempotency
    async fn is_operation_processed(&self, db_tx: &mut Self::DbTx, op_hash: u64) -> Result<bool, DbError>;
    async fn insert_operation(&self, db_tx: &mut Self::DbTx, op: u64) -> Result<(), DbError>;

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
    db: echodb::Db<String, Vec<u8>>,
}

impl EchoDbStorage {
    pub fn new() -> Self {
        Self {
            db: echodb::new(),
        }
    }

    fn get_key_for_tx(tx_id: u32) -> String {
        format!("tx:{}", tx_id)
    }

    fn get_key_for_acc(acc_id: u16) -> String {
        format!("acc:{}", acc_id)
    }

    fn get_key_for_op(op_hash: u64) -> String {
        format!("op:{}", op_hash)
    }
}

impl Storage for EchoDbStorage {
    type DbTx = echodb::Tx<String, Vec<u8>>;

    async fn get_tx(&self, db_tx: &mut Self::DbTx, tx_id: u32) -> Result<Option<Transaction>, DbError> {
        let key = Self::get_key_for_tx(tx_id);
        if let Some(data) = db_tx.get(key)? {
            Ok(Some(rmp_serde::from_slice(&data)?))
        } else {
            Ok(None)
        }
    }

    async fn insert_tx(&self, db_tx: &mut Self::DbTx, tx: &Transaction) -> Result<(), DbError> {
        let key = Self::get_key_for_tx(tx.id());
        let data = rmp_serde::to_vec(tx)?;
        db_tx.put(key, data)?;
        Ok(())
    }

    async fn update_tx(&self, db_tx: &mut Self::DbTx, old_tx: &Transaction, new_tx: &Transaction) -> Result<(), DbError> {
        let key = Self::get_key_for_tx(old_tx.id());
        let old_data = rmp_serde::to_vec(old_tx)?;
        let new_data = rmp_serde::to_vec(new_tx)?;
        db_tx.putc(key, new_data, Some(old_data))?;
        Ok(())
    }

    async fn get_account(&self, db_tx: &mut Self::DbTx, acc_id: u16) -> Result<Option<Account>, DbError> {
        let key = Self::get_key_for_acc(acc_id);
        if let Some(data) = db_tx.get(key)? {
            Ok(Some(rmp_serde::from_slice(&data)?))
        } else {
            Ok(None)
        }
    }

    async fn insert_account(&self, db_tx: &mut Self::DbTx, acc: &Account) -> Result<(), DbError> {
        let key = Self::get_key_for_acc(acc.id());
        let data = rmp_serde::to_vec(acc)?;
        db_tx.put(key, data)?;
        Ok(())
    }

    async fn update_account(&self, db_tx: &mut Self::DbTx, old_acc: &Account, new_acc: &Account) -> Result<(), DbError> {
        let key = Self::get_key_for_acc(old_acc.id());
        let old_data = rmp_serde::to_vec(old_acc)?;
        let new_data = rmp_serde::to_vec(new_acc)?;
        db_tx.putc(key, new_data, Some(old_data))?;
        Ok(())
    }

    async fn is_operation_processed(&self, db_tx: &mut Self::DbTx, op_hash: u64) -> Result<bool, DbError> {
        let key = Self::get_key_for_op(op_hash);
        let exists = db_tx.exi(key)?;
        Ok(exists)
    }

    async fn insert_operation(&self, db_tx: &mut Self::DbTx, op_hash: u64) -> Result<(), DbError> {
        let key = Self::get_key_for_op(op_hash);
        db_tx.put(key, vec![0])?;
        Ok(())
    }

    async fn start_db_tx(&mut self) -> Result<Self::DbTx, DbError> {
        let db_tx = self.db.begin(true).await?;
        Ok(db_tx)
    }

    async fn commit_db_tx(&mut self, mut db_tx: Self::DbTx) -> Result<(), DbError> {
        db_tx.commit()?;
        Ok(())
    }
}

impl From<echodb::err::Error> for DbError {
    fn from(value: Error) -> Self {
        match value {
            Error::DbError => DbError::DatabaseError("Can not open transaction".to_string()),
            Error::TxClosed => DbError::DatabaseError("Transaction is closed".to_string()),
            Error::TxNotWritable => DbError::DatabaseError("Transaction is not writable".to_string()),
            Error::KeyAlreadyExists => DbError::EntityAlreadyExists,
            Error::ValNotExpectedValue => DbError::ConcurrentModification,
        }
    }
}

impl From<rmp_serde::encode::Error> for DbError {
    fn from(value: rmp_serde::encode::Error) -> Self {
        DbError::DatabaseError(format!("Can not encode data: {}", value))
    }
}

impl From<rmp_serde::decode::Error> for DbError {
    fn from(value: rmp_serde::decode::Error) -> Self {
        DbError::DatabaseError(format!("Can not decode data: {}", value))
    }
}
