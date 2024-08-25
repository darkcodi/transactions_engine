use cucumber::{given, then, when, World};
use transactions_engine::account::Account;
use transactions_engine::decimal::Decimal4;
use transactions_engine::engine::Engine;
use transactions_engine::storage::EchoDbStorage;

#[derive(cucumber::World, Debug)]
#[world(init = Self::new)]
struct TransactionsEngineWorld {
    engine: Engine<EchoDbStorage>,
    tx_counter: u32,
    given_acc: Option<Account>,
}

impl TransactionsEngineWorld {
    fn new() -> Self {
        Self {
            engine: Engine::new(EchoDbStorage::new()),
            tx_counter: 0,
            given_acc: None,
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
    world.given_acc = Some(world.engine.get_account(1).await?.ok_or(anyhow::anyhow!("Account not found"))?);
    Ok(())
}

#[when(expr = "the user deposits ${float}")]
async fn user_deposits(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    let _ = world.engine.deposit(1, world.tx_counter, amount.try_into()?).await;
    world.tx_counter += 1;
    Ok(())
}

#[when(expr = "the user withdraws ${float}")]
async fn user_withdraws(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    let _ = world.engine.withdraw(1, world.tx_counter, amount.try_into()?).await;
    world.tx_counter += 1;
    Ok(())
}

#[when("the user disputes the last transaction")]
async fn user_disputes_last_tx(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    let _ = world.engine.dispute(1, world.tx_counter - 1).await;
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
    assert_eq!(acc.total(), world.given_acc.as_ref().unwrap().total());
    assert_eq!(acc.available(), world.given_acc.as_ref().unwrap().available());
    assert_eq!(acc.held(), world.given_acc.as_ref().unwrap().held());
    Ok(())
}

#[tokio::main]
async fn main() {
    TransactionsEngineWorld::run("tests/features/deposit.feature").await;
    TransactionsEngineWorld::run("tests/features/withdrawal.feature").await;
    TransactionsEngineWorld::run("tests/features/dispute.feature").await;
}
