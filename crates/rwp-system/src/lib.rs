//! Windows hardware and system telemetry boundary.

#[cfg(windows)]
mod pdh;

#[cfg(windows)]
mod processes;

#[cfg(windows)]
mod threads;

#[cfg(windows)]
mod windows;

#[cfg(windows)]
pub use windows::{
    DisplayRefreshRate, DisplayRefreshRateError, PresentMonJob, PresentMonJobError,
    PresentMonPermissionError, PresentMonVersion, PresentMonVersionError, QpcTimebase,
    SystemTelemetryError, WindowsSystemSampler, detect_presentmon_version,
    detect_process_display_refresh_rate, preflight_presentmon_permissions,
    request_process_group_break,
};

#[cfg(not(windows))]
/// Error returned when Windows-only telemetry is used on another platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Windows system telemetry is only available on Windows")]
pub struct SystemTelemetryError;

#[cfg(windows)]
pub use pdh::{PdhCpuSampler, PdhError};
