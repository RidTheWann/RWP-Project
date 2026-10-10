//! Windows live capture orchestration.

use std::{
    fs,
    path::PathBuf,
    sync::atomic::{AtomicBool, Ordering},
    thread,
    time::{Duration, Instant},
};

use crate::{CoverageSummary, SessionProvenance};
use rwp_analysis::{StatisticsConfig, StutterConfig};
use rwp_capture::{
    ParseMode, PresentMonCaptureConfig, PresentMonCsvParser, PresentMonProcess, PresentMonRunner,
    PresentMonTarget,
};
use rwp_core::{Diagnostic, SystemSample};
use rwp_diagnostics::{DiagnosisInput, DiagnosticEngine, DiagnosticError, RuleThresholds};
use rwp_report::{
    CaptureDocument, PerformanceReport, ReportError, StutterAnalysisConfig,
    build_report_with_stutter_analysis, with_diagnostics,
};
use rwp_system::{
    DisplayRefreshRate, QpcTimebase, SystemTelemetryError, WindowsSystemSampler,
    detect_process_display_refresh_rate,
};
use sha2::{Digest, Sha256};

/// Minimum Windows telemetry sampling interval in milliseconds.
///
/// Owner bound: below this each tick still pays two ToolHelp snapshots plus
/// per-entity thread/process handle opens, so a 1 ms cadence would approach
/// ~2000 snapshots/s. The GUI enforces the same bound; the Tauri command
/// passes the value through to [`SessionConfig::with_sample_interval`].
pub const MIN_SAMPLE_INTERVAL_MS: u64 = 10;

/// Configuration for one live PresentMon + Windows telemetry session.
#[derive(Debug, Clone)]
pub struct SessionConfig {
    /// PresentMon executable path.
    pub presentmon_executable: PathBuf,
    /// CSV output path for the capture.
    pub output_file: PathBuf,
    /// Target process identifier.
    pub process_id: u32,
    /// Maximum capture duration.
    pub duration: Duration,
    /// Windows telemetry sampling interval.
    pub sample_interval: Duration,
    /// Whether dropped presents should be excluded.
    pub exclude_dropped: bool,
    /// Whether an existing PresentMon trace session may be stopped before capture.
    pub stop_existing_session: bool,
    /// Whether to disable RWP's additional process/thread telemetry probing.
    pub no_process_probing: bool,
    /// Diagnostic thresholds applied after capture.
    pub diagnostic_thresholds: RuleThresholds,
    /// Optional directory where the normalized capture, report, telemetry, and manifest are persisted.
    pub persistence_dir: Option<PathBuf>,
}

impl SessionConfig {
    /// Creates a validated session configuration with conservative sampling defaults.
    pub fn new(
        presentmon_executable: impl Into<PathBuf>,
        output_file: impl Into<PathBuf>,
        process_id: u32,
        duration_seconds: u64,
    ) -> Result<Self, SessionError> {
        if process_id == 0 {
            return Err(SessionError::InvalidConfiguration(
                "process_id must be greater than zero",
            ));
        }
        if duration_seconds == 0 {
            return Err(SessionError::InvalidConfiguration(
                "duration_seconds must be greater than zero",
            ));
        }

        Ok(Self {
            presentmon_executable: presentmon_executable.into(),
            output_file: output_file.into(),
            process_id,
            duration: Duration::from_secs(duration_seconds),
            sample_interval: Duration::from_millis(50),
            exclude_dropped: false,
            stop_existing_session: false,
            no_process_probing: false,
            diagnostic_thresholds: RuleThresholds::default(),
            persistence_dir: None,
        })
    }

    /// Sets the Windows telemetry sampling interval.
    pub fn with_sample_interval(mut self, interval: Duration) -> Result<Self, SessionError> {
        // Owner bound (10 ms): below this each tick still pays two ToolHelp
        // snapshots plus per-entity OpenThread/OpenProcess handles, so a 1 ms
        // cadence would approach ~2000 snapshots/s. The GUI enforces the same
        // bound; the Tauri command passes the value through unchanged.
        if interval < Duration::from_millis(MIN_SAMPLE_INTERVAL_MS)
            || interval > Duration::from_secs(5)
        {
            return Err(SessionError::InvalidConfiguration(
                "sample_interval must be between 10 ms and 5 s",
            ));
        }
        self.sample_interval = interval;
        Ok(self)
    }

    /// Enables exclusion of dropped presents in the PresentMon capture.
    pub fn with_exclude_dropped(mut self, enabled: bool) -> Self {
        self.exclude_dropped = enabled;
        self
    }

