//! Error types for PresentMon capture and CSV normalization.

use std::path::PathBuf;

/// Configuration errors raised before a PresentMon process is started.
#[derive(Debug, thiserror::Error)]
pub enum PresentMonConfigError {
    /// The PresentMon executable path was empty.
    #[error("PresentMon executable path must not be empty")]
    EmptyExecutablePath,
    /// The target executable name was empty.
    #[error("PresentMon process name must not be empty")]
    EmptyProcessName,
    /// The process identifier was zero.
    #[error("PresentMon process ID must be greater than zero")]
    ZeroProcessId,
    /// The output path was empty.
    #[error("PresentMon output path must not be empty")]
    EmptyOutputPath,
    /// The PresentMon executable does not exist.
    #[error("PresentMon executable does not exist: {0}")]
    ExecutableNotFound(PathBuf),
    /// The PresentMon executable path does not point to a regular file.
    #[error("PresentMon executable is not a regular file: {0}")]
    ExecutableNotFile(PathBuf),
    /// The executable basename is not the expected PresentMon binary name.
    #[error("PresentMon executable must be named PresentMon.exe: {0}")]
    ExecutableName(PathBuf),
    /// The PresentMon executable path could not be canonicalized.
    #[error("failed to canonicalize PresentMon executable {path}: {source}")]
    ExecutableCanonicalize {
        /// Executable path that could not be canonicalized.
        path: PathBuf,
        /// Operating-system error returned by path canonicalization.
        #[source]
        source: std::io::Error,
    },
    /// A timed capture was configured with zero seconds.
    #[error("timed capture duration must be greater than zero")]
    ZeroTimedSeconds,
    /// A timed capture exceeded the supported maximum duration.
    #[error("timed capture duration {requested} s exceeds the supported maximum of {max} s")]
    TimedSecondsTooLong {
        /// Requested duration in seconds.
        requested: u64,
        /// Maximum supported duration in seconds.
        max: u64,
    },
}

