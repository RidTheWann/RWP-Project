//! PresentMon capture boundary.

mod command;
mod csv;
mod error;

pub use command::{
    GRACEFUL_STOP_TIMEOUT, MAX_TIMED_SECONDS, PresentMonCaptureConfig, PresentMonProcess,
    PresentMonRunner, PresentMonTarget, StopOutcome,
};

#[cfg(windows)]
pub use command::presentmon_child_creation_flags;
pub use csv::{ParseMode, PresentMonCapture, PresentMonCsvParser, dropped_rows_notice};
pub use error::{PresentMonCaptureError, PresentMonConfigError, PresentMonCsvError};
