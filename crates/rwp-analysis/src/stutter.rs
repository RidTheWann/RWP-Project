use rwp_core::{FrameSample, StutterEvent};

/// Deterministic frame-time stutter detector configuration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StutterConfig {
    /// Number of neighboring frames considered on either side of a candidate.
    pub rolling_half_window_frames: usize,
    /// Candidate multiplier relative to the local median baseline.
    pub multiplier: f64,
    /// Absolute minimum frame time for a candidate, in milliseconds.
    pub absolute_threshold_ms: f64,
    /// Minimum valid neighboring samples required to evaluate a candidate.
    pub minimum_baseline_samples: usize,
    /// Maximum timestamp gap between candidates before they form separate events.
    pub merge_gap_ms: f64,
}

impl Default for StutterConfig {
    fn default() -> Self {
        Self {
            rolling_half_window_frames: 15,
            multiplier: 2.0,
            absolute_threshold_ms: 30.0,
            minimum_baseline_samples: 7,
            merge_gap_ms: 50.0,
        }
    }
}

/// Refresh-aware stutter detector configuration for rule stutter.v2.refresh_aware.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RefreshAwareStutterConfig {
    /// Number of neighboring frames considered on either side of a candidate.
    pub rolling_half_window_frames: usize,
    /// Candidate multiplier relative to the local median baseline.
    pub multiplier: f64,
    /// Active display refresh rate in Hz.
    pub refresh_rate_hz: f64,
    /// Minimum valid neighboring samples required to evaluate a candidate.
    pub minimum_baseline_samples: usize,
    /// Maximum timestamp gap between candidates before they form separate events.
    pub merge_gap_ms: f64,
}

impl RefreshAwareStutterConfig {
    /// Creates a validated refresh-aware configuration for one known display refresh rate.
    pub fn new(refresh_rate_hz: f64) -> Result<Self, StutterError> {
        let config = Self {
            rolling_half_window_frames: 15,
            multiplier: 2.0,
            refresh_rate_hz,
            minimum_baseline_samples: 7,
            merge_gap_ms: 50.0,
        };
        validate_refresh_aware_config(config)?;
        Ok(config)
    }
}

/// Errors from the deterministic stutter detector.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StutterError {
    /// Configuration contains an invalid value.
    #[error("invalid stutter configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// Input samples move backwards in timestamp order.
    #[error("frame timestamps must be non-decreasing; violation at index {index}")]
    NonMonotonicTimestamps {
        /// Index containing the ordering violation.
        index: usize,
    },
    /// A frame sample contains invalid timing.
    #[error("frame sample at index {index} must be finite and greater than zero")]
    InvalidFrameTime {
        /// Index containing the invalid frame time.
        index: usize,
    },
}

/// Detects refresh-aware local frame-time outliers using rule stutter.v2.refresh_aware.
pub fn detect_stutters_refresh_aware(
    frames: &[FrameSample],
    config: RefreshAwareStutterConfig,
) -> Result<Vec<StutterEvent>, StutterError> {
    validate_refresh_aware_config(config)?;
    validate_frames(frames)?;
    if frames.is_empty() {
        return Ok(Vec::new());
    }

    let refresh_period_ms = 1_000.0 / config.refresh_rate_hz;
    let mut candidates = Vec::new();
    for index in 0..frames.len() {
        let baseline = local_median(frames, index, config.rolling_half_window_frames);
        let Some(baseline) = baseline else {
            continue;
        };
        if count_valid_neighbors(frames, index, config.rolling_half_window_frames)
            < config.minimum_baseline_samples
        {
            continue;
        }
        let threshold = (config.multiplier * baseline).max(baseline + refresh_period_ms);
        if frames[index].frame_time_ms > threshold {
            candidates.push((index, baseline));
        }
    }

    Ok(merge_candidates(
        frames,
        candidates,
        vec![
            (
                String::from("rule_id"),
                String::from("stutter.v2.refresh_aware"),
            ),
            (
                String::from("rolling_half_window_frames"),
                config.rolling_half_window_frames.to_string(),
            ),
            (String::from("multiplier"), config.multiplier.to_string()),
            (
                String::from("refresh_rate_hz"),
                config.refresh_rate_hz.to_string(),
            ),
            (
                String::from("refresh_period_ms"),
                refresh_period_ms.to_string(),
            ),
            (
                String::from("minimum_baseline_samples"),
                config.minimum_baseline_samples.to_string(),
            ),
            (
                String::from("merge_gap_ms"),
                config.merge_gap_ms.to_string(),
            ),
        ],
        config.merge_gap_ms,
    ))
}

/// Detects robust local frame-time outliers using the M0/M1 contract.
pub fn detect_stutters(
    frames: &[FrameSample],
    config: StutterConfig,
) -> Result<Vec<StutterEvent>, StutterError> {
    validate_config(config)?;
    validate_frames(frames)?;
    if frames.is_empty() {
        return Ok(Vec::new());
    }

    let mut candidates = Vec::new();
    for index in 0..frames.len() {
        let baseline = local_median(frames, index, config.rolling_half_window_frames);
        let Some(baseline) = baseline else {
            continue;
        };
        let valid_neighbors =
            count_valid_neighbors(frames, index, config.rolling_half_window_frames);
        if valid_neighbors < config.minimum_baseline_samples {
            continue;
        }
        let frame_time = frames[index].frame_time_ms;
        if frame_time > config.multiplier * baseline && frame_time > config.absolute_threshold_ms {
            candidates.push((index, baseline));
        }
    }

    let detector_parameters = vec![
        (String::from("rule_id"), String::from("stutter.v1.absolute")),
        (
            String::from("rolling_half_window_frames"),
            config.rolling_half_window_frames.to_string(),
        ),
        (String::from("multiplier"), config.multiplier.to_string()),
        (
            String::from("absolute_threshold_ms"),
            config.absolute_threshold_ms.to_string(),
        ),
        (
            String::from("minimum_baseline_samples"),
            config.minimum_baseline_samples.to_string(),
        ),
        (
            String::from("merge_gap_ms"),
            config.merge_gap_ms.to_string(),
        ),
    ];
    Ok(merge_candidates(
        frames,
        candidates,
        detector_parameters,
        config.merge_gap_ms,
    ))
}

