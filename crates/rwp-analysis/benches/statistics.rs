#![allow(missing_docs)]

use criterion::{Criterion, criterion_group, criterion_main};
use rwp_analysis::{StatisticsConfig, compute_statistics};
use std::hint::black_box;

fn benchmark_statistics(c: &mut Criterion) {
    let samples_10k = (0..10_000)
        .map(|index| 8.0 + (index % 37) as f64 * 0.1)
        .collect::<Vec<_>>();
    c.bench_function("statistics_10k_frames", |benchmark| {
        benchmark.iter(|| compute_statistics(black_box(&samples_10k), StatisticsConfig::default()));
    });

    let samples_1m = (0..1_000_000)
        .map(|index| 8.0 + (index % 37) as f64 * 0.1)
        .collect::<Vec<_>>();
    c.bench_function("statistics_1m_frames", |benchmark| {
        benchmark.iter(|| compute_statistics(black_box(&samples_1m), StatisticsConfig::default()));
    });
}

criterion_group!(benches, benchmark_statistics);
criterion_main!(benches);
