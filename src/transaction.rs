use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::decimal::Decimal4;

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum TransactionType {
    Deposit = 0,
    Withdrawal = 1,
}

#[derive(Debug, Clone, PartialEq, Eq, Copy, Serialize, Deserialize)]
pub enum TransactionState {
    Posted = 0,
    Disputed = 1,
    Resolved = 2,
    Chargeback = 3,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    id: u32,
    account_id: u16,
    tx_type: TransactionType,
    amount: Decimal4,
    state: TransactionState,
    version: u16, // concurrency token
}

impl Transaction {
    pub fn new(id: u32, account_id: u16, tx_type: TransactionType, amount: Decimal4) -> Self {
        Self {
            id,
            account_id,
            tx_type,
            amount,
            state: TransactionState::Posted,
            version: 0,
        }
    }

    pub fn id(&self) -> u32 {
        self.id
    }

    pub fn account_id(&self) -> u16 {
        self.account_id
    }

    pub fn tx_type(&self) -> TransactionType {
        self.tx_type
    }

    pub fn amount(&self) -> Decimal4 {
        self.amount
    }

    pub fn state(&self) -> TransactionState {
        self.state
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn set_state(&mut self, new_state: TransactionState) -> Result<(), TxUpdateError> {
        if self.state == new_state {
            return Ok(());
        }
        if self.tx_type == TransactionType::Withdrawal {
            return Err(TxUpdateError::InvalidTxType);
        }

        match (self.state, new_state) {
            (TransactionState::Posted, TransactionState::Disputed) => {
                self.state = TransactionState::Disputed;
                self.version += 1;
                Ok(())
            }
            (TransactionState::Disputed, TransactionState::Resolved) => {
                self.state = TransactionState::Resolved;
                self.version += 1;
                Ok(())
            }
            (TransactionState::Disputed, TransactionState::Chargeback) => {
                self.state = TransactionState::Chargeback;
                self.version += 1;
                Ok(())
            }
            _ => Err(TxUpdateError::ForbiddenTxStateTransition {
                from: self.state,
                to: new_state,
            }),
        }
    }
}

#[derive(Error, Debug, PartialEq, Eq)]
pub enum TxUpdateError {
    #[error("invalid transaction type: only deposits can be disputed/resolved/chargebacked")]
    InvalidTxType,

    #[error("forbidden state transition: {from:?} -> {to:?}")]
    ForbiddenTxStateTransition { from: TransactionState, to: TransactionState },
}

#[cfg(test)]
mod transaction_tests {
    use super::*;

    #[test]
    fn create_transaction() {
        let tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.id(), 1);
        assert_eq!(tx.account_id(), 1);
        assert_eq!(tx.tx_type(), TransactionType::Deposit);
        assert_eq!(tx.amount().to_string(), "100.0000");
        assert_eq!(tx.state(), TransactionState::Posted);
        assert_eq!(tx.version(), 0);
    }

    #[test]
    fn resolve_after_dispute_ok() {
        let mut tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.set_state(TransactionState::Disputed), Ok(()));
        assert_eq!(tx.set_state(TransactionState::Resolved), Ok(()));
        assert_eq!(tx.state(), TransactionState::Resolved);
        assert_eq!(tx.version(), 2);
    }

    #[test]
    fn resolve_after_chargeback_err() {
        let mut tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.set_state(TransactionState::Disputed), Ok(()));
        assert_eq!(tx.set_state(TransactionState::Chargeback), Ok(()));
        assert_eq!(tx.set_state(TransactionState::Resolved), Err(TxUpdateError::ForbiddenTxStateTransition { from: TransactionState::Chargeback, to: TransactionState::Resolved }));
        assert_eq!(tx.state(), TransactionState::Chargeback);
        assert_eq!(tx.version(), 2);
    }

    #[test]
    fn resolve_after_posted_err() {
        let mut tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.set_state(TransactionState::Resolved), Err(TxUpdateError::ForbiddenTxStateTransition { from: TransactionState::Posted, to: TransactionState::Resolved }));
        assert_eq!(tx.state(), TransactionState::Posted);
        assert_eq!(tx.version(), 0);
    }

    #[test]
    fn dispute_after_posted_ok() {
        let mut tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.set_state(TransactionState::Disputed), Ok(()));
        assert_eq!(tx.state(), TransactionState::Disputed);
        assert_eq!(tx.version(), 1);
    }

    #[test]
    fn dispute_after_resolved_err() {
        let mut tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.set_state(TransactionState::Disputed), Ok(()));
        assert_eq!(tx.set_state(TransactionState::Resolved), Ok(()));
        assert_eq!(tx.set_state(TransactionState::Disputed), Err(TxUpdateError::ForbiddenTxStateTransition { from: TransactionState::Resolved, to: TransactionState::Disputed }));
        assert_eq!(tx.state(), TransactionState::Resolved);
        assert_eq!(tx.version(), 2);
    }

    #[test]
    fn chargeback_after_posted_err() {
        let mut tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.set_state(TransactionState::Chargeback), Err(TxUpdateError::ForbiddenTxStateTransition { from: TransactionState::Posted, to: TransactionState::Chargeback }));
        assert_eq!(tx.state(), TransactionState::Posted);
        assert_eq!(tx.version(), 0);
    }

    #[test]
    fn chargeback_after_resolved_err() {
        let mut tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.set_state(TransactionState::Disputed), Ok(()));
        assert_eq!(tx.set_state(TransactionState::Resolved), Ok(()));
        assert_eq!(tx.set_state(TransactionState::Chargeback), Err(TxUpdateError::ForbiddenTxStateTransition { from: TransactionState::Resolved, to: TransactionState::Chargeback }));
        assert_eq!(tx.state(), TransactionState::Resolved);
        assert_eq!(tx.version(), 2);
    }

    #[test]
    fn dispute_after_chargeback_err() {
        let mut tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.set_state(TransactionState::Disputed), Ok(()));
        assert_eq!(tx.set_state(TransactionState::Chargeback), Ok(()));
        assert_eq!(tx.set_state(TransactionState::Disputed), Err(TxUpdateError::ForbiddenTxStateTransition { from: TransactionState::Chargeback, to: TransactionState::Disputed }));
        assert_eq!(tx.state(), TransactionState::Chargeback);
        assert_eq!(tx.version(), 2);
    }
}
