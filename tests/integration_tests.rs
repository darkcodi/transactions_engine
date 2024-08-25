use cucumber::{given, then, when, World};
use transactions_engine::account::Account;
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
    last_deposit_tx: u32,
    last_disputed_tx: u32,
}

impl TransactionsEngineWorld {
    fn new() -> Self {
        Self {
            engine: Engine::default(),
            tx_counter: 0,
            given_acc: Account::default(),
            last_result: Ok(()),
            last_deposit_tx: 0,
            last_disputed_tx: 0,
        }
    }
}

#[given("A user has an empty account")]
async fn given_empty_acc(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    given_acc_with_amount(world, 0.0).await
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

#[when(expr = "the user deposits ${float}")]
async fn user_deposits(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    world.last_result = world.engine.deposit(1, world.tx_counter, amount.try_into()?).await;
    world.last_deposit_tx = world.tx_counter;
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
    world.last_disputed_tx = world.tx_counter - 1;
    Ok(())
}

#[when("the user disputes the last deposit transaction")]
async fn user_disputes_last_deposit(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    world.last_result = world.engine.dispute(1, world.last_deposit_tx).await;
    world.last_disputed_tx = world.last_deposit_tx;
    Ok(())
}

#[when("the the last disputed tx is resolved")]
async fn last_disputed_tx_is_resolved(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    world.last_result = world.engine.resolve(1, world.last_disputed_tx).await;
    Ok(())
}

#[when("the the last disputed tx is charged back")]
async fn last_disputed_tx_is_charged_back(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    world.last_result = world.engine.chargeback(1, world.last_disputed_tx).await;
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

#[tokio::main]
async fn main() {
    TransactionsEngineWorld::run("tests/features/deposit.feature").await;
    TransactionsEngineWorld::run("tests/features/withdrawal.feature").await;
    TransactionsEngineWorld::run("tests/features/dispute.feature").await;
    TransactionsEngineWorld::run("tests/features/resolve.feature").await;
    TransactionsEngineWorld::run("tests/features/chargeback.feature").await;
    TransactionsEngineWorld::run("tests/features/lock_account.feature").await;
}
