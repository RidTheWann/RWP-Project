//! Stable CLI error taxonomy and machine-readable error rendering.

use std::io;

use super::doctor::DoctorPreflightFailed;
use anyhow::Error;
use rwp_capture::{PresentMonCaptureError, PresentMonConfigError, PresentMonCsvError};
use rwp_diagnostics::DiagnosticError;
use rwp_report::ReportError;
use rwp_session::{PersistenceError, SessionError};

/// Stable error descriptor exposed by the CLI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorDescriptor {
    /// Stable machine-readable code.
    pub code: &'static str,
    /// Stable process exit code.
    pub exit_code: i32,
    /// Actionable remediation guidance.
    pub remediation: &'static str,
}

/// User-visible classification of one command failure.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorReport {
    /// Stable error descriptor.
    pub descriptor: ErrorDescriptor,
    /// Human-readable failure detail.
    pub message: String,
}

/// Classifies an anyhow error at the CLI boundary without changing the underlying error.
pub fn classify(error: &Error) -> ErrorReport {
    let descriptor = if error.downcast_ref::<DoctorPreflightFailed>().is_some() {
        ErrorDescriptor {
            code: "RWP-E6000",
            exit_code: 60,
            remediation: "Review the doctor report for failed preflight checks and address each reported issue.",
        }
    } else if error.downcast_ref::<PresentMonConfigError>().is_some() {
        ErrorDescriptor {
            code: "RWP-E2000",
            exit_code: 20,
            remediation: "Check the PresentMon path, target process ID, output path, and capture options.",
        }
    } else if error.downcast_ref::<PresentMonCsvError>().is_some()
        || matches!(
            error.downcast_ref::<PresentMonCaptureError>(),
            Some(PresentMonCaptureError::Csv(_))
        )
        || matches!(
            error.downcast_ref::<rwp_session::SessionError>(),
            Some(
                rwp_session::SessionError::CaptureCsv(_)
                    | rwp_session::SessionError::Capture(PresentMonCaptureError::Csv(_)),
            )
        )
    {
        ErrorDescriptor {
            code: "RWP-E2200",
            exit_code: 22,
            remediation: "Validate the PresentMon CSV version, required columns, timestamps, target PID, and resource limits.",
        }
    } else if let Some(session) = error.downcast_ref::<rwp_session::SessionError>() {
        // Session-wrapped failures keep the inner boundary code: config and
        // lifecycle errors must not surface as the generic session code.
        match session {
            rwp_session::SessionError::CaptureConfig(_) => ErrorDescriptor {
                code: "RWP-E2000",
                exit_code: 20,
                remediation: "Check the PresentMon path, target process ID, output path, and capture options.",
            },
            rwp_session::SessionError::Capture(_) => ErrorDescriptor {
                code: "RWP-E2100",
                exit_code: 21,
                remediation: "Verify PresentMon permissions, executable validity, lifecycle state, and stderr output.",
            },
            _ => ErrorDescriptor {
                code: "RWP-E5100",
                exit_code: 51,
                remediation: "Run the command on a supported platform and check the live-session configuration.",
            },
        }
    } else if error.downcast_ref::<PresentMonCaptureError>().is_some() {
        ErrorDescriptor {
            code: "RWP-E2100",
            exit_code: 21,
            remediation: "Verify PresentMon permissions, executable validity, lifecycle state, and stderr output.",
        }
    } else if error.downcast_ref::<ReportError>().is_some() {
        ErrorDescriptor {
            code: "RWP-E3000",
            exit_code: 30,
            remediation: "Check the report/capture schema and ensure the session was produced by a compatible RWP version.",
        }
    } else if error.downcast_ref::<DiagnosticError>().is_some() {
        ErrorDescriptor {
            code: "RWP-E4000",
            exit_code: 40,
            remediation: "Review diagnostic thresholds and provide sufficient frame and telemetry observations.",
        }
    } else if error.downcast_ref::<PersistenceError>().is_some() {
        ErrorDescriptor {
            code: "RWP-E5000",
            exit_code: 50,
            remediation: "Check the session directory, manifest integrity, file limits, and filesystem permissions.",
        }
    } else if error.downcast_ref::<SessionError>().is_some() {
        ErrorDescriptor {
            code: "RWP-E5100",
            exit_code: 51,
            remediation: "Run the command on a supported platform and check the live-session configuration.",
        }
    } else if error.downcast_ref::<io::Error>().is_some() {
        ErrorDescriptor {
            code: "RWP-E1000",
            exit_code: 10,
            remediation: "Check the referenced path, filesystem permissions, and available storage.",
        }
    } else {
        ErrorDescriptor {
            code: "RWP-E9000",
            exit_code: 90,
            remediation: "Inspect the detailed error and verify the input/session state before retrying.",
        }
    };

    ErrorReport {
        descriptor,
        message: error.to_string(),
    }
}

