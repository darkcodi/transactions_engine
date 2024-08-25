use clap::{Arg, Command};

use transactions_engine::csv_parser::{read_csv, write_csv};
use transactions_engine::engine::Engine;
use transactions_engine::storage::EchoDbStorage;

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
