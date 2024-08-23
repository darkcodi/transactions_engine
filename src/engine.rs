use std::hash::{Hash, Hasher};
use thiserror::Error;
use crate::account::{Account, AccountUpdateError};
use crate::decimal::Decimal4;
use crate::storage::{DbError, Storage};
use crate::transaction::{Transaction, TransactionState, TransactionType, TxUpdateError};

pub enum Operation {
    Deposit { account_id: u16, tx_id: u32, amount: Decimal4 },
    Withdraw { account_id: u16, tx_id: u32, amount: Decimal4 },
    Dispute { account_id: u16, tx_id: u32 },
    Resolve { account_id: u16, tx_id: u32 },
    Chargeback { account_id: u16, tx_id: u32 },
}

impl Operation {
    pub fn get_hash_code(&self) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }
}

impl Hash for Operation {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let (op_str, acc_id, tx_id) = match self {
            Operation::Deposit { account_id, tx_id, amount: _ } => ("deposit", account_id, tx_id),
            Operation::Withdraw { account_id, tx_id, amount: _ } => ("withdraw", account_id, tx_id),
            Operation::Dispute { account_id, tx_id } => ("dispute", account_id, tx_id),
            Operation::Resolve { account_id, tx_id } => ("resolve", account_id, tx_id),
            Operation::Chargeback { account_id, tx_id } => ("chargeback", account_id, tx_id),
        };
        op_str.hash(state);
        acc_id.hash(state);
        tx_id.hash(state);
    }
}

pub struct Engine<TStorage: Storage> {
    storage: Box<TStorage>,
}

impl<TStorage: Storage> Engine<TStorage> {
    pub fn new(storage: TStorage) -> Self {
        Self {
            storage: Box::new(storage),
        }
    }

    pub async fn deposit(&mut self, account_id: u16, tx_id: u32, amount: Decimal4) -> Result<(), EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Deposit { account_id, tx_id, amount };
        let op_hash = operation.get_hash_code();
        let operation_processed = self.storage.is_operation_processed(&mut db_tx, op_hash).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&mut db_tx, tx_id).await?;
        let transaction_exists = maybe_tx.is_some();
        if transaction_exists {
            return Err(EngineError::TransactionWithTheSameIdAlreadyExists);
        }

        let tx = Transaction::new(tx_id, account_id, TransactionType::Deposit, amount);
        self.storage.insert_tx(&mut db_tx, &tx).await?;

        let maybe_account = self.storage.get_account(&mut db_tx, account_id).await?;
        if let Some(old_acc) = maybe_account {
            let mut new_acc = old_acc.clone();
            new_acc.deposit(amount)?;
            self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        } else {
            let mut new_acc = Account::new(account_id);
            new_acc.deposit(amount)?;
            self.storage.insert_account(&mut db_tx, &new_acc).await?;
        }

        self.storage.insert_operation(&mut db_tx, op_hash).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn withdraw(&mut self, account_id: u16, tx_id: u32, amount: Decimal4) -> Result<(), EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Withdraw { account_id, tx_id, amount };
        let op_hash = operation.get_hash_code();
        let operation_processed = self.storage.is_operation_processed(&mut db_tx, op_hash).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&mut db_tx, tx_id).await?;
        let transaction_exists = maybe_tx.is_some();
        if transaction_exists {
            return Err(EngineError::TransactionWithTheSameIdAlreadyExists);
        }

        let maybe_account = self.storage.get_account(&mut db_tx, account_id).await?;
        let old_acc = maybe_account.ok_or(EngineError::AccountNotFound)?;
        let mut new_acc = old_acc.clone();
        new_acc.withdraw(amount)?;

        let tx = Transaction::new(tx_id, account_id, TransactionType::Withdrawal, amount);
        self.storage.insert_tx(&mut db_tx, &tx).await?;
        self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        self.storage.insert_operation(&mut db_tx, op_hash).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn dispute(&mut self, account_id: u16, tx_id: u32) -> Result<(), EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Dispute { account_id, tx_id };
        let op_hash = operation.get_hash_code();
        let operation_processed = self.storage.is_operation_processed(&mut db_tx, op_hash).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&mut db_tx, tx_id).await?;
        let old_tx = maybe_tx.ok_or(EngineError::TransactionNotFound)?;
        if old_tx.account_id() != account_id {
            return Err(EngineError::TransactionIsBoundToAnotherAccount(old_tx.account_id()));
        }

        let maybe_account = self.storage.get_account(&mut db_tx, account_id).await?;
        let old_acc = maybe_account.ok_or(EngineError::AccountNotFound)?;

        let mut new_tx = old_tx.clone();
        new_tx.set_state(TransactionState::Disputed)?;

        let mut new_acc = old_acc.clone();
        new_acc.dispute(new_tx.amount());

        self.storage.update_tx(&mut db_tx, &old_tx, &new_tx).await?;
        self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        self.storage.insert_operation(&mut db_tx, op_hash).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn resolve(&mut self, account_id: u16, tx_id: u32) -> Result<(), EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Resolve { account_id, tx_id };
        let op_hash = operation.get_hash_code();
        let operation_processed = self.storage.is_operation_processed(&mut db_tx, op_hash).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&mut db_tx, tx_id).await?;
        let old_tx = maybe_tx.ok_or(EngineError::TransactionNotFound)?;
        if old_tx.account_id() != account_id {
            return Err(EngineError::TransactionIsBoundToAnotherAccount(old_tx.account_id()));
        }

        let maybe_account = self.storage.get_account(&mut db_tx, account_id).await?;
        let old_acc = maybe_account.ok_or(EngineError::AccountNotFound)?;

        let mut new_tx = old_tx.clone();
        new_tx.set_state(TransactionState::Resolved)?;

        let mut new_acc = old_acc.clone();
        new_acc.resolve(new_tx.amount());

        self.storage.update_tx(&mut db_tx, &old_tx, &new_tx).await?;
        self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        self.storage.insert_operation(&mut db_tx, op_hash).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn chargeback(&mut self, account_id: u16, tx_id: u32) -> Result<(), EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Chargeback { account_id, tx_id };
        let op_hash = operation.get_hash_code();
        let operation_processed = self.storage.is_operation_processed(&mut db_tx, op_hash).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&mut db_tx, tx_id).await?;
        let old_tx = maybe_tx.ok_or(EngineError::TransactionNotFound)?;
        if old_tx.account_id() != account_id {
            return Err(EngineError::TransactionIsBoundToAnotherAccount(old_tx.account_id()));
        }

        let maybe_account = self.storage.get_account(&mut db_tx, account_id).await?;
        let old_acc = maybe_account.ok_or(EngineError::AccountNotFound)?;

        let mut new_tx = old_tx.clone();
        new_tx.set_state(TransactionState::Chargeback)?;

        let mut new_acc = old_acc.clone();
        new_acc.chargeback(new_tx.amount());

        self.storage.update_tx(&mut db_tx, &old_tx, &new_tx).await?;
        self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        self.storage.insert_operation(&mut db_tx, op_hash).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }
}