/// Renders a deterministic human-readable CLI error.
pub fn render_text(report: &ErrorReport) -> String {
    format!(
        "{}: {}\nFix: {}",
        report.descriptor.code, report.message, report.descriptor.remediation
    )
}

/// Renders a deterministic machine-readable CLI error object.
pub fn render_json(report: &ErrorReport) -> String {
    let value = serde_json::json!({
        "error": {
            "code": report.descriptor.code,
            "exit_code": report.descriptor.exit_code,
            "message": report.message,
            "remediation": report.descriptor.remediation,
        }
    });

    match serde_json::to_string(&value) {
        Ok(text) => text,
        Err(_) => String::from(
            "{\"error\":{\"code\":\"RWP-E9000\",\"exit_code\":90,\"message\":\"failed to serialize error output\",\"remediation\":\"Inspect stderr and retry the operation.\"}}",
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{classify, render_json, render_text};

    #[test]
    fn failed_doctor_preflight_uses_stable_machine_code() {
        let error = anyhow::Error::new(super::super::doctor::DoctorPreflightFailed);
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E6000");
        assert_eq!(report.descriptor.exit_code, 60);
    }

    #[test]
    fn presentmon_csv_errors_use_stable_machine_code() {
        let error = anyhow::Error::new(rwp_capture::PresentMonCsvError::EmptyCapture);
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E2200");
        assert_eq!(report.descriptor.exit_code, 22);
        assert!(render_text(&report).starts_with("RWP-E2200: "));
        assert!(render_json(&report).contains("\"RWP-E2200\""));
    }

    #[test]
    fn nested_capture_csv_errors_keep_csv_code() {
        let error = anyhow::Error::new(rwp_capture::PresentMonCaptureError::Csv(
            rwp_capture::PresentMonCsvError::EmptyCapture,
        ));
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E2200");
        assert_eq!(report.descriptor.exit_code, 22);
    }

    #[test]
    fn session_csv_errors_keep_csv_code() {
        let error = anyhow::Error::new(rwp_session::SessionError::CaptureCsv(
            rwp_capture::PresentMonCsvError::EmptyCapture,
        ));
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E2200");
        assert_eq!(report.descriptor.exit_code, 22);
    }

    #[test]
    fn session_lifecycle_errors_keep_lifecycle_code() {
        let error = anyhow::Error::new(rwp_session::SessionError::Capture(
            rwp_capture::PresentMonCaptureError::OutputAlreadyExists(std::path::PathBuf::from(
                "run.csv",
            )),
        ));
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E2100");
        assert_eq!(report.descriptor.exit_code, 21);
    }

    #[test]
    fn session_config_errors_keep_config_code() {
        let error = anyhow::Error::new(rwp_session::SessionError::CaptureConfig(
            rwp_capture::PresentMonConfigError::ZeroProcessId,
        ));
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E2000");
        assert_eq!(report.descriptor.exit_code, 20);
    }

    #[test]
    fn other_session_errors_keep_session_code() {
        let error =
            anyhow::Error::new(rwp_session::SessionError::InvalidConfiguration("synthetic"));
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E5100");
        assert_eq!(report.descriptor.exit_code, 51);
    }

    #[test]
    fn report_errors_use_stable_machine_code() {
        let error = anyhow::Error::new(rwp_report::ReportError::UnsupportedReportSchema(
            String::from("rwp.report.v9"),
        ));
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E3000");
        assert_eq!(report.descriptor.exit_code, 30);
    }

    #[cfg(not(windows))]
    #[test]
    fn unsupported_session_errors_use_stable_machine_code() {
        let error = anyhow::Error::new(rwp_session::SessionError::UnsupportedPlatform);
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E5100");
        assert_eq!(report.descriptor.exit_code, 51);
    }

    #[test]
    fn unknown_errors_have_explicit_internal_code() {
        let error = anyhow::anyhow!("synthetic unclassified failure");
        let report = classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E9000");
        assert_eq!(report.descriptor.exit_code, 90);
    }
}
