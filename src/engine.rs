use thiserror::Error;
use crate::account::{Account, AccountUpdateError};
use crate::decimal::Decimal4;
use crate::storage::{Operation, DbError, Storage};
use crate::transaction::{Transaction, TransactionState, TransactionType, TxUpdateError};

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
        let db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Deposit { account_id, tx_id, amount };
        let operation_processed = self.storage.is_operation_processed(&db_tx, &operation).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&db_tx, tx_id).await?;
        let transaction_exists = maybe_tx.is_some();
        if transaction_exists {
            return Err(EngineError::TransactionWithTheSameIdAlreadyExists);
        }

        let tx = Transaction::new(tx_id, account_id, TransactionType::Deposit, amount);
        self.storage.insert_tx(&db_tx, &tx).await?;

        let maybe_account = self.storage.get_account(&db_tx, account_id).await?;
        if let Some(mut account) = maybe_account {
            let prev_account_version = account.version();
            account.deposit(amount)?;
            self.storage.update_account(&db_tx, &account, prev_account_version).await?;
        } else {
            let mut account = Account::new(account_id);
            account.deposit(amount)?;
            self.storage.insert_account(&db_tx, &account).await?;
        }

        self.storage.insert_operation(&db_tx, &operation).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn withdraw(&mut self, account_id: u16, tx_id: u32, amount: Decimal4) -> Result<(), EngineError> {
        let db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Withdraw { account_id, tx_id, amount };
        let operation_processed = self.storage.is_operation_processed(&db_tx, &operation).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&db_tx, tx_id).await?;
        let transaction_exists = maybe_tx.is_some();
        if transaction_exists {
            return Err(EngineError::TransactionWithTheSameIdAlreadyExists);
        }

        let maybe_account = self.storage.get_account(&db_tx, account_id).await?;
        let mut account = maybe_account.ok_or(EngineError::AccountNotFound)?;
        let prev_account_version = account.version();
        account.withdraw(amount)?;

        let tx = Transaction::new(tx_id, account_id, TransactionType::Withdrawal, amount);
        self.storage.insert_tx(&db_tx, &tx).await?;
        self.storage.update_account(&db_tx, &account, prev_account_version).await?;
        self.storage.insert_operation(&db_tx, &operation).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn dispute(&mut self, account_id: u16, tx_id: u32) -> Result<(), EngineError> {
        let db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Dispute { account_id, tx_id };
        let operation_processed = self.storage.is_operation_processed(&db_tx, &operation).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&db_tx, tx_id).await?;
        let mut tx = maybe_tx.ok_or(EngineError::TransactionNotFound)?;
        if tx.account_id() != account_id {
            return Err(EngineError::TransactionIsBoundToAnotherAccount(tx.account_id()));
        }

        let maybe_account = self.storage.get_account(&db_tx, account_id).await?;
        let mut account = maybe_account.ok_or(EngineError::AccountNotFound)?;

        let prev_tx_version = tx.version();
        tx.set_state(TransactionState::Disputed)?;

        let prev_account_version = account.version();
        account.dispute(tx.amount());

        self.storage.update_tx(&db_tx, &tx, prev_tx_version).await?;
        self.storage.update_account(&db_tx, &account, prev_account_version).await?;
        self.storage.insert_operation(&db_tx, &operation).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn resolve(&mut self, account_id: u16, tx_id: u32) -> Result<(), EngineError> {
        let db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Resolve { account_id, tx_id };
        let operation_processed = self.storage.is_operation_processed(&db_tx, &operation).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&db_tx, tx_id).await?;
        let mut tx = maybe_tx.ok_or(EngineError::TransactionNotFound)?;
        if tx.account_id() != account_id {
            return Err(EngineError::TransactionIsBoundToAnotherAccount(tx.account_id()));
        }

        let maybe_account = self.storage.get_account(&db_tx, account_id).await?;
        let mut account = maybe_account.ok_or(EngineError::AccountNotFound)?;

        let prev_tx_version = tx.version();
        tx.set_state(TransactionState::Resolved)?;

        let prev_account_version = account.version();
        account.resolve(tx.amount());

        self.storage.update_tx(&db_tx, &tx, prev_tx_version).await?;
        self.storage.update_account(&db_tx, &account, prev_account_version).await?;
        self.storage.insert_operation(&db_tx, &operation).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn chargeback(&mut self, account_id: u16, tx_id: u32) -> Result<(), EngineError> {
        let db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Chargeback { account_id, tx_id };
        let operation_processed = self.storage.is_operation_processed(&db_tx, &operation).await?;
        if operation_processed {
            return Ok(()); // idempotency
        }

        let maybe_tx = self.storage.get_tx(&db_tx, tx_id).await?;
        let mut tx = maybe_tx.ok_or(EngineError::TransactionNotFound)?;
        if tx.account_id() != account_id {
            return Err(EngineError::TransactionIsBoundToAnotherAccount(tx.account_id()));
        }

        let maybe_account = self.storage.get_account(&db_tx, account_id).await?;
        let mut account = maybe_account.ok_or(EngineError::AccountNotFound)?;

        let prev_tx_version = tx.version();
        tx.set_state(TransactionState::Chargeback)?;

        let prev_account_version = account.version();
        account.chargeback(tx.amount());

        self.storage.update_tx(&db_tx, &tx, prev_tx_version).await?;
        self.storage.update_account(&db_tx, &account, prev_account_version).await?;
        self.storage.insert_operation(&db_tx, &operation).await?;
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
