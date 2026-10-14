//! Platform-neutral console-signal stop decision for live capture.
//!
//! The Win32 codes below mirror the stable console-control ABI, so this pure
//! decision core stays unit-testable on every platform without `windows-sys`.
//! The operating-system handler itself lives in the Windows boundary
//! (`windows.rs`): it atomically claims the stop exactly once and then calls
//! [`decide_console_signal`] with the previous state.

/// Console control code for CTRL+C (stable Win32 ABI).
pub const CTRL_C_EVENT_CODE: u32 = 0;

/// Console control code for CTRL+BREAK (stable Win32 ABI).
pub const CTRL_BREAK_EVENT_CODE: u32 = 1;

/// Console control code for console closing (stable Win32 ABI).
pub const CTRL_CLOSE_EVENT_CODE: u32 = 2;

/// Console control code for user logoff (stable Win32 ABI).
pub const CTRL_LOGOFF_EVENT_CODE: u32 = 5;

/// Console control code for system shutdown (stable Win32 ABI).
pub const CTRL_SHUTDOWN_EVENT_CODE: u32 = 6;

/// Decides whether a console control signal is consumed (`true`) or left to
/// default handling (`false`).
///
/// The first CTRL+C or CTRL+BREAK is consumed so the runner can stop
/// gracefully and persist the partial session. A repeat signal while a stop
/// is already in progress is NOT consumed, so default handling ends the
/// process and the kill-on-close Job Object still terminates PresentMon.
/// Close/logoff/shutdown are never consumed; the Job Object already prevents
/// orphans there.
pub fn decide_console_signal(ctrl_code: u32, already_stopping: bool) -> bool {
    match ctrl_code {
        CTRL_C_EVENT_CODE | CTRL_BREAK_EVENT_CODE => !already_stopping,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CTRL_BREAK_EVENT_CODE, CTRL_C_EVENT_CODE, CTRL_CLOSE_EVENT_CODE, CTRL_LOGOFF_EVENT_CODE,
        CTRL_SHUTDOWN_EVENT_CODE, decide_console_signal,
    };

    #[test]
    fn first_break_signals_are_consumed_repeat_is_not() {
        assert!(decide_console_signal(CTRL_C_EVENT_CODE, false));
        assert!(decide_console_signal(CTRL_BREAK_EVENT_CODE, false));
        assert!(!decide_console_signal(CTRL_C_EVENT_CODE, true));
        assert!(!decide_console_signal(CTRL_BREAK_EVENT_CODE, true));
    }

    #[test]
    fn close_logoff_shutdown_are_never_consumed() {
        for code in [
            CTRL_CLOSE_EVENT_CODE,
            CTRL_LOGOFF_EVENT_CODE,
            CTRL_SHUTDOWN_EVENT_CODE,
            99_u32,
        ] {
            assert!(!decide_console_signal(code, false));
            assert!(!decide_console_signal(code, true));
        }
    }
}
