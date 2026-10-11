#![allow(missing_docs)]

use proptest::prelude::*;
use rwp_analysis::{
    RefreshAwareStutterConfig, StutterConfig, StutterError, detect_stutters,
    detect_stutters_refresh_aware,
};
use rwp_core::FrameSample;

fn frame(timestamp_ns: u64, frame_time_ms: f64) -> Result<FrameSample, Box<dyn std::error::Error>> {
    Ok(FrameSample::new(timestamp_ns, frame_time_ms)?)
}

const GOLDEN_SEED: u64 = 0xA0D8_2026;

fn seeded_refresh_trace(
    seed: u64,
    refresh_rate_hz: f64,
    hitch_ms: f64,
) -> Result<Vec<FrameSample>, Box<dyn std::error::Error>> {
    let period_ms = 1_000.0 / refresh_rate_hz;
    let timestamp_step_ns = (period_ms * 1_000_000.0) as u64;
    let mut state = seed;
    let mut frames = Vec::with_capacity(31);

    for index in 0..31_u64 {
        let frame_time_ms = if index == 15 {
            hitch_ms
        } else {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1);
            let jitter = ((state >> 56) as f64 / 255.0 - 0.5) * 0.1;
            period_ms + jitter
        };
        frames.push(frame(index * timestamp_step_ns, frame_time_ms)?);
    }

    Ok(frames)
}

#[test]
fn refresh_aware_threshold_matches_60_144_and_240_hz_golden_cases()
-> Result<(), Box<dyn std::error::Error>> {
    let cases = [(60.0, false), (144.0, true), (240.0, true)];
    for (refresh_rate_hz, should_detect) in cases {
        let frames = seeded_refresh_trace(GOLDEN_SEED, refresh_rate_hz, 20.0)?;
        let config = RefreshAwareStutterConfig::new(refresh_rate_hz)?;
        let events = detect_stutters_refresh_aware(&frames, config)?;
        assert_eq!(
            !events.is_empty(),
            should_detect,
            "unexpected 20 ms hitch detection at {refresh_rate_hz} Hz"
        );
        if should_detect {
            let event = events.first().ok_or("missing refresh-aware event")?;
            assert_eq!(event.peak_frame_time_ms, 20.0);
            assert_eq!(
                event
                    .detector_parameters
                    .iter()
                    .find(|(key, _)| key == "rule_id")
                    .map(|(_, value)| value.as_str()),
                Some("stutter.v2.refresh_aware")
            );
        }
    }
    Ok(())
}

#[test]
fn refresh_aware_rejects_invalid_refresh_rate() {
    assert!(matches!(
        RefreshAwareStutterConfig::new(0.0),
        Err(StutterError::InvalidConfiguration(
            "refresh_rate_hz must be finite and greater than zero"
        ))
    ));
}

#[test]
fn legacy_stutter_rule_keeps_v1_provenance() -> Result<(), Box<dyn std::error::Error>> {
    let frames = (0..20_u64)
        .map(|index| frame(index * 8_000_000, if index == 10 { 40.0 } else { 8.0 }))
        .collect::<Result<Vec<_>, _>>()?;
    let events = detect_stutters(
        &frames,
        StutterConfig {
            rolling_half_window_frames: 5,
            multiplier: 2.0,
            absolute_threshold_ms: 30.0,
            minimum_baseline_samples: 7,
            merge_gap_ms: 50.0,
        },
    )?;
    let event = events.first().ok_or("missing legacy event")?;
    assert_eq!(
        event
            .detector_parameters
            .iter()
            .find(|(key, _)| key == "rule_id")
            .map(|(_, value)| value.as_str()),
        Some("stutter.v1.absolute")
    );
    Ok(())
}

