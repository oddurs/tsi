//! Optimizer benchmarks.
//!
//! ```sh
//! cargo bench --no-default-features
//! ```
//!
//! Baselines are recorded in cairn item #34.

use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;
use tsiolkovsky::prelude::*;

fn engine(name: &str) -> Engine {
    EngineDatabase::builtin()
        .get(name)
        .expect("engine in the built-in database")
        .clone()
}

fn problem(engines: &[&str], stages: u32, target: f64) -> Problem {
    Problem::builder()
        .payload(Mass::tonnes(5.0))
        .target(Velocity::mps(target))
        .engines(engines.iter().map(|&e| engine(e)))
        .stages(stages)
        .build()
        .expect("a valid problem")
}

fn optimizers(c: &mut Criterion) {
    let two_stage = problem(&["raptor-2"], 2, 9_400.0);
    let three_stage = problem(&["raptor-2"], 3, 12_000.0);
    let multi = problem(&["raptor-2", "merlin-1d", "rs-25"], 2, 9_400.0);

    let mut group = c.benchmark_group("optimize");
    group.sample_size(20);
    group.bench_function("analytical, 2 stages, 1 engine", |b| {
        b.iter(|| AnalyticalOptimizer.optimize(black_box(&two_stage)))
    });
    group.bench_function("analytical, 2 stages, 3 engines", |b| {
        b.iter(|| AnalyticalOptimizer.optimize(black_box(&multi)))
    });
    group.bench_function("brute force, 3 stages, 1 engine", |b| {
        b.iter(|| BruteForceOptimizer::default().optimize(black_box(&three_stage)))
    });
    group.bench_function("brute force, 2 stages, 3 engines", |b| {
        b.iter(|| BruteForceOptimizer::default().optimize(black_box(&multi)))
    });
    group.finish();
}

fn monte_carlo(c: &mut Criterion) {
    let design = AnalyticalOptimizer
        .optimize(&problem(&["raptor-2"], 2, 9_400.0))
        .expect("a feasible design");
    let runner = MonteCarloRunner::new(Uncertainty::default()).with_seed(1);

    let mut group = c.benchmark_group("monte carlo");
    group.sample_size(20);
    group.bench_function("10,000 builds", |b| {
        b.iter(|| runner.run_design(black_box(&design), 10_000))
    });
    group.finish();
}

criterion_group!(benches, optimizers, monte_carlo);
criterion_main!(benches);
