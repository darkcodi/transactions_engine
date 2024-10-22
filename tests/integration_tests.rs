use cucumber::{given, then, when, World};
use cucumber::gherkin::Step;
use transactions_engine::account::Account;
use transactions_engine::csv_parser::CsvOperation;
use transactions_engine::decimal::Decimal4;
use transactions_engine::engine::{Engine, EngineError};
use transactions_engine::storage::EchoDbStorage;

#[derive(cucumber::World, Debug)]
#[world(init = Self::new)]
struct TransactionsEngineWorld {
    engine: Engine<EchoDbStorage>,
    tx_counter: u32,
    given_acc: Account,
    last_result: Result<(), EngineError>,
    last_deposit_tx: Option<u32>,
    last_disputed_tx: Option<u32>,
    csv_operations: Vec<CsvOperation>,
}

impl TransactionsEngineWorld {
    fn new() -> Self {
        Self {
            engine: Engine::default(),
            tx_counter: 0,
            given_acc: Account::default(),
            last_result: Ok(()),
            last_deposit_tx: None,
            last_disputed_tx: None,
            csv_operations: Vec::new(),
        }
    }
}

#[given("A user has an empty account")]
async fn given_empty_acc(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    user_deposits(world, 1.0).await?;
    user_withdraws(world, 1.0).await?;
    Ok(())
}

#[given(expr = "A user has an account with ${float}")]
async fn given_acc_with_amount(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    user_deposits(world, amount).await?;
    world.given_acc = world.engine.get_account(1).await?.ok_or(anyhow::anyhow!("Account not found"))?;
    Ok(())
}

#[given(expr = "A user has an locked account with ${float}")]
async fn given_locked_acc_with_amount(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    user_deposits(world, amount).await?;
    user_deposits(world, 1.0).await?;
    user_disputes_last_deposit(world).await?;
    last_disputed_tx_is_charged_back(world).await?;
    Ok(())
}

#[given("the CSV file with the following content:")]
async fn given_csv_file(world: &mut TransactionsEngineWorld, step: &Step) -> anyhow::Result<()> {
    let csv_content = step.docstring().unwrap();
    let mut rdr = csv::ReaderBuilder::new()
        .trim(csv::Trim::All).from_reader(csv_content.as_bytes());
    for result in rdr.deserialize() {
        let record: CsvOperation = result?;
        world.csv_operations.push(record);
    }

    Ok(())
}

#[when(expr = "the user deposits ${float}")]
async fn user_deposits(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    world.last_result = world.engine.deposit(1, world.tx_counter, amount.try_into()?).await;
    world.last_deposit_tx = Some(world.tx_counter);
    world.tx_counter += 1;
    Ok(())
}

#[when(expr = "the user withdraws ${float}")]
async fn user_withdraws(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    world.last_result = world.engine.withdraw(1, world.tx_counter, amount.try_into()?).await;
    world.tx_counter += 1;
    Ok(())
}

#[when("the user disputes the last transaction")]
async fn user_disputes_last_tx(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    world.last_result = world.engine.dispute(1, world.tx_counter - 1).await;
    world.last_disputed_tx = Some(world.tx_counter - 1);
    Ok(())
}

#[when("the user disputes the last deposit transaction")]
async fn user_disputes_last_deposit(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    world.last_result = world.engine.dispute(1, world.last_deposit_tx.ok_or(anyhow::anyhow!("No deposit transaction found"))?).await;
    world.last_disputed_tx = world.last_deposit_tx;
    Ok(())
}

#[when("the the last disputed tx is resolved")]
async fn last_disputed_tx_is_resolved(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    world.last_result = world.engine.resolve(1, world.last_disputed_tx.ok_or(anyhow::anyhow!("No disputed transaction found"))?).await;
    Ok(())
}

#[when("the the last disputed tx is charged back")]
async fn last_disputed_tx_is_charged_back(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    world.last_result = world.engine.chargeback(1, world.last_disputed_tx.ok_or(anyhow::anyhow!("No disputed transaction found"))?).await;
    Ok(())
}

