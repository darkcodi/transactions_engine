use std::io;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::account::Account;
use crate::decimal::Decimal4;
use crate::engine::{Engine, Operation};
use crate::storage::EchoDbStorage;

#[derive(Debug, Clone, Deserialize)]
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

#[derive(Debug, Clone, Serialize)]
pub struct CsvAccount {
    client: u16,
    available: Decimal4,
    held: Decimal4,
    total: Decimal4,
    locked: bool,
}

impl From<Account> for CsvAccount {
    fn from(value: Account) -> Self {
        Self {
            client: value.id(),
            available: value.available(),
            held: value.held(),
            total: value.total(),
            locked: value.locked(),
        }
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

pub async fn read_csv(filepath: &String, engine: &mut Engine<EchoDbStorage>) -> anyhow::Result<u64> {
    let mut csv_reader = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(filepath)
        .context("error reading csv file")?;

    let mut counter = 0;

    for deserialize_result in csv_reader.deserialize() {
        if deserialize_result.is_err() {
            // eprintln!("csv error: {:?}", deserialize_result.err());
            continue;
        }
        let csv_operation: CsvOperation = deserialize_result.unwrap();
        let parse_result: Result<Operation, CsvParseError> = csv_operation.try_into();
        if parse_result.is_err() {
            // eprintln!("parse error: {:?}", parse_result.err());
            continue;
        }

        let operation = parse_result.unwrap();
        let execution_result = engine.execute_operation(operation).await;
        if execution_result.is_err() {
            // eprintln!("execution error: {:?}", execution_result.err());
            continue;
        }

        counter += 1;
    }

    Ok(counter)
}

pub async fn write_csv(engine: &mut Engine<EchoDbStorage>) -> anyhow::Result<()> {
    let all_accounts = engine.get_all_accounts().await
        .context("error getting all accounts")?;

    let mut writer = csv::Writer::from_writer(io::stdout());

    for account in all_accounts {
        let csv_account: CsvAccount = account.into();
        writer.serialize(csv_account).context("error writing csv")?;
    }

    writer.flush().context("error flushing csv")?;

    Ok(())
}
