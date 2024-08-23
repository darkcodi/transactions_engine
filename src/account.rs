use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::decimal::Decimal4;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    id: u16,
    available: Decimal4,
    held: Decimal4,
    locked: bool,
    version: u16, // concurrency token
}

impl Account {
    pub fn new(id: u16) -> Self {
        Self {
            id,
            available: Decimal4::zero(),
            held: Decimal4::zero(),
            locked: false,
            version: 0,
        }
    }

    pub fn id(&self) -> u16 {
        self.id
    }

    pub fn available(&self) -> Decimal4 {
        self.available
    }

    pub fn held(&self) -> Decimal4 {
        self.held
    }

    pub fn total(&self) -> Decimal4 {
        self.available + self.held
    }

    pub fn locked(&self) -> bool {
        self.locked
    }

    pub fn version(&self) -> u16 {
        self.version
    }

    pub fn deposit(&mut self, amount: Decimal4) -> Result<(), AccountUpdateError> {
        if self.locked {
            return Err(AccountUpdateError::AccountLocked);
        }
        self.available += amount;
        self.version += 1;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: Decimal4) -> Result<(), AccountUpdateError> {
        if self.locked {
            return Err(AccountUpdateError::AccountLocked);
        }
        if amount > self.available {
            return Err(AccountUpdateError::InsufficientFunds);
        }
        self.available -= amount;
        self.version += 1;
        Ok(())
    }

    pub fn dispute(&mut self, amount: Decimal4) -> Result<(), AccountUpdateError> {
        self.available -= amount;
        self.held += amount;
        self.version += 1;
        Ok(())
    }

    pub fn resolve(&mut self, amount: Decimal4) -> Result<(), AccountUpdateError> {
        self.held -= amount;
        self.available += amount;
        self.version += 1;
        Ok(())
    }

    pub fn chargeback(&mut self, amount: Decimal4) -> Result<(), AccountUpdateError> {
        self.held -= amount;
        self.locked = true;
        self.version += 1;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum AccountUpdateError {
    #[error("account locked")]
    AccountLocked,

    #[error("insufficient funds")]
    InsufficientFunds,
}

#[cfg(test)]
mod account_tests {
    use super::*;

    #[test]
    fn account_deposit_on_locked_account() {
        let mut acc = Account::new(1);
        acc.deposit(4.into()).unwrap();
        acc.dispute(2.into()).unwrap();
        acc.chargeback(2.into()).unwrap();
        let result = acc.deposit(1.into());
        assert_eq!(result, Err(AccountUpdateError::AccountLocked));
    }

    #[test]
    fn account_withdraw_on_locked_account() {
        let mut acc = Account::new(1);
        acc.deposit(4.into()).unwrap();
        acc.dispute(2.into()).unwrap();
        acc.chargeback(2.into()).unwrap();
        let result = acc.withdraw(1.into());
        assert_eq!(result, Err(AccountUpdateError::AccountLocked));
    }

    #[test]
    fn account_withdraw_on_insufficient_funds() {
        let mut acc = Account::new(1);
        acc.deposit(4.into()).unwrap();
        let result = acc.withdraw(5.into());
        assert_eq!(result, Err(AccountUpdateError::InsufficientFunds));
    }
}