#[derive(Error, Debug)]
pub enum EngineError {
    #[error("account not found")]
    AccountNotFound,

    #[error("transaction not found")]
    TransactionNotFound,

    #[error("account is locked")]
    AccountLocked,

    #[error("insufficient funds")]
    InsufficientFunds,

    #[error("transaction with the same id already exists")]
    TransactionWithTheSameIdAlreadyExists,

    #[error("transaction is bound to another account")]
    TransactionIsBoundToAnotherAccount(u16),

    #[error("invalid transaction type: only deposits can be disputed/resolved/chargebacked")]
    InvalidTxType,

    #[error("forbidden state transition from {from:?} to {to:?}")]
    ForbiddenTxStateTransition { from: TransactionState, to: TransactionState },

    #[error("concurrent operation detected for the same entities")]
    ConcurrentOperationDetected,

    #[error("database error: {0}")]
    DatabaseError(String),
}

impl From<DbError> for EngineError {
    fn from(err: DbError) -> Self {
        match err {
            DbError::EntityAlreadyExists => EngineError::ConcurrentOperationDetected,
            DbError::ConcurrentModification => EngineError::ConcurrentOperationDetected,
            DbError::DatabaseError(msg) => EngineError::DatabaseError(msg),
        }
    }
}

impl From<AccountUpdateError> for EngineError {
    fn from(err: AccountUpdateError) -> Self {
        match err {
            AccountUpdateError::AccountLocked => EngineError::AccountLocked,
            AccountUpdateError::InsufficientFunds => EngineError::InsufficientFunds,
        }
    }
}

impl From<TxUpdateError> for EngineError {
    fn from(err: TxUpdateError) -> Self {
        match err {
            TxUpdateError::InvalidTxType => EngineError::InvalidTxType,
            TxUpdateError::ForbiddenTxStateTransition { from, to } => EngineError::ForbiddenTxStateTransition { from, to },
        }
    }
}
