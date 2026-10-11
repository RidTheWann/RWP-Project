//! Deterministic A/B comparison of already-normalized analysis results.

use serde::{Deserialize, Serialize};

use crate::{FrameStatistics, LowFpsMetrics};

/// Schema version for the M6 comparison result.
pub const COMPARISON_SCHEMA_VERSION: &str = "rwp.comparison.v1";

/// Controls when two analysis results are considered comparable.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct ComparisonConfig {
    /// Minimum smaller/larger sample-count ratio for normal comparability.
    pub minimum_sample_ratio: f64,
    /// Minimum absolute relative change to call a metric improved or regressed.
    pub change_threshold_percent: f64,
}

impl Default for ComparisonConfig {
    fn default() -> Self {
        Self {
            minimum_sample_ratio: 0.80,
            change_threshold_percent: 1.0,
        }
    }
}

/// Errors raised by invalid comparison configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ComparisonError {
    /// Minimum sample ratio is outside the open interval (0, 1].
    #[error("minimum_sample_ratio must be greater than zero and at most one")]
    InvalidSampleRatio,
    /// Change threshold is negative or non-finite.
    #[error("change_threshold_percent must be finite and non-negative")]
    InvalidChangeThreshold,
    /// Comparison inputs contain no frame observations.
    #[error("comparison requires at least one frame sample in both runs")]
    EmptyComparison,
    /// A comparison metric is non-finite or non-positive.
    #[error("comparison metric '{0}' must be finite and greater than zero")]
    InvalidComparisonMetric(&'static str),
    /// Minimum paired-run count is zero.
    #[error("minimum_pairs must be greater than zero")]
    InvalidMinimumPairs,
    /// Minimum paired-run count cannot reach the requested alpha.
    #[error("minimum_pairs is too small for the configured alpha")]
    InvalidMinimumPairsForAlpha,
    /// Sign-test alpha must be finite and in the open interval (0, 1).
    #[error("alpha must be finite and between zero and one")]
    InvalidAlpha,
    /// Equivalence margin is negative or non-finite.
    #[error("equivalence_margin_percent must be finite and non-negative")]
    InvalidEquivalenceMargin,
    /// A paired run identity is empty or duplicated.
    #[error("paired run identity must be non-empty and unique")]
    InvalidPairIdentity,
    /// A paired run has no frame observations.
    #[error("paired run sample counts must be greater than zero")]
    InvalidPairSampleCount,
    /// A paired P99 metric is non-finite or non-positive.
    #[error("paired P99 frame-time metrics must be finite and greater than zero")]
    InvalidPairMetric,
}

/// How much evidence the comparison has for a descriptive A/B read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Comparability {
    /// Same scenario with sufficiently similar sample coverage.
    Comparable,
    /// Same scenario, but sample coverage is materially different.
    LimitedSampleEquivalence,
    /// Different scenarios cannot support an A/B interpretation.
    InconclusiveDifferentScenario,
}

/// Explicit statement of what an M6 comparison can and cannot establish.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InferenceLimit {
    /// The output is descriptive and does not prove causality.
    DescriptiveOnly,
    /// The output uses a run-level statistical test but does not prove causality.
    InferentialNonCausal,
}

/// Direction of a metric delta after applying the configured change threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DeltaDirection {
    /// The candidate is materially better for this metric.
    Improved,
    /// The candidate is materially worse for this metric.
    Regressed,
    /// The candidate change is smaller than the configured threshold.
    Unchanged,
}

/// A deterministic baseline-to-candidate metric delta.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MetricDelta {
    /// Baseline value.
    pub baseline: f64,
    /// Candidate value.
    pub candidate: f64,
    /// Candidate minus baseline.
    pub absolute_delta: f64,
    /// Relative change from baseline, when baseline is non-zero.
    pub relative_percent: Option<f64>,
    /// Interpretation after applying the threshold.
    pub direction: DeltaDirection,
}

/// Core frame-time metrics compared by M6.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonMetrics {
    /// Average FPS; higher is better.
    pub average_fps: MetricDelta,
    /// Mean frame time; lower is better.
    pub mean_frame_time_ms: MetricDelta,
    /// Median frame time; lower is better.
    pub median_frame_time_ms: MetricDelta,
    /// P95 frame time; lower is better.
    pub p95_frame_time_ms: MetricDelta,
    /// P99 frame time; lower is better.
    pub p99_frame_time_ms: MetricDelta,
    /// Reciprocal-P99 one-percent-low FPS definition; higher is better.
    pub percentile_1_low_fps: MetricDelta,
    /// Reciprocal-P99.9 zero-point-one-percent-low FPS definition; higher is better.
    pub percentile_0_1_low_fps: MetricDelta,
    /// Mean FPS of the worst one percent; higher is better.
    pub mean_fps_of_worst_1pct_frames: MetricDelta,
    /// Mean FPS of the worst zero-point-one percent; higher is better.
    pub mean_fps_of_worst_0_1pct_frames: MetricDelta,
}

/// Complete deterministic M6 comparison result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ComparisonResult {
    /// Comparison schema identifier.
    pub schema_version: String,
    /// Exact configuration used to produce the result.
    pub config: ComparisonConfig,
    /// Whether the two runs are safe to compare descriptively.
    pub comparability: Comparability,
    /// Explicit inference limitation.
    pub inference_limit: InferenceLimit,
    /// Baseline sample count.
    pub baseline_sample_count: usize,
    /// Candidate sample count.
    pub candidate_sample_count: usize,
    /// Smaller/larger sample-count ratio.
    pub sample_ratio: f64,
    /// Frame-time metric deltas.
    pub metrics: ComparisonMetrics,
    /// Baseline stutter-event count.
    pub baseline_stutter_count: usize,
    /// Candidate stutter-event count.
    pub candidate_stutter_count: usize,
    /// Candidate minus baseline stutter-event count.
    pub stutter_count_delta: i64,
}