    /// Explicitly allows this session to stop an existing PresentMon trace session.
    pub fn with_stop_existing_session(mut self, enabled: bool) -> Self {
        self.stop_existing_session = enabled;
        self
    }

    /// Disables RWP's process/thread telemetry probing for this session.
    pub fn with_no_process_probing(mut self, disabled: bool) -> Self {
        self.no_process_probing = disabled;
        self
    }

    /// Selects a directory for deterministic session persistence.
    pub fn with_persistence_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.persistence_dir = Some(directory.into());
        self
    }

    /// Overrides the deterministic diagnostic thresholds.
    pub fn with_diagnostic_thresholds(mut self, thresholds: RuleThresholds) -> Self {
        self.diagnostic_thresholds = thresholds;
        self
    }

    fn presentmon_config(&self) -> Result<PresentMonCaptureConfig, SessionError> {
        let config = PresentMonCaptureConfig::new(
            self.presentmon_executable.clone(),
            self.output_file.clone(),
            PresentMonTarget::ProcessId(self.process_id),
        )?;
        let config = config
            .with_timed_seconds(self.duration.as_secs())?
            .with_exclude_dropped(self.exclude_dropped)
            .with_stop_existing_session(self.stop_existing_session);
        Ok(config)
    }
}

/// Abstracts Windows telemetry collection so session orchestration can be tested with deterministic fakes.
pub trait SystemSampler {
    /// Collects one telemetry observation; sampling failures are non-fatal to the capture.
    fn sample(&mut self) -> Result<SystemSample, SystemTelemetryError>;
}

impl SystemSampler for WindowsSystemSampler {
    fn sample(&mut self) -> Result<SystemSample, SystemTelemetryError> {
        WindowsSystemSampler::sample(self)
    }
}

/// A non-fatal telemetry collection warning attached to a completed session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TelemetryWarning {
    /// Stable human-readable warning detail.
    pub message: String,
}

/// Completed, fully analyzed session output.
#[derive(Debug)]
pub struct SessionResult {
    /// Target process identifier used for the capture.
    pub process_id: u32,
    /// Active-display refresh rate captured at session start, when available.
    pub display_refresh_rate: Option<DisplayRefreshRate>,
    /// Non-fatal display refresh-rate lookup warning.
    pub display_refresh_warning: Option<String>,
    /// Dominant PresentMon swap-chain address, when available.
    pub dominant_swap_chain_address: Option<u64>,
    /// Number of source rows discarded outside the dominant swap chain.
    pub discarded_frame_count: usize,
    /// PresentMon frame-type distribution retained after swap-chain filtering.
    pub frame_type_counts: Vec<(String, usize)>,
    /// Application name observed by PresentMon.
    pub application: String,
    /// Raw normalized Windows telemetry retained for later analysis/export.
    pub system_samples: Vec<SystemSample>,
    /// Deterministic performance report.
    pub report: PerformanceReport,
    /// Non-fatal telemetry failures observed during capture.
    pub telemetry_warnings: Vec<TelemetryWarning>,
    /// Deterministic diagnosis results.
    pub diagnostics: Vec<Diagnostic>,
    /// Symmetric frame/telemetry observation coverage.
    pub coverage: CoverageSummary,
}

fn timestamp_span_seconds(start: Option<u64>, end: Option<u64>) -> Option<f64> {
    Some(end?.saturating_sub(start?) as f64 / 1_000_000_000.0)
}

fn build_coverage(frames: &[rwp_core::FrameSample], samples: &[SystemSample]) -> CoverageSummary {
    let frame_start = frames.first().map(|frame| frame.timestamp_ns);
    let frame_end = frames.last().map(|frame| frame.timestamp_ns);
    let telemetry_start = samples.first().map(|sample| sample.timestamp_ns);
    let telemetry_end = samples.last().map(|sample| sample.timestamp_ns);
    let frame_seconds = timestamp_span_seconds(frame_start, frame_end);
    let telemetry_seconds = timestamp_span_seconds(telemetry_start, telemetry_end);
    let intersection_seconds = match (frame_start, frame_end, telemetry_start, telemetry_end) {
        (Some(frame_start), Some(frame_end), Some(telemetry_start), Some(telemetry_end)) => {
            let start = frame_start.max(telemetry_start);
            let end = frame_end.min(telemetry_end);
            Some(end.saturating_sub(start) as f64 / 1_000_000_000.0)
        }
        _ => None,
    };
    CoverageSummary {
        frame_seconds,
        telemetry_seconds,
        intersection_seconds,
    }
}