#[when("the CSV operations are performed")]
async fn csv_operations_are_performed(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    for csv_op in world.csv_operations.iter() {
        let op = csv_op.clone().try_into()?;
        world.last_result = world.engine.execute_operation(op).await;
    }

    Ok(())
}

#[then(expr = "the user's available balance should be ${float}")]
async fn user_available_balance_is(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    let acc = world.engine.get_account(1).await?.ok_or(anyhow::anyhow!("Account not found"))?;
    assert_eq!(acc.available(), amount.try_into()?);
    Ok(())
}

#[then(expr = "the user's held balance should be ${float}")]
async fn user_held_balance_is(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    let acc = world.engine.get_account(1).await?.ok_or(anyhow::anyhow!("Account not found"))?;
    assert_eq!(acc.held(), amount.try_into()?);
    Ok(())
}

#[then(expr = "the user's total balance should be ${float}")]
async fn user_total_balance_is(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    let acc = world.engine.get_account(1).await?.ok_or(anyhow::anyhow!("Account not found"))?;
    assert_eq!(acc.total(), amount.try_into()?);
    Ok(())
}

#[then(expr = "the user's balance should be ${float}")]
async fn user_balance_is(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    let acc = world.engine.get_account(1).await?.ok_or(anyhow::anyhow!("Account not found"))?;
    let amount: Decimal4 = amount.try_into()?;
    assert_eq!(acc.available(), amount);
    assert_eq!(acc.total(), amount);
    Ok(())
}

#[then(expr = "the user's balance should be unchanged")]
async fn user_balance_is_unchanged(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    let acc = world.engine.get_account(1).await?.ok_or(anyhow::anyhow!("Account not found"))?;
    assert_eq!(acc.total(), world.given_acc.total());
    assert_eq!(acc.available(), world.given_acc.available());
    assert_eq!(acc.held(), world.given_acc.held());
    Ok(())
}

#[then("the last operation should fail")]
async fn last_operation_fails(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    assert!(world.last_result.is_err());
    Ok(())
}

#[then("the last operation should succeed")]
async fn last_operation_succeeds(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    assert!(world.last_result.is_ok());
    Ok(())
}

#[then("the user's account should not be locked")]
async fn user_account_not_locked(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    let acc = world.engine.get_account(1).await?.ok_or(anyhow::anyhow!("Account not found"))?;
    assert!(!acc.locked());
    Ok(())
}

#[then("the user's account should be locked")]
async fn user_account_locked(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    let acc = world.engine.get_account(1).await?.ok_or(anyhow::anyhow!("Account not found"))?;
    assert!(acc.locked());
    Ok(())
}

#[then("the accounts should be as follows:")]
async fn accounts_should_be(world: &mut TransactionsEngineWorld, step: &Step) -> anyhow::Result<()> {
    if step.table.is_none() {
        return Err(anyhow::anyhow!("Table not found"));
    }

    let table = step.table.as_ref().unwrap();
    for row in table.rows.iter().skip(1) { // NOTE: skip header
        let id: u16 = row[0].parse()?;
        let available: Decimal4 = row[1].parse()?;
        let held: Decimal4 = row[2].parse()?;
        let total: Decimal4 = row[3].parse()?;
        let locked: bool = row[4].parse()?;
        let acc = world.engine.get_account(id).await?.ok_or(anyhow::anyhow!("Account not found"))?;
        assert_eq!(acc.available(), available);
        assert_eq!(acc.held(), held);
        assert_eq!(acc.total(), total);
        assert_eq!(acc.locked(), locked);
    }

    Ok(())
}

#[tokio::main]
async fn main() {
    TransactionsEngineWorld::run("tests/features/deposit.feature").await;
    TransactionsEngineWorld::run("tests/features/withdrawal.feature").await;
    TransactionsEngineWorld::run("tests/features/dispute.feature").await;
    TransactionsEngineWorld::run("tests/features/resolve.feature").await;
    TransactionsEngineWorld::run("tests/features/chargeback.feature").await;
    TransactionsEngineWorld::run("tests/features/lock_account.feature").await;
    TransactionsEngineWorld::run("tests/features/csv_input.feature").await;
}