/// Configuration for paired run-level inference.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PairedInferenceConfig {
    /// Minimum number of comparable paired runs before an inferential outcome is allowed.
    pub minimum_pairs: usize,
    /// Minimum smaller/larger sample-count ratio for a pair to be eligible.
    pub minimum_sample_ratio: f64,
    /// Two-sided sign-test significance level.
    pub alpha: f64,
    /// Relative change within this margin is treated as practically equivalent.
    pub equivalence_margin_percent: f64,
}

impl Default for PairedInferenceConfig {
    fn default() -> Self {
        Self {
            minimum_pairs: 6,
            minimum_sample_ratio: 0.80,
            alpha: 0.05,
            equivalence_margin_percent: 1.0,
        }
    }
}

/// A single matched baseline/candidate run pair for P99 frame-time inference.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairedRun {
    /// Stable pair identifier.
    pub pair_id: String,
    /// Number of baseline frame observations.
    pub baseline_sample_count: usize,
    /// Number of candidate frame observations.
    pub candidate_sample_count: usize,
    /// Baseline P99 frame time in milliseconds.
    pub baseline_p99_frame_time_ms: f64,
    /// Candidate P99 frame time in milliseconds.
    pub candidate_p99_frame_time_ms: f64,
}

/// Inferential classification from matched runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PairedInferenceOutcome {
    /// Candidate shows a statistically supported improvement beyond the practical margin.
    Improved,
    /// Candidate shows a statistically supported regression beyond the practical margin.
    Regressed,
    /// All informative run-pair differences fall inside the equivalence margin.
    Equivalent,
    /// Data are insufficient or directional evidence is not statistically supported.
    Inconclusive,
}

/// Deterministic paired-run inference result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PairedInferenceResult {
    /// Comparison schema identifier.
    pub schema_version: String,
    /// Exact configuration used to produce the result.
    pub config: PairedInferenceConfig,
    /// Explicit statement that this is not causal proof.
    pub inference_limit: InferenceLimit,
    /// Final inferential classification.
    pub outcome: PairedInferenceOutcome,
    /// Number of supplied matched run pairs.
    pub pair_count: usize,
    /// Number of pairs with sufficient sample-count comparability.
    pub comparable_pair_count: usize,
    /// Number of pairs excluded for insufficient sample-count comparability.
    pub excluded_pair_count: usize,
    /// Number of pairs outside the practical-equivalence margin.
    pub informative_pair_count: usize,
    /// Informative pairs where candidate improved P99.
    pub improvement_wins: usize,
    /// Informative pairs where candidate regressed P99.
    pub regression_losses: usize,
    /// Pairs inside the practical-equivalence margin.
    pub practical_ties: usize,
    /// Median relative P99 improvement across all pairs.
    pub median_relative_improvement_percent: f64,
    /// Exact two-sided sign-test p-value across informative pairs.
    pub p_value: Option<f64>,
}

/// Validates paired-run inference configuration.
fn validate_paired_config(config: PairedInferenceConfig) -> Result<(), ComparisonError> {
    if config.minimum_pairs == 0 {
        return Err(ComparisonError::InvalidMinimumPairs);
    }
    if !config.minimum_sample_ratio.is_finite()
        || config.minimum_sample_ratio <= 0.0
        || config.minimum_sample_ratio > 1.0
    {
        return Err(ComparisonError::InvalidSampleRatio);
    }
    if !config.alpha.is_finite() || config.alpha <= 0.0 || config.alpha >= 1.0 {
        return Err(ComparisonError::InvalidAlpha);
    }
    if config.minimum_pairs < minimum_pairs_for_alpha(config.alpha)? {
        return Err(ComparisonError::InvalidMinimumPairsForAlpha);
    }
    if !config.equivalence_margin_percent.is_finite() || config.equivalence_margin_percent < 0.0 {
        return Err(ComparisonError::InvalidEquivalenceMargin);
    }
    Ok(())
}

/// Performs deterministic paired run-level inference on P99 frame time.
pub fn paired_infer(
    pairs: &[PairedRun],
    config: PairedInferenceConfig,
) -> Result<PairedInferenceResult, ComparisonError> {
    validate_paired_config(config)?;
    let mut seen = std::collections::BTreeSet::new();
    let mut effects = Vec::with_capacity(pairs.len());
    let mut comparable_pairs = 0_usize;
    let mut excluded_pairs = 0_usize;
    let mut wins = 0_usize;
    let mut losses = 0_usize;
    let mut ties = 0_usize;

    for pair in pairs {
        if pair.pair_id.is_empty() || !seen.insert(pair.pair_id.as_str()) {
            return Err(ComparisonError::InvalidPairIdentity);
        }
        if pair.baseline_sample_count == 0 || pair.candidate_sample_count == 0 {
            return Err(ComparisonError::InvalidPairSampleCount);
        }
        if !pair.baseline_p99_frame_time_ms.is_finite()
            || pair.baseline_p99_frame_time_ms <= 0.0
            || !pair.candidate_p99_frame_time_ms.is_finite()
            || pair.candidate_p99_frame_time_ms <= 0.0
        {
            return Err(ComparisonError::InvalidPairMetric);
        }

        let larger = pair.baseline_sample_count.max(pair.candidate_sample_count);
        let smaller = pair.baseline_sample_count.min(pair.candidate_sample_count);
        let sample_ratio = smaller as f64 / larger as f64;
        if sample_ratio < config.minimum_sample_ratio {
            excluded_pairs += 1;
            continue;
        }

        comparable_pairs += 1;
        let improvement_percent = (pair.baseline_p99_frame_time_ms
            - pair.candidate_p99_frame_time_ms)
            / pair.baseline_p99_frame_time_ms
            * 100.0;
        effects.push(improvement_percent);

        if improvement_percent > config.equivalence_margin_percent {
            wins += 1;
        } else if improvement_percent < -config.equivalence_margin_percent {
            losses += 1;
        } else {
            ties += 1;
        }
    }

    effects.sort_by(|left, right| left.total_cmp(right));
    let median = if effects.is_empty() {
        0.0
    } else {
        let middle = effects.len() / 2;
        if effects.len() % 2 == 0 {
            (effects[middle - 1] + effects[middle]) / 2.0
        } else {
            effects[middle]
        }
    };

    let informative = wins + losses;
    let p_value = if informative == 0 {
        None
    } else {
        Some(exact_two_sided_sign_test_p_value(
            informative,
            wins.min(losses),
        ))
    };

    let outcome = if comparable_pairs < config.minimum_pairs {
        PairedInferenceOutcome::Inconclusive
    } else if informative == 0 {
        PairedInferenceOutcome::Equivalent
    } else if let Some(p_value) = p_value {
        if p_value <= config.alpha && wins > losses {
            PairedInferenceOutcome::Improved
        } else if p_value <= config.alpha && losses > wins {
            PairedInferenceOutcome::Regressed
        } else {
            PairedInferenceOutcome::Inconclusive
        }
    } else {
        PairedInferenceOutcome::Inconclusive
    };

    Ok(PairedInferenceResult {
        schema_version: String::from(COMPARISON_SCHEMA_VERSION),
        config,

        inference_limit: InferenceLimit::InferentialNonCausal,
        outcome,
        pair_count: pairs.len(),
        comparable_pair_count: comparable_pairs,
        excluded_pair_count: excluded_pairs,
        informative_pair_count: informative,
        improvement_wins: wins,
        regression_losses: losses,
        practical_ties: ties,
        median_relative_improvement_percent: median,
        p_value,
    })
}

