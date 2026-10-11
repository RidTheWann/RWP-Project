//! Deterministic report serialization and rendering boundary.

use rwp_analysis::{
    FrameStatistics, RefreshAwareStutterConfig, StatisticsConfig, StutterConfig,
    compute_statistics, detect_stutters, detect_stutters_refresh_aware,
};
use rwp_core::{Diagnostic, FrameSample, StutterEvent};
use serde::{Deserialize, Serialize};

/// Current JSON schema version emitted by the report builder.
pub const REPORT_SCHEMA_VERSION: &str = "rwp.report.v2";
/// Schema identifier for the version 1 report contract.
pub const REPORT_SCHEMA_VERSION_V1: &str = "rwp.report.v1";

const MAX_REPORT_JSON_BYTES: usize = 64 * 1024 * 1024;
const MAX_REPORT_JSON_DEPTH: usize = 64;

/// JSON input contract accepted by the M4 CLI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaptureDocument {
    /// Input schema identifier.
    pub schema_version: String,
    /// Normalized frame samples in capture order.
    pub frames: Vec<FrameSample>,
}

/// Reproducibility metadata for the deterministic analysis settings.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AnalysisConfigSummary {
    /// P99.9 reliability threshold in frame samples.
    pub p999_reliability_threshold: usize,
    /// Stable detector rule identifier applied to the report.
    #[serde(default)]
    pub stutter_rule_id: String,
    /// Refresh rate supplied to the refresh-aware detector, when used.
    #[serde(default)]
    pub stutter_refresh_rate_hz: Option<f64>,
    /// Display refresh period derived from the refresh rate, when used.
    #[serde(default)]
    pub stutter_refresh_period_ms: Option<f64>,
    /// Local-median half-window in frames.
    pub stutter_rolling_half_window_frames: usize,
    /// Stutter candidate multiplier.
    pub stutter_multiplier: f64,
    /// Legacy V1 absolute threshold in milliseconds. The V2 refresh-aware
    /// rule derives its threshold from the local baseline plus the display
    /// refresh period, so V2 reports carry the explicit not-applicable
    /// sentinel 0.0 (V1 validation requires a positive threshold, keeping
    /// the sentinel unambiguous).
    pub stutter_absolute_threshold_ms: f64,
    /// Minimum baseline neighbors.
    pub stutter_minimum_baseline_samples: usize,
    /// Candidate merge gap in milliseconds.
    pub stutter_merge_gap_ms: f64,
}

/// Deterministic report produced from one normalized frame capture.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PerformanceReport {
    /// Report schema identifier.
    pub schema_version: String,
    /// Number of frame observations analyzed.
    pub frame_count: usize,
    /// Complete deterministic frame-time statistics.
    pub frame_statistics: FrameStatistics,
    /// Detected stutter events.
    pub stutter_events: Vec<StutterEvent>,
    /// Exact analysis settings used to build the report.
    pub analysis_config: AnalysisConfigSummary,
    /// Deterministic diagnosis results attached after system telemetry analysis.
    #[serde(default)]
    pub diagnostics: Vec<Diagnostic>,
    /// Whether the capture ended by explicit user interruption.
    #[serde(default)]
    pub interrupted: bool,
}

/// Explicit stutter analysis mode used to build a deterministic report.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StutterAnalysisConfig {
    /// Legacy absolute-threshold rule.
    V1(StutterConfig),
    /// Refresh-aware rule with explicit display refresh rate.
    V2(RefreshAwareStutterConfig),
}

struct StutterAnalysisOutput {
    events: Vec<StutterEvent>,
    rule_id: String,
    refresh_rate_hz: Option<f64>,
    refresh_period_ms: Option<f64>,
}

impl StutterAnalysisConfig {
    fn detect(self, frames: &[FrameSample]) -> Result<StutterAnalysisOutput, ReportError> {
        match self {
            Self::V1(config) => Ok(StutterAnalysisOutput {
                events: detect_stutters(frames, config)?,
                rule_id: String::from("stutter.v1.absolute"),
                refresh_rate_hz: None,
                refresh_period_ms: None,
            }),
            Self::V2(config) => {
                let refresh_period_ms = 1_000.0 / config.refresh_rate_hz;
                Ok(StutterAnalysisOutput {
                    events: detect_stutters_refresh_aware(frames, config)?,
                    rule_id: String::from("stutter.v2.refresh_aware"),
                    refresh_rate_hz: Some(config.refresh_rate_hz),
                    refresh_period_ms: Some(refresh_period_ms),
                })
            }
        }
    }
}

