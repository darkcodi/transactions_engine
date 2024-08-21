use crate::decimal::Decimal4;

#[derive(Debug, Clone, PartialEq, Eq, Copy)]
pub enum TransactionType {
    Deposit = 0,
    Withdrawal = 1,
}

#[derive(Debug, Clone)]
pub struct Transaction {
    id: u32,
    account_id: u16,
    tx_type: TransactionType,
    amount: Decimal4,
}

impl Transaction {
    pub fn new(id: u32, account_id: u16, tx_type: TransactionType, amount: Decimal4) -> Self {
        Self {
            id,
            account_id,
            tx_type,
            amount,
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
}

#[cfg(test)]
mod transaction_tests {
    use super::*;

    #[test]
    fn test_transaction() {
        let tx = Transaction::new(1, 1, TransactionType::Deposit, Decimal4::from(100));
        assert_eq!(tx.id(), 1);
        assert_eq!(tx.account_id(), 1);
        assert_eq!(tx.tx_type(), TransactionType::Deposit);
        assert_eq!(tx.amount().to_string(), "100.0000");
    }
}