#[test]
fn detects_and_merges_stutter_candidates() -> Result<(), Box<dyn std::error::Error>> {
    let mut frames = Vec::new();
    for index in 0..40_u64 {
        let value = match index {
            20 | 21 => 40.0,
            _ => 8.0,
        };
        frames.push(frame(index * 8_000_000, value)?);
    }

    let config = StutterConfig {
        rolling_half_window_frames: 5,
        multiplier: 2.0,
        absolute_threshold_ms: 30.0,
        minimum_baseline_samples: 7,
        merge_gap_ms: 50.0,
    };
    let events = detect_stutters(&frames, config)?;

    assert_eq!(events.len(), 1);
    let event = events
        .first()
        .ok_or_else(|| std::io::Error::other("missing event"))?;
    assert_eq!(event.start_timestamp_ns, 20 * 8_000_000);
    assert_eq!(event.end_timestamp_ns, 21 * 8_000_000);
    assert_eq!(event.peak_frame_time_ms, 40.0);
    assert_eq!(event.baseline_frame_time_ms, 8.0);
    assert_eq!(event.candidate_frame_count, 2);
    assert_eq!(
        event
            .detector_parameters
            .iter()
            .find(|(key, _)| key == "multiplier")
            .map(|(_, value)| value.as_str()),
        Some("2")
    );
    Ok(())
}

#[test]
fn does_not_evaluate_when_baseline_is_insufficient() -> Result<(), Box<dyn std::error::Error>> {
    let frames = vec![frame(0, 40.0)?, frame(8_000_000, 8.0)?];
    let config = StutterConfig {
        rolling_half_window_frames: 15,
        minimum_baseline_samples: 7,
        ..StutterConfig::default()
    };
    assert!(detect_stutters(&frames, config)?.is_empty());
    Ok(())
}

#[test]
fn rejects_non_monotonic_timestamps() -> Result<(), Box<dyn std::error::Error>> {
    let frames = vec![frame(10, 8.0)?, frame(9, 40.0)?];
    assert_eq!(
        detect_stutters(&frames, StutterConfig::default()).err(),
        Some(StutterError::NonMonotonicTimestamps { index: 1 })
    );
    Ok(())
}

#[test]
fn invalid_stutter_configs_are_rejected() {
    let invalid = [
        StutterConfig {
            rolling_half_window_frames: 0,
            ..StutterConfig::default()
        },
        StutterConfig {
            multiplier: 0.0,
            ..StutterConfig::default()
        },
        StutterConfig {
            multiplier: f64::NAN,
            ..StutterConfig::default()
        },
        StutterConfig {
            absolute_threshold_ms: 0.0,
            ..StutterConfig::default()
        },
        StutterConfig {
            minimum_baseline_samples: 0,
            ..StutterConfig::default()
        },
        StutterConfig {
            merge_gap_ms: -1.0,
            ..StutterConfig::default()
        },
    ];
    for config in invalid {
        assert!(detect_stutters(&[], config).is_err());
    }
    let zero_gap = detect_stutters(
        &[],
        StutterConfig {
            merge_gap_ms: 0.0,
            ..StutterConfig::default()
        },
    );
    assert!(zero_gap.is_ok());
}

#[test]
fn invalid_refresh_aware_configs_are_rejected() {
    let base = RefreshAwareStutterConfig {
        rolling_half_window_frames: 2,
        multiplier: 2.0,
        refresh_rate_hz: 60.0,
        minimum_baseline_samples: 1,
        merge_gap_ms: 50.0,
    };
    let invalid = [
        RefreshAwareStutterConfig {
            refresh_rate_hz: 0.0,
            ..base
        },
        RefreshAwareStutterConfig {
            refresh_rate_hz: f64::NAN,
            ..base
        },
        RefreshAwareStutterConfig {
            multiplier: 0.0,
            ..base
        },
        RefreshAwareStutterConfig {
            merge_gap_ms: -1.0,
            ..base
        },
    ];
    for config in invalid {
        assert!(detect_stutters_refresh_aware(&[], config).is_err());
    }
    let zero_gap = detect_stutters_refresh_aware(
        &[],
        RefreshAwareStutterConfig {
            merge_gap_ms: 0.0,
            ..base
        },
    );
    assert!(zero_gap.is_ok());
}