/// Errors raised while launching or waiting for the external PresentMon process.
#[derive(Debug, thiserror::Error)]
pub enum PresentMonCaptureError {
    /// Capture configuration is invalid.
    #[error(transparent)]
    Config(#[from] PresentMonConfigError),
    /// Required Windows trace permissions are unavailable.
    #[cfg(windows)]
    #[error(transparent)]
    PermissionPreflight(#[from] rwp_system::PresentMonPermissionError),
    /// PresentMon could not be attached to a kill-on-close Job Object.
    #[cfg(windows)]
    #[error(transparent)]
    Job(#[from] rwp_system::PresentMonJobError),
    /// PresentMon executable version could not be determined or is unsupported.
    #[cfg(windows)]
    #[error(transparent)]
    Version(#[from] rwp_system::PresentMonVersionError),
    /// PresentMon could not be started.
    #[error("failed to start PresentMon at {path}: {source}")]
    Spawn {
        /// Executable path that was passed to the operating system.
        path: PathBuf,
        /// Operating-system error returned by process creation.
        #[source]
        source: std::io::Error,
    },
    /// The configured CSV output already exists and is protected from accidental reuse.
    #[error("PresentMon output already exists: {0}")]
    OutputAlreadyExists(PathBuf),
    /// Waiting for PresentMon failed.
    #[error("failed to wait for PresentMon: {0}")]
    Wait(#[source] std::io::Error),
    /// Terminating PresentMon failed.
    #[error("failed to terminate PresentMon: {0}")]
    Kill(#[source] std::io::Error),
    /// PresentMon exited unsuccessfully.
    #[error("PresentMon exited unsuccessfully with status {status:?}; stderr: {stderr:?}")]
    ProcessFailed {
        /// Exit status code returned by PresentMon.
        status: Option<i32>,
        /// Best-effort stderr captured from the PresentMon process.
        stderr: Option<String>,
    },
    /// PresentMon's CSV could not be normalized.
    #[error(transparent)]
    Csv(#[from] PresentMonCsvError),
}

/// Errors raised while parsing PresentMon CSV output.
#[derive(Debug, thiserror::Error)]
pub enum PresentMonCsvError {
    /// Reading the CSV file failed.
    #[error("failed to read PresentMon CSV: {0}")]
    Io(#[source] std::io::Error),
    /// The parser was given an invalid QPC frequency.
    #[error("QPC frequency must be greater than zero")]
    InvalidQpcFrequency,
    /// The CSV contained no rows after the header.
    #[error("PresentMon CSV contains no frame rows")]
    EmptyCapture,
    /// The CSV file exceeds the parser resource limit.
    #[error(
        "PresentMon CSV exceeds the maximum supported size of {maximum_bytes} bytes: {actual_bytes}"
    )]
    InputTooLarge {
        /// Maximum accepted byte size.
        maximum_bytes: u64,
        /// Actual file size reported by the filesystem.
        actual_bytes: u64,
    },
    /// The CSV contains more frame rows than the parser resource limit.
    #[error("PresentMon CSV exceeds the maximum supported frame-row count of {maximum_rows}")]
    TooManyRows {
        /// Maximum accepted frame rows.
        maximum_rows: usize,
    },
    /// A required column was absent.
    #[error("PresentMon CSV is missing required column {0}")]
    MissingColumn(&'static str),
    /// A CSV record used invalid quoting or separators.
    #[error("malformed PresentMon CSV at row {row}: {reason}")]
    MalformedCsv {
        /// One-based CSV data row number, excluding the header.
        row: usize,
        /// Parser explanation.
        reason: String,
    },
    /// A required or optional metric could not be parsed.
    #[error("invalid PresentMon metric at row {row}, column {column}: {value} ({reason})")]
    InvalidMetric {
        /// One-based CSV data row number, excluding the header.
        row: usize,
        /// CSV column name.
        column: String,
        /// Original field value.
        value: String,
        /// Parsing or validation explanation.
        reason: String,
    },
    /// A PresentMon QPC timestamp predates the shared session origin.
    #[error(
        "PresentMon QPC timestamp is before the shared origin at row {row}: origin {origin}, current {current}"
    )]
    QpcBeforeOrigin {
        /// One-based CSV data row number, excluding the header.
        row: usize,
        /// Shared session QPC origin.
        origin: u64,
        /// Current PresentMon QPC value.
        current: u64,
    },
    /// A QPC delta could not be represented as nanoseconds.
    #[error("QPC timestamp conversion overflowed at row {row}")]
    TimestampOverflow {
        /// One-based CSV data row number, excluding the header.
        row: usize,
    },
    /// A capture contained rows for more than one process.
    #[error(
        "PresentMon CSV changed process identity at row {row}: expected PID {expected_pid}, got {observed_pid}"
    )]
    ProcessIdChanged {
        /// One-based CSV data row number, excluding the header.
        row: usize,
        /// Process identifier established by the first row.
        expected_pid: u32,
        /// Process identifier found in the current row.
        observed_pid: u32,
    },
    /// A row did not match the configured process target.
    #[error("PresentMon CSV row {row} does not match target process: {reason}")]
    TargetMismatch {
        /// One-based CSV data row number, excluding the header.
        row: usize,
        /// Explanation of the mismatch.
        reason: String,
    },
}

#[cfg(test)]
mod tests {
    use super::{PresentMonCaptureError, PresentMonCsvError};

    #[test]
    fn resource_limit_errors_are_actionable() {
        let error = PresentMonCsvError::InputTooLarge {
            maximum_bytes: 512,
            actual_bytes: 513,
        };
        assert!(error.to_string().contains("maximum supported size"));
        let error = PresentMonCsvError::TooManyRows { maximum_rows: 10 };
        assert!(
            error
                .to_string()
                .contains("maximum supported frame-row count")
        );
    }

    #[test]
    fn process_failure_includes_captured_stderr() {
        let error = PresentMonCaptureError::ProcessFailed {
            status: Some(6),
            stderr: Some(String::from("access denied")),
        };
        assert!(error.to_string().contains("access denied"));
    }
}
