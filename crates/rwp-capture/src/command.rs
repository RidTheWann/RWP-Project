//! PresentMon process invocation boundary.

use std::{
    ffi::OsString,
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    thread,
    time::Duration,
};

#[cfg(windows)]
use rwp_system::{
    detect_presentmon_version, preflight_presentmon_permissions, request_process_group_break,
};

#[cfg(windows)]
use std::time::Instant;

use crate::{
    csv::{PresentMonCapture, PresentMonCsvParser},
    error::{PresentMonCaptureError, PresentMonConfigError},
};

/// Win32 `CREATE_NEW_PROCESS_GROUP` creation flag.
#[cfg(any(windows, test))]
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

/// Win32 `CREATE_NO_WINDOW` creation flag: the child keeps a console (so a
/// console-less parent can attach and signal) but no visible window appears.
#[cfg(any(windows, test))]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Bounded wait for a graceful PresentMon stop before the kill fallback fires.
///
/// Documented contract, kept at two seconds: Batch 7 adds a smoke-test
/// measurement step (`docs/acceptance/smoke-test.md`) so the owner can
/// revisit the value with real-PresentMon data instead of changing it blind.
pub const GRACEFUL_STOP_TIMEOUT: Duration = Duration::from_secs(2);

/// Longest timed capture accepted, in seconds. Matches the native UI limit.
pub const MAX_TIMED_SECONDS: u64 = 3600;

fn validate_timed_seconds(seconds: u64) -> Result<(), PresentMonConfigError> {
    if seconds == 0 {
        return Err(PresentMonConfigError::ZeroTimedSeconds);
    }
    if seconds > MAX_TIMED_SECONDS {
        return Err(PresentMonConfigError::TimedSecondsTooLong {
            requested: seconds,
            max: MAX_TIMED_SECONDS,
        });
    }
    Ok(())
}

/// Upper bound on PresentMon stderr text retained for diagnostics.
const MAX_STDERR_BYTES: u64 = 64 * 1024;

/// Reads at most `limit` bytes, converts them lossily to UTF-8, and drains the remainder.
///
/// The remainder is consumed and discarded so a chatty child never blocks on a full pipe.
/// Non-UTF-8 output is preserved with replacement characters instead of failing the read.
fn read_bounded_lossy<R: Read>(mut reader: R, limit: u64) -> io::Result<String> {
    let mut retained = Vec::new();
    (&mut reader).take(limit).read_to_end(&mut retained)?;
    let discarded = io::copy(&mut reader, &mut io::sink())?;
    let mut output = String::from_utf8_lossy(&retained).into_owned();
    if discarded > 0 {
        output.push_str("\n[stderr truncated; remaining output omitted]");
    }
    Ok(output)
}

/// The target process for a PresentMon capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PresentMonTarget {
    /// Match a process by executable name.
    ProcessName(String),
    /// Match a process by process identifier.
    ProcessId(u32),
}

impl PresentMonTarget {
    fn validate(&self) -> Result<(), PresentMonConfigError> {
        match self {
            Self::ProcessName(name) if name.trim().is_empty() => {
                Err(PresentMonConfigError::EmptyProcessName)
            }
            Self::ProcessId(0) => Err(PresentMonConfigError::ZeroProcessId),
            _ => Ok(()),
        }
    }
}

/// Configuration for one external PresentMon console capture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresentMonCaptureConfig {
    /// Path to the PresentMon console executable.
    pub executable: PathBuf,
    /// CSV path that PresentMon should write.
    pub output_file: PathBuf,
    /// Process that PresentMon should record.
    pub target: PresentMonTarget,
    /// Optional finite capture duration in seconds.
    pub timed_seconds: Option<u64>,
    /// Exclude dropped presents from the CSV when enabled.
    pub exclude_dropped: bool,
    /// Allow PresentMon to stop an existing trace session with the same session name.
    pub stop_existing_session: bool,
}

impl PresentMonCaptureConfig {
    fn validate(&self) -> Result<(), PresentMonConfigError> {
        if self.executable.as_os_str().is_empty() {
            return Err(PresentMonConfigError::EmptyExecutablePath);
        }
        if self.output_file.as_os_str().is_empty() {
            return Err(PresentMonConfigError::EmptyOutputPath);
        }
        self.target.validate()?;
        if let Some(seconds) = self.timed_seconds {
            validate_timed_seconds(seconds)?;
        }
        Ok(())
    }

    /// Creates a capture configuration with the M2-safe PresentMon defaults.
    pub fn new(
        executable: impl Into<PathBuf>,
        output_file: impl Into<PathBuf>,
        target: PresentMonTarget,
    ) -> Result<Self, PresentMonConfigError> {
        let executable = executable.into();
        let output_file = output_file.into();
        if executable.as_os_str().is_empty() {
            return Err(PresentMonConfigError::EmptyExecutablePath);
        }
        if output_file.as_os_str().is_empty() {
            return Err(PresentMonConfigError::EmptyOutputPath);
        }
        target.validate()?;
        Ok(Self {
            executable,
            output_file,
            target,
            timed_seconds: None,
            exclude_dropped: false,
            stop_existing_session: false,
        })
    }

    /// Sets a finite capture duration in seconds.
    pub fn with_timed_seconds(mut self, seconds: u64) -> Result<Self, PresentMonConfigError> {
        validate_timed_seconds(seconds)?;
        self.timed_seconds = Some(seconds);
        Ok(self)
    }

    /// Requests PresentMon to exclude dropped presents from its CSV.
    pub fn with_exclude_dropped(mut self, enabled: bool) -> Self {
        self.exclude_dropped = enabled;
        self
    }

    /// Explicitly allows PresentMon to stop an existing trace session with the same name.
    pub fn with_stop_existing_session(mut self, enabled: bool) -> Self {
        self.stop_existing_session = enabled;
        self
    }