/// Returns the smallest informative-pair count whose best possible two-sided sign-test
/// p-value can reach `alpha`.
pub fn minimum_pairs_for_alpha(alpha: f64) -> Result<usize, ComparisonError> {
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return Err(ComparisonError::InvalidAlpha);
    }
    Ok((1.0 - alpha.log2()).ceil() as usize)
}

fn exact_two_sided_sign_test_p_value(informative_pairs: usize, tail_count: usize) -> f64 {
    if tail_count == 0 {
        let one_sided = (-(informative_pairs as f64) * std::f64::consts::LN_2).exp();
        return (2.0 * one_sided).min(1.0);
    }

    let k = tail_count.min(informative_pairs - tail_count);
    let mut probability_k = (log_binomial(informative_pairs, k)
        - (informative_pairs as f64) * std::f64::consts::LN_2)
        .exp();
    let mut tail = probability_k;

    for i in (1..=k).rev() {
        probability_k *= i as f64 / (informative_pairs - i + 1) as f64;
        tail += probability_k;
    }

    (2.0 * tail).min(1.0)
}

fn log_binomial(n: usize, k: usize) -> f64 {
    if k == 0 || k == n {
        return 0.0;
    }
    let k = k.min(n - k);
    let mut value = 0.0;
    for i in 1..=k {
        value += ((n - k + i) as f64).ln() - (i as f64).ln();
    }
    value
}

/// The two analysis results and metadata required for an A/B comparison.
#[derive(Debug, Clone, Copy)]
pub struct ComparisonInput<'a> {
    /// Baseline frame statistics.
    pub baseline: &'a FrameStatistics,
    /// Candidate frame statistics.
    pub candidate: &'a FrameStatistics,
    /// Baseline stutter-event count.
    pub baseline_stutter_count: usize,
    /// Candidate stutter-event count.
    pub candidate_stutter_count: usize,
    /// Whether the two runs represent the same intended workload/scenario.
    pub same_scenario: bool,
}

/// Compares two deterministic analysis results without making a causal claim.
pub fn compare(
    input: ComparisonInput<'_>,
    config: ComparisonConfig,
) -> Result<ComparisonResult, ComparisonError> {
    validate_config(config)?;

    if input.baseline.sample_count == 0 || input.candidate.sample_count == 0 {
        return Err(ComparisonError::EmptyComparison);
    }
    validate_comparison_statistics(input.baseline)?;
    validate_comparison_statistics(input.candidate)?;

    let larger = input
        .baseline
        .sample_count
        .max(input.candidate.sample_count);
    let smaller = input
        .baseline
        .sample_count
        .min(input.candidate.sample_count);
    let sample_ratio = smaller as f64 / larger as f64;

    let comparability = if !input.same_scenario {
        Comparability::InconclusiveDifferentScenario
    } else if sample_ratio < config.minimum_sample_ratio {
        Comparability::LimitedSampleEquivalence
    } else {
        Comparability::Comparable
    };

    Ok(ComparisonResult {
        schema_version: String::from(COMPARISON_SCHEMA_VERSION),
        config,
        comparability,
        inference_limit: InferenceLimit::DescriptiveOnly,
        baseline_sample_count: input.baseline.sample_count,
        candidate_sample_count: input.candidate.sample_count,
        sample_ratio,
        metrics: ComparisonMetrics {
            average_fps: metric_delta(
                input.baseline.average_fps,
                input.candidate.average_fps,
                true,
                config.change_threshold_percent,
            ),
            mean_frame_time_ms: metric_delta(
                input.baseline.mean_frame_time_ms,
                input.candidate.mean_frame_time_ms,
                false,
                config.change_threshold_percent,
            ),
            median_frame_time_ms: metric_delta(
                input.baseline.median_frame_time_ms,
                input.candidate.median_frame_time_ms,
                false,
                config.change_threshold_percent,
            ),
            p95_frame_time_ms: metric_delta(
                input.baseline.percentiles.p95_ms,
                input.candidate.percentiles.p95_ms,
                false,
                config.change_threshold_percent,
            ),
            p99_frame_time_ms: metric_delta(
                input.baseline.percentiles.p99_ms,
                input.candidate.percentiles.p99_ms,
                false,
                config.change_threshold_percent,
            ),
            percentile_1_low_fps: metric_delta(
                input.baseline.low_fps.percentile_1_low_fps,
                input.candidate.low_fps.percentile_1_low_fps,
                true,
                config.change_threshold_percent,
            ),
            percentile_0_1_low_fps: metric_delta(
                input.baseline.low_fps.percentile_0_1_low_fps,
                input.candidate.low_fps.percentile_0_1_low_fps,
                true,
                config.change_threshold_percent,
            ),
            mean_fps_of_worst_1pct_frames: metric_delta(
                input.baseline.low_fps.mean_fps_of_worst_1pct_frames,
                input.candidate.low_fps.mean_fps_of_worst_1pct_frames,
                true,
                config.change_threshold_percent,
            ),
            mean_fps_of_worst_0_1pct_frames: metric_delta(
                input.baseline.low_fps.mean_fps_of_worst_0_1pct_frames,
                input.candidate.low_fps.mean_fps_of_worst_0_1pct_frames,
                true,
                config.change_threshold_percent,
            ),
        },
        baseline_stutter_count: input.baseline_stutter_count,
        candidate_stutter_count: input.candidate_stutter_count,
        stutter_count_delta: input.candidate_stutter_count as i64
            - input.baseline_stutter_count as i64,
    })
}