fn spike_trace(
    spike_index: u64,
    spike_ms: f64,
) -> Result<Vec<FrameSample>, Box<dyn std::error::Error>> {
    (0..7_u64)
        .map(|index| {
            frame(
                index * 8_000_000,
                if index == spike_index { spike_ms } else { 10.0 },
            )
        })
        .collect()
}

#[test]
fn exact_neighbor_minimum_is_sufficient() -> Result<(), Box<dyn std::error::Error>> {
    let frames = spike_trace(3, 50.0)?;
    let events = detect_stutters(
        &frames,
        StutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 2.0,
            absolute_threshold_ms: 30.0,
            minimum_baseline_samples: 4,
            merge_gap_ms: 50.0,
        },
    )?;
    assert_eq!(events.len(), 1);

    let events = detect_stutters_refresh_aware(
        &frames,
        RefreshAwareStutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 2.0,
            refresh_rate_hz: 60.0,
            minimum_baseline_samples: 4,
            merge_gap_ms: 50.0,
        },
    )?;
    assert_eq!(events.len(), 1);
    Ok(())
}

#[test]
fn interior_spike_below_combined_threshold_is_ignored() -> Result<(), Box<dyn std::error::Error>> {
    let frames = spike_trace(3, 15.0)?;
    let events = detect_stutters(
        &frames,
        StutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 2.0,
            absolute_threshold_ms: 5.0,
            minimum_baseline_samples: 1,
            merge_gap_ms: 50.0,
        },
    )?;
    assert!(events.is_empty());
    Ok(())
}

#[test]
fn single_true_conjunct_does_not_fire() -> Result<(), Box<dyn std::error::Error>> {
    let frames = spike_trace(3, 25.0)?;
    let events = detect_stutters(
        &frames,
        StutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 2.0,
            absolute_threshold_ms: 30.0,
            minimum_baseline_samples: 1,
            merge_gap_ms: 50.0,
        },
    )?;
    assert!(events.is_empty());
    Ok(())
}

#[test]
fn multiplier_boundary_equality_does_not_fire() -> Result<(), Box<dyn std::error::Error>> {
    let frames = spike_trace(3, 20.0)?;
    let events = detect_stutters(
        &frames,
        StutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 2.0,
            absolute_threshold_ms: 5.0,
            minimum_baseline_samples: 1,
            merge_gap_ms: 50.0,
        },
    )?;
    assert!(events.is_empty());
    Ok(())
}

#[test]
fn absolute_boundary_equality_does_not_fire() -> Result<(), Box<dyn std::error::Error>> {
    let frames = spike_trace(3, 30.0)?;
    let events = detect_stutters(
        &frames,
        StutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 2.0,
            absolute_threshold_ms: 30.0,
            minimum_baseline_samples: 1,
            merge_gap_ms: 50.0,
        },
    )?;
    assert!(events.is_empty());
    Ok(())
}

#[test]
fn truncated_window_median_uses_available_neighbors() -> Result<(), Box<dyn std::error::Error>> {
    let frames = (0..7_u64)
        .map(|index| {
            let value = match index {
                0 => 20.0,
                2 => 50.0,
                3 => 60.0,
                _ => 10.0,
            };
            frame(index * 8_000_000, value)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let events = detect_stutters(
        &frames,
        StutterConfig {
            rolling_half_window_frames: 3,
            multiplier: 0.5,
            absolute_threshold_ms: 5.0,
            minimum_baseline_samples: 1,
            merge_gap_ms: 50.0,
        },
    )?;
    assert!(events.iter().all(|event| event.start_timestamp_ns != 0));
    Ok(())
}

#[test]
fn edge_spike_uses_truncated_window() -> Result<(), Box<dyn std::error::Error>> {
    let frames = (0..5_u64)
        .map(|index| frame(index * 8_000_000, if index == 0 { 50.0 } else { 10.0 }))
        .collect::<Result<Vec<_>, _>>()?;
    let events = detect_stutters(
        &frames,
        StutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 2.0,
            absolute_threshold_ms: 5.0,
            minimum_baseline_samples: 2,
            merge_gap_ms: 50.0,
        },
    )?;
    assert_eq!(events.len(), 1);
    Ok(())
}

#[test]
fn refresh_aware_spike_below_period_aware_threshold_is_ignored()
-> Result<(), Box<dyn std::error::Error>> {
    let frames = spike_trace(3, 20.0)?;
    let events = detect_stutters_refresh_aware(
        &frames,
        RefreshAwareStutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 1.5,
            refresh_rate_hz: 60.0,
            minimum_baseline_samples: 1,
            merge_gap_ms: 50.0,
        },
    )?;
    assert!(events.is_empty());
    Ok(())
}

