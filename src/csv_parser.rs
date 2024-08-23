use serde::{Deserialize, Serialize};
use thiserror::Error;
use crate::decimal::Decimal4;
use crate::engine::Operation;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsvOperation {
    #[serde(rename = "type")]
    op_type: Option<String>,
    client: Option<u16>,
    tx: Option<u32>,
    amount: Option<Decimal4>,
}

impl TryInto<Operation> for CsvOperation {
    type Error = CsvParseError;

    fn try_into(self) -> Result<Operation, Self::Error> {
        let op_type = self.op_type.ok_or(CsvParseError::MissingField("type".to_string()))?;
        let client = self.client.ok_or(CsvParseError::MissingField("client".to_string()))?;
        let tx = self.tx.ok_or(CsvParseError::MissingField("tx".to_string()))?;
        let maybe_amount = self.amount;

        if (op_type == "deposit" || op_type == "withdraw") && maybe_amount.is_none() {
            return Err(CsvParseError::MissingField("amount".to_string()));
        }

        if let Some(amount) = maybe_amount {
            if amount < Decimal4::zero() {
                return Err(CsvParseError::NegativeAmount);
            }
        }

        let op_type = match op_type.as_str() {
            "deposit" => Operation::Deposit { acc_id: client, tx_id: tx, amount: maybe_amount.unwrap() },
            "withdrawal" => Operation::Withdraw { acc_id: client, tx_id: tx, amount: maybe_amount.unwrap() },
            "dispute" => Operation::Dispute { acc_id: client, tx_id: tx },
            "resolve" => Operation::Resolve { acc_id: client, tx_id: tx },
            "chargeback" => Operation::Chargeback { acc_id: client, tx_id: tx },
            _ => return Err(CsvParseError::InvalidType),
        };

        Ok(op_type)
    }
}

#[derive(Debug, Error, PartialEq)]
pub enum CsvParseError {
    #[error("missing field: {0}")]
    MissingField(String),

    #[error("invalid operation type")]
    InvalidType,

    #[error("amount cannot be negative")]
    NegativeAmount,
}