fn validate_config(config: ComparisonConfig) -> Result<(), ComparisonError> {
    if !config.minimum_sample_ratio.is_finite()
        || config.minimum_sample_ratio <= 0.0
        || config.minimum_sample_ratio > 1.0
    {
        return Err(ComparisonError::InvalidSampleRatio);
    }
    if !config.change_threshold_percent.is_finite() || config.change_threshold_percent < 0.0 {
        return Err(ComparisonError::InvalidChangeThreshold);
    }
    Ok(())
}

fn validate_comparison_statistics(statistics: &FrameStatistics) -> Result<(), ComparisonError> {
    let metrics = [
        (statistics.average_fps, "average_fps"),
        (statistics.mean_frame_time_ms, "mean_frame_time_ms"),
        (statistics.median_frame_time_ms, "median_frame_time_ms"),
        (statistics.percentiles.p95_ms, "p95_frame_time_ms"),
        (statistics.percentiles.p99_ms, "p99_frame_time_ms"),
        (
            statistics.low_fps.percentile_1_low_fps,
            "percentile_1_low_fps",
        ),
        (
            statistics.low_fps.percentile_0_1_low_fps,
            "percentile_0_1_low_fps",
        ),
        (
            statistics.low_fps.mean_fps_of_worst_1pct_frames,
            "mean_fps_of_worst_1pct_frames",
        ),
        (
            statistics.low_fps.mean_fps_of_worst_0_1pct_frames,
            "mean_fps_of_worst_0_1pct_frames",
        ),
    ];

    for (value, name) in metrics {
        if !value.is_finite() || value <= 0.0 {
            return Err(ComparisonError::InvalidComparisonMetric(name));
        }
    }
    Ok(())
}

fn validate_low_fps_metric(value: f64, name: &'static str) -> Result<(), ComparisonError> {
    if !value.is_finite() || value <= 0.0 {
        return Err(ComparisonError::InvalidComparisonMetric(name));
    }
    Ok(())
}

fn metric_delta(
    baseline: f64,
    candidate: f64,
    higher_is_better: bool,
    threshold_percent: f64,
) -> MetricDelta {
    let absolute_delta = candidate - baseline;
    let relative_percent = if baseline == 0.0 {
        None
    } else {
        Some(absolute_delta / baseline * 100.0)
    };

    let direction = match relative_percent {
        Some(relative) if relative.abs() < threshold_percent => DeltaDirection::Unchanged,
        Some(relative)
            if (higher_is_better && relative > 0.0) || (!higher_is_better && relative < 0.0) =>
        {
            DeltaDirection::Improved
        }
        Some(_) => DeltaDirection::Regressed,
        None => DeltaDirection::Unchanged,
    };

    MetricDelta {
        baseline,
        candidate,
        absolute_delta,
        relative_percent,
        direction,
    }
}

/// Converts low-FPS metrics into serializable compatibility witnesses.
pub fn compare_low_fps(
    baseline: &LowFpsMetrics,
    candidate: &LowFpsMetrics,
    config: ComparisonConfig,
) -> Result<(MetricDelta, MetricDelta), ComparisonError> {
    validate_config(config)?;
    validate_low_fps_metric(baseline.percentile_1_low_fps, "percentile_1_low_fps")?;
    validate_low_fps_metric(candidate.percentile_1_low_fps, "percentile_1_low_fps")?;
    validate_low_fps_metric(baseline.percentile_0_1_low_fps, "percentile_0_1_low_fps")?;
    validate_low_fps_metric(candidate.percentile_0_1_low_fps, "percentile_0_1_low_fps")?;
    Ok((
        metric_delta(
            baseline.percentile_1_low_fps,
            candidate.percentile_1_low_fps,
            true,
            config.change_threshold_percent,
        ),
        metric_delta(
            baseline.percentile_0_1_low_fps,
            candidate.percentile_0_1_low_fps,
            true,
            config.change_threshold_percent,
        ),
    ))
}

#[cfg(test)]
mod tests {
    use super::{
        Comparability, ComparisonConfig, ComparisonError, ComparisonInput, DeltaDirection,
        InferenceLimit, PairedInferenceConfig, PairedInferenceOutcome, PairedRun, compare,
        compare_low_fps, minimum_pairs_for_alpha, paired_infer,
    };
    use crate::{FrameStatistics, LowFpsMetrics, PercentileSet};

    fn stats(sample_count: usize, mean_ms: f64, fps: f64) -> FrameStatistics {
        let percentiles = PercentileSet {
            p0_1_ms: mean_ms,
            p1_ms: mean_ms,
            p5_ms: mean_ms,
            p10_ms: mean_ms,
            p50_ms: mean_ms,
            p95_ms: mean_ms * 1.2,
            p99_ms: mean_ms * 1.3,
            p99_9_ms: mean_ms * 1.4,
        };
        FrameStatistics {
            sample_count,
            mean_frame_time_ms: mean_ms,
            median_frame_time_ms: mean_ms,
            average_fps: fps,
            variance_ms2: Some(1.0),
            stdev_ms: Some(1.0),
            percentiles,
            low_fps: LowFpsMetrics {
                percentile_1_low_fps: 60.0,
                percentile_0_1_low_fps: 55.0,
                mean_fps_of_worst_1pct_frames: 58.0,
                mean_fps_of_worst_0_1pct_frames: 54.0,
            },
            p999_reliable: true,
        }
    }

