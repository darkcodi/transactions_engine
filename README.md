# Transactions Engine

This is a simple transactions engine that can be used to process simple transactions.

## Table of Contents
- [Features](#features)
- [Usage](#usage)
    - [CLI](#cli)
    - [Library](#library)
- [Assumptions](#assumptions)
- [Design](#design)
    - [Storage trait](#storage-trait)
    - [Multi-threading](#multi-threading)
    - [Idempotency](#idempotency)
    - [Error handling](#error-handling)
    - [Precision](#precision)
- [Testing](#testing)
    - [Unit tests](#unit-tests)
    - [Integration tests](#integration-tests)
    - [Benchmarks](#benchmarks)

## Features

The transactions engine supports the following transaction types:
- **deposit**: deposit an amount to a client account
- **withdraw**: withdraw an amount from a client account
- **dispute**: dispute a transaction
- **resolve**: resolve a dispute
- **chargeback**: chargeback a transaction

Each account has the following fields:
- **available**: the amount of money that is available for the client to withdraw
- **held**: the amount of money that is held in disputes
- **total**: the total amount of money in the account (available + held)
- **locked**: a flag that indicates if the account is locked

When a transaction is disputed, the amount is moved from the available balance to the held balance.  
When a dispute is resolved, the amount is moved back from the held balance to the available balance.  
When a chargeback is made, the amount is removed from the held balance and the account is locked.  
Locked accounts cannot receive new deposits or initiate withdrawals.
Both input and output have the precision of 4 decimal places.

## Usage

### CLI

To run the transactions engine, you can use the following command:

```bash
cargo run -- transactions.csv
```

The transactions file should be a CSV file with the following columns:
- **type**: the type of the transaction (deposit, withdraw, dispute, resolve, chargeback)
- **client**: the client ID / account ID
- **tx**: the transaction ID
- **amount**: the amount of the transaction (only for deposit and withdraw)

Example of a CSV file with transactions:
```csv
type, client, tx, amount
deposit, 1, 1, 1.0
deposit, 2, 2, 2.0
deposit, 1, 3, 2.0
withdrawal, 1, 4, 1.5
withdrawal, 2, 5, 3.0
```

The transactions engine will process the transactions and output the final state of the client accounts to stdout in CSV format.

Example of the output:
```csv
client, available, held, total, locked
1, 1.5, 0.0, 1.5, false
2, 2.0, 0.0, 2.0, false
```

### Library

The transactions engine can also be used as a library in multi-threaded applications.

```rust
let engine = Engine::new(EchoDbStorage::new());
let mut handles = vec![];

// launch 100 concurrent deposit operations (each deposits $3)
for i in 0..100 {
    let engine = engine.clone();
    let handle = tokio::spawn(async move {
        let max_retries = 100;
        // retry are needed because of the concurrent operations for the same account
        for _ in 0..max_retries {
            if engine.deposit(1, i, Decimal4::from(3)).await.is_ok() {
                return Ok(());
            }
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
        }
        Err(EngineError::ConcurrentOperationDetected)
    });
    handles.push(handle);
}

// wait for all deposit operations to complete
for handle in handles {
    assert_eq!(handle.await.unwrap(), Ok(()));
}

// check the final account state
let acc = engine.get_account(1).await.unwrap().unwrap();
assert_eq!(acc.available(), Decimal4::from(300));
```

## Assumptions

- Only the deposit transactions can be disputed.
- Locked account prevents new deposits and withdrawals, but the transactions on the account can still be disputed and resolved / charged back.
- Deposits and withdrawals are always positive (no negative amounts and no zero amounts).
- After resolving a dispute, the transaction can be disputed again (unlike with chargeback, which is final).
- CSV file can contain whitespaces in both the header and the values, the parser will trim them.
- Only deposits can create new accounts, withdrawals can only be made from existing accounts (with a positive balance).
- Decimal rounding strategy is MidpointTowardZero.

## Design

The transactions engine is designed to be _fast_, _correct_, _extensible_, and close to the real-world requirements.

### Storage trait

The `Engine<TStorage>` struct is the main entry point for the transactions engine.
It is generic over the storage type `TStorage`, which is used to store the client accounts.
The storage type must implement the `Storage` trait, which provides the necessary methods to interact with the storage.
  
Currently implemented storage types are:
- `EchoDbStorage`: uses a fast transactional in-memory key-value DB - [EchoDB](https://github.com/surrealdb/echodb)

The trait `Storage` is the main extension point for adding new storage types.
It's designed for easy implementation for different storage backends, including both - SQL databases and NoSQL databases.
You can easily implement the `Storage` trait for Postgres, MySQL, SQLite, or any other database.

### Multi-threading

The transactions engine is designed to be _thread-safe_. It wraps the storage in `Arc`, so you can cheaply clone the engine and use it in multiple threads.  
Also all the methods have _no mutable references_.

NOTE: Doing concurrent operations on the same account can lead to `EngineError::ConcurrentOperationDetected` error, because the engine is designed to be _correct_ and _consistent_.  
Just retry the operation after a short delay.

### Idempotency

The deposit and withdraw operations are _idempotent_. Idempotency key is the transaction ID.

### Error handling

The transactions engine uses the [thiserror](https://crates.io/crates/thiserror) crate for error handling.
The `EngineError` enum represents all possible errors that can occur during the transactions processing.

### Precision

The transactions engine uses the [rust_decimal](https://github.com/paupino/rust-decimal) crate for decimal arithmetic.  
On top of that, there is also a custom `Decimal4` wrapper that provides a fixed-point decimal with 4 decimal places.

## Testing

The transactions engine is covered with unit tests, integration tests and benchmarks.  

### Unit tests

All _unit tests_ are located in the same file as the tested module, e.g. `engine.rs` for the `engine` module.
Almost all modules have unit tests that cover the main functionality.
As for now, there are 60+ unit tests.

### Integration tests

The _integration tests_ are located in the `tests` directory. They test all the main features of the transactions engine.  
The `features` directory contains the feature files that describe the scenarios that are tested, e.g. `deposit.feature`, `withdraw.feature`, etc.  
The tests are written using the [cucumber](https://github.com/cucumber-rs/cucumber) crate, which allows writing tests in a Gherkin-like syntax (Given-When-Then).  
You can run integration tests together with the unit tests using a `cargo test` command.

### Benchmarks

The _benchmarks_ are located in the `benches` directory. They test the performance of the transactions engine.  
The benchmarks are written using the [criterion](https://github.com/bheisler/criterion.rs) crate, which provides a powerful and flexible benchmarking framework.  
You can run benchmarks using a `cargo bench` command.

As for now, `engine/deposit_random` shows **1.8950 µs +- 0.0040 µs** per iteration with `EchoDbStorage`, which is quite fast.

NOTE: The benchmarks are run in a _single-threaded_ mode, so the results may vary in a _multi-threaded_ environment.  

NOTE2: Other storage implementations, like Postgres, will have different performance characteristics. They will be slower, but still fast enough for most use-cases. The DB performance will most likely be the bottleneck.

