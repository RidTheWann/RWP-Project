#![allow(missing_docs)]

use rwp_analysis::{StatisticsConfig, StatisticsError, compute_statistics};

#[test]
fn computes_deterministic_statistics() -> Result<(), Box<dyn std::error::Error>> {
    let values = [10.0, 20.0, 30.0, 40.0, 50.0];
    let stats = compute_statistics(&values, StatisticsConfig::default())?;

    assert_eq!(stats.sample_count, 5);
    assert_eq!(stats.mean_frame_time_ms, 30.0);
    assert_eq!(stats.median_frame_time_ms, 30.0);
    assert_eq!(stats.average_fps, 1000.0 / 30.0);
    assert_eq!(stats.percentiles.p0_1_ms, 10.0);
    assert_eq!(stats.percentiles.p1_ms, 10.0);
    assert_eq!(stats.percentiles.p5_ms, 10.0);
    assert_eq!(stats.percentiles.p10_ms, 10.0);
    assert_eq!(stats.percentiles.p95_ms, 50.0);
    assert_eq!(stats.percentiles.p99_ms, 50.0);
    assert_eq!(stats.percentiles.p99_9_ms, 50.0);
    assert!(!stats.p999_reliable);
    Ok(())
}

#[test]
fn handles_single_sample_without_variance() -> Result<(), Box<dyn std::error::Error>> {
    let stats = compute_statistics(&[8.0], StatisticsConfig::default())?;
    assert_eq!(stats.variance_ms2, None);
    assert_eq!(stats.stdev_ms, None);
    assert_eq!(stats.average_fps, 125.0);
    Ok(())
}

#[test]
fn rejects_empty_and_invalid_samples() {
    assert_eq!(
        compute_statistics(&[], StatisticsConfig::default()).err(),
        Some(StatisticsError::EmptyInput)
    );
    assert_eq!(
        compute_statistics(&[10.0, 0.0], StatisticsConfig::default()).err(),
        Some(StatisticsError::InvalidSample { index: 1 })
    );
    assert_eq!(
        compute_statistics(&[10.0, f64::NAN], StatisticsConfig::default()).err(),
        Some(StatisticsError::InvalidSample { index: 1 })
    );
}

#[test]
fn worst_fraction_uses_at_least_one_frame() -> Result<(), Box<dyn std::error::Error>> {
    let stats = compute_statistics(&[10.0, 20.0, 30.0], StatisticsConfig::default())?;
    assert_eq!(stats.low_fps.mean_fps_of_worst_1pct_frames, 1000.0 / 30.0);
    assert_eq!(stats.low_fps.mean_fps_of_worst_0_1pct_frames, 1000.0 / 30.0);
    Ok(())
}
