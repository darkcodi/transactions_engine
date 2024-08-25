use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::decimal::Decimal4;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
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
        if !amount.is_positive() {
            return Err(AccountUpdateError::AmountIsNotPositive);
        }
        if self.locked {
            return Err(AccountUpdateError::AccountLocked);
        }
        self.available += amount;
        self.version += 1;
        Ok(())
    }

    pub fn withdraw(&mut self, amount: Decimal4) -> Result<(), AccountUpdateError> {
        if !amount.is_positive() {
            return Err(AccountUpdateError::AmountIsNotPositive);
        }
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
        if !amount.is_positive() {
            return Err(AccountUpdateError::AmountIsNotPositive);
        }
        self.available -= amount;
        self.held += amount;
        self.version += 1;
        Ok(())
    }

    pub fn resolve(&mut self, amount: Decimal4) -> Result<(), AccountUpdateError> {
        if !amount.is_positive() {
            return Err(AccountUpdateError::AmountIsNotPositive);
        }
        self.held -= amount;
        self.available += amount;
        self.version += 1;
        Ok(())
    }

    pub fn chargeback(&mut self, amount: Decimal4) -> Result<(), AccountUpdateError> {
        if !amount.is_positive() {
            return Err(AccountUpdateError::AmountIsNotPositive);
        }
        self.held -= amount;
        self.locked = true;
        self.version += 1;
        Ok(())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AccountUpdateError {
    #[error("account locked")]
    AccountLocked,

    #[error("insufficient funds")]
    InsufficientFunds,

    #[error("amount is not positive")]
    AmountIsNotPositive,
}

#[cfg(test)]
mod account_tests {
    use super::*;

    #[test]
    fn account_deposit_on_locked_account_err() {
        let mut acc = Account::new(1);
        acc.deposit(4.into()).unwrap();
        acc.dispute(2.into()).unwrap();
        acc.chargeback(2.into()).unwrap();
        assert_eq!(acc.deposit(1.into()), Err(AccountUpdateError::AccountLocked));
    }

    #[test]
    fn account_withdraw_on_locked_account_err() {
        let mut acc = Account::new(1);
        acc.deposit(4.into()).unwrap();
        acc.dispute(2.into()).unwrap();
        acc.chargeback(2.into()).unwrap();
        assert_eq!(acc.withdraw(1.into()), Err(AccountUpdateError::AccountLocked));
    }

    #[test]
    fn account_withdraw_on_insufficient_funds_err() {
        let mut acc = Account::new(1);
        acc.deposit(4.into()).unwrap();
        assert_eq!(acc.withdraw(5.into()), Err(AccountUpdateError::InsufficientFunds));
    }

    #[test]
    fn account_deposit_ok() {
        let mut acc = Account::new(1);
        acc.deposit(4.into()).unwrap();
        assert_eq!(acc.available(), 4.into());
    }

    #[test]
    fn account_withdraw_ok() {
        let mut acc = Account::new(1);
        acc.deposit(5.into()).unwrap();
        acc.withdraw(2.into()).unwrap();
        assert_eq!(acc.available(), 3.into());
    }

    #[test]
    fn account_dispute_ok() {
        let mut acc = Account::new(1);
        acc.deposit(5.into()).unwrap();
        acc.dispute(2.into()).unwrap();
        assert_eq!(acc.available(), 3.into());
        assert_eq!(acc.held(), 2.into());
    }

    #[test]
    fn account_resolve_ok() {
        let mut acc = Account::new(1);
        acc.deposit(5.into()).unwrap();
        acc.dispute(2.into()).unwrap();
        acc.resolve(2.into()).unwrap();
        assert_eq!(acc.available(), 5.into());
        assert_eq!(acc.held(), 0.into());
    }

    #[test]
    fn account_chargeback_ok() {
        let mut acc = Account::new(1);
        acc.deposit(5.into()).unwrap();
        acc.dispute(2.into()).unwrap();
        acc.chargeback(2.into()).unwrap();
        assert_eq!(acc.available(), 3.into());
        assert_eq!(acc.held(), 0.into());
        assert_eq!(acc.locked(), true);
    }

    #[test]
    fn account_version_incremented_on_deposit() {
        let mut acc = Account::new(1);
        acc.deposit(5.into()).unwrap();
        assert_eq!(acc.version(), 1);
    }

    #[test]
    fn account_version_incremented_on_withdraw() {
        let mut acc = Account::new(1);
        acc.deposit(5.into()).unwrap();
        acc.withdraw(2.into()).unwrap();
        assert_eq!(acc.version(), 2);
    }

    #[test]
    fn account_version_incremented_on_dispute() {
        let mut acc = Account::new(1);
        acc.deposit(5.into()).unwrap();
        acc.dispute(5.into()).unwrap();
        assert_eq!(acc.version(), 2);
    }

    #[test]
    fn account_version_incremented_on_resolve() {
        let mut acc = Account::new(1);
        acc.deposit(5.into()).unwrap();
        acc.dispute(5.into()).unwrap();
        acc.resolve(5.into()).unwrap();
        assert_eq!(acc.version(), 3);
    }

    #[test]
    fn account_version_incremented_on_chargeback() {
        let mut acc = Account::new(1);
        acc.deposit(5.into()).unwrap();
        acc.dispute(5.into()).unwrap();
        acc.chargeback(5.into()).unwrap();
        assert_eq!(acc.version(), 3);
    }

    #[test]
    fn account_deposit_amount_not_positive_err() {
        let mut acc = Account::new(1);
        assert_eq!(acc.deposit(Decimal4::zero()), Err(AccountUpdateError::AmountIsNotPositive));
    }

    #[test]
    fn account_withdraw_amount_not_positive_err() {
        let mut acc = Account::new(1);
        assert_eq!(acc.withdraw(Decimal4::zero()), Err(AccountUpdateError::AmountIsNotPositive));
    }

    #[test]
    fn account_dispute_amount_not_positive_err() {
        let mut acc = Account::new(1);
        assert_eq!(acc.dispute(Decimal4::zero()), Err(AccountUpdateError::AmountIsNotPositive));
    }

    #[test]
    fn account_resolve_amount_not_positive_err() {
        let mut acc = Account::new(1);
        assert_eq!(acc.resolve(Decimal4::zero()), Err(AccountUpdateError::AmountIsNotPositive));
    }

    #[test]
    fn account_chargeback_amount_not_positive_err() {
        let mut acc = Account::new(1);
        assert_eq!(acc.chargeback(Decimal4::zero()), Err(AccountUpdateError::AmountIsNotPositive));
    }
}
