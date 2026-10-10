#![allow(missing_docs)]

use rwp_analysis::{StatisticsConfig, StutterConfig, compute_statistics, detect_stutters};
use rwp_core::FrameSample;

fn frames(values: &[f64]) -> Result<Vec<FrameSample>, Box<dyn std::error::Error>> {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| {
            FrameSample::new(index as u64 * 8_000_000, *value)
                .map_err(|error| Box::new(error) as Box<dyn std::error::Error>)
        })
        .collect()
}

#[test]
fn golden_stable_60_fps_has_no_stutter() -> Result<(), Box<dyn std::error::Error>> {
    let samples = [16.666; 120];
    let trace = frames(&samples)?;
    let events = detect_stutters(&trace, StutterConfig::default())?;
    assert!(events.is_empty());
    Ok(())
}

#[test]
fn golden_isolated_spike_is_detected() -> Result<(), Box<dyn std::error::Error>> {
    let mut samples = vec![16.0; 120];
    samples[60] = 50.0;
    let trace = frames(&samples)?;
    let events = detect_stutters(
        &trace,
        StutterConfig {
            rolling_half_window_frames: 15,
            multiplier: 2.0,
            absolute_threshold_ms: 30.0,
            minimum_baseline_samples: 7,
            merge_gap_ms: 50.0,
        },
    )?;
    assert_eq!(events.len(), 1);
    let event = events
        .first()
        .ok_or_else(|| std::io::Error::other("missing golden event"))?;
    assert_eq!(event.candidate_frame_count, 1);
    assert_eq!(event.peak_frame_time_ms, 50.0);
    Ok(())
}

#[test]
fn golden_statistics_preserve_distribution_shape() -> Result<(), Box<dyn std::error::Error>> {
    let values = [8.0, 8.0, 8.0, 16.0, 16.0, 16.0, 32.0, 64.0];
    let stats = compute_statistics(&values, StatisticsConfig::default())?;
    assert_eq!(stats.percentiles.p50_ms, 16.0);
    assert_eq!(stats.percentiles.p99_ms, 64.0);
    assert!(stats.low_fps.percentile_1_low_fps < stats.average_fps);
    Ok(())
}
