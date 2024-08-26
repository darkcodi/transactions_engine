use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use thiserror::Error;

use crate::account::{Account, AccountUpdateError};
use crate::decimal::Decimal4;
use crate::storage::{DbError, Storage};
use crate::transaction::{Transaction, TransactionState, TransactionType, TxUpdateError};

#[derive(Debug, PartialEq, Eq)]
pub enum Operation {
    Deposit { acc_id: u16, tx_id: u32, amount: Decimal4 },
    Withdraw { acc_id: u16, tx_id: u32, amount: Decimal4 },
    Dispute { acc_id: u16, tx_id: u32 },
    Resolve { acc_id: u16, tx_id: u32 },
    Chargeback { acc_id: u16, tx_id: u32 },
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
            Operation::Deposit { acc_id, tx_id, amount: _ } => ("deposit", acc_id, tx_id),
            Operation::Withdraw { acc_id, tx_id, amount: _ } => ("withdraw", acc_id, tx_id),
            Operation::Dispute { acc_id, tx_id } => ("dispute", acc_id, tx_id),
            Operation::Resolve { acc_id, tx_id } => ("resolve", acc_id, tx_id),
            Operation::Chargeback { acc_id, tx_id } => ("chargeback", acc_id, tx_id),
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

    pub async fn execute_operation(&mut self, operation: Operation) -> Result<(), EngineError> {
        match operation {
            Operation::Deposit { acc_id, tx_id, amount } => self.deposit(acc_id, tx_id, amount).await,
            Operation::Withdraw { acc_id, tx_id, amount } => self.withdraw(acc_id, tx_id, amount).await,
            Operation::Dispute { acc_id, tx_id } => self.dispute(acc_id, tx_id).await,
            Operation::Resolve { acc_id, tx_id } => self.resolve(acc_id, tx_id).await,
            Operation::Chargeback { acc_id, tx_id } => self.chargeback(acc_id, tx_id).await,
        }
    }

    pub async fn get_account(&mut self, acc_id: u16) -> Result<Option<Account>, EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;
        let account = self.storage.get_account(&mut db_tx, acc_id).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(account)
    }

    pub async fn get_all_accounts(&mut self) -> Result<Vec<Account>, EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;
        let accounts = self.storage.get_all_accounts(&mut db_tx).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(accounts)
    }

    pub async fn deposit(&mut self, acc_id: u16, tx_id: u32, amount: Decimal4) -> Result<(), EngineError> {
        if !amount.is_positive() {
            return Err(EngineError::AmountIsNotPositive);
        }

        let mut db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Deposit { acc_id, tx_id, amount };
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

        let tx = Transaction::new(tx_id, acc_id, TransactionType::Deposit, amount);
        self.storage.insert_tx(&mut db_tx, &tx).await?;

        let maybe_account = self.storage.get_account(&mut db_tx, acc_id).await?;
        if let Some(old_acc) = maybe_account {
            let mut new_acc = old_acc.clone();
            new_acc.deposit(amount)?;
            self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        } else {
            let mut new_acc = Account::new(acc_id);
            new_acc.deposit(amount)?;
            self.storage.insert_account(&mut db_tx, &new_acc).await?;
        }

        self.storage.insert_operation(&mut db_tx, op_hash).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn withdraw(&mut self, acc_id: u16, tx_id: u32, amount: Decimal4) -> Result<(), EngineError> {
        if !amount.is_positive() {
            return Err(EngineError::AmountIsNotPositive);
        }

        let mut db_tx = self.storage.start_db_tx().await?;

        let operation = Operation::Withdraw { acc_id, tx_id, amount };
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

        let maybe_account = self.storage.get_account(&mut db_tx, acc_id).await?;
        let old_acc = maybe_account.ok_or(EngineError::AccountNotFound)?;
        let mut new_acc = old_acc.clone();
        new_acc.withdraw(amount)?;

        let tx = Transaction::new(tx_id, acc_id, TransactionType::Withdrawal, amount);
        self.storage.insert_tx(&mut db_tx, &tx).await?;
        self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        self.storage.insert_operation(&mut db_tx, op_hash).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn dispute(&mut self, acc_id: u16, tx_id: u32) -> Result<(), EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;

        let maybe_tx = self.storage.get_tx(&mut db_tx, tx_id).await?;
        let old_tx = maybe_tx.ok_or(EngineError::TransactionNotFound)?;
        if old_tx.account_id() != acc_id {
            return Err(EngineError::TransactionIsBoundToAnotherAccount(old_tx.account_id()));
        }

        let maybe_account = self.storage.get_account(&mut db_tx, acc_id).await?;
        let old_acc = maybe_account.ok_or(EngineError::AccountNotFound)?;

        let mut new_tx = old_tx.clone();
        new_tx.set_state(TransactionState::Disputed)?;

        let mut new_acc = old_acc.clone();
        new_acc.dispute(new_tx.amount())?;

        self.storage.update_tx(&mut db_tx, &old_tx, &new_tx).await?;
        self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn resolve(&mut self, acc_id: u16, tx_id: u32) -> Result<(), EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;

        let maybe_tx = self.storage.get_tx(&mut db_tx, tx_id).await?;
        let old_tx = maybe_tx.ok_or(EngineError::TransactionNotFound)?;
        if old_tx.account_id() != acc_id {
            return Err(EngineError::TransactionIsBoundToAnotherAccount(old_tx.account_id()));
        }

        let maybe_account = self.storage.get_account(&mut db_tx, acc_id).await?;
        let old_acc = maybe_account.ok_or(EngineError::AccountNotFound)?;

        let mut new_tx = old_tx.clone();
        new_tx.set_state(TransactionState::Posted)?;

        let mut new_acc = old_acc.clone();
        new_acc.resolve(new_tx.amount())?;

        self.storage.update_tx(&mut db_tx, &old_tx, &new_tx).await?;
        self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }

    pub async fn chargeback(&mut self, acc_id: u16, tx_id: u32) -> Result<(), EngineError> {
        let mut db_tx = self.storage.start_db_tx().await?;

        let maybe_tx = self.storage.get_tx(&mut db_tx, tx_id).await?;
        let old_tx = maybe_tx.ok_or(EngineError::TransactionNotFound)?;
        if old_tx.account_id() != acc_id {
            return Err(EngineError::TransactionIsBoundToAnotherAccount(old_tx.account_id()));
        }

        let maybe_account = self.storage.get_account(&mut db_tx, acc_id).await?;
        let old_acc = maybe_account.ok_or(EngineError::AccountNotFound)?;

        let mut new_tx = old_tx.clone();
        new_tx.set_state(TransactionState::Chargeback)?;

        let mut new_acc = old_acc.clone();
        new_acc.chargeback(new_tx.amount())?;

        self.storage.update_tx(&mut db_tx, &old_tx, &new_tx).await?;
        self.storage.update_account(&mut db_tx, &old_acc, &new_acc).await?;
        self.storage.commit_db_tx(db_tx).await?;
        Ok(())
    }
}