    /// Returns the exact PresentMon command-line arguments used by the runner.
    pub fn arguments(&self) -> Vec<OsString> {
        let mut args = Vec::with_capacity(16);
        match &self.target {
            PresentMonTarget::ProcessName(name) => {
                args.push(OsString::from("--process_name"));
                args.push(OsString::from(name));
            }
            PresentMonTarget::ProcessId(id) => {
                args.push(OsString::from("--process_id"));
                args.push(OsString::from(id.to_string()));
            }
        }
        args.push(OsString::from("--output_file"));
        args.push(self.output_file.clone().into_os_string());
        args.push(OsString::from("--no_console_stats"));
        args.push(OsString::from("--qpc_time"));
        args.push(OsString::from("--v2_metrics"));
        args.push(OsString::from("--track_frame_type"));
        if self.stop_existing_session {
            args.push(OsString::from("--stop_existing_session"));
        }
        args.push(OsString::from("--terminate_on_proc_exit"));
        if let Some(seconds) = self.timed_seconds {
            args.push(OsString::from("--timed"));
            args.push(OsString::from(seconds.to_string()));
            args.push(OsString::from("--terminate_after_timed"));
        }
        if self.exclude_dropped {
            args.push(OsString::from("--exclude_dropped"));
        }
        args
    }
}

/// A running external PresentMon capture.
pub struct PresentMonProcess {
    child: Child,
    output_file: PathBuf,
    target: PresentMonTarget,
    stderr_reader: Option<std::thread::JoinHandle<std::io::Result<String>>>,
    #[cfg(windows)]
    _job: Option<rwp_system::PresentMonJob>,
    /// Whether the child shares the caller's console, recorded at spawn.
    /// Console-sharing children receive a direct process-group signal; children
    /// of a console-less caller are signaled through attach-then-signal.
    #[cfg(windows)]
    shares_console: bool,
}

/// How a `terminate` request finished.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopOutcome {
    /// The child exited before the kill fallback was needed.
    Graceful,
    /// The bounded wait expired and the kill fallback fired.
    Killed,
}

impl PresentMonProcess {
    /// Returns the operating-system process identifier of the capture process.
    pub fn process_id(&self) -> u32 {
        self.child.id()
    }

    /// Waits for PresentMon to exit successfully and normalizes its CSV output.
    pub fn finish(
        mut self,
        parser: &PresentMonCsvParser,
    ) -> Result<PresentMonCapture, PresentMonCaptureError> {
        let status = self.child.wait().map_err(PresentMonCaptureError::Wait)?;
        let stderr = self.take_stderr();
        if !status.success() {
            return Err(PresentMonCaptureError::ProcessFailed {
                status: status.code(),
                stderr,
            });
        }
        parser
            .parse_path(&self.output_file, &self.target)
            .map_err(PresentMonCaptureError::from)
    }

    /// Requests a graceful PresentMon shutdown, falling back to process termination after a timeout.
    pub fn terminate(&mut self) -> Result<(), PresentMonCaptureError> {
        self.terminate_with_outcome(GRACEFUL_STOP_TIMEOUT)
            .map(|_| ())
    }

    /// Requests CTRL+BREAK for the PresentMon process group and falls back to kill on timeout.
    pub fn terminate_gracefully(
        &mut self,
        timeout: Duration,
    ) -> Result<(), PresentMonCaptureError> {
        self.terminate_with_outcome(timeout).map(|_| ())
    }

    /// Stops the child like [`PresentMonProcess::terminate_gracefully`], and
    /// additionally reports whether the kill fallback fired.
    pub fn terminate_with_outcome(
        &mut self,
        timeout: Duration,
    ) -> Result<StopOutcome, PresentMonCaptureError> {
        #[cfg(windows)]
        {
            if self
                .child
                .try_wait()
                .map_err(PresentMonCaptureError::Wait)?
                .is_some()
            {
                return Ok(StopOutcome::Graceful);
            }
            let process_id = self.child.id();
            let delivered = if self.shares_console {
                request_process_group_break(process_id).is_ok()
            } else {
                rwp_system::send_break_to_detached_group(process_id).is_ok()
            };
            if !delivered {
                if self
                    .child
                    .try_wait()
                    .map_err(PresentMonCaptureError::Wait)?
                    .is_some()
                {
                    return Ok(StopOutcome::Graceful);
                }
                self.child.kill().map_err(PresentMonCaptureError::Kill)?;
                tracing::warn!(target: "rwp", event = "presentmon_kill_fallback");
                return Ok(StopOutcome::Killed);
            }
            let deadline = Instant::now() + timeout;
            loop {
                if self
                    .child
                    .try_wait()
                    .map_err(PresentMonCaptureError::Wait)?
                    .is_some()
                {
                    return Ok(StopOutcome::Graceful);
                }
                if Instant::now() >= deadline {
                    self.child.kill().map_err(PresentMonCaptureError::Kill)?;
                    tracing::warn!(target: "rwp", event = "presentmon_kill_fallback");
                    return Ok(StopOutcome::Killed);
                }
                thread::sleep(Duration::from_millis(25));
            }
        }

        #[cfg(not(windows))]
        {
            let _ = timeout;
            let _ = thread::sleep;
            self.child.kill().map_err(PresentMonCaptureError::Kill)?;
            Ok(StopOutcome::Killed)
        }
    }

    /// Wraps an already-spawned child for stop-path testing.
    ///
    /// Test support only: production children come from
    /// [`PresentMonRunner::start`]. Console sharing is recorded with the same
    /// production detector used at spawn, so the real signal path is exercised.
    #[cfg(windows)]
    #[doc(hidden)]
    pub fn from_child_for_testing(
        child: Child,
        output_file: PathBuf,
        target: PresentMonTarget,
    ) -> Self {
        Self {
            child,
            output_file,
            target,
            stderr_reader: None,
            _job: None,
            shares_console: rwp_system::caller_has_console(),
        }
    }