/// Errors raised by live session orchestration.
#[derive(Debug, thiserror::Error)]
pub enum SessionError {
    /// Session configuration is invalid.
    #[error("invalid session configuration: {0}")]
    InvalidConfiguration(&'static str),
    /// PresentMon failed during capture setup or collection.
    #[error(transparent)]
    Capture(#[from] rwp_capture::PresentMonCaptureError),
    /// PresentMon configuration validation failed.
    #[error(transparent)]
    CaptureConfig(#[from] rwp_capture::PresentMonConfigError),
    /// PresentMon CSV normalization failed.
    #[error(transparent)]
    CaptureCsv(#[from] rwp_capture::PresentMonCsvError),
    /// Windows telemetry collection failed.
    #[error(transparent)]
    Telemetry(#[from] rwp_system::SystemTelemetryError),
    /// Report construction failed.
    #[error(transparent)]
    Report(#[from] ReportError),
    /// Diagnostic analysis failed.
    #[error(transparent)]
    Diagnostics(#[from] DiagnosticError),
    /// Session persistence failed.
    #[error(transparent)]
    Persistence(#[from] crate::persistence::PersistenceError),
}

/// Blocking Windows session runner.
///
/// The runner owns the capture lifecycle; the UI only observes its result.
#[derive(Debug, Default, Clone, Copy)]
pub struct SessionRunner;

impl SessionRunner {
    /// Runs a live timed session until the configured duration elapses or cancellation is requested.
    pub fn run(
        &self,
        config: &SessionConfig,
        cancellation: &AtomicBool,
    ) -> Result<SessionResult, SessionError> {
        let timebase = QpcTimebase::new()?;
        let mut sampler = if config.no_process_probing {
            WindowsSystemSampler::new_system_only_with_timebase(timebase)?
        } else {
            WindowsSystemSampler::new_for_process_with_timebase(config.process_id, timebase)?
        };
        let (cpu, gpu, driver) = sampler.hardware_provenance();

        let presentmon_config = config.presentmon_config()?;
        let (display_refresh, display_refresh_warning) = if config.no_process_probing {
            (None, None)
        } else {
            match detect_process_display_refresh_rate(config.process_id) {
                Ok(value) => (value, None),
                Err(error) => (None, Some(error.to_string())),
            }
        };
        let target = PresentMonTarget::ProcessId(config.process_id);
        let parser = PresentMonCsvParser::new_with_qpc_origin(
            timebase.frequency_hz(),
            timebase.origin_qpc(),
        )?;

        let runner = PresentMonRunner;
        let process = runner.start(&presentmon_config)?;
        let mut system_samples = Vec::new();
        let mut telemetry_warnings = Vec::new();

        let mut capture_context = CaptureContext {
            parser: &parser,
            target: &target,
            sampler: &mut sampler,
            config,
            cancellation,
            system_samples: &mut system_samples,
            telemetry_warnings: &mut telemetry_warnings,
        };
        let (capture, interrupted) = collect_capture(process, &mut capture_context)?;

        let coverage = build_coverage(&capture.frames, &system_samples);
        let capture_document = CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames: capture.frames,
        };
        let stutter_config = match display_refresh.as_ref() {
            Some(refresh) => StutterAnalysisConfig::V2(
                rwp_analysis::RefreshAwareStutterConfig::new(refresh.hz)
                    .map_err(ReportError::Stutter)?,
            ),
            None => StutterAnalysisConfig::V1(StutterConfig::default()),
        };
        let report = build_report_with_stutter_analysis(
            &capture_document,
            StatisticsConfig::default(),
            stutter_config,
            interrupted,
        )?;

        let diagnostic_engine = DiagnosticEngine::new(config.diagnostic_thresholds);
        let diagnostics = diagnostic_engine.diagnose(DiagnosisInput {
            statistics: &report.frame_statistics,
            frames: &capture_document.frames,
            stutters: &report.stutter_events,
            system_samples: &system_samples,
        })?;
        let report = with_diagnostics(report, diagnostics.clone());

        let result = SessionResult {
            process_id: capture.process_id,
            display_refresh_rate: display_refresh,
            display_refresh_warning,
            dominant_swap_chain_address: capture.dominant_swap_chain_address,
            discarded_frame_count: capture.discarded_frame_count,
            frame_type_counts: capture.frame_type_counts,
            application: capture.application,
            system_samples,
            report,
            telemetry_warnings,
            diagnostics,
            coverage,
        };

        if let Some(directory) = config.persistence_dir.as_deref() {
            let input = crate::PersistedSessionInput {
                process_id: result.process_id,
                application: &result.application,
                interrupted: result.report.interrupted,
                display_refresh_rate_hz: result.display_refresh_rate.as_ref().map(|value| value.hz),
                dominant_swap_chain_address: result.dominant_swap_chain_address,
                discarded_frame_count: result.discarded_frame_count,
                frame_type_counts: &result.frame_type_counts,
                telemetry_warning_count: result.telemetry_warnings.len(),
                coverage: &result.coverage,
                capture: &capture_document,
                report: &result.report,
                system_samples: &result.system_samples,
                source_csv: &config.output_file,
                provenance: SessionProvenance {
                    rwp_version: env!("CARGO_PKG_VERSION").to_owned(),
                    git_sha: option_env!("RWP_GIT_SHA").map(str::to_owned),
                    build_target: option_env!("RWP_BUILD_TARGET").map(str::to_owned),
                    build_profile: option_env!("RWP_BUILD_PROFILE").map(str::to_owned),
                    os: Some(std::env::consts::OS.to_owned()),
                    cpu,
                    gpu,
                    driver,
                    presentmon_version: rwp_system::detect_presentmon_version(
                        &config.presentmon_executable,
                    )
                    .ok()
                    .map(|version| version.to_string()),
                    presentmon_sha256: fs::read(&config.presentmon_executable).ok().map(|bytes| {
                        let mut hasher = Sha256::new();
                        hasher.update(bytes);
                        format!("{:x}", hasher.finalize())
                    }),
                    qpc_frequency_hz: Some(timebase.frequency_hz()),
                    sample_interval_ms: Some(config.sample_interval.as_millis() as u64),
                    exclude_dropped: Some(config.exclude_dropped),
                    stop_existing_session: Some(config.stop_existing_session),
                    no_process_probing: Some(config.no_process_probing),
                },
            };
            crate::persist_session(directory, &input)?;
        }

        Ok(result)
    }
}

fn collect_telemetry_sample<S: SystemSampler + ?Sized>(
    sampler: &mut S,
    system_samples: &mut Vec<SystemSample>,
    telemetry_warnings: &mut Vec<TelemetryWarning>,
) -> Result<(), SessionError> {
    match sampler.sample() {
        Ok(sample) => system_samples.push(sample),
        Err(error) if error.is_non_essential() => telemetry_warnings.push(TelemetryWarning {
            message: error.to_string(),
        }),
        Err(error) => return Err(SessionError::Telemetry(error)),
    }
    Ok(())
}

struct CaptureContext<'a, S: SystemSampler + ?Sized> {
    parser: &'a PresentMonCsvParser,
    target: &'a PresentMonTarget,
    sampler: &'a mut S,
    config: &'a SessionConfig,
    cancellation: &'a AtomicBool,
    system_samples: &'a mut Vec<SystemSample>,
    telemetry_warnings: &'a mut Vec<TelemetryWarning>,
}

fn collect_capture<S: SystemSampler + ?Sized>(
    mut process: PresentMonProcess,
    context: &mut CaptureContext<'_, S>,
) -> Result<(rwp_capture::PresentMonCapture, bool), SessionError> {
    let started = Instant::now();

    loop {
        if context.cancellation.load(Ordering::Acquire) {
            process.terminate()?;
            let _ = process.wait()?;
            let capture = context.parser.parse_path_with_mode(
                context.config.output_file.as_path(),
                context.target,
                ParseMode::TolerateTruncatedTail,
            )?;
            return Ok((capture, true));
        }

        if started.elapsed() >= context.config.duration {
            return process
                .finish(context.parser)
                .map(|capture| (capture, false))
                .map_err(SessionError::from);
        }

        let status = match process.try_wait() {
            Ok(status) => status,
            Err(error) => {
                let _ = process.terminate();
                let _ = process.wait();
                return Err(error.into());
            }
        };

        if status.is_some() {
            return process
                .finish(context.parser)
                .map(|capture| (capture, false))
                .map_err(SessionError::from);
        }

        collect_telemetry_sample(
            context.sampler,
            context.system_samples,
            context.telemetry_warnings,
        )?;

        let elapsed = started.elapsed();
        let remaining = context
            .config
            .duration
            .checked_sub(elapsed)
            .unwrap_or(Duration::ZERO);
        let sleep_for = remaining.min(context.config.sample_interval);
        if !sleep_for.is_zero() {
            thread::sleep(sleep_for);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SessionConfig, SystemSampler, build_coverage, collect_telemetry_sample};
    use rwp_core::SystemSample;
    use rwp_system::SystemTelemetryError;
    use std::{path::PathBuf, time::Duration};

    struct IntermittentSampler {
        remaining_failures: usize,
    }

    impl SystemSampler for IntermittentSampler {
        fn sample(&mut self) -> Result<SystemSample, SystemTelemetryError> {
            if self.remaining_failures > 0 {
                self.remaining_failures -= 1;
                return Err(SystemTelemetryError::MemoryStatus);
            }
            Ok(SystemSample::new(1_000_000))
        }
    }

    #[test]
    fn intermittent_telemetry_failure_becomes_warning_not_session_failure() {
        let mut sampler = IntermittentSampler {
            remaining_failures: 1,
        };
        let mut samples = Vec::new();
        let mut warnings = Vec::new();

        assert!(collect_telemetry_sample(&mut sampler, &mut samples, &mut warnings).is_ok());
        assert!(collect_telemetry_sample(&mut sampler, &mut samples, &mut warnings).is_ok());

        assert_eq!(samples.len(), 1);
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].message, "GlobalMemoryStatusEx failed");
    }

    #[test]
    fn defaults_to_fifty_millisecond_sampling() -> Result<(), Box<dyn std::error::Error>> {
        let config = SessionConfig::new("PresentMon.exe", "run.csv", 42, 10)?;
        assert_eq!(config.sample_interval, Duration::from_millis(50));
        Ok(())
    }

    #[test]
    fn persistence_directory_is_opt_in() -> Result<(), Box<dyn std::error::Error>> {
        let config = SessionConfig::new("PresentMon.exe", "run.csv", 42, 10)?
            .with_persistence_dir("sessions/42");
        assert_eq!(config.persistence_dir, Some(PathBuf::from("sessions/42")));
        Ok(())
    }

    #[test]
    fn existing_session_stop_is_disabled_by_default() -> Result<(), Box<dyn std::error::Error>> {
        let config = SessionConfig::new("PresentMon.exe", "run.csv", 42, 10)?;
        assert!(!config.stop_existing_session);
        assert!(!config.no_process_probing);
        Ok(())
    }

    #[test]
    fn no_process_probing_is_an_explicit_session_option() -> Result<(), Box<dyn std::error::Error>>
    {
        let config =
            SessionConfig::new("PresentMon.exe", "run.csv", 42, 10)?.with_no_process_probing(true);
        assert!(config.no_process_probing);
        Ok(())
    }

    #[test]
    fn coverage_uses_only_observed_timestamp_ranges() -> Result<(), Box<dyn std::error::Error>> {
        let frames = vec![
            rwp_core::FrameSample::new(2_000_000_000, 8.0)?,
            rwp_core::FrameSample::new(4_000_000_000, 8.0)?,
        ];
        let samples = vec![
            SystemSample::new(0),
            SystemSample::new(3_000_000_000),
            SystemSample::new(5_000_000_000),
        ];
        let coverage = build_coverage(&frames, &samples);
        assert_eq!(coverage.frame_seconds, Some(2.0));
        assert_eq!(coverage.telemetry_seconds, Some(5.0));
        assert_eq!(coverage.intersection_seconds, Some(2.0));
        Ok(())
    }

    #[test]
    fn rejects_zero_process_and_duration() {
        assert!(SessionConfig::new("PresentMon.exe", "run.csv", 0, 10).is_err());
        assert!(SessionConfig::new("PresentMon.exe", "run.csv", 42, 0).is_err());
    }

    #[test]
    fn rejects_excessive_sample_interval() -> Result<(), Box<dyn std::error::Error>> {
        let config = SessionConfig::new("PresentMon.exe", "run.csv", 42, 10)?;
        assert!(config.with_sample_interval(Duration::from_secs(6)).is_err());
        Ok(())
    }

    #[test]
    fn rejects_sub_minimum_sample_interval() -> Result<(), Box<dyn std::error::Error>> {
        for millis in [1_u64, 9_u64] {
            let result = SessionConfig::new("PresentMon.exe", "run.csv", 42, 10)?
                .with_sample_interval(Duration::from_millis(millis));
            assert!(result.is_err(), "{millis} ms must be rejected");
            if let Err(error) = result {
                assert!(
                    error.to_string().contains("10 ms"),
                    "rejection must name the 10 ms lower bound, got: {error}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn accepts_minimum_sample_interval() -> Result<(), Box<dyn std::error::Error>> {
        let config = SessionConfig::new("PresentMon.exe", "run.csv", 42, 10)?;
        let config = config.with_sample_interval(Duration::from_millis(10))?;
        assert_eq!(config.sample_interval, Duration::from_millis(10));
        Ok(())
    }
}
