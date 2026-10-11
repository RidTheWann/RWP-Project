//! Framework-neutral presentation model for the M7 dashboard.

use rwp_analysis::{ComparisonResult, DeltaDirection};
use rwp_core::{Diagnostic, DiagnosticCategory, EvidenceItem};
use rwp_report::PerformanceReport;
use serde::Serialize;

/// Stable top-level dashboard sections.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DashboardTab {
    /// Frame-time and low-FPS overview.
    Overview,
    /// Frame-pacing event list.
    Stutters,
    /// System/diagnostic evidence.
    Diagnostics,
    /// Baseline/candidate comparison.
    Compare,
}

impl DashboardTab {
    /// Returns every supported tab in stable display order.
    pub const fn all() -> [Self; 4] {
        [
            Self::Overview,
            Self::Stutters,
            Self::Diagnostics,
            Self::Compare,
        ]
    }

    /// Returns the user-facing label.
    pub const fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Stutters => "Stutters",
            Self::Diagnostics => "Diagnostics",
            Self::Compare => "Compare",
        }
    }
}

/// One summary metric designed for a UI card.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MetricCard {
    /// Stable field identifier.
    pub id: &'static str,
    /// User-facing title.
    pub title: &'static str,
    /// Formatted value.
    pub value: String,
    /// Optional supporting context.
    pub detail: String,
    /// Whether a lower value is favorable.
    pub lower_is_better: bool,
}

/// One stutter event row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StutterRow {
    /// Start timestamp in milliseconds.
    pub start_ms: f64,
    /// Event duration in milliseconds.
    pub duration_ms: f64,
    /// Peak frame time in milliseconds.
    pub peak_frame_time_ms: f64,
    /// Local baseline in milliseconds.
    pub baseline_frame_time_ms: f64,
    /// Number of candidate frames.
    pub candidate_frame_count: u32,
}

/// Diagnostic card for presentation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DiagnosticCard {
    /// Category shown to the user.
    pub category: String,
    /// Stable rule identifier.
    pub rule_id: String,
    /// Heuristic evidence score.
    pub evidence_score: u8,
    /// Short summary of supporting evidence.
    pub supporting_evidence: Vec<String>,
    /// Short summary of contradicting evidence.
    pub contradicting_evidence: Vec<String>,
}

/// Comparison summary for the Compare tab.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ComparisonView {
    /// Human-readable comparability status.
    pub comparability: String,
    /// P99 delta as a formatted relative percentage.
    pub p99_change: String,
    /// Average-FPS delta as a formatted relative percentage.
    pub average_fps_change: String,
    /// Stutter-count change.
    pub stutter_change: String,
    /// Whether the candidate P99 moved in the favorable direction.
    pub p99_improved: Option<bool>,
}

/// Immutable dashboard data consumed by any renderer.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DashboardModel {
    /// Active tab on first render.
    pub active_tab: DashboardTab,
    /// Overview metric cards.
    pub metrics: Vec<MetricCard>,
    /// Stutter rows.
    pub stutters: Vec<StutterRow>,
    /// Diagnostic cards.
    pub diagnostics: Vec<DiagnosticCard>,
    /// Optional comparison view.
    pub comparison: Option<ComparisonView>,
}

/// Mutable UI navigation state kept separate from analysis state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct UiState {
    active_tab: DashboardTab,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            active_tab: DashboardTab::Overview,
        }
    }
}

impl UiState {
    /// Returns the currently selected tab.
    pub const fn active_tab(self) -> DashboardTab {
        self.active_tab
    }

    /// Selects a dashboard tab.
    pub fn select(&mut self, tab: DashboardTab) {
        self.active_tab = tab;
    }
}

/// Builds the framework-neutral dashboard model from one report.
pub fn build_dashboard_model(
    report: &PerformanceReport,
    diagnostics: &[Diagnostic],
    comparison: Option<&ComparisonResult>,
) -> DashboardModel {
    DashboardModel {
        active_tab: DashboardTab::Overview,
        metrics: build_metric_cards(report),
        stutters: report
            .stutter_events
            .iter()
            .map(|event| StutterRow {
                start_ms: event.start_timestamp_ns as f64 / 1_000_000.0,
                duration_ms: event
                    .end_timestamp_ns
                    .saturating_sub(event.start_timestamp_ns) as f64
                    / 1_000_000.0,
                peak_frame_time_ms: event.peak_frame_time_ms,
                baseline_frame_time_ms: event.baseline_frame_time_ms,
                candidate_frame_count: event.candidate_frame_count,
            })
            .collect(),
        diagnostics: diagnostics.iter().map(to_diagnostic_card).collect(),
        comparison: comparison.map(to_comparison_view),
    }
}

