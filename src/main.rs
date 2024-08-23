use std::io;

use anyhow::Context;
use clap::{Arg, Command};

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
async fn main() -> anyhow::Result<()> {
    let matches = Command::new("Transactions Engine")
        .version("0.1.0")
        .about("A simple transactions engine")
        .arg(
            Arg::new("filepath")
                .help("The path to the CSV file to process")
                .required(true)
                .index(1),
        )
        .get_matches();

    let filepath: &String = matches.get_one("filepath").unwrap();

    let mut engine = Engine::new(EchoDbStorage::new());
    read_csv(filepath, &mut engine).await?;
    write_csv(&mut engine).await?;

    Ok(())
}

async fn read_csv(filepath: &String, engine: &mut Engine<EchoDbStorage>) -> anyhow::Result<u64> {
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

async fn write_csv(engine: &mut Engine<EchoDbStorage>) -> anyhow::Result<()> {
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