    #[test]
    fn marks_same_scenario_as_comparable_and_reports_improvement()
    -> Result<(), Box<dyn std::error::Error>> {
        let baseline = stats(1_000, 16.0, 62.5);
        let candidate = stats(1_000, 14.0, 71.4);
        let result = compare(
            ComparisonInput {
                baseline: &baseline,
                candidate: &candidate,
                baseline_stutter_count: 8,
                candidate_stutter_count: 3,
                same_scenario: true,
            },
            ComparisonConfig::default(),
        )?;
        assert_eq!(result.comparability, Comparability::Comparable);
        assert_eq!(result.inference_limit, InferenceLimit::DescriptiveOnly);
        assert_eq!(result.config, ComparisonConfig::default());
        assert_eq!(
            result.metrics.average_fps.direction,
            DeltaDirection::Improved
        );
        assert_eq!(
            result.metrics.mean_frame_time_ms.direction,
            DeltaDirection::Improved
        );
        assert_eq!(result.stutter_count_delta, -5);
        Ok(())
    }

    #[test]
    fn rejects_causal_interpretation_for_different_scenarios()
    -> Result<(), Box<dyn std::error::Error>> {
        let baseline = stats(1_000, 16.0, 62.5);
        let candidate = stats(1_000, 14.0, 71.4);
        let result = compare(
            ComparisonInput {
                baseline: &baseline,
                candidate: &candidate,
                baseline_stutter_count: 1,
                candidate_stutter_count: 0,
                same_scenario: false,
            },
            ComparisonConfig::default(),
        )?;
        assert_eq!(
            result.comparability,
            Comparability::InconclusiveDifferentScenario
        );
        Ok(())
    }

    #[test]
    fn rejects_invalid_alpha_before_minimum_pair_validation() {
        for alpha in [f64::NAN, 0.0, 1.0] {
            let config = PairedInferenceConfig {
                alpha,
                ..PairedInferenceConfig::default()
            };
            assert!(matches!(
                paired_infer(&[], config),
                Err(ComparisonError::InvalidAlpha)
            ));
        }
    }

    #[test]
    fn default_minimum_pairs_is_statistically_eligible() {
        assert_eq!(PairedInferenceConfig::default().minimum_pairs, 6);
    }

    #[test]
    fn minimum_pairs_recommendation_tracks_alpha() -> Result<(), ComparisonError> {
        assert_eq!(minimum_pairs_for_alpha(0.05)?, 6);
        assert_eq!(minimum_pairs_for_alpha(0.25)?, 3);
        assert_eq!(minimum_pairs_for_alpha(0.5)?, 2);
        assert_eq!(minimum_pairs_for_alpha(f64::from_bits(1))?, 1_075);
        assert_eq!(
            minimum_pairs_for_alpha(f64::from_bits(1.0_f64.to_bits() - 1))?,
            2
        );
        for alpha in [f64::NAN, 0.0, 1.0, f64::INFINITY] {
            assert_eq!(
                minimum_pairs_for_alpha(alpha),
                Err(ComparisonError::InvalidAlpha)
            );
        }
        Ok(())
    }

    #[test]
    fn paired_inference_detects_consistent_improvement() -> Result<(), Box<dyn std::error::Error>> {
        let pairs = (0..7)
            .map(|index| PairedRun {
                pair_id: format!("pair-{index}"),
                baseline_sample_count: 1_000,
                candidate_sample_count: 1_000,
                baseline_p99_frame_time_ms: 30.0,
                candidate_p99_frame_time_ms: 28.0,
            })
            .collect::<Vec<_>>();
        let result = paired_infer(&pairs, PairedInferenceConfig::default())?;
        assert_eq!(result.outcome, PairedInferenceOutcome::Improved);
        assert_eq!(result.config, PairedInferenceConfig::default());
        assert_eq!(result.improvement_wins, 7);
        assert_eq!(result.regression_losses, 0);
        assert!(result.p_value.is_some_and(|value| value < 0.05));
        Ok(())
    }

    #[test]
    fn paired_inference_detects_consistent_regression() -> Result<(), Box<dyn std::error::Error>> {
        let pairs = (0..7)
            .map(|index| PairedRun {
                pair_id: format!("pair-{index}"),
                baseline_sample_count: 1_000,
                candidate_sample_count: 1_000,
                baseline_p99_frame_time_ms: 28.0,
                candidate_p99_frame_time_ms: 31.0,
            })
            .collect::<Vec<_>>();
        let result = paired_infer(&pairs, PairedInferenceConfig::default())?;
        assert_eq!(result.outcome, PairedInferenceOutcome::Regressed);
        assert_eq!(result.regression_losses, 7);
        Ok(())
    }

    #[test]
    fn paired_inference_excludes_sample_imbalanced_pairs() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut pairs = (0..5)
            .map(|index| PairedRun {
                pair_id: format!("pair-{index}"),
                baseline_sample_count: 1_000,
                candidate_sample_count: 1_000,
                baseline_p99_frame_time_ms: 30.0,
                candidate_p99_frame_time_ms: 28.0,
            })
            .collect::<Vec<_>>();
        pairs[0].candidate_sample_count = 700;