impl<TStorage: Storage> Debug for Engine<TStorage> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Engine").finish()
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum EngineError {
    #[error("account not found")]
    AccountNotFound,

    #[error("transaction not found")]
    TransactionNotFound,

    #[error("account is locked")]
    AccountLocked,

    #[error("insufficient funds")]
    InsufficientFunds,

    #[error("amount is not positive")]
    AmountIsNotPositive,

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
            AccountUpdateError::AmountIsNotPositive => EngineError::AmountIsNotPositive,
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

#[cfg(test)]
mod engine_tests {
    use crate::storage::EchoDbStorage;

    use super::*;

    #[tokio::test]
    async fn deposit_ok() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.deposit(1, 2, Decimal4::from(200)).await, Ok(()));
        assert_eq!(engine.deposit(2, 3, Decimal4::from(300)).await, Ok(()));
    }

    #[tokio::test]
    async fn withdraw_ok() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.withdraw(1, 2, Decimal4::from(50)).await, Ok(()));
    }

    #[tokio::test]
    async fn dispute_ok() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
    }

    #[tokio::test]
    async fn resolve_ok() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.resolve(1, 1).await, Ok(()));
    }

    #[tokio::test]
    async fn chargeback_ok() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.chargeback(1, 1).await, Ok(()));
    }

    #[tokio::test]
    async fn deposit_idempotency() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        let mut db_tx = engine.storage.start_db_tx().await.unwrap();
        let acc = engine.storage.get_account(&mut db_tx, 1).await.unwrap().unwrap();
        assert_eq!(acc.available(), Decimal4::from(100));
    }

    #[tokio::test]
    async fn withdraw_idempotency() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.withdraw(1, 2, Decimal4::from(50)).await, Ok(()));
        assert_eq!(engine.withdraw(1, 2, Decimal4::from(50)).await, Ok(()));
        let mut db_tx = engine.storage.start_db_tx().await.unwrap();
        let acc = engine.storage.get_account(&mut db_tx, 1).await.unwrap().unwrap();
        assert_eq!(acc.available(), Decimal4::from(50));
    }

    #[tokio::test]
    async fn dispute_no_idempotency() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Err(EngineError::ForbiddenTxStateTransition { from: TransactionState::Disputed, to: TransactionState::Disputed }));
        let mut db_tx = engine.storage.start_db_tx().await.unwrap();
        let acc = engine.storage.get_account(&mut db_tx, 1).await.unwrap().unwrap();
        assert_eq!(acc.available(), Decimal4::from(0));
        assert_eq!(acc.held(), Decimal4::from(100));
    }

    #[tokio::test]
    async fn resolve_no_idempotency() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.resolve(1, 1).await, Ok(()));
        assert_eq!(engine.resolve(1, 1).await, Err(EngineError::ForbiddenTxStateTransition { from: TransactionState::Posted, to: TransactionState::Posted }));
        let mut db_tx = engine.storage.start_db_tx().await.unwrap();
        let acc = engine.storage.get_account(&mut db_tx, 1).await.unwrap().unwrap();
        assert_eq!(acc.available(), Decimal4::from(100));
        assert_eq!(acc.held(), Decimal4::from(0));
    }

    #[tokio::test]
    async fn chargeback_no_idempotency() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.chargeback(1, 1).await, Ok(()));
        assert_eq!(engine.chargeback(1, 1).await, Err(EngineError::ForbiddenTxStateTransition { from: TransactionState::Chargeback, to: TransactionState::Chargeback }));
        let mut db_tx = engine.storage.start_db_tx().await.unwrap();
        let acc = engine.storage.get_account(&mut db_tx, 1).await.unwrap().unwrap();
        assert_eq!(acc.available(), Decimal4::from(0));
        assert_eq!(acc.held(), Decimal4::from(0));
        assert!(acc.locked());
    }

    #[tokio::test]
    async fn withdraw_insufficient_funds_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.withdraw(1, 2, Decimal4::from(200)).await, Err(EngineError::InsufficientFunds));
    }

    #[tokio::test]
    async fn deposit_on_locked_account_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.chargeback(1, 1).await, Ok(()));
        assert_eq!(engine.deposit(1, 2, Decimal4::from(100)).await, Err(EngineError::AccountLocked));
    }

    #[tokio::test]
    async fn withdraw_on_locked_account_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.chargeback(1, 1).await, Ok(()));
        assert_eq!(engine.withdraw(1, 2, Decimal4::from(50)).await, Err(EngineError::AccountLocked));
    }

    #[tokio::test]
    async fn deposit_on_nonexistent_account_ok() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
    }

    #[tokio::test]
    async fn withdraw_on_nonexistent_account_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.withdraw(1, 1, Decimal4::from(100)).await, Err(EngineError::AccountNotFound));
    }

    #[tokio::test]
    async fn dispute_on_nonexistent_tx_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.dispute(1, 1).await, Err(EngineError::TransactionNotFound));
    }

    #[tokio::test]
    async fn resolve_on_nonexistent_tx_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.resolve(1, 1).await, Err(EngineError::TransactionNotFound));
    }

    #[tokio::test]
    async fn chargeback_on_nonexistent_tx_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.chargeback(1, 1).await, Err(EngineError::TransactionNotFound));
    }

    #[tokio::test]
    async fn dispute_on_nonexistent_transaction_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.dispute(1, 1).await, Err(EngineError::TransactionNotFound));
    }

    #[tokio::test]
    async fn resolve_on_nonexistent_transaction_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.resolve(1, 1).await, Err(EngineError::TransactionNotFound));
    }

    #[tokio::test]
    async fn chargeback_on_nonexistent_transaction_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.chargeback(1, 1).await, Err(EngineError::TransactionNotFound));
    }

    #[tokio::test]
    async fn dispute_on_transaction_bound_to_another_account_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(2, 1).await, Err(EngineError::TransactionIsBoundToAnotherAccount(1)));
    }

    #[tokio::test]
    async fn resolve_on_transaction_bound_to_another_account_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.resolve(2, 1).await, Err(EngineError::TransactionIsBoundToAnotherAccount(1)));
    }

    #[tokio::test]
    async fn chargeback_on_transaction_bound_to_another_account_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.chargeback(2, 1).await, Err(EngineError::TransactionIsBoundToAnotherAccount(1)));
    }

    #[tokio::test]
    async fn resolve_after_chargeback_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.chargeback(1, 1).await, Ok(()));
        assert_eq!(engine.resolve(1, 1).await, Err(EngineError::ForbiddenTxStateTransition { from: TransactionState::Chargeback, to: TransactionState::Posted }));
    }

    #[tokio::test]
    async fn resolve_after_posted_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.resolve(1, 1).await, Err(EngineError::ForbiddenTxStateTransition { from: TransactionState::Posted, to: TransactionState::Posted }));
    }

    #[tokio::test]
    async fn chargeback_after_posted_err() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.chargeback(1, 1).await, Err(EngineError::ForbiddenTxStateTransition { from: TransactionState::Posted, to: TransactionState::Chargeback }));
    }

    #[tokio::test]
    async fn chargeback_after_resolve_and_second_dispute_ok() {
        let mut engine = Engine::new(EchoDbStorage::new());
        assert_eq!(engine.deposit(1, 1, Decimal4::from(100)).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.resolve(1, 1).await, Ok(()));
        assert_eq!(engine.dispute(1, 1).await, Ok(()));
        assert_eq!(engine.chargeback(1, 1).await, Ok(()));
    }
}
