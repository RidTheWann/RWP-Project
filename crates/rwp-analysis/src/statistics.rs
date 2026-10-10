use serde::{Deserialize, Serialize};

/// Configuration for deterministic frame-time statistics.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StatisticsConfig {
    /// Warn when P99.9 is computed from fewer than this many frames.
    pub p999_reliability_threshold: usize,
}

impl Default for StatisticsConfig {
    fn default() -> Self {
        Self {
            p999_reliability_threshold: 1_000,
        }
    }
}

/// Percentile values in milliseconds, using nearest-rank ordering.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PercentileSet {
    /// 0.1th percentile.
    pub p0_1_ms: f64,
    /// 1st percentile.
    pub p1_ms: f64,
    /// 5th percentile.
    pub p5_ms: f64,
    /// 10th percentile.
    pub p10_ms: f64,
    /// 50th percentile.
    pub p50_ms: f64,
    /// 95th percentile.
    pub p95_ms: f64,
    /// 99th percentile.
    pub p99_ms: f64,
    /// 99.9th percentile.
    pub p99_9_ms: f64,
}

/// Both supported low-FPS definitions required by the product contract.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct LowFpsMetrics {
    /// Reciprocal of P99 frame time.
    pub percentile_1_low_fps: f64,
    /// Reciprocal of P99.9 frame time.
    pub percentile_0_1_low_fps: f64,
    /// Arithmetic mean FPS of the worst 1% frame times.
    pub mean_fps_of_worst_1pct_frames: f64,
    /// Arithmetic mean FPS of the worst 0.1% frame times.
    pub mean_fps_of_worst_0_1pct_frames: f64,
}

/// Complete deterministic frame-time statistics.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FrameStatistics {
    /// Number of valid frame samples.
    pub sample_count: usize,
    /// Arithmetic mean frame time.
    pub mean_frame_time_ms: f64,
    /// Median frame time (P50).
    pub median_frame_time_ms: f64,
    /// Arithmetic mean FPS defined as 1000 * N / sum(frame_time_ms).
    pub average_fps: f64,
    /// Unbiased sample variance, when at least two samples exist.
    pub variance_ms2: Option<f64>,
    /// Square root of sample variance, when at least two samples exist.
    pub stdev_ms: Option<f64>,
    /// Required nearest-rank percentiles.
    pub percentiles: PercentileSet,
    /// Both low-FPS definitions.
    pub low_fps: LowFpsMetrics,
    /// True when the configured P99.9 sample-size warning threshold is met.
    pub p999_reliable: bool,
}

/// Errors from frame-time statistics.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StatisticsError {
    /// No samples were supplied.
    #[error("at least one frame-time sample is required")]
    EmptyInput,
    /// A sample was not finite or positive.
    #[error("frame-time sample at index {index} must be finite and greater than zero")]
    InvalidSample {
        /// Index of the invalid sample.
        index: usize,
    },
    /// The reliability threshold is invalid.
    #[error("P99.9 reliability threshold must be greater than zero")]
    InvalidReliabilityThreshold,
}

/// Computes all v1 frame-time statistics using nearest-rank percentiles.
pub fn compute_statistics(
    frame_times_ms: &[f64],
    config: StatisticsConfig,
) -> Result<FrameStatistics, StatisticsError> {
    if frame_times_ms.is_empty() {
        return Err(StatisticsError::EmptyInput);
    }
    if config.p999_reliability_threshold == 0 {
        return Err(StatisticsError::InvalidReliabilityThreshold);
    }

    for (index, value) in frame_times_ms.iter().copied().enumerate() {
        if !value.is_finite() || value <= 0.0 {
            return Err(StatisticsError::InvalidSample { index });
        }
    }

    let mut sorted = frame_times_ms.to_vec();
    sorted.sort_by(|left, right| left.total_cmp(right));

    let sum: f64 = sorted.iter().sum();
    let sample_count = sorted.len();
    let mean = sum / sample_count as f64;
    let variance = if sample_count >= 2 {
        let sum_squared_deviation: f64 = sorted
            .iter()
            .map(|value| {
                let delta = *value - mean;
                delta * delta
            })
            .sum();
        Some(sum_squared_deviation / (sample_count - 1) as f64)
    } else {
        None
    };
    let stdev = variance.map(f64::sqrt);

    let percentiles = PercentileSet {
        p0_1_ms: nearest_rank(&sorted, 0.001),
        p1_ms: nearest_rank(&sorted, 0.01),
        p5_ms: nearest_rank(&sorted, 0.05),
        p10_ms: nearest_rank(&sorted, 0.10),
        p50_ms: nearest_rank(&sorted, 0.50),
        p95_ms: nearest_rank(&sorted, 0.95),
        p99_ms: nearest_rank(&sorted, 0.99),
        p99_9_ms: nearest_rank(&sorted, 0.999),
    };

    let low_fps = LowFpsMetrics {
        percentile_1_low_fps: reciprocal_fps(percentiles.p99_ms),
        percentile_0_1_low_fps: reciprocal_fps(percentiles.p99_9_ms),
        mean_fps_of_worst_1pct_frames: mean_fps_of_worst_fraction(&sorted, 0.01),
        mean_fps_of_worst_0_1pct_frames: mean_fps_of_worst_fraction(&sorted, 0.001),
    };

    Ok(FrameStatistics {
        sample_count,
        mean_frame_time_ms: mean,
        median_frame_time_ms: percentiles.p50_ms,
        average_fps: 1_000.0 * sample_count as f64 / sum,
        variance_ms2: variance,
        stdev_ms: stdev,
        percentiles,
        low_fps,
        p999_reliable: sample_count >= config.p999_reliability_threshold,
    })
}

fn nearest_rank(sorted: &[f64], percentile: f64) -> f64 {
    debug_assert!(!sorted.is_empty());
    let rank = (percentile * sorted.len() as f64).ceil() as usize;
    let index = rank.saturating_sub(1).min(sorted.len() - 1);
    sorted[index]
}

fn reciprocal_fps(frame_time_ms: f64) -> f64 {
    1_000.0 / frame_time_ms
}

fn mean_fps_of_worst_fraction(sorted: &[f64], fraction: f64) -> f64 {
    let count = ((sorted.len() as f64 * fraction).floor() as usize).max(1);
    let start = sorted.len().saturating_sub(count);
    let sum: f64 = sorted[start..]
        .iter()
        .map(|value| reciprocal_fps(*value))
        .sum();
    sum / (sorted.len() - start) as f64
}
