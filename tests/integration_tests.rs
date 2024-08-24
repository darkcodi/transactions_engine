use cucumber::{given, then, when, World};
use transactions_engine::decimal::Decimal4;
use transactions_engine::engine::Engine;
use transactions_engine::storage::EchoDbStorage;

#[derive(cucumber::World, Debug)]
#[world(init = Self::new)]
struct TransactionsEngineWorld {
    engine: Engine<EchoDbStorage>,
    tx_counter: u32,
}

impl TransactionsEngineWorld {
    fn new() -> Self {
        Self {
            engine: Engine::new(EchoDbStorage::new()),
            tx_counter: 0,
        }
    }
}

#[given("A user has an account")]
async fn user_has_an_account(world: &mut TransactionsEngineWorld) -> anyhow::Result<()> {
    let _ = world.engine.deposit(1, world.tx_counter, Decimal4::zero()).await;
    world.tx_counter += 1;
    Ok(())
}

#[when(expr = "the user deposits ${float}")]
async fn user_deposits(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    let _ = world.engine.deposit(1, world.tx_counter, amount.try_into()?).await;
    world.tx_counter += 1;
    Ok(())
}

#[then(expr = "the user's available balance should be ${float}")]
async fn user_available_balance_is(world: &mut TransactionsEngineWorld, amount: f32) -> anyhow::Result<()> {
    let maybe_account = world.engine.get_account(1).await?;
    assert!(maybe_account.is_some());
    let account = maybe_account.unwrap();
    assert_eq!(account.available(), amount.try_into()?);
    Ok(())
}

#[tokio::main]
async fn main() {
    TransactionsEngineWorld::run("tests/features/deposit.feature").await;
}
