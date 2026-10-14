//! Windows hardware and system telemetry boundary.

mod console_stop;

pub use console_stop::{
    CTRL_BREAK_EVENT_CODE, CTRL_C_EVENT_CODE, CTRL_CLOSE_EVENT_CODE, CTRL_LOGOFF_EVENT_CODE,
    CTRL_SHUTDOWN_EVENT_CODE, decide_console_signal,
};

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
    AuthenticodeStatus, ConsoleStopGuard, DetachedBreakError, DisplayRefreshRate,
    DisplayRefreshRateError, PresentMonJob, PresentMonJobError, PresentMonPermissionError,
    PresentMonVersion, PresentMonVersionError, QpcTimebase, SystemTelemetryError,
    WindowsSystemSampler, caller_has_console, capture_stop_flag, detect_presentmon_version,
    detect_process_display_refresh_rate, preflight_presentmon_permissions,
    request_process_group_break, send_break_to_detached_group, verify_presentmon_authenticode,
};

#[cfg(windows)]
pub use processes::{ProcessSummary, list_processes};

#[cfg(not(windows))]
/// Redacted live-process identity for UI target selection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessSummary {
    /// Live process identifier.
    pub process_id: u32,
    /// Executable basename reported by the snapshot, lossily decoded.
    pub name: String,
}

#[cfg(not(windows))]
/// Process listing is only available on Windows.
pub fn list_processes() -> Result<Vec<ProcessSummary>, SystemTelemetryError> {
    Err(SystemTelemetryError)
}

#[cfg(not(windows))]
/// Error returned when Windows-only telemetry is used on another platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("Windows system telemetry is only available on Windows")]
pub struct SystemTelemetryError;

#[cfg(windows)]
pub use pdh::{PdhCpuSampler, PdhError};
