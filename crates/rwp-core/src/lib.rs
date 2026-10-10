//! Platform-neutral contracts shared by RidTheWann Performance crates.

mod diagnostic;
mod sample;

pub use diagnostic::{Diagnostic, DiagnosticCategory, EvidenceItem, EvidencePolarity};
pub use sample::{
    BackgroundActivity, FrameSample, ProcessThreadSample, StutterEvent, SystemSample,
};

/// Errors produced when constructing normalized samples or derived events.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CoreError {
    /// A required floating-point value was not finite.
    #[error("{field} must be finite")]
    NonFinite {
        /// Name of the invalid field.
        field: &'static str,
    },
    /// A required positive floating-point value was not strictly positive.
    #[error("{field} must be greater than zero")]
    NonPositive {
        /// Name of the invalid field.
        field: &'static str,
    },
    /// A bounded floating-point value fell outside its allowed range.
    #[error("{field} must be between {min} and {max}")]
    InvalidRange {
        /// Name of the invalid field.
        field: &'static str,
        /// Inclusive lower bound.
        min: u8,
        /// Inclusive upper bound.
        max: u8,
    },
    /// An optional duration was negative.
    #[error("{field} must not be negative")]
    NegativeValue {
        /// Name of the invalid field.
        field: &'static str,
    },
    /// An evidence score fell outside the supported 0..=100 range.
    #[error("evidence score must be in 0..=100, got {0}")]
    InvalidEvidenceScore(u8),
}