        let result = paired_infer(&pairs, PairedInferenceConfig::default())?;
        assert_eq!(result.pair_count, 5);
        assert_eq!(result.comparable_pair_count, 4);
        assert_eq!(result.excluded_pair_count, 1);
        assert_eq!(result.outcome, PairedInferenceOutcome::Inconclusive);
        Ok(())
    }

    #[test]
    fn paired_inference_rejects_duplicate_pair_ids() {
        let pairs = vec![
            PairedRun {
                pair_id: String::from("duplicate"),
                baseline_sample_count: 1_000,
                candidate_sample_count: 1_000,
                baseline_p99_frame_time_ms: 30.0,
                candidate_p99_frame_time_ms: 28.0,
            },
            PairedRun {
                pair_id: String::from("duplicate"),
                baseline_sample_count: 1_000,
                candidate_sample_count: 1_000,
                baseline_p99_frame_time_ms: 30.0,
                candidate_p99_frame_time_ms: 28.0,
            },
        ];
        assert!(matches!(
            paired_infer(&pairs, PairedInferenceConfig::default()),
            Err(ComparisonError::InvalidPairIdentity)
        ));
    }

    #[test]
    fn paired_inference_requires_minimum_pairs() -> Result<(), Box<dyn std::error::Error>> {
        let pairs = (0..4)
            .map(|index| PairedRun {
                pair_id: format!("pair-{index}"),
                baseline_sample_count: 1_000,
                candidate_sample_count: 1_000,
                baseline_p99_frame_time_ms: 30.0,
                candidate_p99_frame_time_ms: 28.0,
            })
            .collect::<Vec<_>>();
        let result = paired_infer(&pairs, PairedInferenceConfig::default())?;
        assert_eq!(result.outcome, PairedInferenceOutcome::Inconclusive);
        Ok(())
    }

    #[test]
    fn paired_inference_calls_all_pairs_equivalent_inside_margin()
    -> Result<(), Box<dyn std::error::Error>> {
        let pairs = (0..5)
            .map(|index| PairedRun {
                pair_id: format!("pair-{index}"),
                baseline_sample_count: 1_000,
                candidate_sample_count: 1_000,
                baseline_p99_frame_time_ms: 30.0,
                candidate_p99_frame_time_ms: 29.9,
            })
            .collect::<Vec<_>>();
        let result = paired_infer(&pairs, PairedInferenceConfig::default())?;
        assert_eq!(result.outcome, PairedInferenceOutcome::Inconclusive);
        Ok(())
    }

    #[test]
    fn rejects_non_finite_comparison_metrics() {
        let mut baseline = stats(1_000, 16.0, 62.5);
        baseline.average_fps = f64::NAN;
        let candidate = stats(1_000, 16.0, 62.5);
        let result = compare(
            ComparisonInput {
                baseline: &baseline,
                candidate: &candidate,
                baseline_stutter_count: 0,
                candidate_stutter_count: 0,
                same_scenario: true,
            },
            ComparisonConfig::default(),
        );
        assert!(matches!(
            result,
            Err(ComparisonError::InvalidComparisonMetric("average_fps"))
        ));
    }

    #[test]
    fn rejects_empty_comparison_runs() {
        let baseline = stats(0, 16.0, 62.5);
        let candidate = stats(0, 16.0, 62.5);
        let result = compare(
            ComparisonInput {
                baseline: &baseline,
                candidate: &candidate,
                baseline_stutter_count: 0,
                candidate_stutter_count: 0,
                same_scenario: true,
            },
            ComparisonConfig::default(),
        );
        assert!(matches!(result, Err(ComparisonError::EmptyComparison)));
    }

    #[test]
    fn sign_test_matches_bruteforce_for_all_outcomes_up_to_twenty_pairs() {
        for n in 1..=20 {
            for tail_count in 0..=n {
                let exact = super::exact_two_sided_sign_test_p_value(n, tail_count);
                let k = tail_count.min(n - tail_count);
                let denominator = 2_f64.powi(n as i32);
                let numerator = (0..=k)
                    .map(|i| {
                        (0..i).fold(1_u128, |acc, value| {
                            acc * (n - value) as u128 / (value + 1) as u128
                        })
                    })
                    .sum::<u128>();
                let brute_force = (2.0 * numerator as f64 / denominator).min(1.0);
                assert!(
                    (exact - brute_force).abs() < 1e-12,
                    "n={n}, tail_count={tail_count}"
                );
            }
        }
    }

    #[test]
    fn sign_test_uses_exact_two_sided_probability() -> Result<(), Box<dyn std::error::Error>> {
        let pairs = (0..7)
            .map(|index| PairedRun {
                pair_id: format!("pair-{index}"),
                baseline_sample_count: 1_000,
                candidate_sample_count: 1_000,
                baseline_p99_frame_time_ms: 30.0,
                candidate_p99_frame_time_ms: 28.0,
            })
            .collect::<Vec<_>>();
        let result = paired_infer(&pairs, PairedInferenceConfig::default())?;
        assert!(
            result
                .p_value
                .is_some_and(|value| (value - 0.015625).abs() < f64::EPSILON)
        );
        Ok(())
    }

    #[test]
    fn flags_material_sample_imbalance() -> Result<(), Box<dyn std::error::Error>> {
        let baseline = stats(1_000, 16.0, 62.5);
        let candidate = stats(700, 16.0, 62.5);
        let result = compare(
            ComparisonInput {
                baseline: &baseline,
                candidate: &candidate,
                baseline_stutter_count: 0,
                candidate_stutter_count: 0,
                same_scenario: true,
            },
            ComparisonConfig::default(),
        )?;
        assert_eq!(
            result.comparability,
            Comparability::LimitedSampleEquivalence
        );
        assert!((result.sample_ratio - 0.7).abs() < 0.001);
        Ok(())
    }

    fn paired_run(id: usize, baseline_p99: f64, candidate_p99: f64) -> PairedRun {
        PairedRun {
            pair_id: format!("pair-{id}"),
            baseline_sample_count: 1_000,
            candidate_sample_count: 1_000,
            baseline_p99_frame_time_ms: baseline_p99,
            candidate_p99_frame_time_ms: candidate_p99,
        }
    }

    #[test]
    fn paired_inference_rejects_empty_and_nonfinite_pairs() {
        let mut zero_baseline = paired_run(0, 30.0, 28.0);
        zero_baseline.baseline_sample_count = 0;
        assert!(matches!(
            paired_infer(&[zero_baseline], PairedInferenceConfig::default()),
            Err(ComparisonError::InvalidPairSampleCount)
        ));

        let mut zero_candidate = paired_run(0, 30.0, 28.0);
        zero_candidate.candidate_sample_count = 0;
        assert!(matches!(
            paired_infer(&[zero_candidate], PairedInferenceConfig::default()),
            Err(ComparisonError::InvalidPairSampleCount)
        ));

        assert!(matches!(
            paired_infer(
                &[paired_run(0, f64::NAN, 28.0)],
                PairedInferenceConfig::default()
            ),
            Err(ComparisonError::InvalidPairMetric)
        ));
        assert!(matches!(
            paired_infer(
                &[paired_run(0, 30.0, 0.0)],
                PairedInferenceConfig::default()
            ),
            Err(ComparisonError::InvalidPairMetric)
        ));
        assert!(matches!(
            paired_infer(
                &[paired_run(0, -5.0, 28.0)],
                PairedInferenceConfig::default()
            ),
            Err(ComparisonError::InvalidPairMetric)
        ));
        assert!(matches!(
            paired_infer(
                &[PairedRun {
                    pair_id: String::new(),
                    ..paired_run(0, 30.0, 28.0)
                }],
                PairedInferenceConfig::default()
            ),
            Err(ComparisonError::InvalidPairIdentity)
        ));
    }

    #[test]
    fn paired_inference_keeps_ratio_boundary_pairs() -> Result<(), Box<dyn std::error::Error>> {
        let mut pair = paired_run(0, 30.0, 28.0);
        pair.candidate_sample_count = 800;
        let result = paired_infer(&[pair], PairedInferenceConfig::default())?;
        assert_eq!(result.comparable_pair_count, 1);
        assert_eq!(result.excluded_pair_count, 0);
        Ok(())
    }

    #[test]
    fn paired_inference_enforces_alpha_pair_minimum() {
        let config = PairedInferenceConfig {
            minimum_pairs: 5,
            ..PairedInferenceConfig::default()
        };
        assert!(matches!(
            paired_infer(&[], config),
            Err(ComparisonError::InvalidMinimumPairsForAlpha)
        ));
        let config = PairedInferenceConfig {
            minimum_pairs: 0,
            ..PairedInferenceConfig::default()
        };
        assert!(matches!(
            paired_infer(&[], config),
            Err(ComparisonError::InvalidMinimumPairs)
        ));
    }

    #[test]
    fn paired_inference_rejects_invalid_ratio_and_margin() {
        for ratio in [0.0, f64::NAN, 2.0] {
            let config = PairedInferenceConfig {
                minimum_sample_ratio: ratio,
                ..PairedInferenceConfig::default()
            };
            assert!(
                matches!(
                    paired_infer(&[], config),
                    Err(ComparisonError::InvalidSampleRatio)
                ),
                "ratio {ratio} must be rejected"
            );
        }
        for margin in [-1.0, f64::NAN] {
            let config = PairedInferenceConfig {
                equivalence_margin_percent: margin,
                ..PairedInferenceConfig::default()
            };
            assert!(
                matches!(
                    paired_infer(&[], config),
                    Err(ComparisonError::InvalidEquivalenceMargin)
                ),
                "margin {margin} must be rejected"
            );
        }
        let config = PairedInferenceConfig {
            equivalence_margin_percent: 0.0,
            ..PairedInferenceConfig::default()
        };
        assert!(paired_infer(&[], config).is_ok());
        let config = PairedInferenceConfig {
            minimum_sample_ratio: 1.0,
            ..PairedInferenceConfig::default()
        };
        assert!(paired_infer(&[], config).is_ok());
    }

    #[test]
    fn paired_improvement_reports_exact_median() -> Result<(), Box<dyn std::error::Error>> {
        let result = paired_infer(
            &[paired_run(0, 20.0, 10.0)],
            PairedInferenceConfig::default(),
        )?;
        assert_eq!(result.median_relative_improvement_percent, 50.0);
        assert_eq!(result.improvement_wins, 1);
        assert_eq!(result.practical_ties, 0);
        Ok(())
    }

    #[test]
    fn equivalence_margin_boundary_counts_ties() -> Result<(), Box<dyn std::error::Error>> {
        let config = PairedInferenceConfig {
            equivalence_margin_percent: 25.0,
            ..PairedInferenceConfig::default()
        };
        let result = paired_infer(&[paired_run(0, 100.0, 75.0)], config)?;
        assert_eq!(result.practical_ties, 1);
        assert_eq!(result.improvement_wins, 0);
        assert_eq!(result.regression_losses, 0);
        Ok(())
    }

    #[test]
    fn negative_margin_boundary_counts_ties() -> Result<(), Box<dyn std::error::Error>> {
        let config = PairedInferenceConfig {
            equivalence_margin_percent: 25.0,
            ..PairedInferenceConfig::default()
        };
        let result = paired_infer(&[paired_run(0, 100.0, 125.0)], config)?;
        assert_eq!(result.practical_ties, 1);
        assert_eq!(result.improvement_wins, 0);
        assert_eq!(result.regression_losses, 0);
        Ok(())
    }

    #[test]
    fn even_pair_count_uses_averaged_median() -> Result<(), Box<dyn std::error::Error>> {
        let config = PairedInferenceConfig {
            minimum_pairs: 2,
            alpha: 0.5,
            ..PairedInferenceConfig::default()
        };
        let result = paired_infer(&[paired_run(0, 8.0, 7.0), paired_run(1, 8.0, 5.0)], config)?;
        assert_eq!(result.median_relative_improvement_percent, 25.0);
        assert_eq!(result.outcome, PairedInferenceOutcome::Improved);
        Ok(())
    }

    #[test]
    fn weak_direction_without_significance_stays_inconclusive()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut pairs = vec![paired_run(0, 100.0, 75.0)];
        for id in 1..6 {
            pairs.push(paired_run(id, 100.0, 125.0));
        }
        let result = paired_infer(&pairs, PairedInferenceConfig::default())?;
        assert_eq!(result.improvement_wins, 1);
        assert_eq!(result.regression_losses, 5);
        assert_eq!(result.outcome, PairedInferenceOutcome::Inconclusive);
        Ok(())
    }

    #[test]
    fn rejects_invalid_comparison_config() {
        let baseline = stats(1_000, 16.0, 62.5);
        let candidate = stats(1_000, 16.0, 62.5);
        let input = ComparisonInput {
            baseline: &baseline,
            candidate: &candidate,
            baseline_stutter_count: 0,
            candidate_stutter_count: 0,
            same_scenario: true,
        };
        for ratio in [0.0, f64::NAN, 2.0] {
            let config = ComparisonConfig {
                minimum_sample_ratio: ratio,
                ..ComparisonConfig::default()
            };
            assert!(
                matches!(
                    compare(input, config),
                    Err(ComparisonError::InvalidSampleRatio)
                ),
                "ratio {ratio} must be rejected"
            );
        }
        for threshold in [-1.0, f64::NAN] {
            let config = ComparisonConfig {
                change_threshold_percent: threshold,
                ..ComparisonConfig::default()
            };
            assert!(
                matches!(
                    compare(input, config),
                    Err(ComparisonError::InvalidChangeThreshold)
                ),
                "threshold {threshold} must be rejected"
            );
        }
        assert!(
            compare(
                input,
                ComparisonConfig {
                    change_threshold_percent: 0.0,
                    ..ComparisonConfig::default()
                }
            )
            .is_ok()
        );
        assert!(
            compare(
                input,
                ComparisonConfig {
                    minimum_sample_ratio: 1.0,
                    ..ComparisonConfig::default()
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn boundary_sample_ratio_stays_comparable() -> Result<(), Box<dyn std::error::Error>> {
        let baseline = stats(1_000, 16.0, 62.5);
        let candidate = stats(800, 16.0, 62.5);
        let result = compare(
            ComparisonInput {
                baseline: &baseline,
                candidate: &candidate,
                baseline_stutter_count: 0,
                candidate_stutter_count: 0,
                same_scenario: true,
            },
            ComparisonConfig::default(),
        )?;
        assert_eq!(result.comparability, Comparability::Comparable);
        Ok(())
    }

    #[test]
    fn rejects_single_empty_comparison_run() {
        let empty = stats(0, 16.0, 62.5);
        let full = stats(1_000, 16.0, 62.5);
        let result = compare(
            ComparisonInput {
                baseline: &empty,
                candidate: &full,
                baseline_stutter_count: 0,
                candidate_stutter_count: 0,
                same_scenario: true,
            },
            ComparisonConfig::default(),
        );
        assert!(matches!(result, Err(ComparisonError::EmptyComparison)));
    }

    #[test]
    fn rejects_nonpositive_low_fps_metrics() {
        let valid = LowFpsMetrics {
            percentile_1_low_fps: 60.0,
            percentile_0_1_low_fps: 55.0,
            mean_fps_of_worst_1pct_frames: 58.0,
            mean_fps_of_worst_0_1pct_frames: 54.0,
        };
        for invalid in [f64::NAN, 0.0, -3.0] {
            let baseline = LowFpsMetrics {
                percentile_1_low_fps: invalid,
                ..valid
            };
            assert!(
                matches!(
                    compare_low_fps(&baseline, &valid, ComparisonConfig::default()),
                    Err(ComparisonError::InvalidComparisonMetric(_))
                ),
                "metric {invalid} must be rejected"
            );
        }
    }

    #[test]
    fn accepts_positive_low_fps_metrics() -> Result<(), Box<dyn std::error::Error>> {
        let valid = LowFpsMetrics {
            percentile_1_low_fps: 60.0,
            percentile_0_1_low_fps: 55.0,
            mean_fps_of_worst_1pct_frames: 58.0,
            mean_fps_of_worst_0_1pct_frames: 54.0,
        };
        let (first, second) = compare_low_fps(&valid, &valid, ComparisonConfig::default())?;
        assert_eq!(first.direction, DeltaDirection::Unchanged);
        assert_eq!(second.direction, DeltaDirection::Unchanged);
        Ok(())
    }

    #[test]
    fn metric_delta_pins_threshold_boundary_and_direction() -> Result<(), Box<dyn std::error::Error>>
    {
        let threshold_config = ComparisonConfig {
            change_threshold_percent: 12.5,
            ..ComparisonConfig::default()
        };
        let baseline = stats(1_000, 8.0, 62.5);
        let candidate = stats(1_000, 9.0, 62.5);
        let result = compare(
            ComparisonInput {
                baseline: &baseline,
                candidate: &candidate,
                baseline_stutter_count: 0,
                candidate_stutter_count: 0,
                same_scenario: true,
            },
            threshold_config,
        )?;
        assert_eq!(
            result.metrics.mean_frame_time_ms.direction,
            DeltaDirection::Regressed
        );

        let candidate = stats(1_000, 8.5, 62.5);
        let result = compare(
            ComparisonInput {
                baseline: &baseline,
                candidate: &candidate,
                baseline_stutter_count: 0,
                candidate_stutter_count: 0,
                same_scenario: true,
            },
            threshold_config,
        )?;
        assert_eq!(
            result.metrics.mean_frame_time_ms.direction,
            DeltaDirection::Unchanged
        );

        let slow_fps = stats(1_000, 16.0, 30.0);
        let fast_fps = stats(1_000, 16.0, 60.0);
        let result = compare(
            ComparisonInput {
                baseline: &fast_fps,
                candidate: &slow_fps,
                baseline_stutter_count: 0,
                candidate_stutter_count: 0,
                same_scenario: true,
            },
            ComparisonConfig::default(),
        )?;
        assert_eq!(
            result.metrics.average_fps.direction,
            DeltaDirection::Regressed
        );

        let zero_config = ComparisonConfig {
            change_threshold_percent: 0.0,
            ..ComparisonConfig::default()
        };
        let same_a = stats(1_000, 16.0, 60.0);
        let same_b = stats(1_000, 16.0, 60.0);
        let result = compare(
            ComparisonInput {
                baseline: &same_a,
                candidate: &same_b,
                baseline_stutter_count: 0,
                candidate_stutter_count: 0,
                same_scenario: true,
            },
            zero_config,
        )?;
        assert_eq!(
            result.metrics.average_fps.direction,
            DeltaDirection::Regressed
        );
        assert_eq!(
            result.metrics.mean_frame_time_ms.direction,
            DeltaDirection::Regressed
        );
        Ok(())
    }
}