fn build_metric_cards(report: &PerformanceReport) -> Vec<MetricCard> {
    let stats = &report.frame_statistics;
    vec![
        MetricCard {
            id: "average_fps",
            title: "Average FPS",
            value: format!("{:.1}", stats.average_fps),
            detail: "Capture-wide arithmetic FPS".to_owned(),
            lower_is_better: false,
        },
        MetricCard {
            id: "p99_frame_time",
            title: "P99 frame time",
            value: format!("{:.2} ms", stats.percentiles.p99_ms),
            detail: "Tail frame-time percentile".to_owned(),
            lower_is_better: true,
        },
        MetricCard {
            id: "one_percent_low",
            title: "1% low",
            value: format!("{:.1} FPS", stats.low_fps.percentile_1_low_fps),
            detail: "1000 / P99 frame time".to_owned(),
            lower_is_better: false,
        },
        MetricCard {
            id: "stutter_events",
            title: "Stutter events",
            value: report.stutter_events.len().to_string(),
            detail: "Detected by local-median rule".to_owned(),
            lower_is_better: true,
        },
    ]
}

fn to_diagnostic_card(diagnostic: &Diagnostic) -> DiagnosticCard {
    DiagnosticCard {
        category: diagnostic_category_label(diagnostic.category),
        rule_id: diagnostic.rule_id.clone(),
        evidence_score: diagnostic.evidence_score,
        supporting_evidence: diagnostic
            .supporting_evidence
            .iter()
            .map(evidence_text)
            .collect(),
        contradicting_evidence: diagnostic
            .contradicting_evidence
            .iter()
            .map(evidence_text)
            .collect(),
    }
}

fn evidence_text(item: &EvidenceItem) -> String {
    format!("{}: {}", item.evidence_id, item.statement)
}

fn diagnostic_category_label(category: DiagnosticCategory) -> String {
    let label = match category {
        DiagnosticCategory::CpuBound => "CPU bound",
        DiagnosticCategory::GpuBound => "GPU bound",
        DiagnosticCategory::FramePacing => "Frame pacing",
        DiagnosticCategory::Thermal => "Thermal",
        DiagnosticCategory::MemoryPressure => "Memory pressure",
        DiagnosticCategory::VramPressure => "VRAM pressure",
        DiagnosticCategory::BackgroundInterference => "Background interference",
        DiagnosticCategory::StreamingOverhead => "Streaming overhead",
        DiagnosticCategory::Unknown => "Unknown",
        _ => "Unknown",
    };
    label.to_owned()
}

fn to_comparison_view(result: &ComparisonResult) -> ComparisonView {
    ComparisonView {
        comparability: format!("{:?}", result.comparability),
        p99_change: format_optional_percent(result.metrics.p99_frame_time_ms.relative_percent),
        average_fps_change: format_optional_percent(result.metrics.average_fps.relative_percent),
        stutter_change: format_signed(result.stutter_count_delta),
        p99_improved: match result.metrics.p99_frame_time_ms.direction {
            DeltaDirection::Improved => Some(true),
            DeltaDirection::Regressed => Some(false),
            DeltaDirection::Unchanged => None,
        },
    }
}

fn format_optional_percent(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:+.2}%"))
        .unwrap_or_else(|| "n/a".to_owned())
}