/// Errors raised while building a report.
#[derive(Debug, thiserror::Error)]
pub enum ReportError {
    /// Report JSON could not be decoded.
    #[error("invalid report JSON: {0}")]
    InvalidReportJson(String),
    /// The report schema is not understood by this analyzer.
    #[error("unsupported report schema: {0}")]
    UnsupportedReportSchema(String),
    /// Statistics could not be computed.
    #[error("statistics analysis failed: {0}")]
    Statistics(#[from] rwp_analysis::StatisticsError),
    /// Stutter analysis could not be computed.
    #[error("stutter analysis failed: {0}")]
    Stutter(#[from] rwp_analysis::StutterError),
    /// The capture schema is not understood by this analyzer.
    #[error("unsupported capture schema: {0}")]
    UnsupportedCaptureSchema(String),
}

/// Builds a deterministic report using the legacy V1 stutter detector.
pub fn build_report(
    capture: &CaptureDocument,
    statistics_config: StatisticsConfig,
    stutter_config: StutterConfig,
) -> Result<PerformanceReport, ReportError> {
    build_report_with_stutter_analysis(
        capture,
        statistics_config,
        StutterAnalysisConfig::V1(stutter_config),
        false,
    )
}

/// Builds a deterministic report using explicit stutter analysis and interruption status.
pub fn build_report_with_stutter_analysis(
    capture: &CaptureDocument,
    statistics_config: StatisticsConfig,
    stutter_config: StutterAnalysisConfig,
    interrupted: bool,
) -> Result<PerformanceReport, ReportError> {
    if capture.schema_version != "rwp.capture.v1" {
        return Err(ReportError::UnsupportedCaptureSchema(
            capture.schema_version.clone(),
        ));
    }

    let frame_times = capture
        .frames
        .iter()
        .map(|frame| frame.frame_time_ms)
        .collect::<Vec<_>>();
    let frame_statistics = compute_statistics(&frame_times, statistics_config)?;
    let stutter_analysis = stutter_config.detect(&capture.frames)?;

    Ok(PerformanceReport {
        schema_version: String::from(REPORT_SCHEMA_VERSION),
        frame_count: capture.frames.len(),
        frame_statistics,
        stutter_events: stutter_analysis.events,
        analysis_config: AnalysisConfigSummary {
            p999_reliability_threshold: statistics_config.p999_reliability_threshold,
            stutter_rule_id: stutter_analysis.rule_id,
            stutter_refresh_rate_hz: stutter_analysis.refresh_rate_hz,
            stutter_refresh_period_ms: stutter_analysis.refresh_period_ms,
            stutter_rolling_half_window_frames: match stutter_config {
                StutterAnalysisConfig::V1(config) => config.rolling_half_window_frames,
                StutterAnalysisConfig::V2(config) => config.rolling_half_window_frames,
            },
            stutter_multiplier: match stutter_config {
                StutterAnalysisConfig::V1(config) => config.multiplier,
                StutterAnalysisConfig::V2(config) => config.multiplier,
            },
            stutter_absolute_threshold_ms: match stutter_config {
                StutterAnalysisConfig::V1(config) => config.absolute_threshold_ms,
                // The V2 rule has no absolute threshold; 0.0 is the explicit
                // not-applicable sentinel (V1 requires a positive value).
                StutterAnalysisConfig::V2(_) => 0.0,
            },
            stutter_minimum_baseline_samples: match stutter_config {
                StutterAnalysisConfig::V1(config) => config.minimum_baseline_samples,
                StutterAnalysisConfig::V2(config) => config.minimum_baseline_samples,
            },
            stutter_merge_gap_ms: match stutter_config {
                StutterAnalysisConfig::V1(config) => config.merge_gap_ms,
                StutterAnalysisConfig::V2(config) => config.merge_gap_ms,
            },
        },
        diagnostics: Vec::new(),
        interrupted,
    })
}

/// Attaches deterministic diagnosis results to an existing report.
pub fn with_diagnostics(
    mut report: PerformanceReport,
    diagnostics: Vec<Diagnostic>,
) -> PerformanceReport {
    report.diagnostics = diagnostics;
    report
}

/// Builds a deterministic V1 report and records whether the capture was interrupted.
pub fn build_report_with_status(
    capture: &CaptureDocument,
    statistics_config: StatisticsConfig,
    stutter_config: StutterConfig,
    interrupted: bool,
) -> Result<PerformanceReport, ReportError> {
    build_report_with_stutter_analysis(
        capture,
        statistics_config,
        StutterAnalysisConfig::V1(stutter_config),
        interrupted,
    )
}

fn ensure_json_depth(bytes: &[u8], limit: usize) -> Result<(), ReportError> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;

    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth = depth.saturating_add(1);
                if depth > limit {
                    return Err(ReportError::InvalidReportJson(format!(
                        "JSON nesting depth exceeds the configured limit ({depth} > {limit})"
                    )));
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

/// Serializes a report as deterministic compact JSON.
pub fn to_json(report: &PerformanceReport) -> Result<String, serde_json::Error> {
    serde_json::to_string(report)
}

/// Serializes a report as deterministic human-readable JSON.
pub fn to_pretty_json(report: &PerformanceReport) -> Result<String, serde_json::Error> {
    serde_json::to_string_pretty(report)
}

/// Deserializes and validates a deterministic performance report.
pub fn from_json(input: &str) -> Result<PerformanceReport, ReportError> {
    if input.len() > MAX_REPORT_JSON_BYTES {
        return Err(ReportError::InvalidReportJson(format!(
            "report JSON exceeds the configured limit ({} > {} bytes)",
            input.len(),
            MAX_REPORT_JSON_BYTES
        )));
    }
    ensure_json_depth(input.as_bytes(), MAX_REPORT_JSON_DEPTH)?;
    let report: PerformanceReport = serde_json::from_str(input)
        .map_err(|error| ReportError::InvalidReportJson(error.to_string()))?;
    if report.schema_version != REPORT_SCHEMA_VERSION
        && report.schema_version != REPORT_SCHEMA_VERSION_V1
    {
        return Err(ReportError::UnsupportedReportSchema(report.schema_version));
    }
    if report.frame_count != report.frame_statistics.sample_count {
        return Err(ReportError::InvalidReportJson(format!(
            "frame_count {} does not match frame_statistics.sample_count {}",
            report.frame_count, report.frame_statistics.sample_count
        )));
    }
    // Deserialization bypasses `Diagnostic::new`, so re-enforce the
    // constructor's 0..=100 evidence-score contract on untrusted input.
    for diagnostic in &report.diagnostics {
        if diagnostic.evidence_score > 100 {
            return Err(ReportError::InvalidReportJson(format!(
                "diagnostic '{}' evidence_score {} exceeds the supported 0..=100 range",
                diagnostic.rule_id, diagnostic.evidence_score
            )));
        }
    }
    validate_stutter_events(&report.stutter_events)?;
    validate_frame_statistics(&report.frame_statistics)?;
    validate_analysis_config(&report.analysis_config)?;
    Ok(report)
}

/// Rejects stutter events that violate detector output invariants.
fn validate_stutter_events(events: &[rwp_core::StutterEvent]) -> Result<(), ReportError> {
    for (index, event) in events.iter().enumerate() {
        if event.start_timestamp_ns > event.end_timestamp_ns {
            return Err(ReportError::InvalidReportJson(format!(
                "stutter_events[{index}] starts after it ends ({} > {})",
                event.start_timestamp_ns, event.end_timestamp_ns
            )));
        }
        if !event.peak_frame_time_ms.is_finite() || event.peak_frame_time_ms <= 0.0 {
            return Err(ReportError::InvalidReportJson(format!(
                "stutter_events[{index}].peak_frame_time_ms must be finite and greater than zero"
            )));
        }
        if !event.baseline_frame_time_ms.is_finite() || event.baseline_frame_time_ms <= 0.0 {
            return Err(ReportError::InvalidReportJson(format!(
                "stutter_events[{index}].baseline_frame_time_ms must be finite and greater than zero"
            )));
        }
        if event.candidate_frame_count == 0 {
            return Err(ReportError::InvalidReportJson(format!(
                "stutter_events[{index}].candidate_frame_count must be greater than zero"
            )));
        }
    }
    Ok(())
}

/// Rejects frame statistics that violate measurement invariants.
fn validate_frame_statistics(statistics: &FrameStatistics) -> Result<(), ReportError> {
    fn positive(name: &str, value: f64) -> Result<(), ReportError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(ReportError::InvalidReportJson(format!(
                "frame_statistics.{name} must be finite and greater than zero"
            )));
        }
        Ok(())
    }
    fn non_negative(name: &str, value: Option<f64>) -> Result<(), ReportError> {
        if let Some(value) = value
            && (!value.is_finite() || value < 0.0)
        {
            return Err(ReportError::InvalidReportJson(format!(
                "frame_statistics.{name} must be finite and non-negative"
            )));
        }
        Ok(())
    }
    positive("mean_frame_time_ms", statistics.mean_frame_time_ms)?;
    positive("median_frame_time_ms", statistics.median_frame_time_ms)?;
    positive("average_fps", statistics.average_fps)?;
    non_negative("variance_ms2", statistics.variance_ms2)?;
    non_negative("stdev_ms", statistics.stdev_ms)?;
    let percentiles = [
        ("p0_1_ms", statistics.percentiles.p0_1_ms),
        ("p1_ms", statistics.percentiles.p1_ms),
        ("p5_ms", statistics.percentiles.p5_ms),
        ("p10_ms", statistics.percentiles.p10_ms),
        ("p50_ms", statistics.percentiles.p50_ms),
        ("p95_ms", statistics.percentiles.p95_ms),
        ("p99_ms", statistics.percentiles.p99_ms),
        ("p99_9_ms", statistics.percentiles.p99_9_ms),
    ];
    for (name, value) in percentiles {
        positive(&format!("percentiles.{name}"), value)?;
    }
    let low_fps = [
        (
            "percentile_1_low_fps",
            statistics.low_fps.percentile_1_low_fps,
        ),
        (
            "percentile_0_1_low_fps",
            statistics.low_fps.percentile_0_1_low_fps,
        ),
        (
            "mean_fps_of_worst_1pct_frames",
            statistics.low_fps.mean_fps_of_worst_1pct_frames,
        ),
        (
            "mean_fps_of_worst_0_1pct_frames",
            statistics.low_fps.mean_fps_of_worst_0_1pct_frames,
        ),
    ];
    for (name, value) in low_fps {
        if !value.is_finite() || value < 0.0 {
            return Err(ReportError::InvalidReportJson(format!(
                "frame_statistics.low_fps.{name} must be finite and non-negative"
            )));
        }
    }
    Ok(())
}

/// Rejects analysis configurations that mirror invalid detector settings.
///
/// `stutter_absolute_threshold_ms` is intentionally not checked: V2 reports
/// carry the documented 0.0 not-applicable sentinel there.
fn validate_analysis_config(config: &AnalysisConfigSummary) -> Result<(), ReportError> {
    if config.stutter_rolling_half_window_frames == 0 {
        return Err(ReportError::InvalidReportJson(String::from(
            "analysis_config.stutter_rolling_half_window_frames must be greater than zero",
        )));
    }
    if !config.stutter_multiplier.is_finite() || config.stutter_multiplier <= 0.0 {
        return Err(ReportError::InvalidReportJson(String::from(
            "analysis_config.stutter_multiplier must be finite and greater than zero",
        )));
    }
    if config.stutter_minimum_baseline_samples == 0 {
        return Err(ReportError::InvalidReportJson(String::from(
            "analysis_config.stutter_minimum_baseline_samples must be greater than zero",
        )));
    }
    if !config.stutter_merge_gap_ms.is_finite() || config.stutter_merge_gap_ms < 0.0 {
        return Err(ReportError::InvalidReportJson(String::from(
            "analysis_config.stutter_merge_gap_ms must be finite and non-negative",
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        CaptureDocument, REPORT_SCHEMA_VERSION, StutterAnalysisConfig, build_report,
        build_report_with_status, build_report_with_stutter_analysis, from_json, to_json,
        to_pretty_json,
    };
    use proptest::prelude::*;
    use rwp_analysis::{RefreshAwareStutterConfig, StatisticsConfig, StutterConfig};
    use rwp_core::{Diagnostic, DiagnosticCategory};

    fn capture() -> Result<CaptureDocument, rwp_core::CoreError> {
        let mut frames = Vec::new();
        for (index, frame_time_ms) in [8.0, 8.1, 8.2, 40.0, 8.0, 8.1, 8.2].into_iter().enumerate() {
            frames.push(rwp_core::FrameSample::new(
                index as u64 * 8_000_000,
                frame_time_ms,
            )?);
        }
        Ok(CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames,
        })
    }

    proptest! {
        #[test]
        fn arbitrary_report_json_text_does_not_panic(
            input in prop::collection::vec(any::<char>(), 0..512)
                .prop_map(|chars| chars.into_iter().collect::<String>())
        ) {
            let _ = from_json(&input);
        }
    }

    const REPORT_SEED_JSON: &str =
        include_str!("../../../fuzz/corpus/report_json/report.v2.golden.json");

    fn mutate_seed_text(seed: &str, cases: usize, mut apply: impl FnMut(&str)) {
        let mut state = 0xD1B5_4A32_7E13_5F19_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 56) as u8
        };
        let seed_bytes = seed.as_bytes();
        for _ in 0..cases {
            let mut input = seed_bytes.to_vec();
            let operations = usize::from(next() % 4);
            for _ in 0..=operations {
                let index = usize::from(next()) % (input.len() + 1);
                match next() % 3 {
                    0 => {
                        if index < input.len() {
                            input[index] ^= next() | 1;
                        }
                    }
                    1 => {
                        if index < input.len() {
                            input.remove(index);
                        }
                    }
                    _ => {
                        if input.len() < 4096 {
                            input.insert(index.min(input.len()), next());
                        }
                    }
                }
            }
            if let Ok(text) = std::str::from_utf8(&input) {
                apply(text);
            }
        }
    }

    #[test]
    fn seeded_report_mutations_do_not_panic() {
        assert!(from_json(REPORT_SEED_JSON).is_ok());
        mutate_seed_text(REPORT_SEED_JSON, 256, |text| {
            let _ = from_json(text);
        });
    }

    #[test]
    fn report_json_rejects_excessive_nesting() -> Result<(), Box<dyn std::error::Error>> {
        let input = format!("{}0{}", "[".repeat(65), "]".repeat(65));
        assert!(from_json(&input).is_err());
        Ok(())
    }

    #[test]
    fn report_json_depth_guard_ignores_brackets_inside_strings()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(super::ensure_json_depth(br#"{ "value": "[[[[[[" }"#, 1).is_ok());
        Ok(())
    }

    #[test]
    fn report_contains_schema_and_stats() -> Result<(), Box<dyn std::error::Error>> {
        let report = build_report(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig {
                minimum_baseline_samples: 3,
                ..StutterConfig::default()
            },
        )?;

        assert_eq!(report.schema_version, REPORT_SCHEMA_VERSION);
        assert!(!report.interrupted);
        assert_eq!(report.frame_count, 7);
        assert_eq!(report.frame_statistics.sample_count, 7);
        assert!(!report.stutter_events.is_empty());

        let compact = to_json(&report)?;
        let pretty = to_pretty_json(&report)?;
        assert!(compact.contains(r#""schema_version":"rwp.report.v2""#));
        assert!(pretty.contains(r#""stutter_events""#));
        Ok(())
    }

    #[test]
    fn interrupted_report_is_explicitly_marked() -> Result<(), Box<dyn std::error::Error>> {
        let report = build_report_with_status(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig {
                minimum_baseline_samples: 3,
                ..StutterConfig::default()
            },
            true,
        )?;
        assert!(report.interrupted);
        assert_eq!(report.schema_version, REPORT_SCHEMA_VERSION);
        Ok(())
    }

    #[test]
    fn refresh_aware_report_records_rule_and_refresh_provenance()
    -> Result<(), Box<dyn std::error::Error>> {
        let report = build_report_with_stutter_analysis(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterAnalysisConfig::V2(RefreshAwareStutterConfig::new(144.0)?),
            false,
        )?;
        assert_eq!(
            report.analysis_config.stutter_rule_id,
            "stutter.v2.refresh_aware"
        );
        assert_eq!(report.analysis_config.stutter_refresh_rate_hz, Some(144.0));
        assert!(report.analysis_config.stutter_refresh_period_ms.is_some());
        assert!(report.stutter_events.iter().all(|event| {
            event
                .detector_parameters
                .iter()
                .any(|(key, value)| key == "rule_id" && value == "stutter.v2.refresh_aware")
        }));
        Ok(())
    }

    #[test]
    fn diagnostics_are_persisted_in_report_json() -> Result<(), Box<dyn std::error::Error>> {
        let report = build_report(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig {
                minimum_baseline_samples: 3,
                ..StutterConfig::default()
            },
        )?;
        let diagnostic = Diagnostic::new(
            DiagnosticCategory::Unknown,
            String::from("unknown"),
            0,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            0,
            1,
        )?;
        let report = super::with_diagnostics(report, vec![diagnostic.clone()]);
        let encoded = to_json(&report)?;
        let decoded = super::from_json(&encoded)?;
        assert_eq!(decoded.diagnostics, vec![diagnostic]);
        Ok(())
    }

    #[test]
    fn report_json_round_trips_for_comparison() -> Result<(), Box<dyn std::error::Error>> {
        let report = build_report(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig {
                minimum_baseline_samples: 3,
                ..StutterConfig::default()
            },
        )?;
        let encoded = to_json(&report)?;
        let decoded = super::from_json(&encoded)?;
        assert_eq!(decoded, report);

        let repeated_report = build_report(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig {
                minimum_baseline_samples: 3,
                ..StutterConfig::default()
            },
        )?;
        assert_eq!(to_json(&repeated_report)?, encoded);
        Ok(())
    }

    #[test]
    fn rejects_unknown_capture_schema() {
        let capture = CaptureDocument {
            schema_version: String::from("other"),
            frames: Vec::new(),
        };
        let result = build_report(
            &capture,
            StatisticsConfig::default(),
            StutterConfig::default(),
        );
        assert!(matches!(
            result,
            Err(super::ReportError::UnsupportedCaptureSchema(_))
        ));
    }

    #[test]
    fn v2_report_records_actual_minimum_baseline_and_merge_gap()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = RefreshAwareStutterConfig {
            minimum_baseline_samples: 3,
            merge_gap_ms: 25.0,
            ..RefreshAwareStutterConfig::new(144.0)?
        };
        let report = build_report_with_stutter_analysis(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterAnalysisConfig::V2(config),
            false,
        )?;
        assert_eq!(
            report.analysis_config.stutter_rule_id,
            "stutter.v2.refresh_aware"
        );
        assert_eq!(report.analysis_config.stutter_minimum_baseline_samples, 3);
        assert_eq!(report.analysis_config.stutter_merge_gap_ms, 25.0);
        Ok(())
    }

    #[test]
    fn v2_report_marks_unused_absolute_threshold_not_applicable()
    -> Result<(), Box<dyn std::error::Error>> {
        let report = build_report_with_stutter_analysis(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterAnalysisConfig::V2(RefreshAwareStutterConfig::new(144.0)?),
            false,
        )?;
        assert_eq!(
            report.analysis_config.stutter_rule_id,
            "stutter.v2.refresh_aware"
        );
        assert_eq!(report.analysis_config.stutter_absolute_threshold_ms, 0.0);
        Ok(())
    }
    #[test]
    fn v1_report_records_actual_legacy_thresholds() -> Result<(), Box<dyn std::error::Error>> {
        let report = build_report(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig {
                absolute_threshold_ms: 20.0,
                minimum_baseline_samples: 3,
                merge_gap_ms: 25.0,
                ..StutterConfig::default()
            },
        )?;
        assert_eq!(
            report.analysis_config.stutter_rule_id,
            "stutter.v1.absolute"
        );
        assert_eq!(report.analysis_config.stutter_absolute_threshold_ms, 20.0);
        assert_eq!(report.analysis_config.stutter_minimum_baseline_samples, 3);
        assert_eq!(report.analysis_config.stutter_merge_gap_ms, 25.0);
        Ok(())
    }

    fn diagnostic_with_score(evidence_score: u8) -> Diagnostic {
        Diagnostic {
            category: DiagnosticCategory::Unknown,
            rule_id: String::from("unknown"),
            evidence_score,
            supporting_evidence: Vec::new(),
            contradicting_evidence: Vec::new(),
            parameters: Vec::new(),
            sample_count: 0,
            minimum_sample_count: 1,
        }
    }

    fn report_json_with_diagnostic_score(
        evidence_score: u8,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let report = build_report(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig {
                minimum_baseline_samples: 3,
                ..StutterConfig::default()
            },
        )?;
        let report = super::with_diagnostics(report, vec![diagnostic_with_score(evidence_score)]);
        Ok(to_json(&report)?)
    }

    #[test]
    fn report_json_rejects_evidence_score_above_maximum() -> Result<(), Box<dyn std::error::Error>>
    {
        for score in [101_u8, 255_u8] {
            let encoded = report_json_with_diagnostic_score(score)?;
            assert!(
                matches!(
                    from_json(&encoded),
                    Err(super::ReportError::InvalidReportJson(_))
                ),
                "evidence_score {score} must be rejected"
            );
        }
        Ok(())
    }

    fn report_json_value() -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let report = build_report(
            &capture()?,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig {
                minimum_baseline_samples: 3,
                ..StutterConfig::default()
            },
        )?;
        Ok(serde_json::from_str(&to_json(&report)?)?)
    }

    fn stutter_event_json(
        start: u64,
        end: u64,
        peak: f64,
        baseline: f64,
        candidates: u32,
    ) -> serde_json::Value {
        serde_json::json!({
            "start_timestamp_ns": start,
            "end_timestamp_ns": end,
            "peak_frame_time_ms": peak,
            "baseline_frame_time_ms": baseline,
            "candidate_frame_count": candidates,
            "detector_parameters": [],
        })
    }

    #[test]
    fn report_json_rejects_inverted_stutter_window() -> Result<(), Box<dyn std::error::Error>> {
        let mut value = report_json_value()?;
        value["stutter_events"] = serde_json::json!([stutter_event_json(200, 100, 40.0, 8.0, 2)]);
        assert!(
            matches!(
                from_json(&serde_json::to_string(&value)?),
                Err(super::ReportError::InvalidReportJson(_))
            ),
            "stutter event with start > end must be rejected"
        );
        Ok(())
    }

    #[test]
    fn report_json_rejects_nonpositive_stutter_magnitudes() -> Result<(), Box<dyn std::error::Error>>
    {
        for (peak, baseline, candidates) in
            [(0.0, 8.0, 2_u32), (40.0, 0.0, 2_u32), (40.0, 8.0, 0_u32)]
        {
            let mut value = report_json_value()?;
            value["stutter_events"] =
                serde_json::json!([stutter_event_json(100, 200, peak, baseline, candidates)]);
            assert!(
                matches!(
                    from_json(&serde_json::to_string(&value)?),
                    Err(super::ReportError::InvalidReportJson(_))
                ),
                "peak {peak}, baseline {baseline}, candidates {candidates} must be rejected"
            );
        }
        Ok(())
    }

    #[test]
    fn report_json_accepts_valid_stutter_event() -> Result<(), Box<dyn std::error::Error>> {
        let mut value = report_json_value()?;
        value["stutter_events"] = serde_json::json!([stutter_event_json(100, 200, 40.0, 8.0, 2)]);
        let decoded = from_json(&serde_json::to_string(&value)?)?;
        assert_eq!(decoded.stutter_events.len(), 1);
        Ok(())
    }

    #[test]
    fn report_json_rejects_nonpositive_statistics() -> Result<(), Box<dyn std::error::Error>> {
        for (field, value) in [
            ("mean_frame_time_ms", -1.0),
            ("mean_frame_time_ms", 0.0),
            ("median_frame_time_ms", 0.0),
            ("average_fps", 0.0),
        ] {
            let mut report = report_json_value()?;
            report["frame_statistics"][field] = serde_json::json!(value);
            assert!(
                matches!(
                    from_json(&serde_json::to_string(&report)?),
                    Err(super::ReportError::InvalidReportJson(_))
                ),
                "{field} = {value} must be rejected"
            );
        }
        let mut report = report_json_value()?;
        report["frame_statistics"]["variance_ms2"] = serde_json::json!(-1.0);
        assert!(
            matches!(
                from_json(&serde_json::to_string(&report)?),
                Err(super::ReportError::InvalidReportJson(_))
            ),
            "negative variance must be rejected"
        );
        Ok(())
    }

    #[test]
    fn report_json_rejects_invalid_analysis_config() -> Result<(), Box<dyn std::error::Error>> {
        let mut report = report_json_value()?;
        report["analysis_config"]["stutter_multiplier"] = serde_json::json!(0.0);
        assert!(
            matches!(
                from_json(&serde_json::to_string(&report)?),
                Err(super::ReportError::InvalidReportJson(_))
            ),
            "zero multiplier must be rejected"
        );
        let mut report = report_json_value()?;
        report["analysis_config"]["stutter_rolling_half_window_frames"] = serde_json::json!(0);
        assert!(
            matches!(
                from_json(&serde_json::to_string(&report)?),
                Err(super::ReportError::InvalidReportJson(_))
            ),
            "zero half-window must be rejected"
        );
        Ok(())
    }

    #[test]
    fn report_json_accepts_maximum_evidence_score() -> Result<(), Box<dyn std::error::Error>> {
        let encoded = report_json_with_diagnostic_score(100)?;
        let decoded = from_json(&encoded)?;
        assert_eq!(decoded.diagnostics, vec![diagnostic_with_score(100)]);
        Ok(())
    }

    #[test]
    fn comparison_and_paired_inference_determinism_and_serde_round_trip()
    -> Result<(), Box<dyn std::error::Error>> {
        let baseline = rwp_analysis::compute_statistics(
            &[16.0, 16.25, 16.5, 16.75],
            StatisticsConfig::default(),
        )?;
        let candidate = rwp_analysis::compute_statistics(
            &[15.0, 15.25, 15.5, 15.75],
            StatisticsConfig::default(),
        )?;
        let input = rwp_analysis::ComparisonInput {
            baseline: &baseline,
            candidate: &candidate,
            baseline_stutter_count: 0,
            candidate_stutter_count: 0,
            same_scenario: true,
        };

        let comp1 = rwp_analysis::compare(input, rwp_analysis::ComparisonConfig::default())?;
        let comp2 = rwp_analysis::compare(input, rwp_analysis::ComparisonConfig::default())?;
        let comp_json1 = serde_json::to_string(&comp1)?;
        let comp_json2 = serde_json::to_string(&comp2)?;
        assert_eq!(
            comp_json1, comp_json2,
            "compare must produce byte-identical JSON across two runs"
        );
        let comp_decoded: rwp_analysis::ComparisonResult = serde_json::from_str(&comp_json1)?;
        assert_eq!(comp_decoded, comp1);

        let pairs = (0..6)
            .map(|index| rwp_analysis::PairedRun {
                pair_id: format!("pair-{index}"),
                baseline_sample_count: 1_000,
                candidate_sample_count: 1_000,
                baseline_p99_frame_time_ms: 30.0,
                candidate_p99_frame_time_ms: 25.0,
            })
            .collect::<Vec<_>>();

        let paired1 =
            rwp_analysis::paired_infer(&pairs, rwp_analysis::PairedInferenceConfig::default())?;
        let paired2 =
            rwp_analysis::paired_infer(&pairs, rwp_analysis::PairedInferenceConfig::default())?;
        let paired_json1 = serde_json::to_string(&paired1)?;
        let paired_json2 = serde_json::to_string(&paired2)?;
        assert_eq!(
            paired_json1, paired_json2,
            "paired_infer must produce byte-identical JSON across two runs"
        );
        let paired_decoded: rwp_analysis::PairedInferenceResult =
            serde_json::from_str(&paired_json1)?;
        assert_eq!(paired_decoded, paired1);
        Ok(())
    }

    #[test]
    fn public_types_serde_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let frame = rwp_core::FrameSample::new(100_000, 16.6)?.with_telemetry(
            Some(10.0),
            Some(1.0),
            Some(12.0),
            Some(11.0),
            Some(0.5),
            Some(5.0),
        )?;
        let frame_json = serde_json::to_string(&frame)?;
        let frame_decoded: rwp_core::FrameSample = serde_json::from_str(&frame_json)?;
        assert_eq!(frame_decoded, frame);

        let thread = rwp_core::ProcessThreadSample::new(42, 65.5)?;
        let thread_json = serde_json::to_string(&thread)?;
        let thread_decoded: rwp_core::ProcessThreadSample = serde_json::from_str(&thread_json)?;
        assert_eq!(thread_decoded, thread);

        let bg = rwp_core::BackgroundActivity::new(42, 15.0)?;
        let bg_json = serde_json::to_string(&bg)?;
        let bg_decoded: rwp_core::BackgroundActivity = serde_json::from_str(&bg_json)?;
        assert_eq!(bg_decoded, bg);

        let stutter = rwp_core::StutterEvent {
            start_timestamp_ns: 100_000,
            end_timestamp_ns: 200_000,
            peak_frame_time_ms: 33.3,
            baseline_frame_time_ms: 16.6,
            candidate_frame_count: 1,
            detector_parameters: vec![(String::from("k"), String::from("2.0"))],
        };
        let stutter_json = serde_json::to_string(&stutter)?;
        let stutter_decoded: rwp_core::StutterEvent = serde_json::from_str(&stutter_json)?;
        assert_eq!(stutter_decoded, stutter);

        let mut system_sample = rwp_core::SystemSample::new(100_000);
        system_sample.cpu_utilization = Some(50.0);
        system_sample.gpu_utilization = Some(80.0);
        system_sample.vram_used_mb = Some(4096);
        system_sample.vram_utilization = Some(50.0);
        system_sample.ram_used_mb = Some(8192);
        system_sample.ram_utilization = Some(51.2);
        system_sample.gpu_clock_mhz = Some(2400);
        system_sample.cpu_clock_mhz = Some(4200);
        system_sample.gpu_power_w = Some(175.0);
        system_sample.gpu_temperature_c = Some(68.5);
        system_sample.process_threads = vec![thread];
        system_sample.background_activity = vec![bg];
        let system_json = serde_json::to_string(&system_sample)?;
        let system_decoded: rwp_core::SystemSample = serde_json::from_str(&system_json)?;
        assert_eq!(system_decoded, system_sample);

        let item = rwp_core::EvidenceItem {
            evidence_id: String::from("ev-1"),
            polarity: rwp_core::EvidencePolarity::Supporting,
            statement: String::from("high cpu"),
            weight: 70,
        };
        let diag = rwp_core::Diagnostic::new(
            rwp_core::DiagnosticCategory::CpuBound,
            String::from("cpu.v1"),
            70,
            vec![item],
            vec![],
            vec![(String::from("rule"), String::from("cpu"))],
            100,
            60,
        )?;
        let diag_json = serde_json::to_string(&diag)?;
        let diag_decoded: rwp_core::Diagnostic = serde_json::from_str(&diag_json)?;
        assert_eq!(diag_decoded, diag);

        let stats = rwp_analysis::compute_statistics(
            &[16.0, 16.0, 16.0, 16.0],
            StatisticsConfig::default(),
        )?;
        let stats_json = serde_json::to_string(&stats)?;
        let stats_decoded: rwp_analysis::FrameStatistics = serde_json::from_str(&stats_json)?;
        assert_eq!(stats_decoded, stats);

        let cmp_cfg = rwp_analysis::ComparisonConfig::default();
        let cmp_cfg_json = serde_json::to_string(&cmp_cfg)?;
        let cmp_cfg_decoded: rwp_analysis::ComparisonConfig = serde_json::from_str(&cmp_cfg_json)?;
        assert_eq!(cmp_cfg_decoded, cmp_cfg);

        let pair_cfg = rwp_analysis::PairedInferenceConfig::default();
        let pair_cfg_json = serde_json::to_string(&pair_cfg)?;
        let pair_cfg_decoded: rwp_analysis::PairedInferenceConfig =
            serde_json::from_str(&pair_cfg_json)?;
        assert_eq!(pair_cfg_decoded, pair_cfg);

        let capture_doc = CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames: vec![frame],
        };
        let cap_json = serde_json::to_string(&capture_doc)?;
        let cap_decoded: CaptureDocument = serde_json::from_str(&cap_json)?;
        assert_eq!(cap_decoded, capture_doc);
        Ok(())
    }
}
