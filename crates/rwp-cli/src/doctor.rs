//! Preflight diagnostics and redacted diagnostic bundle generation.

use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

use anyhow::Result;
use rwp_capture::{PresentMonConfigError, PresentMonRunner};
use serde::Serialize;

/// Returned by the CLI when at least one executed preflight check fails.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DoctorPreflightFailed;

impl std::fmt::Display for DoctorPreflightFailed {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("one or more doctor preflight checks failed")
    }
}

impl std::error::Error for DoctorPreflightFailed {}

/// Deterministic result of one doctor preflight.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorCheck {
    /// Stable check identifier.
    pub id: &'static str,
    /// Whether the check passed.
    pub ok: Option<bool>,
    /// Redacted actionable detail.
    pub detail: String,
}

/// Deterministic doctor report suitable for stdout or a diagnostic bundle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorReport {
    /// Report schema identifier.
    pub schema_version: &'static str,
    /// RWP package version.
    pub rwp_version: &'static str,
    /// Runtime operating-system family.
    pub os: &'static str,
    /// Runtime architecture.
    pub arch: &'static str,
    /// Optional PresentMon preflight checks.
    pub presentmon: Option<DoctorPresentMon>,
    /// Ordered preflight checks.
    pub checks: Vec<DoctorCheck>,
}

impl DoctorReport {
    /// Returns true when at least one performed check failed.
    pub fn has_failures(&self) -> bool {
        self.checks.iter().any(|check| check.ok == Some(false))
    }
}

/// Redacted PresentMon executable diagnostics.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoctorPresentMon {
    /// Whether the supplied path resolves to a valid PresentMon executable.
    pub executable_valid: bool,
    /// Canonical file size in bytes, when validation succeeds.
    pub size_bytes: Option<u64>,
    /// Detected major version, when available.
    pub major_version: Option<u16>,
    /// Whether the version is supported by the current capture adapter.
    pub supported_version: Option<bool>,
    /// Whether the current Windows token satisfies the capture permission preflight.
    pub permissions_ok: Option<bool>,
    /// Authenticode signature status (`signed`/`unsigned`/`verify-failed`),
    /// when evaluated. Advisory only: v1.0 keeps user-supplied executables.
    pub signature_status: Option<&'static str>,
}

/// Runs deterministic local preflight diagnostics.
pub fn run(presentmon: Option<&Path>) -> DoctorReport {
    let mut checks = Vec::new();
    let presentmon_report = match presentmon {
        Some(path) => Some(check_presentmon(path, &mut checks)),
        None => {
            checks.push(DoctorCheck {
                id: "presentmon.path",
                ok: None,
                detail: String::from(
                    "not supplied; pass --presentmon <PATH> to validate PresentMon",
                ),
            });
            None
        }
    };

    checks.push(DoctorCheck {
        id: "offline.engine",
        ok: Some(true),
        detail: String::from("doctor performs no network access"),
    });

    DoctorReport {
        schema_version: "rwp.doctor.v1",
        rwp_version: env!("CARGO_PKG_VERSION"),
        os: std::env::consts::OS,
        arch: std::env::consts::ARCH,
        presentmon: presentmon_report,
        checks,
    }
}

/// Writes a redacted doctor report as a single deterministic JSON bundle.
pub fn write_bundle(path: &Path, report: &DoctorReport) -> Result<()> {
    if path.as_os_str().is_empty() {
        anyhow::bail!("doctor bundle path must not be empty");
    }
    let json = serde_json::to_vec_pretty(report)?;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                std::io::Error::new(
                    std::io::ErrorKind::AlreadyExists,
                    "doctor bundle already exists",
                )
            } else {
                error
            }
        })?;

    if let Err(error) = file.write_all(&json).and_then(|()| file.sync_all()) {
        drop(file);
        let _ = fs::remove_file(path);
        return Err(error.into());
    }
    Ok(())
}

fn check_presentmon(path: &Path, checks: &mut Vec<DoctorCheck>) -> DoctorPresentMon {
    let canonical = match PresentMonRunner::validate_executable(path) {
        Ok(canonical) => {
            checks.push(DoctorCheck {
                id: "presentmon.executable",
                ok: Some(true),
                detail: String::from("canonical regular file named PresentMon.exe"),
            });
            canonical
        }
        Err(error) => {
            checks.push(DoctorCheck {
                id: "presentmon.executable",
                ok: Some(false),
                detail: redact_presentmon_config_error(&error),
            });
            return DoctorPresentMon {
                executable_valid: false,
                size_bytes: None,
                major_version: None,
                supported_version: None,
                permissions_ok: presentmon_permissions(checks),
                signature_status: None,
            };
        }
    };

    let size_bytes = match fs::metadata(&canonical) {
        Ok(metadata) => Some(metadata.len()),
        Err(error) => {
            checks.push(DoctorCheck {
                id: "presentmon.metadata",
                ok: Some(false),
                detail: format!("unable to read executable metadata: {error}"),
            });
            None
        }
    };

    let (major_version, supported_version) = presentmon_version(&canonical, checks);
    let signature_status = presentmon_signature(&canonical, checks);
    DoctorPresentMon {
        executable_valid: true,
        size_bytes,
        major_version,
        supported_version,
        permissions_ok: presentmon_permissions(checks),
        signature_status,
    }
}