    fn take_stderr(&mut self) -> Option<String> {
        let reader = self.stderr_reader.take()?;
        let Ok(Ok(output)) = reader.join() else {
            return None;
        };
        let output = output.trim().to_owned();
        (!output.is_empty()).then_some(output)
    }

    /// Waits for the underlying child without parsing its CSV output.
    pub fn wait(&mut self) -> Result<ExitStatus, PresentMonCaptureError> {
        self.child.wait().map_err(PresentMonCaptureError::Wait)
    }

    /// Polls the underlying child without blocking.
    pub fn try_wait(&mut self) -> Result<Option<ExitStatus>, PresentMonCaptureError> {
        self.child.try_wait().map_err(PresentMonCaptureError::Wait)
    }

    /// Returns the CSV output path owned by this capture.
    pub fn output_file(&self) -> &Path {
        &self.output_file
    }
}

/// Starts PresentMon as a separate process without injecting into the target.
#[derive(Debug, Default, Clone, Copy)]
pub struct PresentMonRunner;

impl PresentMonRunner {
    /// Validates and canonicalizes a PresentMon executable path without launching it.
    pub fn validate_executable(path: &Path) -> Result<PathBuf, PresentMonConfigError> {
        if !path.exists() {
            return Err(PresentMonConfigError::ExecutableNotFound(
                path.to_path_buf(),
            ));
        }
        if !path.is_file() {
            return Err(PresentMonConfigError::ExecutableNotFile(path.to_path_buf()));
        }
        let canonical = fs::canonicalize(path).map_err(|source| {
            PresentMonConfigError::ExecutableCanonicalize {
                path: path.to_path_buf(),
                source,
            }
        })?;
        let valid_name = canonical
            .file_name()
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.eq_ignore_ascii_case("PresentMon.exe"));
        if !valid_name {
            return Err(PresentMonConfigError::ExecutableName(canonical));
        }
        Ok(canonical)
    }

    /// Starts a configured PresentMon capture.
    pub fn start(
        &self,
        config: &PresentMonCaptureConfig,
    ) -> Result<PresentMonProcess, PresentMonCaptureError> {
        config.validate()?;
        let executable = Self::validate_executable(&config.executable)?;
        #[cfg(windows)]
        {
            preflight_presentmon_permissions()?;
            let version = detect_presentmon_version(&executable)?;
            if !version.is_supported() {
                return Err(PresentMonCaptureError::Version(
                    rwp_system::PresentMonVersionError::UnsupportedVersion { detected: version },
                ));
            }
        }
        if config.output_file.exists() {
            return Err(PresentMonCaptureError::OutputAlreadyExists(
                config.output_file.clone(),
            ));
        }

        let mut command = Command::new(&executable);
        command
            .args(config.arguments())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(presentmon_child_creation_flags());
        }

        let mut child = command
            .spawn()
            .map_err(|source| PresentMonCaptureError::Spawn {
                path: executable.clone(),
                source,
            })?;

        #[cfg(windows)]
        let job = match rwp_system::PresentMonJob::assign_process(&child) {
            Ok(job) => Some(job),
            Err(error) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(PresentMonCaptureError::Job(error));
            }
        };

        let stderr_reader = child
            .stderr
            .take()
            .map(|stderr| thread::spawn(move || read_bounded_lossy(stderr, MAX_STDERR_BYTES)));

        Ok(PresentMonProcess {
            child,
            output_file: config.output_file.clone(),
            target: config.target.clone(),
            stderr_reader,
            #[cfg(windows)]
            _job: job,
            #[cfg(windows)]
            shares_console: rwp_system::caller_has_console(),
        })
    }
}