fn format_signed(value: i64) -> String {
    if value > 0 {
        format!("+{value}")
    } else {
        value.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{DashboardTab, UiState, build_dashboard_model};
    use rwp_analysis::{StatisticsConfig, StutterConfig};
    use rwp_core::{Diagnostic, DiagnosticCategory};
    use rwp_report::{CaptureDocument, build_report};

    fn report() -> Result<rwp_report::PerformanceReport, Box<dyn std::error::Error>> {
        let frames = (0..10)
            .map(|index| rwp_core::FrameSample::new(index * 16_000_000, 16.0))
            .collect::<Result<Vec<_>, _>>()?;
        let capture = CaptureDocument {
            schema_version: "rwp.capture.v1".to_owned(),
            frames,
        };
        Ok(build_report(
            &capture,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig::default(),
        )?)
    }

    #[test]
    fn model_exposes_stable_overview_cards() -> Result<(), Box<dyn std::error::Error>> {
        let report = report()?;
        let diagnostics = vec![Diagnostic::new(
            DiagnosticCategory::Unknown,
            "unknown.insufficient_evidence.v1".to_owned(),
            0,
            Vec::new(),
            Vec::new(),
            Vec::new(),
            report.frame_count,
            60,
        )?];
        let model = build_dashboard_model(&report, &diagnostics, None);
        assert_eq!(model.active_tab, DashboardTab::Overview);
        assert_eq!(model.metrics.len(), 4);
        assert_eq!(model.stutters.len(), 0);
        assert_eq!(model.diagnostics.len(), 1);
        assert_eq!(model.metrics[1].id, "p99_frame_time");
        Ok(())
    }

    #[test]
    fn ui_state_changes_only_navigation() {
        let mut state = UiState::default();
        assert_eq!(state.active_tab(), DashboardTab::Overview);
        state.select(DashboardTab::Diagnostics);
        assert_eq!(state.active_tab(), DashboardTab::Diagnostics);
    }

    #[test]
    fn all_tabs_have_stable_labels() {
        assert_eq!(
            DashboardTab::all().map(DashboardTab::label),
            ["Overview", "Stutters", "Diagnostics", "Compare",]
        );
    }

    #[test]
    fn ui_state_cycles_through_every_tab() {
        let mut state = UiState::default();
        for tab in DashboardTab::all() {
            state.select(tab);
            assert_eq!(state.active_tab(), tab);
        }
    }

    #[test]
    fn model_converts_stutter_events_to_rows() -> Result<(), Box<dyn std::error::Error>> {
        let frames = (0..20_u64)
            .map(|index| {
                rwp_core::FrameSample::new(index * 8_000_000, if index == 10 { 40.0 } else { 8.0 })
            })
            .collect::<Result<Vec<_>, _>>()?;
        let capture = CaptureDocument {
            schema_version: "rwp.capture.v1".to_owned(),
            frames,
        };
        let report = build_report(
            &capture,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig::default(),
        )?;
        let model = build_dashboard_model(&report, &[], None);
        assert_eq!(model.stutters.len(), 1);
        let row = &model.stutters[0];
        assert_eq!(row.start_ms, 80.0);
        assert_eq!(row.duration_ms, 0.0);
        assert_eq!(row.peak_frame_time_ms, 40.0);
        assert_eq!(row.baseline_frame_time_ms, 8.0);
        assert_eq!(row.candidate_frame_count, 1);
        Ok(())
    }

    #[test]
    fn model_maps_diagnostic_evidence_to_cards() -> Result<(), Box<dyn std::error::Error>> {
        use rwp_core::{EvidenceItem, EvidencePolarity};
        let report = report()?;
        let diagnostics = vec![Diagnostic::new(
            DiagnosticCategory::CpuBound,
            "cpu_bound.v1".to_owned(),
            70,
            vec![EvidenceItem {
                evidence_id: String::from("cpu.frame_share"),
                polarity: EvidencePolarity::Supporting,
                statement: String::from("high CPU share"),
                weight: 35,
            }],
            vec![EvidenceItem {
                evidence_id: String::from("gpu.strong_share"),
                polarity: EvidencePolarity::Contradicting,
                statement: String::from("high GPU share"),
                weight: 40,
            }],
            Vec::new(),
            report.frame_count,
            60,
        )?];
        let model = build_dashboard_model(&report, &diagnostics, None);
        assert_eq!(model.diagnostics.len(), 1);
        let card = &model.diagnostics[0];
        assert_eq!(card.category, "CPU bound");
        assert_eq!(card.rule_id, "cpu_bound.v1");
        assert_eq!(card.evidence_score, 70);
        assert_eq!(
            card.supporting_evidence,
            vec!["cpu.frame_share: high CPU share"]
        );
        assert_eq!(
            card.contradicting_evidence,
            vec!["gpu.strong_share: high GPU share"]
        );
        assert_eq!(model.metrics.len(), 4);
        assert!(model.comparison.is_none());
        Ok(())
    }
}