fn validate_refresh_aware_config(config: RefreshAwareStutterConfig) -> Result<(), StutterError> {
    if config.rolling_half_window_frames == 0 {
        return Err(StutterError::InvalidConfiguration(
            "rolling_half_window_frames must be greater than zero",
        ));
    }
    if config.multiplier <= 0.0 || !config.multiplier.is_finite() {
        return Err(StutterError::InvalidConfiguration(
            "multiplier must be finite and greater than zero",
        ));
    }
    if config.refresh_rate_hz <= 0.0 || !config.refresh_rate_hz.is_finite() {
        return Err(StutterError::InvalidConfiguration(
            "refresh_rate_hz must be finite and greater than zero",
        ));
    }
    if config.minimum_baseline_samples == 0 {
        return Err(StutterError::InvalidConfiguration(
            "minimum_baseline_samples must be greater than zero",
        ));
    }
    if config.merge_gap_ms < 0.0 || !config.merge_gap_ms.is_finite() {
        return Err(StutterError::InvalidConfiguration(
            "merge_gap_ms must be finite and non-negative",
        ));
    }
    Ok(())
}

fn validate_config(config: StutterConfig) -> Result<(), StutterError> {
    if config.rolling_half_window_frames == 0 {
        return Err(StutterError::InvalidConfiguration(
            "rolling_half_window_frames must be greater than zero",
        ));
    }
    if config.multiplier <= 0.0 || !config.multiplier.is_finite() {
        return Err(StutterError::InvalidConfiguration(
            "multiplier must be finite and greater than zero",
        ));
    }
    if config.absolute_threshold_ms <= 0.0 || !config.absolute_threshold_ms.is_finite() {
        return Err(StutterError::InvalidConfiguration(
            "absolute_threshold_ms must be finite and greater than zero",
        ));
    }
    if config.minimum_baseline_samples == 0 {
        return Err(StutterError::InvalidConfiguration(
            "minimum_baseline_samples must be greater than zero",
        ));
    }
    if config.merge_gap_ms < 0.0 || !config.merge_gap_ms.is_finite() {
        return Err(StutterError::InvalidConfiguration(
            "merge_gap_ms must be finite and non-negative",
        ));
    }
    Ok(())
}

fn merge_candidates(
    frames: &[FrameSample],
    candidates: Vec<(usize, f64)>,
    detector_parameters: Vec<(String, String)>,
    merge_gap_ms: f64,
) -> Vec<StutterEvent> {
    let mut events = Vec::new();
    for (index, baseline) in candidates {
        let timestamp_ns = frames[index].timestamp_ns;
        let should_merge = events.last().is_some_and(|event: &StutterEvent| {
            let gap_ns = timestamp_ns.saturating_sub(event.end_timestamp_ns);
            gap_ns as f64 <= merge_gap_ms * 1_000_000.0
        });

        if should_merge {
            if let Some(event) = events.last_mut() {
                event.end_timestamp_ns = timestamp_ns;
                event.peak_frame_time_ms =
                    event.peak_frame_time_ms.max(frames[index].frame_time_ms);
                event.baseline_frame_time_ms = event.baseline_frame_time_ms.min(baseline);
                event.candidate_frame_count = event.candidate_frame_count.saturating_add(1);
            }
        } else {
            events.push(StutterEvent {
                start_timestamp_ns: timestamp_ns,
                end_timestamp_ns: timestamp_ns,
                peak_frame_time_ms: frames[index].frame_time_ms,
                baseline_frame_time_ms: baseline,
                candidate_frame_count: 1,
                detector_parameters: detector_parameters.clone(),
            });
        }
    }
    events
}
fn validate_frames(frames: &[FrameSample]) -> Result<(), StutterError> {
    for (index, frame) in frames.iter().enumerate() {
        if !frame.frame_time_ms.is_finite() || frame.frame_time_ms <= 0.0 {
            return Err(StutterError::InvalidFrameTime { index });
        }
        if index > 0 && frame.timestamp_ns < frames[index - 1].timestamp_ns {
            return Err(StutterError::NonMonotonicTimestamps { index });
        }
    }
    Ok(())
}

fn local_median(frames: &[FrameSample], index: usize, half_window: usize) -> Option<f64> {
    let start = index.saturating_sub(half_window);
    let end = (index + half_window + 1).min(frames.len());
    let mut neighbors = Vec::with_capacity(end.saturating_sub(start).saturating_sub(1));
    for (neighbor_index, neighbor) in frames.iter().enumerate().take(end).skip(start) {
        if neighbor_index != index {
            neighbors.push(neighbor.frame_time_ms);
        }
    }
    if neighbors.is_empty() {
        return None;
    }
    neighbors.sort_by(|left, right| left.total_cmp(right));
    let rank = ((neighbors.len() as f64) * 0.5).ceil() as usize;
    Some(neighbors[rank.saturating_sub(1).min(neighbors.len() - 1)])
}

fn count_valid_neighbors(frames: &[FrameSample], index: usize, half_window: usize) -> usize {
    let start = index.saturating_sub(half_window);
    let end = (index + half_window + 1).min(frames.len());
    end.saturating_sub(start).saturating_sub(1)
}
