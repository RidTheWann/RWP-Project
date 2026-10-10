#![allow(missing_docs)]

use proptest::prelude::*;
use rwp_analysis::{StatisticsConfig, compute_statistics};

proptest! {
    #[test]
    fn nearest_rank_percentiles_match_naive_reference(
        raw in prop::collection::vec(1.0_f64..500.0_f64, 1..200)
    ) {
        let stats = compute_statistics(&raw, StatisticsConfig::default())
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let actual = stats.percentiles;
        let expected = [
            naive_nearest_rank(&raw, 0.001),
            naive_nearest_rank(&raw, 0.01),
            naive_nearest_rank(&raw, 0.05),
            naive_nearest_rank(&raw, 0.10),
            naive_nearest_rank(&raw, 0.50),
            naive_nearest_rank(&raw, 0.95),
            naive_nearest_rank(&raw, 0.99),
            naive_nearest_rank(&raw, 0.999),
        ];
        prop_assert_eq!(
            [
                actual.p0_1_ms,
                actual.p1_ms,
                actual.p5_ms,
                actual.p10_ms,
                actual.p50_ms,
                actual.p95_ms,
                actual.p99_ms,
                actual.p99_9_ms,
            ],
            expected,
        );
    }

    #[test]
    fn nearest_rank_percentiles_are_monotonic(
        mut raw in prop::collection::vec(1.0_f64..500.0_f64, 1..200)
    ) {
        raw.sort_by(|a, b| a.total_cmp(b));
        let stats = compute_statistics(&raw, StatisticsConfig::default())
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let p = stats.percentiles;
        prop_assert!(p.p0_1_ms <= p.p1_ms);
        prop_assert!(p.p1_ms <= p.p5_ms);
        prop_assert!(p.p5_ms <= p.p10_ms);
        prop_assert!(p.p10_ms <= p.p50_ms);
        prop_assert!(p.p50_ms <= p.p95_ms);
        prop_assert!(p.p95_ms <= p.p99_ms);
        prop_assert!(p.p99_ms <= p.p99_9_ms);
    }

    #[test]
    fn nearest_rank_percentiles_are_observed_values(
        raw in prop::collection::vec(1.0_f64..500.0_f64, 1..200)
    ) {
        let stats = compute_statistics(&raw, StatisticsConfig::default())
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let contains = |value: f64| raw.contains(&value);
        prop_assert!(contains(stats.percentiles.p0_1_ms));
        prop_assert!(contains(stats.percentiles.p1_ms));
        prop_assert!(contains(stats.percentiles.p5_ms));
        prop_assert!(contains(stats.percentiles.p10_ms));
        prop_assert!(contains(stats.percentiles.p50_ms));
        prop_assert!(contains(stats.percentiles.p95_ms));
        prop_assert!(contains(stats.percentiles.p99_ms));
        prop_assert!(contains(stats.percentiles.p99_9_ms));
    }

    #[test]
    fn duplicating_the_input_preserves_nearest_rank_values(
        raw in prop::collection::vec(1.0_f64..500.0_f64, 1..100)
    ) {
        let mut duplicated = raw.clone();
        duplicated.extend_from_slice(&raw);
        let left = compute_statistics(&raw, StatisticsConfig::default())
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let right = compute_statistics(&duplicated, StatisticsConfig::default())
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        prop_assert_eq!(left.percentiles.p0_1_ms, right.percentiles.p0_1_ms);
        prop_assert_eq!(left.percentiles.p1_ms, right.percentiles.p1_ms);
        prop_assert_eq!(left.percentiles.p5_ms, right.percentiles.p5_ms);
        prop_assert_eq!(left.percentiles.p10_ms, right.percentiles.p10_ms);
        prop_assert_eq!(left.percentiles.p50_ms, right.percentiles.p50_ms);
        prop_assert_eq!(left.percentiles.p95_ms, right.percentiles.p95_ms);
        prop_assert_eq!(left.percentiles.p99_ms, right.percentiles.p99_ms);
        prop_assert_eq!(left.percentiles.p99_9_ms, right.percentiles.p99_9_ms);
    }
}

fn naive_nearest_rank(values: &[f64], percentile: f64) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let rank = ((sorted.len() as f64 * percentile).ceil() as usize).max(1);
    sorted[rank - 1]
}