#[test]
fn refresh_aware_multiplier_dominated_threshold_holds() -> Result<(), Box<dyn std::error::Error>> {
    let frames = spike_trace(3, 28.0)?;
    let events = detect_stutters_refresh_aware(
        &frames,
        RefreshAwareStutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 3.0,
            refresh_rate_hz: 60.0,
            minimum_baseline_samples: 1,
            merge_gap_ms: 50.0,
        },
    )?;
    assert!(events.is_empty());
    Ok(())
}

#[test]
fn refresh_aware_exact_threshold_equality_does_not_fire() -> Result<(), Box<dyn std::error::Error>>
{
    let spike_ms = 10.0 + 1_000.0 / 60.0;
    let frames = spike_trace(3, spike_ms)?;
    let events = detect_stutters_refresh_aware(
        &frames,
        RefreshAwareStutterConfig {
            rolling_half_window_frames: 2,
            multiplier: 1.5,
            refresh_rate_hz: 60.0,
            minimum_baseline_samples: 1,
            merge_gap_ms: 50.0,
        },
    )?;
    assert!(events.is_empty());
    Ok(())
}

#[test]
fn accepts_equal_timestamps_as_non_decreasing() -> Result<(), Box<dyn std::error::Error>> {
    let frames = vec![frame(10, 8.0)?, frame(10, 40.0)?];
    let result = detect_stutters(
        &frames,
        StutterConfig {
            minimum_baseline_samples: 1,
            ..StutterConfig::default()
        },
    )?;
    let _ = result;
    Ok(())
}

proptest! {
    #[test]
    fn stutter_events_are_ordered_non_overlapping_and_respect_merge_gap(
        hitches in prop::collection::vec(any::<bool>(), 1..128)
    ) {
        let frames = hitches
            .iter()
            .enumerate()
            .map(|(index, hitch)| {
                FrameSample::new(
                    index as u64 * 8_000_000,
                    if *hitch { 40.0 } else { 8.0 },
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| TestCaseError::fail(error.to_string()))?;
        let events = detect_stutters(
            &frames,
            StutterConfig {
                rolling_half_window_frames: 4,
                multiplier: 2.0,
                absolute_threshold_ms: 30.0,
                minimum_baseline_samples: 4,
                merge_gap_ms: 16.0,
            },
        )
        .map_err(|error| TestCaseError::fail(error.to_string()))?;

        let event_bounds_are_valid = events.iter().all(|event| {
            event.start_timestamp_ns <= event.end_timestamp_ns
                && event.candidate_frame_count > 0
        });
        let event_gaps_respect_merge_threshold = events.windows(2).all(|pair| {
            pair[0].end_timestamp_ns < pair[1].start_timestamp_ns
                && pair[1].start_timestamp_ns - pair[0].end_timestamp_ns > 16_000_000
        });
        prop_assert!(event_bounds_are_valid, "event bounds must be valid");
        prop_assert!(
            event_gaps_respect_merge_threshold,
            "separate events must exceed the merge gap"
        );
    }
}
