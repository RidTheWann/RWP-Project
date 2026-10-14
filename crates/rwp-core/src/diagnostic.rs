use crate::CoreError;
use serde::{Deserialize, Serialize};

/// Diagnostic categories defined by the product contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum DiagnosticCategory {
    /// CPU throughput or CPU-side scheduling is the limiting factor according to evidence.
    CpuBound,
    /// GPU throughput is the limiting factor according to evidence.
    GpuBound,
    /// Frame-time distribution indicates pacing instability.
    FramePacing,
    /// Thermal or power behavior is consistent with throttling.
    Thermal,
    /// System memory pressure is implicated.
    MemoryPressure,
    /// Video-memory pressure is implicated.
    VramPressure,
    /// Non-target processes or services correlate with degradation.
    BackgroundInterference,
    /// Streaming or creator tooling correlates with degradation.
    StreamingOverhead,
    /// Evidence is insufficient or contradictory.
    Unknown,
}

/// Whether an evidence item supports or contradicts a diagnostic rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidencePolarity {
    /// Evidence supporting the rule.
    Supporting,
    /// Evidence contradicting the rule.
    Contradicting,
}

/// Preliminary in-memory stability verdict for live capture surfaces.
///
/// This is deliberately separate from persisted [`Diagnostic`] results: it is
/// computed from current observations only, never serialized into sessions or
/// reports, and rendered by the always-on in-app banner. Below-minimum
/// observations yield [`PreliminaryStability::InsufficientData`], never a
/// stability claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PreliminaryStability {
    /// Fewer observations than the minimum, or degenerate input that cannot
    /// be assessed; never presented as stable or unstable.
    InsufficientData,
    /// Enough observations with no stutter events and drift within margin.
    Stable,
    /// Enough observations showing stutter events or drift beyond margin.
    Unstable,
}

/// A structured evidence item emitted by the diagnostic engine.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Stable identifier used for report cross-references.
    pub evidence_id: String,
    /// Whether the item supports or contradicts the rule.
    pub polarity: EvidencePolarity,
    /// Human-readable evidence statement.
    pub statement: String,
    /// Numeric contribution before score normalization.
    pub weight: u16,
}

/// A deterministic diagnostic result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Diagnostic {
    /// Diagnostic category.
    pub category: DiagnosticCategory,
    /// Stable rule identifier.
    pub rule_id: String,
    /// Heuristic evidence score in 0..=100; not a probability.
    pub evidence_score: u8,
    /// Evidence supporting the rule.
    pub supporting_evidence: Vec<EvidenceItem>,
    /// Evidence contradicting the rule.
    pub contradicting_evidence: Vec<EvidenceItem>,
    /// Serialized rule parameters used for the decision.
    pub parameters: Vec<(String, String)>,
    /// Number of valid observations used by the rule.
    pub sample_count: usize,
    /// Minimum observations required by the rule.
    pub minimum_sample_count: usize,
}

impl Diagnostic {
    /// Creates a validated diagnostic.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        category: DiagnosticCategory,
        rule_id: String,
        evidence_score: u8,
        supporting_evidence: Vec<EvidenceItem>,
        contradicting_evidence: Vec<EvidenceItem>,
        parameters: Vec<(String, String)>,
        sample_count: usize,
        minimum_sample_count: usize,
    ) -> Result<Self, CoreError> {
        if evidence_score > 100 {
            return Err(CoreError::InvalidEvidenceScore(evidence_score));
        }
        Ok(Self {
            category,
            rule_id,
            evidence_score,
            supporting_evidence,
            contradicting_evidence,
            parameters,
            sample_count,
            minimum_sample_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Diagnostic, DiagnosticCategory};
    use crate::CoreError;

    #[test]
    fn validates_evidence_score() {
        assert!(
            Diagnostic::new(
                DiagnosticCategory::Unknown,
                String::from("unknown"),
                100,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                0,
                1,
            )
            .is_ok()
        );
        assert_eq!(
            Diagnostic::new(
                DiagnosticCategory::Unknown,
                String::from("unknown"),
                101,
                Vec::new(),
                Vec::new(),
                Vec::new(),
                0,
                1,
            )
            .err(),
            Some(CoreError::InvalidEvidenceScore(101))
        );
    }
}
