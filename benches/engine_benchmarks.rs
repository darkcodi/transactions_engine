use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use transactions_engine::engine::Engine;

fn engine_benchmark(c: &mut Criterion) {
    let rt = Runtime::new().unwrap(); // single-threaded Tokio runtime

    let mut group = c.benchmark_group("Engine");

    group.bench_function("deposit_static", |b| {
        b.iter(|| {
            rt.block_on(async {
                black_box(Engine::default().deposit(1, 1, 3.into()).await)
            })
        });
    });

    group.bench_function("deposit_random", |b| {
        b.iter(|| {
            rt.block_on(async {
                let acc = fastrand::u16(..);
                let tx = fastrand::u32(..);
                let amount = fastrand::u32(1..10);
                black_box(Engine::default().deposit(acc, tx, amount.into()).await)
            })
        });
    });

    group.finish();
}

criterion_group!(benches, engine_benchmark);
criterion_main!(benches);