#[cfg(windows)]
fn presentmon_permissions(checks: &mut Vec<DoctorCheck>) -> Option<bool> {
    match rwp_system::preflight_presentmon_permissions() {
        Ok(()) => {
            checks.push(DoctorCheck {
                id: "presentmon.permissions",
                ok: Some(true),
                detail: String::from("Administrator or Performance Log Users membership detected"),
            });
            Some(true)
        }
        Err(error) => {
            checks.push(DoctorCheck {
                id: "presentmon.permissions",
                ok: Some(false),
                detail: error.to_string(),
            });
            Some(false)
        }
    }
}

#[cfg(not(windows))]
fn presentmon_permissions(checks: &mut Vec<DoctorCheck>) -> Option<bool> {
    checks.push(DoctorCheck {
        id: "presentmon.permissions",
        ok: None,
        detail: String::from(
            "Windows PresentMon permission preflight is unavailable on this platform",
        ),
    });
    None
}

#[cfg(windows)]
fn presentmon_version(path: &Path, checks: &mut Vec<DoctorCheck>) -> (Option<u16>, Option<bool>) {
    match rwp_system::detect_presentmon_version(path) {
        Ok(version) => {
            let supported = version.is_supported();
            checks.push(DoctorCheck {
                id: "presentmon.version",
                ok: Some(supported),
                detail: format!("detected PresentMon version {version}"),
            });
            (Some(version.major), Some(supported))
        }
        // Binaries without a version resource (observed on genuine PresentMon
        // 2.6.0) are version-unknown, not unsupported: advisory only, with the
        // SHA-256 recorded in session provenance and the v2 CSV schema gate
        // still rejecting non-v2 output.
        Err(rwp_system::PresentMonVersionError::UnknownVersion { .. }) => {
            checks.push(DoctorCheck {
                id: "presentmon.version",
                ok: None,
                detail: String::from(
                    "PresentMon version is unknown (no version resource); SHA-256 recorded in manifest; CSV schema gate applies",
                ),
            });
            (None, None)
        }
        Err(error) => {
            checks.push(DoctorCheck {
                id: "presentmon.version",
                ok: Some(false),
                detail: error.to_string(),
            });
            (None, None)
        }
    }
}

#[cfg(not(windows))]
fn presentmon_version(_path: &Path, checks: &mut Vec<DoctorCheck>) -> (Option<u16>, Option<bool>) {
    checks.push(DoctorCheck {
        id: "presentmon.version",
        ok: None,
        detail: String::from(
            "Windows PresentMon version-resource validation is unavailable on this platform",
        ),
    });
    (None, None)
}

/// Reports the Authenticode signature status of a validated PresentMon path.
///
/// Advisory only: an unsigned executable keeps `ok` unset (never failing the
/// preflight) because v1.0 keeps user-supplied executables; only a failed
/// verification itself is a failure. Returns the bundle-facing status string.
#[cfg(windows)]
fn presentmon_signature(path: &Path, checks: &mut Vec<DoctorCheck>) -> Option<&'static str> {
    match rwp_system::verify_presentmon_authenticode(path) {
        rwp_system::AuthenticodeStatus::Signed => {
            checks.push(DoctorCheck {
                id: "presentmon.signature",
                ok: Some(true),
                detail: String::from("Authenticode signature verified"),
            });
            Some("signed")
        }
        rwp_system::AuthenticodeStatus::NotSigned => {
            checks.push(DoctorCheck {
                id: "presentmon.signature",
                ok: None,
                detail: String::from(
                    "unsigned executable accepted for v1.0 user-supplied capture; signature enforcement deferred",
                ),
            });
            Some("unsigned")
        }
        rwp_system::AuthenticodeStatus::VerifyFailed { code } => {
            checks.push(DoctorCheck {
                id: "presentmon.signature",
                ok: Some(false),
                detail: format!("signature verification failed with status code {code}"),
            });
            Some("verify-failed")
        }
    }
}

#[cfg(not(windows))]
fn presentmon_signature(_path: &Path, checks: &mut Vec<DoctorCheck>) -> Option<&'static str> {
    checks.push(DoctorCheck {
        id: "presentmon.signature",
        ok: None,
        detail: String::from("Windows Authenticode verification is unavailable on this platform"),
    });
    None
}

