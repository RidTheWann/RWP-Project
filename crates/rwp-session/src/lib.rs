//! Live capture session orchestration and persistence boundary.

mod persistence;

pub use persistence::{
    PersistedSessionInput, PersistenceError, SESSION_MANIFEST_SCHEMA, SessionManifest,
    SessionManifestFile, SessionProvenance, persist_session, verify_manifest,
};

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{SessionConfig, SessionError, SessionResult, SessionRunner, TelemetryWarning};

/// Deterministic timestamp-range coverage summary for one session.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct CoverageSummary {
    /// Span from first to last frame timestamp, in seconds.
    pub frame_seconds: Option<f64>,
    /// Span from first to last telemetry timestamp, in seconds.
    pub telemetry_seconds: Option<f64>,
    /// Overlap span between frame and telemetry timestamp ranges, in seconds.
    pub intersection_seconds: Option<f64>,
}

#[cfg(not(windows))]
/// Errors raised when live session orchestration is requested on an unsupported platform.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// Live capture is currently Windows-only.
    #[error("live session orchestration is only available on Windows")]
    UnsupportedPlatform,
}
