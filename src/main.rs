use std::{env, io};
use crate::csv_parser::{CsvAccount, CsvOperation, CsvParseError};
use crate::engine::{Engine, Operation};
use crate::storage::EchoDbStorage;

mod decimal;
mod transaction;
mod engine;
mod storage;
mod account;
mod csv_parser;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() <= 1 {
        eprintln!("no arguments provided");
        return;
    }
    let file_path = &args[1];

    let mut csv_reader_result = csv::ReaderBuilder::new()
        .trim(csv::Trim::All)
        .from_path(file_path);
    if csv_reader_result.is_err() {
        eprintln!("error reading csv file: {:?}", csv_reader_result.err());
        return;
    }
    let mut csv_reader = csv_reader_result.unwrap();
    let mut engine = Engine::new(EchoDbStorage::new());
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

    let all_accounts = engine.get_all_accounts().await;
    if all_accounts.is_err() {
        eprintln!("error getting all accounts: {:?}", all_accounts.err());
        return;
    }

    let mut writer = csv::Writer::from_writer(io::stdout());

    for account in all_accounts.unwrap() {
        let csv_account: CsvAccount = account.into();
        if writer.serialize(csv_account).is_err() {
            eprintln!("error writing csv: {:?}", writer.into_inner().err());
            return;
        }
    }
}