/// Creation flags for the PresentMon child console.
///
/// A caller with a console keeps exactly the historical flags, so the child
/// shares that console and direct process-group delivery works. A console-less
/// caller (release GUI, detached helpers) additionally passes
/// `CREATE_NO_WINDOW`: the child keeps a console for attach-then-signal
/// delivery but no visible window ever appears.
#[cfg(windows)]
pub fn presentmon_child_creation_flags() -> u32 {
    if rwp_system::caller_has_console() {
        CREATE_NEW_PROCESS_GROUP
    } else {
        CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use super::{
        CREATE_NEW_PROCESS_GROUP, CREATE_NO_WINDOW, MAX_TIMED_SECONDS, PresentMonCaptureConfig,
        PresentMonRunner, PresentMonTarget, read_bounded_lossy,
    };
    #[cfg(unix)]
    use crate::csv::PresentMonCsvParser;
    use crate::error::PresentMonConfigError;

    #[test]
    fn process_group_flag_matches_the_win32_value() {
        // The production spawn path uses this documented Win32 creation flag.
        assert_eq!(CREATE_NEW_PROCESS_GROUP, 0x0000_0200);
    }

    #[test]
    fn no_window_flag_matches_the_win32_value() {
        // The no-console spawn path adds this documented Win32 creation flag
        // so the child keeps a (hidden) console for attach-then-signal.
        assert_eq!(CREATE_NO_WINDOW, 0x0800_0000);
    }

    #[test]
    fn builds_v2_qpc_process_name_command() -> Result<(), Box<dyn std::error::Error>> {
        let config = PresentMonCaptureConfig::new(
            r"C:\Tools\PresentMon.exe",
            r"C:\Captures\run.csv",
            PresentMonTarget::ProcessName("Game.exe".to_owned()),
        )?
        .with_timed_seconds(30)?
        .with_exclude_dropped(true)
        .with_stop_existing_session(true);

        let args = config
            .arguments()
            .into_iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();

        assert_eq!(
            args,
            vec![
                "--process_name",
                "Game.exe",
                "--output_file",
                r"C:\Captures\run.csv",
                "--no_console_stats",
                "--qpc_time",
                "--v2_metrics",
                "--track_frame_type",
                "--stop_existing_session",
                "--terminate_on_proc_exit",
                "--timed",
                "30",
                "--terminate_after_timed",
                "--exclude_dropped",
            ]
        );
        Ok(())
    }

    #[test]
    fn rejects_non_presentmon_executable_names() -> Result<(), Box<dyn std::error::Error>> {
        let path =
            std::env::temp_dir().join(format!("rwp-not-presentmon-{}.exe", std::process::id()));
        std::fs::write(&path, b"test")?;
        let result = PresentMonRunner::validate_executable(&path);
        let _ = std::fs::remove_file(&path);
        assert!(matches!(
            result,
            Err(crate::error::PresentMonConfigError::ExecutableName(_))
        ));
        Ok(())
    }

    #[test]
    fn validates_empty_name_and_zero_duration() -> Result<(), Box<dyn std::error::Error>> {
        assert!(
            PresentMonCaptureConfig::new(
                "PresentMon.exe",
                "run.csv",
                PresentMonTarget::ProcessName(String::new()),
            )
            .is_err()
        );

        let config = PresentMonCaptureConfig::new(
            "PresentMon.exe",
            "run.csv",
            PresentMonTarget::ProcessId(42),
        )?;
        assert!(config.with_timed_seconds(0).is_err());
        Ok(())
    }

    #[test]
    fn timed_seconds_enforces_the_one_hour_maximum() -> Result<(), Box<dyn std::error::Error>> {
        let config = || {
            PresentMonCaptureConfig::new(
                "PresentMon.exe",
                "run.csv",
                PresentMonTarget::ProcessId(42),
            )
        };

        assert_eq!(
            config()?
                .with_timed_seconds(MAX_TIMED_SECONDS)?
                .timed_seconds,
            Some(MAX_TIMED_SECONDS)
        );
        assert!(matches!(
            config()?.with_timed_seconds(MAX_TIMED_SECONDS + 1),
            Err(PresentMonConfigError::TimedSecondsTooLong { requested, max })
                if requested == MAX_TIMED_SECONDS + 1 && max == MAX_TIMED_SECONDS
        ));
        Ok(())
    }

    #[test]
    fn public_timed_seconds_field_cannot_bypass_the_maximum()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut config = PresentMonCaptureConfig::new(
            "PresentMon.exe",
            "run.csv",
            PresentMonTarget::ProcessId(42),
        )?;
        config.timed_seconds = Some(u64::MAX);

        assert!(matches!(
            config.validate(),
            Err(PresentMonConfigError::TimedSecondsTooLong { requested, max })
                if requested == u64::MAX && max == MAX_TIMED_SECONDS
        ));
        Ok(())
    }

    #[test]
    fn stderr_reader_does_not_mark_exact_limit_as_truncated()
    -> Result<(), Box<dyn std::error::Error>> {
        let data = vec![b'x'; 10];
        let mut cursor = Cursor::new(data);

        let output = read_bounded_lossy(&mut cursor, 10)?;

        assert_eq!(output, "x".repeat(10));
        assert_eq!(cursor.position(), 10);
        Ok(())
    }
    #[test]
    fn stderr_reader_truncates_and_drains() -> Result<(), Box<dyn std::error::Error>> {
        let data = vec![b'x'; 1_000];
        let mut cursor = Cursor::new(data);

        let output = read_bounded_lossy(&mut cursor, 10)?;

        assert!(output.starts_with(&"x".repeat(10)));
        assert!(output.contains("[stderr truncated; remaining output omitted]"));
        assert_eq!(cursor.position(), 1_000);
        Ok(())
    }

    #[test]
    fn stderr_reader_keeps_non_utf8_output_lossily() -> Result<(), Box<dyn std::error::Error>> {
        let output = read_bounded_lossy(Cursor::new(vec![b'o', b'k', 0xff, b'!']), 1_024)?;

        assert_eq!(output, "ok\u{fffd}!");
        Ok(())
    }

    #[test]
    fn validate_rejects_zero_process_id() {
        // The constructor validates the target up front.
        assert!(matches!(
            PresentMonCaptureConfig::new(
                "PresentMon.exe",
                "run.csv",
                PresentMonTarget::ProcessId(0),
            ),
            Err(PresentMonConfigError::ZeroProcessId)
        ));
    }

    #[test]
    fn validate_rejects_empty_paths() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = PresentMonCaptureConfig::new(
            "PresentMon.exe",
            "run.csv",
            PresentMonTarget::ProcessId(42),
        )?;
        config.executable = std::path::PathBuf::new();
        assert!(matches!(
            config.validate(),
            Err(PresentMonConfigError::EmptyExecutablePath)
        ));
        config.executable = std::path::PathBuf::from("PresentMon.exe");
        config.output_file = std::path::PathBuf::new();
        assert!(matches!(
            config.validate(),
            Err(PresentMonConfigError::EmptyOutputPath)
        ));
        Ok(())
    }

    #[test]
    fn new_rejects_empty_paths() {
        assert!(matches!(
            PresentMonCaptureConfig::new("", "run.csv", PresentMonTarget::ProcessId(42)),
            Err(PresentMonConfigError::EmptyExecutablePath)
        ));
        assert!(matches!(
            PresentMonCaptureConfig::new("PresentMon.exe", "", PresentMonTarget::ProcessId(42)),
            Err(PresentMonConfigError::EmptyOutputPath)
        ));
    }

    #[test]
    fn builds_process_id_arguments_without_optionals() -> Result<(), Box<dyn std::error::Error>> {
        let config = PresentMonCaptureConfig::new(
            "PresentMon.exe",
            "run.csv",
            PresentMonTarget::ProcessId(7),
        )?;
        let args = config
            .arguments()
            .into_iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            args,
            vec![
                "--process_id",
                "7",
                "--output_file",
                "run.csv",
                "--no_console_stats",
                "--qpc_time",
                "--v2_metrics",
                "--track_frame_type",
                "--terminate_on_proc_exit",
            ]
        );
        Ok(())
    }

    #[test]
    fn validate_executable_rejects_missing_and_directory() -> Result<(), Box<dyn std::error::Error>>
    {
        let missing =
            std::env::temp_dir().join(format!("rwp-missing-presentmon-{}.exe", std::process::id()));
        let _ = std::fs::remove_file(&missing);
        assert!(matches!(
            PresentMonRunner::validate_executable(&missing),
            Err(PresentMonConfigError::ExecutableNotFound(_))
        ));
        let dir = std::env::temp_dir().join(format!("rwp-presentmon-dir-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir)?;
        let result = PresentMonRunner::validate_executable(&dir);
        let _ = std::fs::remove_dir_all(&dir);
        assert!(matches!(
            result,
            Err(PresentMonConfigError::ExecutableNotFile(_))
        ));
        Ok(())
    }

    // Windows runs PresentMon preflight and version detection before the
    // output-collision check, which needs a real PresentMon executable that
    // CI does not have; the coverage gate runs on Linux.
    #[cfg(unix)]
    #[test]
    fn start_rejects_existing_output() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("rwp-existing-output-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        let executable = root.join("PresentMon.exe");
        let output = root.join("run.csv");
        std::fs::write(&executable, b"stub")?;
        std::fs::write(&output, b"existing")?;
        let config =
            PresentMonCaptureConfig::new(&executable, &output, PresentMonTarget::ProcessId(42))?;
        let result = PresentMonRunner.start(&config);
        let _ = std::fs::remove_dir_all(&root);
        assert!(matches!(
            result,
            Err(crate::error::PresentMonCaptureError::OutputAlreadyExists(_))
        ));
        Ok(())
    }

    // Same Windows preflight ordering constraint as above: spawning a fake
    // executable only reaches the spawn error on non-Windows hosts.
    #[cfg(unix)]
    #[test]
    fn start_rejects_unspawnable_executable() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("rwp-unspawnable-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        let executable = root.join("PresentMon.exe");
        let output = root.join("run.csv");
        std::fs::write(&executable, b"not an executable")?;
        let config =
            PresentMonCaptureConfig::new(&executable, &output, PresentMonTarget::ProcessId(42))?;
        let result = PresentMonRunner.start(&config);
        let _ = std::fs::remove_dir_all(&root);
        assert!(matches!(
            result,
            Err(crate::error::PresentMonCaptureError::Spawn { .. })
        ));
        Ok(())
    }

    #[cfg(unix)]
    fn write_unix_stub(
        root: &std::path::Path,
        body: &str,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        use std::os::unix::fs::PermissionsExt;
        let executable = root.join("PresentMon.exe");
        std::fs::write(&executable, body)?;
        let mut permissions = std::fs::metadata(&executable)?.permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(&executable, permissions)?;
        Ok(executable)
    }

    #[cfg(unix)]
    #[test]
    fn unix_stub_process_completes_capture() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("rwp-stub-success-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        let executable = write_unix_stub(
            &root,
            "#!/bin/sh\nout=\"\"\nprev=\"\"\nfor arg in \"$@\"; do\nif [ \"$prev\" = \"--output_file\" ]; then\nout=\"$arg\"\nfi\nprev=\"$arg\"\ndone\nprintf 'Application,ProcessID,CPUStartQPC,FrameTime,CPUBusy,CPUWait,GPUTime,GPUBusy,GPUWait,DisplayLatency\\nGame.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0\\nGame.exe,42,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0\\n' > \"$out\"\necho \"stub ready\"\n",
        )?;
        let output = root.join("run.csv");
        let config =
            PresentMonCaptureConfig::new(&executable, &output, PresentMonTarget::ProcessId(42))?;
        let mut process = PresentMonRunner.start(&config)?;
        assert!(process.process_id() != 0);
        assert_eq!(process.output_file(), output.as_path());
        assert!(process.try_wait()?.is_some() || process.wait()?.success());
        let parser = PresentMonCsvParser::new(1_000_000)?;
        let capture = process.finish(&parser)?;
        assert_eq!(capture.frames.len(), 2);
        assert_eq!(capture.process_id, 42);
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn unix_failing_stub_reports_process_status() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("rwp-stub-fail-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        let executable = write_unix_stub(&root, "#!/bin/sh\necho boom >&2\nexit 3\n")?;
        let output = root.join("run.csv");
        let config =
            PresentMonCaptureConfig::new(&executable, &output, PresentMonTarget::ProcessId(42))?;
        let process = PresentMonRunner.start(&config)?;
        let parser = PresentMonCsvParser::new(1_000_000)?;
        let result = process.finish(&parser);
        let _ = std::fs::remove_dir_all(&root);
        assert!(matches!(
            result,
            Err(crate::error::PresentMonCaptureError::ProcessFailed {
                status: Some(3),
                ..
            })
        ));
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn unix_terminate_kills_stub() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("rwp-stub-kill-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        let executable = write_unix_stub(&root, "#!/bin/sh\nsleep 30\n")?;
        let output = root.join("run.csv");
        let config =
            PresentMonCaptureConfig::new(&executable, &output, PresentMonTarget::ProcessId(42))?;
        let mut process = PresentMonRunner.start(&config)?;
        process.terminate()?;
        process.wait()?;
        let _ = std::fs::remove_dir_all(&root);
        Ok(())
    }

    /// Windows-only behavioral proof for AUD-03 graceful stop.
    ///
    /// These tests exercise the REAL production stop path
    /// (`PresentMonProcess::terminate_gracefully` -> `rwp_system::request_process_group_break`
    /// -> bounded 2 s wait -> kill fallback, then `ParseMode::TolerateTruncatedTail`)
    /// against a fake child console process built from this repo
    /// (`crates/rwp-system/examples/fake_ctrl_break_child.rs`, no new dependencies,
    /// never shipped in the installer or release artifacts).
    ///
    /// Timing rationale (anti-flake): the production bounded wait is 2 s. Tests use
    /// generous outer bounds (ready poll up to 15 s at 50 ms steps; overall completion
    /// up to 15 s; kill-fallback lower bound 1500 ms to allow 500 ms scheduling slack)
    /// so a loaded machine cannot flake a pass into a fail. The lower bound only guards
    /// that the kill fallback actually waited; the upper bounds guard hangs.
    #[cfg(windows)]
    mod graceful_stop_behavior {
        use super::super::{CREATE_NEW_PROCESS_GROUP, GRACEFUL_STOP_TIMEOUT};
        use crate::csv::{ParseMode, PresentMonCsvParser};
        use crate::{PresentMonProcess, PresentMonTarget};
        use std::path::PathBuf;
        use std::time::{Duration, Instant};

        fn fake_child_path() -> Result<PathBuf, Box<dyn std::error::Error>> {
            let exe = std::env::current_exe()?;
            let deps_dir = exe
                .parent()
                .ok_or("current test executable has no parent directory")?;
            let profile_dir = deps_dir
                .parent()
                .ok_or("current test executable has no profile directory")?;
            Ok(profile_dir
                .join("examples")
                .join("fake_ctrl_break_child.exe"))
        }

        fn wait_for_ready(ready: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                if ready.exists() {
                    return Ok(());
                }
                if Instant::now() >= deadline {
                    return Err(format!(
                        "fake child never signaled readiness at {}",
                        ready.display()
                    )
                    .into());
                }
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        fn spawn_fake_child(
            helper: &std::path::Path,
            mode: &str,
            csv: &std::path::Path,
            ready: &std::path::Path,
        ) -> Result<std::process::Child, Box<dyn std::error::Error>> {
            use std::os::windows::process::CommandExt;
            use std::process::{Command, Stdio};
            let child = Command::new(helper)
                .arg("--mode")
                .arg(mode)
                .arg("--csv")
                .arg(csv)
                .arg("--ready")
                .arg(ready)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NEW_PROCESS_GROUP)
                .spawn()?;
            Ok(child)
        }

        #[test]
        fn graceful_break_flushes_final_row_and_exits_zero()
        -> Result<(), Box<dyn std::error::Error>> {
            let root = std::env::temp_dir().join(format!(
                "rwp-graceful-stop-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root)?;
            let csv = root.join("run.csv");
            let ready = root.join("ready");
            let helper = fake_child_path()?;
            if !helper.exists() {
                return Err(format!(
                    "fake child helper missing at {}; build it with: cargo build --locked -p rwp-system --examples",
                    helper.display()
                )
                .into());
            }
            let child = spawn_fake_child(&helper, "graceful", &csv, &ready)?;
            wait_for_ready(&ready)?;
            let mut process = PresentMonProcess {
                child,
                output_file: csv.clone(),
                target: PresentMonTarget::ProcessId(42),
                stderr_reader: None,
                _job: None,
                shares_console: rwp_system::caller_has_console(),
            };
            let started = Instant::now();
            process.terminate_gracefully(GRACEFUL_STOP_TIMEOUT)?;
            let status = process.wait()?;
            let elapsed = started.elapsed();
            assert!(
                status.success(),
                "graceful fake child must exit 0, got {status:?}"
            );
            assert!(
                elapsed < Duration::from_secs(15),
                "graceful stop must complete within the generous 15 s bound, took {elapsed:?}"
            );
            let parser = PresentMonCsvParser::new(1_000_000)?;
            let capture = parser.parse_path_with_mode(
                &csv,
                &PresentMonTarget::ProcessId(42),
                ParseMode::TolerateTruncatedTail,
            )?;
            assert_eq!(
                capture.frames.len(),
                3,
                "graceful child must flush header + 2 startup rows + 1 final row"
            );
            let _ = std::fs::remove_dir_all(&root);
            Ok(())
        }

        #[test]
        fn ignored_break_falls_back_to_kill_and_tolerates_tail()
        -> Result<(), Box<dyn std::error::Error>> {
            let root = std::env::temp_dir().join(format!(
                "rwp-kill-fallback-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_nanos())
                    .unwrap_or(0)
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root)?;
            let csv = root.join("run.csv");
            let ready = root.join("ready");
            let helper = fake_child_path()?;
            if !helper.exists() {
                return Err(format!(
                    "fake child helper missing at {}; build it with: cargo build --locked -p rwp-system --examples",
                    helper.display()
                )
                .into());
            }
            let child = spawn_fake_child(&helper, "ignore", &csv, &ready)?;
            wait_for_ready(&ready)?;
            let mut process = PresentMonProcess {
                child,
                output_file: csv.clone(),
                target: PresentMonTarget::ProcessId(42),
                stderr_reader: None,
                _job: None,
                shares_console: rwp_system::caller_has_console(),
            };
            let started = Instant::now();
            process.terminate_gracefully(GRACEFUL_STOP_TIMEOUT)?;
            let status = process.wait()?;
            let elapsed = started.elapsed();
            assert!(
                elapsed >= Duration::from_millis(1500),
                "kill fallback must wait out the bounded 2 s wait (1500 ms lower bound allows scheduling slack), took {elapsed:?}"
            );
            assert!(
                elapsed < Duration::from_secs(15),
                "kill fallback must complete within the generous 15 s bound, took {elapsed:?}"
            );
            assert!(
                !status.success(),
                "killed fake child must not exit 0, got {status:?}"
            );
            let parser = PresentMonCsvParser::new(1_000_000)?;
            assert!(
                parser
                    .parse_path(&csv, &PresentMonTarget::ProcessId(42))
                    .is_err(),
                "truncated tail after kill must fail strict parsing"
            );
            let capture = parser.parse_path_with_mode(
                &csv,
                &PresentMonTarget::ProcessId(42),
                ParseMode::TolerateTruncatedTail,
            )?;
            assert_eq!(
                capture.frames.len(),
                2,
                "tolerant parse after kill must retain the 2 complete rows and drop the truncated tail"
            );
            let _ = std::fs::remove_dir_all(&root);
            Ok(())
        }

        /// AUD-03b: the same production stop path driven from a console-less
        /// parent (release GUI shape).
        ///
        /// The test spawns `no_console_stop_driver` (this crate's examples)
        /// with `DETACHED_PROCESS`, so the driver has no console exactly like
        /// the GUI. The driver spawns the fake child with the production
        /// spawn policy, stops it with `terminate_with_outcome` (production
        /// 2 s bound), and reports a result file; the test only asserts on
        /// that file, never on timing.
        ///
        /// Timing rationale (anti-flake): no assertion depends on elapsed
        /// time. Generous deadlines only guard hangs — driver completion up
        /// to 180 s (the driver bounds itself far below that), ready poll
        /// inside the driver up to 15 s. Helpers are built by the workspace
        /// gate before any test runs; package-selected runs pre-build them
        /// once (see `stop_helper_paths`). A loaded machine slows the test
        /// down but cannot flip a pass into a fail.
        const DETACHED_PROCESS: u32 = 0x0000_0008;

        struct NoConsoleDriverResult {
            detected_no_console: bool,
            outcome: String,
            child_success: bool,
            frames: usize,
            strict_failed: bool,
        }

        fn example_binary(name: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
            let exe = std::env::current_exe()?;
            let deps_dir = exe
                .parent()
                .ok_or("current test executable has no parent directory")?;
            let profile_dir = deps_dir
                .parent()
                .ok_or("current test executable has no profile directory")?;
            Ok(profile_dir.join("examples").join(name))
        }

        fn stop_helper_paths() -> Result<(PathBuf, PathBuf), Box<dyn std::error::Error>> {
            // The workspace gate (`cargo test --workspace --all-features
            // --locked`, the verification authority) builds every example
            // before running any test — verified by deleting both helper
            // binaries and watching `--no-run` rebuild them — so the helpers
            // exist here. A nested `cargo build` from inside a test is
            // deliberately NOT used: the outer run links example binaries
            // concurrently, and two linkers on the same `.exe` fail with
            // `LNK1104: cannot open file
            // 'target\debug\examples\fake_ctrl_break_child.exe'` (observed
            // twice under full-workspace runs). For package-selected runs,
            // pre-build once with: `cargo build --locked --examples
            // -p rwp-capture -p rwp-system`.
            let driver = example_binary("no_console_stop_driver.exe")?;
            let helper = example_binary("fake_ctrl_break_child.exe")?;
            for (label, path) in [
                ("no-console driver", &driver),
                ("fake child helper", &helper),
            ] {
                if !path.exists() {
                    return Err(format!(
                        "{label} missing at {}; the workspace gate builds it automatically, otherwise pre-build once with: cargo build --locked --examples -p rwp-capture -p rwp-system",
                        path.display()
                    )
                    .into());
                }
            }
            Ok((driver, helper))
        }

        fn run_no_console_driver(
            mode: &str,
        ) -> Result<NoConsoleDriverResult, Box<dyn std::error::Error>> {
            use std::os::windows::process::CommandExt;
            use std::process::{Command, Stdio};

            let (driver, helper) = stop_helper_paths()?;
            let root = std::env::temp_dir().join(format!(
                "rwp-no-console-{mode}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root)?;
            let csv = root.join("run.csv");
            let ready = root.join("ready");
            let result_path = root.join("result.txt");
            let driver_stdout = root.join("driver.stdout.txt");
            let driver_stderr = root.join("driver.stderr.txt");
            let stdout_file = std::fs::File::create(&driver_stdout)?;
            let stderr_file = std::fs::File::create(&driver_stderr)?;
            let mut driver_child = Command::new(&driver)
                .arg("--helper")
                .arg(&helper)
                .arg("--mode")
                .arg(mode)
                .arg("--csv")
                .arg(&csv)
                .arg("--ready")
                .arg(&ready)
                .arg("--result")
                .arg(&result_path)
                .stdin(Stdio::null())
                .stdout(Stdio::from(stdout_file))
                .stderr(Stdio::from(stderr_file))
                .creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
                .spawn()
                .map_err(|error| format!("spawn no-console driver failed: {error}"))?;
            let deadline = Instant::now() + Duration::from_secs(180);
            let driver_status = loop {
                let status = driver_child
                    .try_wait()
                    .map_err(|error| format!("poll no-console driver failed: {error}"))?;
                match status {
                    Some(status) => break status,
                    None => {
                        if Instant::now() >= deadline {
                            let _ = driver_child.kill();
                            return Err("no-console driver timed out after 180 s".into());
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                }
            };
            if !driver_status.success() {
                let stdout_tail = std::fs::read_to_string(&driver_stdout)
                    .unwrap_or_else(|_| String::from("<unreadable>"));
                let stderr_tail = std::fs::read_to_string(&driver_stderr)
                    .unwrap_or_else(|_| String::from("<unreadable>"));
                return Err(format!(
                    "no-console driver exited {driver_status:?}; stdout: {stdout_tail}; stderr: {stderr_tail}"
                )
                .into());
            }
            let contents = std::fs::read_to_string(&result_path)
                .map_err(|error| format!("read no-console driver result failed: {error}"))?;
            let mut detected_no_console = false;
            let mut outcome = String::new();
            let mut child_success = false;
            let mut frames = 0_usize;
            let mut strict_failed = false;
            for line in contents.lines() {
                let (key, value) = line
                    .split_once('=')
                    .ok_or_else(|| format!("malformed driver result line: {line}"))?;
                match key {
                    "detected_no_console" => detected_no_console = value == "true",
                    "outcome" => outcome = value.to_owned(),
                    "child_success" => child_success = value == "true",
                    "frames" => {
                        frames = value
                            .parse()
                            .map_err(|_| format!("malformed frames value: {value}"))?;
                    }
                    "strict_failed" => strict_failed = value == "true",
                    _ => {}
                }
            }
            let _ = std::fs::remove_dir_all(&root);
            Ok(NoConsoleDriverResult {
                detected_no_console,
                outcome,
                child_success,
                frames,
                strict_failed,
            })
        }

        #[test]
        fn no_console_parent_graceful_stop() -> Result<(), Box<dyn std::error::Error>> {
            let result = run_no_console_driver("graceful")?;
            assert!(
                result.detected_no_console,
                "driver must observe no console, or this is not the GUI case"
            );
            assert_eq!(
                result.outcome, "graceful",
                "console-less graceful stop must not use the kill fallback"
            );
            assert_eq!(
                result.frames, 3,
                "console-less graceful stop must flush the final row"
            );
            assert!(
                result.child_success,
                "console-less graceful child must exit 0"
            );
            Ok(())
        }

        /// Console-stop handler probe (Ctrl+C work): a helper in its OWN process
        /// group installs the production guard and the test delivers a real
        /// CTRL+BREAK to that group (never CTRL+C, which cannot be targeted and
        /// would hit the cargo terminal). Survival plus the result file proves
        /// the signal was consumed; a second signal is covered by the pure
        /// `decide_console_signal` matrix in rwp-system.
        ///
        /// Timing rationale (anti-flake): no assertion depends on elapsed
        /// time. Deadlines only guard hangs — helper ready up to 15 s, helper
        /// exit up to 60 s (the probe bounds itself at 30 s).
        fn run_console_stop_probe() -> Result<bool, Box<dyn std::error::Error>> {
            use std::os::windows::process::CommandExt;
            use std::process::{Command, Stdio};

            let probe = example_binary("console_stop_probe.exe")?;
            if !probe.exists() {
                return Err(format!(
                    "console-stop probe missing at {}; the workspace gate builds it automatically, otherwise pre-build once with: cargo build --locked --examples -p rwp-capture",
                    probe.display()
                )
                .into());
            }
            let root = std::env::temp_dir().join(format!(
                "rwp-stop-probe-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|duration| duration.as_nanos())
                    .unwrap_or(0)
            ));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root)?;
            let ready = root.join("ready");
            let result_path = root.join("result.txt");
            let mut probe_child = Command::new(&probe)
                .arg("--ready")
                .arg(&ready)
                .arg("--result")
                .arg(&result_path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(CREATE_NEW_PROCESS_GROUP)
                .spawn()
                .map_err(|error| format!("spawn console-stop probe failed: {error}"))?;
            wait_for_ready(&ready)?;
            rwp_system::request_process_group_break(probe_child.id())
                .map_err(|error| format!("deliver CTRL+BREAK to probe failed: {error}"))?;
            let deadline = Instant::now() + Duration::from_secs(60);
            let probe_status = loop {
                let status = probe_child
                    .try_wait()
                    .map_err(|error| format!("poll console-stop probe failed: {error}"))?;
                match status {
                    Some(status) => break status,
                    None => {
                        if Instant::now() >= deadline {
                            let _ = probe_child.kill();
                            return Err("console-stop probe timed out after 60 s".into());
                        }
                        std::thread::sleep(Duration::from_millis(200));
                    }
                }
            };
            if !probe_status.success() {
                return Err(format!(
                    "console-stop probe exited {probe_status:?} without consuming the signal"
                )
                .into());
            }
            let contents = std::fs::read_to_string(&result_path)
                .map_err(|error| format!("read console-stop probe result failed: {error}"))?;
            let stopping = contents.lines().any(|line| line == "stopping=true");
            let _ = std::fs::remove_dir_all(&root);
            Ok(stopping)
        }

        #[test]
        fn console_break_sets_the_shared_stop_flag() -> Result<(), Box<dyn std::error::Error>> {
            assert!(
                run_console_stop_probe()?,
                "a delivered CTRL+BREAK must flip the shared stop flag"
            );
            Ok(())
        }

        #[test]
        fn no_console_parent_ignored_break_killed_and_tolerated()
        -> Result<(), Box<dyn std::error::Error>> {
            let result = run_no_console_driver("ignore")?;
            assert!(
                result.detected_no_console,
                "driver must observe no console, or this is not the GUI case"
            );
            assert_eq!(
                result.outcome, "killed",
                "an ignoring child must take the kill fallback after the bounded wait"
            );
            assert!(
                result.strict_failed,
                "truncated tail after kill must fail strict parsing"
            );
            assert_eq!(
                result.frames, 2,
                "tolerant parse after kill must retain the 2 complete rows"
            );
            assert!(!result.child_success, "killed child must not exit 0");
            Ok(())
        }
    }
}