fn redact_presentmon_config_error(error: &PresentMonConfigError) -> String {
    match error {
        PresentMonConfigError::ExecutableNotFound(_) => {
            String::from("PresentMon executable was not found")
        }
        PresentMonConfigError::ExecutableNotFile(_) => {
            String::from("PresentMon path is not a regular file")
        }
        PresentMonConfigError::ExecutableName(_) => {
            String::from("PresentMon executable basename must be PresentMon.exe")
        }
        PresentMonConfigError::ExecutableCanonicalize { .. } => {
            String::from("PresentMon executable path could not be canonicalized")
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::{DoctorCheck, DoctorReport, run, write_bundle};
    use std::fs;

    #[test]
    fn doctor_without_presentmon_is_deterministic() -> Result<(), Box<dyn std::error::Error>> {
        let first = run(None);
        let second = run(None);
        assert_eq!(first, second);
        assert_eq!(serde_json::to_vec(&first)?, serde_json::to_vec(&second)?);
        assert_eq!(first.schema_version, "rwp.doctor.v1");
        assert!(first.presentmon.is_none());
        assert!(!first.has_failures());
        assert!(
            first
                .checks
                .iter()
                .any(|check| check.id == "offline.engine" && check.ok == Some(true))
        );
        assert!(
            first
                .checks
                .iter()
                .any(|check| check.id == "presentmon.path" && check.ok.is_none())
        );
        Ok(())
    }

    #[test]
    fn invalid_presentmon_path_is_redacted_from_bundle() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join(format!(
            "rwp-doctor-private-path-{}-not-presentmon.exe",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        fs::write(&path, b"synthetic not-presentmon executable")?;

        let report = run(Some(&path));
        assert!(report.has_failures());
        assert!(
            report
                .checks
                .iter()
                .any(|check| check.id == "presentmon.executable" && check.ok == Some(false))
        );
        let encoded = serde_json::to_string(&report)?;
        assert!(!encoded.contains(&path.to_string_lossy().to_string()));
        assert!(!encoded.contains("rwp-doctor-private-path"));
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn bundle_refuses_overwrite_and_writes_json() -> Result<(), Box<dyn std::error::Error>> {
        let report = DoctorReport {
            schema_version: "rwp.doctor.v1",
            rwp_version: "0.4.0",
            os: "test",
            arch: "test",
            presentmon: None,
            checks: vec![DoctorCheck {
                id: "synthetic",
                ok: Some(true),
                detail: String::from("ok"),
            }],
        };
        let path = std::env::temp_dir().join(format!(
            "rwp-doctor-bundle-test-{}.json",
            std::process::id()
        ));
        let _ = fs::remove_file(&path);
        write_bundle(&path, &report)?;
        assert!(fs::read_to_string(&path)?.contains("rwp.doctor.v1"));
        assert!(write_bundle(&path, &report).is_err());
        fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn versionless_presentmon_is_advisory_not_failure() -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("rwp-doctor-noversion-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        // The test binary itself is a genuine versionless PE under the
        // pinned toolchain; copying it satisfies the basename validation
        // while preserving the absent version resource.
        let path = dir.join("PresentMon.exe");
        fs::copy(std::env::current_exe()?, &path)?;

        let report = run(Some(&path));
        let presentmon = report.presentmon.ok_or("presentmon section missing")?;
        assert!(presentmon.executable_valid);
        let version = report
            .checks
            .iter()
            .find(|check| check.id == "presentmon.version")
            .ok_or("version check missing")?;
        if cfg!(windows) {
            assert_eq!(version.ok, None);
            assert_eq!(presentmon.major_version, None);
            assert_eq!(presentmon.supported_version, None);
            assert!(version.detail.contains("unknown"));
        } else {
            assert_eq!(version.ok, None);
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }

    #[test]
    fn unsigned_presentmon_is_advisory_not_failure() -> Result<(), Box<dyn std::error::Error>> {
        let dir = std::env::temp_dir().join(format!("rwp-doctor-sig-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir)?;
        let path = dir.join("PresentMon.exe");
        fs::write(&path, b"synthetic unsigned executable")?;

        let report = run(Some(&path));
        let presentmon = report.presentmon.ok_or("presentmon section missing")?;
        assert!(presentmon.executable_valid);
        let signature = report
            .checks
            .iter()
            .find(|check| check.id == "presentmon.signature")
            .ok_or("signature check missing")?;
        if cfg!(windows) {
            assert_eq!(signature.ok, None);
            assert_eq!(presentmon.signature_status, Some("unsigned"));
        } else {
            assert_eq!(signature.ok, None);
            assert_eq!(presentmon.signature_status, None);
        }
        fs::remove_dir_all(dir)?;
        Ok(())
    }
}
