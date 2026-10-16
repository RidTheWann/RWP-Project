//! rwp command-line application entry point.

mod doctor;
mod error;

use doctor::{DoctorPreflightFailed, run as run_doctor, write_bundle as write_doctor_bundle};
use error::{classify, render_json, render_text};

use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
    process,
};

use anyhow::{Context, Result, bail};
use clap::{ArgAction, Parser, Subcommand, ValueEnum};
use rwp_analysis::{
    ComparisonConfig, ComparisonInput, PairedInferenceConfig, PairedRun, RefreshAwareStutterConfig,
    StatisticsConfig, StutterConfig, compare, minimum_pairs_for_alpha, paired_infer,
};
use rwp_capture::{ParseMode, PresentMonCsvParser, PresentMonTarget};
use rwp_core::SystemSample;
use rwp_diagnostics::{DiagnosisInput, DiagnosticEngine, RuleThresholds};
use rwp_report::{
    CaptureDocument, PerformanceReport, REPORT_SCHEMA_VERSION, StutterAnalysisConfig,
    build_report_with_stutter_analysis, from_json, to_json, to_pretty_json, with_diagnostics,
};
use rwp_session::{
    PersistedSessionInput, SessionManifest, SessionProvenance, persist_session, verify_manifest,
};

/// RidTheWann Performance command-line interface.
#[derive(Debug, Parser)]
#[command(
    name = "rwp",
    version,
    about = "Evidence-based Windows gaming and streaming performance analyzer"
)]
struct Cli {
    /// Emit machine-readable JSON for command failures.
    #[arg(long, global = true)]
    json: bool,
    /// CLI subcommand.
    #[command(subcommand)]
    command: Command,
}

/// Supported CLI operations.
#[derive(Debug, Subcommand)]
enum Command {
    /// Import a PresentMon CSV into a persisted session.
    ImportPresentmon {
        /// PresentMon CSV path.
        csv: PathBuf,
        /// Target process identifier.
        #[arg(long)]
        process_id: u32,
        /// PresentMon QPC frequency in Hz.
        #[arg(long)]
        qpc_frequency_hz: u64,
        /// Destination session directory.
        #[arg(long)]
        output_dir: PathBuf,
        /// Display refresh rate in Hz for refresh-aware stutter analysis.
        #[arg(long)]
        refresh_rate_hz: Option<f64>,
    },
    /// Analyze a persisted session and write a new immutable analyzed session.
    Analyze {
        /// Input session directory.
        #[arg(long)]
        session: PathBuf,
        /// Destination analyzed session directory.
        #[arg(long)]
        output_dir: PathBuf,
        /// Emit indented human-readable JSON to stdout after persistence.
        #[arg(long)]
        pretty: bool,
    },
    /// Render a persisted report.
    Report {
        /// Persisted session directory.
        #[arg(long)]
        session: PathBuf,
        /// Output format.
        #[arg(long, value_enum, default_value_t = ReportFormat::Json)]
        format: ReportFormat,
        /// Redact identifying session fields from human-readable report output.
        #[arg(long, default_value_t = true, action = ArgAction::Set)]
        redact: bool,
        /// Optional output file. Defaults to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Compare two persisted session reports.
    Compare {
        /// Baseline session directory.
        #[arg(long, conflicts_with = "pairs")]
        baseline: Option<PathBuf>,
        /// Candidate session directory.
        #[arg(long, conflicts_with = "pairs")]
        candidate: Option<PathBuf>,
        /// Root containing <pair-id>/baseline and <pair-id>/candidate session directories.
        #[arg(long, conflicts_with_all = ["baseline", "candidate"])]
        pairs: Option<PathBuf>,
        /// Treat both reports as the same intended workload/scenario.
        #[arg(long)]
        same_scenario: bool,
        /// Minimum smaller/larger sample ratio for normal comparability.
        #[arg(long, default_value_t = 0.80)]
        minimum_sample_ratio: f64,
        /// Minimum relative metric change, in percent, before classifying direction.
        #[arg(long, default_value_t = 1.0)]
        change_threshold_percent: f64,
        /// Optional output file. Defaults to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Run local preflight diagnostics for RWP and an optional PresentMon executable.
    Doctor {
        /// Optional PresentMon executable to validate.
        #[arg(long)]
        presentmon: Option<PathBuf>,
        /// Optional path for a redacted deterministic JSON diagnostic bundle.
        #[arg(long)]
        bundle: Option<PathBuf>,
    },
    /// Calibrate A/A paired sessions and recommend an equivalence margin.
    Calibrate {
        /// Root containing <pair-id>/baseline and <pair-id>/candidate A/A session directories.
        #[arg(long)]
        pairs: PathBuf,
        /// Two-sided alpha used for the minimum-pair recommendation.
        #[arg(long, default_value_t = 0.05)]
        alpha: f64,
        /// Optional output file. Defaults to stdout.
        #[arg(long)]
        output: Option<PathBuf>,
    },
    /// Migrate a legacy report or session to the current schema version.
    Migrate {
        /// Standalone report JSON file using a legacy `rwp.report` schema.
        #[arg(long, conflicts_with = "session")]
        report: Option<PathBuf>,
        /// Persisted session directory using legacy schemas.
        #[arg(long, conflicts_with = "report")]
        session: Option<PathBuf>,
        /// Destination file for a migrated standalone report.
        #[arg(long)]
        output: Option<PathBuf>,
        /// Destination directory for a migrated session.
        #[arg(long)]
        output_dir: Option<PathBuf>,
    },
    /// Capture a live PresentMon session on Windows (same orchestration as the GUI).
    Capture {
        /// Path to the PresentMon console executable.
        #[arg(long)]
        presentmon: PathBuf,
        /// Target process identifier (mutually exclusive with --process-name).
        #[arg(long, conflicts_with = "process_name")]
        process_id: Option<u32>,
        /// Target process executable name (mutually exclusive with --process-id).
        #[arg(long, conflicts_with = "process_id")]
        process_name: Option<String>,
        /// Capture duration in seconds (1 through 3600).
        #[arg(long)]
        duration_seconds: u64,
        /// Windows telemetry sampling interval in milliseconds (10 through 5000, default 50).
        #[arg(long, default_value_t = 50)]
        interval_ms: u64,
        /// Disable RWP's additional process/thread telemetry probing.
        #[arg(long)]
        no_process_probing: bool,
        /// Allow PresentMon to stop an existing trace session of the same name
        /// (also stops another tool's session; default off, use with consent).
        #[arg(long)]
        stop_existing_session: bool,
        /// Display refresh rate in Hz for refresh-aware stutter analysis (overrides auto-detect).
        #[arg(long)]
        refresh_rate_hz: Option<f64>,
        /// Destination session directory (same layout and manifest as the GUI).
        #[arg(long)]
        output_dir: PathBuf,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ReportFormat {
    Json,
    Md,
    Html,
}

fn main() {
    let cli = Cli::parse();
    let json_errors = cli.json;

    if let Err(error) = run(cli.command) {
        let report = classify(&error);
        if json_errors {
            eprintln!("{}", render_json(&report));
        } else {
            eprintln!("{}", render_text(&report));
        }
        process::exit(report.descriptor.exit_code);
    }
}

fn run(command: Command) -> Result<()> {
    match command {
        Command::ImportPresentmon {
            csv,
            process_id,
            qpc_frequency_hz,
            output_dir,
            refresh_rate_hz,
        } => import_presentmon(
            csv,
            process_id,
            qpc_frequency_hz,
            refresh_rate_hz,
            output_dir,
        ),
        Command::Analyze {
            session,
            output_dir,
            pretty,
        } => analyze_session(session, output_dir, pretty),
        Command::Report {
            session,
            format,
            redact,
            output,
        } => report_session(session, format, redact, output),
        Command::Compare {
            baseline,
            candidate,
            pairs,
            same_scenario,
            minimum_sample_ratio,
            change_threshold_percent,
            output,
        } => compare_sessions(
            baseline,
            candidate,
            pairs,
            same_scenario,
            ComparisonConfig {
                minimum_sample_ratio,
                change_threshold_percent,
            },
            output,
        ),
        Command::Doctor { presentmon, bundle } => doctor(presentmon, bundle),
        Command::Calibrate {
            pairs,
            alpha,
            output,
        } => calibrate_pairs(pairs, alpha, output),
        Command::Migrate {
            report,
            session,
            output,
            output_dir,
        } => migrate(report, session, output, output_dir),
        Command::Capture {
            presentmon,
            process_id,
            process_name,
            duration_seconds,
            interval_ms,
            no_process_probing,
            stop_existing_session,
            refresh_rate_hz,
            output_dir,
        } => run_capture(
            presentmon,
            process_id,
            process_name,
            duration_seconds,
            interval_ms,
            no_process_probing,
            stop_existing_session,
            refresh_rate_hz,
            output_dir,
        ),
    }
}

fn doctor(presentmon: Option<PathBuf>, bundle: Option<PathBuf>) -> Result<()> {
    let report = run_doctor(presentmon.as_deref());
    if let Some(path) = bundle.as_deref() {
        write_doctor_bundle(path, &report)?;
    }

    let output = serde_json::to_string_pretty(&report)?;
    println!("{output}");
    if report.has_failures() {
        return Err(anyhow::Error::new(DoctorPreflightFailed));
    }
    Ok(())
}

/// Target selector for `rwp capture` after mutual-exclusion validation.
#[derive(Debug, Clone, PartialEq, Eq)]
enum CaptureTarget {
    /// Capture by process identifier.
    Pid(u32),
    /// Capture by executable name (resolved to a PID via the live process list).
    Name(String),
}

/// Validates a live-capture duration by calling rwp-capture's existing timed
/// validation on a throwaway config, so the bound and the error variants stay
/// single-sourced (the dummy target never spawns). Failures classify as
/// `RWP-E2000` through the existing taxonomy.
fn validate_capture_duration(seconds: u64) -> Result<()> {
    rwp_capture::PresentMonCaptureConfig::new(
        "PresentMon.exe",
        "run.csv",
        rwp_capture::PresentMonTarget::ProcessId(1),
    )?
    .with_timed_seconds(seconds)?;
    Ok(())
}

/// Validates a live-capture sampling interval against the shared 10 ms–5 s
/// session bound (`rwp_session` `SessionConfig::with_sample_interval`; the GUI
/// enforces the same range and the default stays 50 ms).
fn validate_capture_interval_ms(interval_ms: u64) -> Result<()> {
    if !(10..=5000).contains(&interval_ms) {
        bail!("interval_ms must be between 10 and 5000 milliseconds");
    }
    Ok(())
}

/// Validates the mutually exclusive `--process-id` / `--process-name`
/// selectors without touching the live process list.
fn resolve_capture_target(
    process_id: Option<u32>,
    process_name: Option<String>,
) -> Result<CaptureTarget> {
    match (process_id, process_name) {
        (Some(_), Some(_)) => bail!("--process-id and --process-name are mutually exclusive"),
        (None, None) => bail!("one of --process-id or --process-name is required"),
        (Some(0), _) => bail!("process_id must be greater than zero"),
        (Some(pid), None) => Ok(CaptureTarget::Pid(pid)),
        (None, Some(name)) if name.trim().is_empty() => {
            bail!("process_name must not be empty")
        }
        (None, Some(name)) => Ok(CaptureTarget::Name(name)),
    }
}

#[cfg(not(windows))]
#[allow(clippy::too_many_arguments)]
fn run_capture(
    _presentmon: PathBuf,
    _process_id: Option<u32>,
    _process_name: Option<String>,
    _duration_seconds: u64,
    _interval_ms: u64,
    _no_process_probing: bool,
    _stop_existing_session: bool,
    _refresh_rate_hz: Option<f64>,
    _output_dir: PathBuf,
) -> Result<()> {
    Err(rwp_session::SessionError::UnsupportedPlatform.into())
}

#[cfg(windows)]
#[allow(clippy::too_many_arguments)]
fn run_capture(
    presentmon: PathBuf,
    process_id: Option<u32>,
    process_name: Option<String>,
    duration_seconds: u64,
    interval_ms: u64,
    no_process_probing: bool,
    stop_existing_session: bool,
    refresh_rate_hz: Option<f64>,
    output_dir: PathBuf,
) -> Result<()> {
    use std::time::Duration;

    validate_capture_duration(duration_seconds)?;
    validate_capture_interval_ms(interval_ms)?;
    let target = resolve_capture_target(process_id, process_name)?;
    if let Some(hz) = refresh_rate_hz {
        RefreshAwareStutterConfig::new(hz)
            .with_context(|| "refresh_rate_hz must be finite and greater than zero")?;
    }
    let pid = match target {
        CaptureTarget::Pid(pid) => pid,
        CaptureTarget::Name(name) => resolve_process_name_to_pid(name.as_str())?,
    };
    let temp_csv = live_capture_temp_csv()?;
    let session_dir = output_dir.clone();
    let mut config =
        rwp_session::SessionConfig::new(presentmon, temp_csv.clone(), pid, duration_seconds)?
            .with_sample_interval(Duration::from_millis(interval_ms))?
            .with_no_process_probing(no_process_probing)
            .with_stop_existing_session(stop_existing_session)
            .with_persistence_dir(output_dir);
    if let Some(hz) = refresh_rate_hz {
        config = config.with_display_refresh_rate_hz(hz)?;
    }
    // Safety: PresentMon runs in its own process group (CREATE_NEW_PROCESS_GROUP,
    // plus CREATE_NO_WINDOW when the caller has no console), so console Ctrl+C
    // never reaches it directly. The shared PresentMonRunner assigns the child
    // to a KILL_ON_JOB_CLOSE Job Object (rwp_system::PresentMonJob via
    // rwp-capture), so a second signal or a crash still closes the job handle
    // and the OS terminates PresentMon — no orphan, no duplicated unsafe code.
    // The first Ctrl+C/Ctrl+Break flips the shared stop flag below, the runner
    // stops PresentMon gracefully (Batch 5 path), and the partial session
    // persists with interrupted=true.
    let _stop_guard = rwp_system::ConsoleStopGuard::install()
        .with_context(|| "failed to install console stop handler for Ctrl+C")?;
    let result = run_capture_session(
        &config,
        temp_csv.as_path(),
        rwp_system::capture_stop_flag(),
        |session_config, cancellation| rwp_session::SessionRunner.run(session_config, cancellation),
    )?;
    if let Some(notice) = capture_completion_notice(result.report.interrupted, &session_dir) {
        eprintln!("{notice}");
    }
    if let Some(hint) = etw_kill_hint_for_outcome(result.stop_outcome) {
        eprintln!("{hint}");
    }
    if let Some(notice) = dropped_rows_notice(&result.telemetry_warnings) {
        eprintln!("{notice}");
    }
    Ok(())
}

/// Runs one live session through an injected runner and removes the temporary
/// PresentMon CSV afterwards, on success and on failure alike.
///
/// This is the minimum CLI-level seam for live capture: production passes the
/// shared `SessionRunner`, while tests inject a fake that persists fixture
/// data without PresentMon. No `rwp-session` API change was needed.
#[cfg(windows)]
fn run_capture_session(
    config: &rwp_session::SessionConfig,
    temp_csv: &std::path::Path,
    cancellation: &std::sync::atomic::AtomicBool,
    runner: impl FnOnce(
        &rwp_session::SessionConfig,
        &std::sync::atomic::AtomicBool,
    ) -> Result<rwp_session::SessionResult, rwp_session::SessionError>,
) -> Result<rwp_session::SessionResult> {
    let result = runner(config, cancellation);
    let _ = std::fs::remove_file(temp_csv);
    result.map_err(anyhow::Error::from)
}

/// Renders the post-capture notice for an interrupted session, if any.
///
/// Exit-code policy (OWNER DECISION, default): an interrupted capture still
/// exits 0 — the partial session is the successful product — with this
/// stderr notice naming where it was saved.
fn capture_completion_notice(interrupted: bool, session_dir: &std::path::Path) -> Option<String> {
    interrupted.then(|| {
        format!(
            "interrupted: partial session saved at {}",
            session_dir.display()
        )
    })
}

/// Renders the dropped-rows notice for captures that skipped isolated
/// out-of-order QPC rows, if any. Reads the stable parser wording from the
/// session warnings so CLI, session, and GUI surfaces stay aligned.
/// Windows-only like the live-capture path that produces these warnings.
#[cfg(windows)]
fn dropped_rows_notice(warnings: &[rwp_session::TelemetryWarning]) -> Option<String> {
    warnings
        .iter()
        .find(|warning| warning.message.contains("backwards QPC"))
        .map(|warning| format!("note: {}", warning.message))
}

/// Renders the ETW-hygiene hint for a killed stop, if any.
///
/// A hard kill can leave a PresentMon trace session open; the hint tells the
/// operator how to check and what rerun flag clears it. Graceful stops, and
/// sessions that never went through the stop path, carry no hint. Pure across
/// platforms so the matrix is unit-tested everywhere.
fn etw_kill_hint_for_outcome(outcome: Option<rwp_capture::StopOutcome>) -> Option<&'static str> {
    match outcome {
        Some(rwp_capture::StopOutcome::Killed) => Some(
            "note: PresentMon was killed rather than stopped gracefully; if `logman query -ets` shows a leftover PresentMon trace session, rerun with --stop-existing-session (see docs/troubleshooting.md)",
        ),
        Some(rwp_capture::StopOutcome::Graceful) | None => None,
    }
}

#[cfg(windows)]
fn live_capture_temp_csv() -> Result<PathBuf> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let path = std::env::temp_dir().join(format!("rwp-capture-{}-{nanos}.csv", std::process::id()));
    if path.exists() {
        bail!("temporary capture CSV already exists: {}", path.display());
    }
    Ok(path)
}

#[cfg(windows)]
fn resolve_process_name_to_pid(name: &str) -> Result<u32> {
    let wanted = name.trim();
    let mut pids: Vec<u32> = Vec::new();
    for summary in rwp_system::list_processes()? {
        if summary.name.eq_ignore_ascii_case(wanted) {
            pids.push(summary.process_id);
        }
    }
    pids.sort_unstable();
    pids.dedup();
    match pids.len() {
        0 => bail!("no live process named '{wanted}' was found"),
        1 => match pids.first() {
            Some(pid) => Ok(*pid),
            None => bail!("no live process named '{wanted}' was found"),
        },
        _ => {
            bail!("multiple live processes named '{wanted}' were found; pass --process-id instead")
        }
    }
}

// Selects the stutter detector from persisted provenance: a recorded display
// refresh rate keeps refresh-aware V2 analysis, otherwise legacy V1 applies.
fn stutter_config_for_refresh_rate(
    refresh_rate_hz: Option<f64>,
) -> Result<StutterAnalysisConfig, rwp_report::ReportError> {
    match refresh_rate_hz {
        Some(hz) => Ok(StutterAnalysisConfig::V2(
            RefreshAwareStutterConfig::new(hz).map_err(rwp_report::ReportError::from)?,
        )),
        None => Ok(StutterAnalysisConfig::V1(StutterConfig::default())),
    }
}

fn import_presentmon(
    csv: PathBuf,
    process_id: u32,
    qpc_frequency_hz: u64,
    refresh_rate_hz: Option<f64>,
    output_dir: PathBuf,
) -> Result<()> {
    if process_id == 0 {
        bail!("process_id must be greater than zero");
    }
    let parser = PresentMonCsvParser::new(qpc_frequency_hz)?;
    let capture = parser.parse_path_with_mode(
        &csv,
        &PresentMonTarget::ProcessId(process_id),
        ParseMode::Strict,
    )?;
    if let Some(notice) = rwp_capture::dropped_rows_notice(capture.dropped_nonmonotonic_rows) {
        eprintln!("note: {notice}");
    };
    let document = CaptureDocument {
        schema_version: String::from("rwp.capture.v1"),
        frames: capture.frames.clone(),
    };
    let report = build_report_with_stutter_analysis(
        &document,
        StatisticsConfig::default(),
        stutter_config_for_refresh_rate(refresh_rate_hz)?,
        false,
    )?;
    let coverage = rwp_session::CoverageSummary {
        frame_seconds: duration_seconds(&capture.frames),
        telemetry_seconds: None,
        intersection_seconds: None,
    };
    let samples = Vec::<SystemSample>::new();
    let input = PersistedSessionInput {
        process_id: capture.process_id,
        application: &capture.application,
        interrupted: false,
        display_refresh_rate_hz: refresh_rate_hz,
        dominant_swap_chain_address: capture.dominant_swap_chain_address,
        discarded_frame_count: capture.discarded_frame_count,
        frame_type_counts: &capture.frame_type_counts,
        telemetry_warning_count: 0,
        coverage: &coverage,
        capture: &document,
        report: &report,
        system_samples: &samples,
        source_csv: &csv,
        provenance: SessionProvenance {
            rwp_version: env!("CARGO_PKG_VERSION").to_owned(),
            git_sha: option_env!("RWP_GIT_SHA").map(str::to_owned),
            build_target: option_env!("RWP_BUILD_TARGET").map(str::to_owned),
            build_profile: option_env!("RWP_BUILD_PROFILE").map(str::to_owned),
            os: Some(std::env::consts::OS.to_owned()),
            cpu: None,
            gpu: None,
            driver: None,
            presentmon_version: None,
            presentmon_sha256: None,
            qpc_frequency_hz: Some(qpc_frequency_hz),
            sample_interval_ms: None,
            exclude_dropped: None,
            stop_existing_session: None,
            no_process_probing: None,
        },
    };
    persist_session(&output_dir, &input)?;
    Ok(())
}

fn analyze_session(session: PathBuf, output_dir: PathBuf, pretty: bool) -> Result<()> {
    let loaded = load_session(&session)?;
    let mut report = build_report_with_stutter_analysis(
        &loaded.capture,
        StatisticsConfig::default(),
        stutter_config_for_refresh_rate(loaded.manifest.display_refresh_rate_hz)?,
        loaded.manifest.interrupted,
    )?;
    let diagnostics =
        DiagnosticEngine::new(RuleThresholds::default()).diagnose(DiagnosisInput {
            statistics: &report.frame_statistics,
            frames: &loaded.capture.frames,
            stutters: &report.stutter_events,
            system_samples: &loaded.system_samples,
        })?;
    report = with_diagnostics(report, diagnostics);

    let input = PersistedSessionInput {
        process_id: loaded.manifest.process_id,
        application: &loaded.manifest.application,
        interrupted: loaded.manifest.interrupted,
        display_refresh_rate_hz: loaded.manifest.display_refresh_rate_hz,
        dominant_swap_chain_address: loaded.manifest.dominant_swap_chain_address,
        discarded_frame_count: loaded.manifest.discarded_frame_count,
        frame_type_counts: &loaded.manifest.frame_type_counts,
        telemetry_warning_count: loaded.manifest.telemetry_warning_count,
        coverage: &loaded.manifest.coverage,
        capture: &loaded.capture,
        report: &report,
        system_samples: &loaded.system_samples,
        source_csv: &loaded.source_csv,
        provenance: loaded.manifest.provenance.clone(),
    };
    persist_session(&output_dir, &input)?;

    if pretty {
        println!("{}", to_pretty_json(&report)?);
    }
    Ok(())
}

fn report_session(
    session: PathBuf,
    format: ReportFormat,
    redact: bool,
    output: Option<PathBuf>,
) -> Result<()> {
    let loaded = load_session(&session)?;
    let text = match format {
        ReportFormat::Json => to_json(&loaded.report)?,
        ReportFormat::Md => render_markdown(&loaded.report, &loaded.manifest, redact),
        ReportFormat::Html => render_html(&loaded.report, &loaded.manifest, redact),
    };
    write_output(output, &text)
}

fn compare_sessions(
    baseline: Option<PathBuf>,
    candidate: Option<PathBuf>,
    pairs: Option<PathBuf>,
    same_scenario: bool,
    config: ComparisonConfig,
    output: Option<PathBuf>,
) -> Result<()> {
    if let Some(root) = pairs {
        let runs = load_pair_runs(&root)?;
        let inference = paired_infer(&runs, PairedInferenceConfig::default())?;
        return write_output(output, &serde_json::to_string(&inference)?);
    }

    let baseline = baseline.context("baseline session is required")?;
    let candidate = candidate.context("candidate session is required")?;
    let baseline_report = load_session(&baseline)?.report;
    let candidate_report = load_session(&candidate)?.report;
    let comparison = compare(
        ComparisonInput {
            baseline: &baseline_report.frame_statistics,
            candidate: &candidate_report.frame_statistics,
            baseline_stutter_count: baseline_report.stutter_events.len(),
            candidate_stutter_count: candidate_report.stutter_events.len(),
            same_scenario,
        },
        config,
    )?;
    write_output(output, &serde_json::to_string(&comparison)?)
}

fn calibrate_pairs(pairs: PathBuf, alpha: f64, output: Option<PathBuf>) -> Result<()> {
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        bail!("alpha must be between 0 and 1");
    }
    let runs = load_pair_runs(&pairs)?;
    if runs.is_empty() {
        bail!("calibration requires at least one A/A pair");
    }

    let mut absolute_differences = runs
        .iter()
        .map(|pair| {
            ((pair.baseline_p99_frame_time_ms - pair.candidate_p99_frame_time_ms)
                / pair.baseline_p99_frame_time_ms)
                .abs()
                * 100.0
        })
        .collect::<Vec<_>>();
    absolute_differences.sort_by(f64::total_cmp);
    let index = ((absolute_differences.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(absolute_differences.len() - 1);
    let p95_absolute_relative_difference_percent = absolute_differences[index];
    let equivalence_margin_percent = p95_absolute_relative_difference_percent.max(1.0);
    let recommended_minimum_pairs = minimum_pairs_for_alpha(alpha)?;

    let result = serde_json::json!({
        "schema_version": "rwp.calibration.v1",
        "method": "A/A paired P99 frame-time absolute relative difference; equivalence margin is max(1%, empirical P95)",
        "pair_count": runs.len(),
        "alpha": alpha,
        "p95_absolute_relative_difference_percent": p95_absolute_relative_difference_percent,
        "recommended_equivalence_margin_percent": equivalence_margin_percent,
        "recommended_minimum_pairs": recommended_minimum_pairs,
        "inference_limit": "The recommended minimum is the number of informative, non-tie pairs; practical-equivalence ties or ineligible pairs can require more total pairs. Calibration describes observed A/A variability and does not prove causality or hardware-wide equivalence."
    });
    write_output(output, &serde_json::to_string_pretty(&result)?)
}

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_CAPTURE_OR_REPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TELEMETRY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TELEMETRY_SAMPLES: usize = 5_000_000;
const MAX_JSON_NESTING_DEPTH: usize = 64;

struct LoadedSession {
    manifest: SessionManifest,
    capture: CaptureDocument,
    report: PerformanceReport,
    system_samples: Vec<SystemSample>,
    source_csv: PathBuf,
}

fn load_session(path: &Path) -> Result<LoadedSession> {
    verify_manifest(path)
        .with_context(|| format!("invalid persisted session {}", path.display()))?;

    let manifest_bytes = read_bounded_file(
        &path.join("manifest.json"),
        MAX_MANIFEST_BYTES,
        "read manifest",
    )?;
    ensure_json_depth(&manifest_bytes, MAX_JSON_NESTING_DEPTH)?;
    let manifest: SessionManifest = serde_json::from_slice(&manifest_bytes)?;
    read_session_after_manifest(path, manifest)
}

/// Reads session content without manifest verification, for migrating legacy
/// sessions whose schema stamp current readers reject. Bounds and content
/// validation are identical to the verified path; only the schema gate is
/// skipped because normalization is the purpose of the call.
fn read_unverified_session(path: &Path) -> Result<LoadedSession> {
    let manifest_bytes = read_bounded_file(
        &path.join("manifest.json"),
        MAX_MANIFEST_BYTES,
        "read manifest",
    )?;
    ensure_json_depth(&manifest_bytes, MAX_JSON_NESTING_DEPTH)?;
    let manifest: SessionManifest = serde_json::from_slice(&manifest_bytes)?;
    read_session_after_manifest(path, manifest)
}

fn read_session_after_manifest(path: &Path, manifest: SessionManifest) -> Result<LoadedSession> {
    let capture_bytes = read_bounded_file(
        &path.join("capture.json"),
        MAX_CAPTURE_OR_REPORT_BYTES,
        "read capture",
    )?;
    ensure_json_depth(&capture_bytes, MAX_JSON_NESTING_DEPTH)?;
    let capture: CaptureDocument = serde_json::from_slice(&capture_bytes)?;
    validate_persisted_frames(&capture.frames)?;

    let report_bytes = read_bounded_file(
        &path.join("report.json"),
        MAX_CAPTURE_OR_REPORT_BYTES,
        "read report",
    )?;
    ensure_json_depth(&report_bytes, MAX_JSON_NESTING_DEPTH)?;
    let report_text = std::str::from_utf8(&report_bytes).context("report JSON is not UTF-8")?;
    let report = from_json(report_text)?;

    let telemetry_bytes = read_bounded_file(
        &path.join("telemetry.jsonl"),
        MAX_TELEMETRY_BYTES,
        "read telemetry",
    )?;
    let telemetry =
        std::str::from_utf8(&telemetry_bytes).context("telemetry JSONL is not UTF-8")?;
    let mut system_samples = Vec::new();
    for line in telemetry.lines().filter(|line| !line.trim().is_empty()) {
        if system_samples.len() >= MAX_TELEMETRY_SAMPLES {
            bail!("telemetry sample count exceeds the configured limit ({MAX_TELEMETRY_SAMPLES})");
        }
        ensure_json_depth(line.as_bytes(), MAX_JSON_NESTING_DEPTH)?;
        system_samples.push(serde_json::from_str(line)?);
    }
    validate_persisted_telemetry(&system_samples)?;
    Ok(LoadedSession {
        manifest,
        capture,
        report,
        system_samples,
        source_csv: path.join("presentmon.csv"),
    })
}

/// Rejects persisted captures that violate frame-sample contracts.
///
/// Mirrors the `FrameSample` constructor rules (positive finite frame time,
/// non-negative finite telemetry) plus the single non-decreasing timestamp
/// policy, with row indices for actionable errors.
fn validate_persisted_frames(frames: &[rwp_core::FrameSample]) -> Result<()> {
    let mut previous: Option<u64> = None;
    for (index, frame) in frames.iter().enumerate() {
        if !frame.frame_time_ms.is_finite() || frame.frame_time_ms <= 0.0 {
            bail!("capture frame {index} has non-positive frame_time_ms");
        }
        for (name, value) in [
            ("cpu_busy_ms", frame.cpu_busy_ms),
            ("cpu_wait_ms", frame.cpu_wait_ms),
            ("gpu_time_ms", frame.gpu_time_ms),
            ("gpu_busy_ms", frame.gpu_busy_ms),
            ("gpu_wait_ms", frame.gpu_wait_ms),
            ("display_latency_ms", frame.display_latency_ms),
        ] {
            if let Some(value) = value
                && (!value.is_finite() || value < 0.0)
            {
                bail!("capture frame {index} has invalid {name}");
            }
        }
        if let Some(previous) = previous
            && frame.timestamp_ns < previous
        {
            bail!("capture frame timestamps move backwards at index {index}");
        }
        previous = Some(frame.timestamp_ns);
    }
    Ok(())
}

/// Rejects persisted telemetry that violates sample contracts.
///
/// Percentages mirror the constructor 0..=100 range; power draw must be
/// finite and non-negative; temperature must be finite; clocks and byte
/// counters are unsigned and inherently valid. `SystemSample::new` stays
/// guard-free so programmatic construction contracts do not change.
fn validate_persisted_telemetry(samples: &[rwp_core::SystemSample]) -> Result<()> {
    fn percentage(name: &str, index: usize, value: Option<f64>) -> Result<()> {
        if let Some(value) = value
            && (!value.is_finite() || !(0.0..=100.0).contains(&value))
        {
            bail!("telemetry sample {index} has invalid {name}");
        }
        Ok(())
    }
    let mut previous: Option<u64> = None;
    for (index, sample) in samples.iter().enumerate() {
        percentage("cpu_utilization", index, sample.cpu_utilization)?;
        percentage("gpu_utilization", index, sample.gpu_utilization)?;
        percentage("vram_utilization", index, sample.vram_utilization)?;
        percentage("ram_utilization", index, sample.ram_utilization)?;
        for (thread, observation) in sample.process_threads.iter().enumerate() {
            if !observation.cpu_utilization.is_finite()
                || !(0.0..=100.0).contains(&observation.cpu_utilization)
            {
                bail!("telemetry sample {index} thread {thread} has invalid cpu_utilization");
            }
        }
        for (activity, observation) in sample.background_activity.iter().enumerate() {
            if !observation.cpu_utilization.is_finite()
                || !(0.0..=100.0).contains(&observation.cpu_utilization)
            {
                bail!(
                    "telemetry sample {index} background activity {activity} has invalid cpu_utilization"
                );
            }
        }
        if let Some(power) = sample.gpu_power_w
            && (!power.is_finite() || power < 0.0)
        {
            bail!("telemetry sample {index} has invalid gpu_power_w");
        }
        if let Some(temperature) = sample.gpu_temperature_c
            && !temperature.is_finite()
        {
            bail!("telemetry sample {index} has invalid gpu_temperature_c");
        }
        if let Some(previous) = previous
            && sample.timestamp_ns < previous
        {
            bail!("telemetry timestamps move backwards at index {index}");
        }
        previous = Some(sample.timestamp_ns);
    }
    Ok(())
}

fn read_bounded_file(path: &Path, limit: u64, operation: &str) -> Result<Vec<u8>> {
    let file = fs::File::open(path).with_context(|| format!("{operation}: {}", path.display()))?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .with_context(|| format!("{operation}: {}", path.display()))?;
    if bytes.len() as u64 > limit {
        bail!("{operation} exceeds the configured limit ({limit} bytes)");
    }
    Ok(bytes)
}

fn ensure_json_depth(bytes: &[u8], limit: usize) -> Result<()> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;

    for &byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth = depth.saturating_add(1);
                if depth > limit {
                    bail!("JSON nesting depth exceeds the configured limit ({depth} > {limit})");
                }
            }
            b'}' | b']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    Ok(())
}

fn load_pair_runs(root: &Path) -> Result<Vec<PairedRun>> {
    let mut pair_dirs = fs::read_dir(root)?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    pair_dirs.sort();
    let mut runs = Vec::with_capacity(pair_dirs.len());

    for pair_dir in pair_dirs {
        let pair_id = pair_dir
            .file_name()
            .and_then(|value| value.to_str())
            .context("pair directory name is not valid UTF-8")?
            .to_owned();
        let baseline = load_session(&pair_dir.join("baseline"))?;
        let candidate = load_session(&pair_dir.join("candidate"))?;
        runs.push(PairedRun {
            pair_id,
            baseline_sample_count: baseline.report.frame_count,
            candidate_sample_count: candidate.report.frame_count,
            baseline_p99_frame_time_ms: baseline.report.frame_statistics.percentiles.p99_ms,
            candidate_p99_frame_time_ms: candidate.report.frame_statistics.percentiles.p99_ms,
        });
    }
    Ok(runs)
}

fn migrate(
    report: Option<PathBuf>,
    session: Option<PathBuf>,
    output: Option<PathBuf>,
    output_dir: Option<PathBuf>,
) -> Result<()> {
    match (report, session) {
        (Some(path), None) => {
            let destination = output.context("migrate --report requires --output")?;
            migrate_report_file(&path, &destination)
        }
        (None, Some(dir)) => {
            let destination = output_dir.context("migrate --session requires --output-dir")?;
            migrate_session(&dir, &destination)
        }
        (None, None) => bail!("migrate requires --report or --session"),
        (Some(_), Some(_)) => bail!("migrate accepts only one of --report or --session"),
    }
}

/// Normalizes one standalone report to the current report schema version.
///
/// Accepted inputs are exactly the schemas `from_json` validates, so content
/// validation is unchanged; only the version stamp is normalized, which lets
/// legacy files flow into readers that require the current contract.
fn migrate_report_file(source: &Path, destination: &Path) -> Result<()> {
    let bytes = read_bounded_file(source, MAX_CAPTURE_OR_REPORT_BYTES, "read report")?;
    ensure_json_depth(&bytes, MAX_JSON_NESTING_DEPTH)?;
    let text = std::str::from_utf8(&bytes).context("report JSON is not UTF-8")?;
    let mut report = from_json(text)?;
    report.schema_version = String::from(REPORT_SCHEMA_VERSION);
    fs::write(destination, to_json(&report)?)
        .with_context(|| format!("write migrated report {}", destination.display()))?;
    Ok(())
}

/// Normalizes one persisted session to the current schemas, preserving the
/// authored report content and manifest provenance. Unlike analysis, migration
/// never recomputes diagnostics; unlike loading, it accepts legacy manifest
/// schema stamps and rewrites them with fresh content hashes.
fn migrate_session(source: &Path, destination: &Path) -> Result<()> {
    let loaded = read_unverified_session(source)?;
    let mut report = loaded.report;
    report.schema_version = String::from(REPORT_SCHEMA_VERSION);
    let input = PersistedSessionInput {
        process_id: loaded.manifest.process_id,
        application: &loaded.manifest.application,
        interrupted: loaded.manifest.interrupted,
        display_refresh_rate_hz: loaded.manifest.display_refresh_rate_hz,
        dominant_swap_chain_address: loaded.manifest.dominant_swap_chain_address,
        discarded_frame_count: loaded.manifest.discarded_frame_count,
        frame_type_counts: &loaded.manifest.frame_type_counts,
        telemetry_warning_count: loaded.manifest.telemetry_warning_count,
        coverage: &loaded.manifest.coverage,
        capture: &loaded.capture,
        report: &report,
        system_samples: &loaded.system_samples,
        source_csv: &loaded.source_csv,
        provenance: loaded.manifest.provenance.clone(),
    };
    persist_session(destination, &input)?;
    Ok(())
}

fn duration_seconds(frames: &[rwp_core::FrameSample]) -> Option<f64> {
    let (first, last) = match (frames.first(), frames.last()) {
        (Some(first), Some(last)) => (first.timestamp_ns, last.timestamp_ns),
        _ => return None,
    };
    Some((last.saturating_sub(first)) as f64 / 1_000_000_000.0)
}

/// Escapes untrusted text for a Markdown text context and prevents newline structure injection.
fn markdown_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\r' | '\n' => escaped.push(' '),
            '&' => escaped.push_str("&amp;"),
            '\\' | '`' | '*' | '_' | '{' | '}' | '[' | ']' | '(' | ')' | '#' | '+' | '-' | '.'
            | '!' | '|' | '>' | '<' | '~' | '=' => {
                escaped.push('\\');
                escaped.push(character);
            }
            _ => escaped.push(character),
        }
    }
    escaped
}

fn render_markdown(report: &PerformanceReport, manifest: &SessionManifest, redact: bool) -> String {
    let diagnostics = report
        .diagnostics
        .iter()
        .map(|diagnostic| {
            format!(
                "- {:?}: {} (evidence {})",
                diagnostic.category, diagnostic.rule_id, diagnostic.evidence_score
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let application = if redact {
        "`<redacted>`".to_owned()
    } else {
        markdown_escape(&manifest.application)
    };
    let process_line = if redact {
        String::new()
    } else {
        format!("- Process ID: {}\n", manifest.process_id)
    };
    format!(
        "# RidTheWann Performance Report\n\n- Application: {}\n{}- Frames: {}\n- P99 frame time: {:.3} ms\n- Stutter events: {}\n- Interrupted: {}\n\n## Diagnostics\n{}\n",
        application,
        process_line,
        report.frame_count,
        report.frame_statistics.percentiles.p99_ms,
        report.stutter_events.len(),
        report.interrupted,
        if diagnostics.is_empty() {
            "- none"
        } else {
            &diagnostics
        }
    )
}

fn render_html(report: &PerformanceReport, manifest: &SessionManifest, redact: bool) -> String {
    let application = if redact {
        "&lt;redacted&gt;".to_owned()
    } else {
        html_escape(&manifest.application)
    };
    let process_paragraph = if redact {
        String::new()
    } else {
        format!("<p>Process ID: {}</p>", manifest.process_id)
    };
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>RWP Report</title></head><body><h1>RidTheWann Performance Report</h1><p>Application: {}</p>{}<p>Frames: {}</p><p>P99 frame time: {:.3} ms</p><p>Stutter events: {}</p><p>Interrupted: {}</p></body></html>",
        application,
        process_paragraph,
        report.frame_count,
        report.frame_statistics.percentiles.p99_ms,
        report.stutter_events.len(),
        report.interrupted
    )
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn write_output(output: Option<PathBuf>, text: &str) -> Result<()> {
    match output {
        Some(path) => {
            fs::write(&path, text.as_bytes())
                .with_context(|| format!("failed to write output {}", path.display()))?;
        }
        None => println!("{text}"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Cli, ReportFormat, markdown_escape};
    use clap::Parser;

    #[test]
    fn markdown_escape_neutralizes_structure_and_line_breaks() {
        assert_eq!(
            markdown_escape("Game`tick`\n- item | [link](url) <tag>&value"),
            "Game\\`tick\\` \\- item \\| \\[link\\]\\(url\\) \\<tag\\>&amp;value"
        );
    }
    #[test]
    fn untrusted_json_depth_is_bounded() -> Result<(), Box<dyn std::error::Error>> {
        assert!(super::ensure_json_depth(b"[[[0]]]", 2).is_err());
        assert!(super::ensure_json_depth(br#"{ "text": "[[[[[[" }"#, 1).is_ok());
        Ok(())
    }

    #[test]
    fn bounded_input_reader_rejects_oversized_files() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join("rwp-cli-bounded-input-test.json");
        std::fs::write(&path, b"12345")?;
        assert!(super::read_bounded_file(&path, 4, "test input").is_err());
        std::fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn parses_global_json_error_flag() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from(["rwp", "--json", "report", "--session", "sessions/run"])?;
        assert!(cli.json);
        Ok(())
    }

    #[test]
    fn parses_doctor_bundle_command() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from([
            "rwp",
            "doctor",
            "--presentmon",
            r"C:\Tools\PresentMon.exe",
            "--bundle",
            "doctor.json",
        ])?;
        assert!(matches!(
            cli.command,
            super::Command::Doctor {
                presentmon: Some(_),
                bundle: Some(_),
            }
        ));
        Ok(())
    }

    #[test]
    fn parses_import_command() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from([
            "rwp",
            "import-presentmon",
            "capture.csv",
            "--process-id",
            "42",
            "--qpc-frequency-hz",
            "10000000",
            "--output-dir",
            "sessions/run",
        ])?;
        assert!(matches!(
            cli.command,
            super::Command::ImportPresentmon { .. }
        ));
        Ok(())
    }

    #[test]
    fn refresh_rate_selects_stutter_analysis_mode() -> Result<(), Box<dyn std::error::Error>> {
        assert!(matches!(
            super::stutter_config_for_refresh_rate(None)?,
            rwp_report::StutterAnalysisConfig::V1(_)
        ));
        let v2 = super::stutter_config_for_refresh_rate(Some(144.0))?;
        assert!(
            matches!(v2, rwp_report::StutterAnalysisConfig::V2(config) if config.refresh_rate_hz == 144.0)
        );
        assert!(super::stutter_config_for_refresh_rate(Some(0.0)).is_err());
        assert!(super::stutter_config_for_refresh_rate(Some(f64::NAN)).is_err());
        assert!(super::stutter_config_for_refresh_rate(Some(f64::INFINITY)).is_err());
        Ok(())
    }

    #[test]
    fn parses_compare_pairs_command() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from(["rwp", "compare", "--pairs", "pairs"])?;
        assert!(matches!(
            cli.command,
            super::Command::Compare { pairs: Some(_), .. }
        ));
        Ok(())
    }

    #[test]
    fn parses_report_redaction_default() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from([
            "rwp",
            "report",
            "--session",
            "sessions/run",
            "--format",
            "md",
        ])?;
        assert!(matches!(
            cli.command,
            super::Command::Report { redact: true, .. }
        ));
        Ok(())
    }

    #[test]
    fn parses_report_formats() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from([
            "rwp",
            "report",
            "--session",
            "sessions/run",
            "--format",
            "html",
        ])?;
        assert!(matches!(
            cli.command,
            super::Command::Report {
                format: ReportFormat::Html,
                ..
            }
        ));
        Ok(())
    }

    fn persist_validation_fixture(
        root: &std::path::Path,
        name: &str,
        frames: Vec<rwp_core::FrameSample>,
        samples: Vec<rwp_core::SystemSample>,
    ) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        use rwp_analysis::{StatisticsConfig, StutterConfig};
        let good_frames = vec![
            rwp_core::FrameSample::new(0, 8.0)?,
            rwp_core::FrameSample::new(8_000_000, 8.1)?,
            rwp_core::FrameSample::new(16_000_000, 8.2)?,
            rwp_core::FrameSample::new(24_000_000, 40.0)?,
            rwp_core::FrameSample::new(32_000_000, 8.0)?,
            rwp_core::FrameSample::new(40_000_000, 8.1)?,
            rwp_core::FrameSample::new(48_000_000, 8.2)?,
        ];
        let good_capture = rwp_report::CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames: good_frames,
        };
        let report = rwp_report::build_report(
            &good_capture,
            StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            StutterConfig {
                minimum_baseline_samples: 3,
                ..StutterConfig::default()
            },
        )?;
        let capture = rwp_report::CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames,
        };
        let coverage = rwp_session::CoverageSummary {
            frame_seconds: None,
            telemetry_seconds: None,
            intersection_seconds: None,
        };
        let provenance = rwp_session::SessionProvenance {
            rwp_version: String::from("0.4.0"),
            git_sha: None,
            build_target: None,
            build_profile: None,
            os: None,
            cpu: None,
            gpu: None,
            driver: None,
            presentmon_version: None,
            presentmon_sha256: None,
            qpc_frequency_hz: Some(1_000_000),
            sample_interval_ms: None,
            exclude_dropped: None,
            stop_existing_session: None,
            no_process_probing: None,
        };
        let csv = root.join(format!("{name}.csv"));
        std::fs::write(&csv, b"stub")?;
        let dir = root.join(name);
        let frame_type_counts: Vec<(String, usize)> = Vec::new();
        let input = rwp_session::PersistedSessionInput {
            process_id: 42,
            application: "test",
            interrupted: false,
            display_refresh_rate_hz: None,
            dominant_swap_chain_address: None,
            discarded_frame_count: 0,
            frame_type_counts: &frame_type_counts,
            telemetry_warning_count: 0,
            coverage: &coverage,
            capture: &capture,
            report: &report,
            system_samples: &samples,
            source_csv: &csv,
            provenance,
        };
        rwp_session::persist_session(&dir, &input)?;
        Ok(dir)
    }

    fn validation_root(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("rwp-cli-validation-{name}"));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        Ok(root)
    }

    #[test]
    fn persisted_session_with_valid_content_loads() -> Result<(), Box<dyn std::error::Error>> {
        let root = validation_root("valid")?;
        let dir = persist_validation_fixture(
            &root,
            "session",
            vec![rwp_core::FrameSample::new(0, 8.0)?],
            vec![rwp_core::SystemSample::new(0)],
        )?;
        let loaded = super::load_session(&dir)?;
        assert_eq!(loaded.capture.frames.len(), 1);
        assert_eq!(loaded.system_samples.len(), 1);
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn persisted_session_rejects_nonpositive_frame_time() -> Result<(), Box<dyn std::error::Error>>
    {
        // FrameSample::new would reject this at construction, so build the
        // struct literally to simulate hostile persisted input.
        let bad = rwp_core::FrameSample {
            timestamp_ns: 0,
            frame_time_ms: 0.0,
            cpu_busy_ms: None,
            cpu_wait_ms: None,
            gpu_time_ms: None,
            gpu_busy_ms: None,
            gpu_wait_ms: None,
            display_latency_ms: None,
        };
        let root = validation_root("frame-time")?;
        let dir = persist_validation_fixture(&root, "session", vec![bad], Vec::new())?;
        let result = super::load_session(&dir);
        std::fs::remove_dir_all(&root)?;
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn persisted_session_rejects_negative_frame_telemetry() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut frame = rwp_core::FrameSample::new(0, 8.0)?;
        frame.cpu_busy_ms = Some(-1.0);
        let root = validation_root("frame-telemetry")?;
        let dir = persist_validation_fixture(&root, "session", vec![frame], Vec::new())?;
        let result = super::load_session(&dir);
        std::fs::remove_dir_all(&root)?;
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn persisted_session_rejects_out_of_range_utilization() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut sample = rwp_core::SystemSample::new(0);
        sample.cpu_utilization = Some(101.0);
        let root = validation_root("percent")?;
        let dir = persist_validation_fixture(
            &root,
            "session",
            vec![rwp_core::FrameSample::new(0, 8.0)?],
            vec![sample],
        )?;
        let result = super::load_session(&dir);
        std::fs::remove_dir_all(&root)?;
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn persisted_session_rejects_negative_power_but_keeps_finite_temperature()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bad_power = rwp_core::SystemSample::new(0);
        bad_power.gpu_power_w = Some(-5.0);
        let mut hot = rwp_core::SystemSample::new(0);
        hot.gpu_power_w = Some(250.0);
        hot.gpu_temperature_c = Some(95.0);
        let root = validation_root("power-temp")?;
        let bad_dir = persist_validation_fixture(
            &root,
            "bad",
            vec![rwp_core::FrameSample::new(0, 8.0)?],
            vec![bad_power],
        )?;
        assert!(super::load_session(&bad_dir).is_err());
        let hot_dir = persist_validation_fixture(
            &root,
            "hot",
            vec![rwp_core::FrameSample::new(0, 8.0)?],
            vec![hot],
        )?;
        assert!(super::load_session(&hot_dir).is_ok());
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn persisted_session_rejects_backwards_timestamps() -> Result<(), Box<dyn std::error::Error>> {
        let frames = vec![
            rwp_core::FrameSample::new(200, 8.0)?,
            rwp_core::FrameSample::new(100, 8.0)?,
        ];
        let samples = vec![
            rwp_core::SystemSample::new(200),
            rwp_core::SystemSample::new(100),
        ];
        let root = validation_root("timestamps")?;
        let dir = persist_validation_fixture(&root, "session", frames, samples)?;
        let result = super::load_session(&dir);
        std::fs::remove_dir_all(&root)?;
        assert!(result.is_err());
        Ok(())
    }

    #[test]
    fn html_escape_neutralizes_markup_and_quotes() {
        assert_eq!(
            super::html_escape("Game<test>&\"Co\" 'Ltd'"),
            "Game&lt;test&gt;&amp;&quot;Co&quot; &#39;Ltd&#39;"
        );
    }

    fn render_fixture() -> Result<
        (rwp_report::PerformanceReport, rwp_session::SessionManifest),
        Box<dyn std::error::Error>,
    > {
        let frames = (0..10_u64)
            .map(|index| rwp_core::FrameSample::new(index * 16_000_000, 16.0))
            .collect::<Result<Vec<_>, _>>()?;
        let capture = rwp_report::CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames,
        };
        let report = rwp_report::build_report(
            &capture,
            rwp_analysis::StatisticsConfig {
                p999_reliability_threshold: 1,
            },
            rwp_analysis::StutterConfig::default(),
        )?;
        let manifest = rwp_session::SessionManifest {
            provenance: rwp_session::SessionProvenance {
                rwp_version: String::from("0.4.0"),
                git_sha: None,
                build_target: None,
                build_profile: None,
                os: None,
                cpu: None,
                gpu: None,
                driver: None,
                presentmon_version: None,
                presentmon_sha256: None,
                qpc_frequency_hz: None,
                sample_interval_ms: None,
                exclude_dropped: None,
                stop_existing_session: None,
                no_process_probing: None,
            },
            schema_version: String::from("rwp.session.manifest.v2"),
            process_id: 42,
            application: String::from("Game<test>&Co"),
            interrupted: false,
            display_refresh_rate_hz: None,
            dominant_swap_chain_address: None,
            discarded_frame_count: 0,
            frame_type_counts: Vec::new(),
            telemetry_warning_count: 0,
            coverage: rwp_session::CoverageSummary {
                frame_seconds: None,
                telemetry_seconds: None,
                intersection_seconds: None,
            },
            files: Vec::new(),
        };
        Ok((report, manifest))
    }

    #[test]
    fn markdown_render_redacts_application_and_process() -> Result<(), Box<dyn std::error::Error>> {
        let (report, manifest) = render_fixture()?;
        let redacted = super::render_markdown(&report, &manifest, true);
        assert!(redacted.contains("`<redacted>`"));
        assert!(!redacted.contains("Process ID"));
        assert!(!redacted.contains("Game"));

        let open = super::render_markdown(&report, &manifest, false);
        assert!(open.contains("Process ID: 42"));
        assert!(open.contains("&amp;"));
        assert!(!open.contains("Game<test>"));
        Ok(())
    }

    #[test]
    fn html_render_redacts_application_and_process() -> Result<(), Box<dyn std::error::Error>> {
        let (report, manifest) = render_fixture()?;
        let redacted = super::render_html(&report, &manifest, true);
        assert!(redacted.contains("&lt;redacted&gt;"));
        assert!(!redacted.contains("Process ID"));
        assert!(redacted.starts_with("<!doctype html>"));

        let open = super::render_html(&report, &manifest, false);
        assert!(open.contains("Process ID: 42"));
        assert!(open.contains("Game&lt;test&gt;&amp;Co"));
        Ok(())
    }

    #[test]
    fn migrate_report_normalizes_legacy_schema() -> Result<(), Box<dyn std::error::Error>> {
        let (report, _) = render_fixture()?;
        let legacy = super::to_json(&report)?.replacen(
            "\"schema_version\":\"rwp.report.v2\"",
            "\"schema_version\":\"rwp.report.v1\"",
            1,
        );
        assert!(legacy.contains("\"schema_version\":\"rwp.report.v1\""));
        let root = validation_root("migrate-report")?;
        let source = root.join("legacy.json");
        let destination = root.join("migrated.json");
        std::fs::write(&source, &legacy)?;
        super::migrate_report_file(&source, &destination)?;
        let migrated = std::fs::read_to_string(&destination)?;
        assert!(migrated.contains("\"schema_version\":\"rwp.report.v2\""));
        assert!(super::from_json(&migrated).is_ok());
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn migrate_report_rejects_unknown_schema() -> Result<(), Box<dyn std::error::Error>> {
        let (report, _) = render_fixture()?;
        let unknown = super::to_json(&report)?.replacen(
            "\"schema_version\":\"rwp.report.v2\"",
            "\"schema_version\":\"rwp.report.v9\"",
            1,
        );
        let root = validation_root("migrate-unknown")?;
        let source = root.join("unknown.json");
        let destination = root.join("migrated.json");
        std::fs::write(&source, &unknown)?;
        let result = super::migrate_report_file(&source, &destination);
        let error = result.err().ok_or("unknown schema must fail migration")?;
        let report = super::classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E3000");
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn migrate_session_normalizes_legacy_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let root = validation_root("migrate-session")?;
        let dir = persist_validation_fixture(
            &root,
            "legacy",
            vec![rwp_core::FrameSample::new(0, 8.0)?],
            vec![rwp_core::SystemSample::new(0)],
        )?;
        let manifest_path = dir.join("manifest.json");
        let legacy_manifest = std::fs::read_to_string(&manifest_path)?.replacen(
            "rwp.session.manifest.v2",
            "rwp.session.manifest.v1",
            1,
        );
        std::fs::write(&manifest_path, &legacy_manifest)?;
        assert!(super::load_session(&dir).is_err());

        let migrated = root.join("migrated");
        super::migrate_session(&dir, &migrated)?;
        let loaded = super::load_session(&migrated)?;
        assert_eq!(loaded.report.schema_version, "rwp.report.v2");
        assert_eq!(loaded.manifest.schema_version, "rwp.session.manifest.v2");
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn migrate_requires_exactly_one_source() {
        assert!(super::migrate(None, None, None, None).is_err());
    }

    #[test]
    fn parses_capture_with_process_id() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from([
            "rwp",
            "capture",
            "--presentmon",
            r"C:\Tools\PresentMon.exe",
            "--process-id",
            "42",
            "--duration-seconds",
            "10",
            "--output-dir",
            "sessions/live",
        ])?;
        assert!(matches!(cli.command, super::Command::Capture { .. }));
        Ok(())
    }

    #[test]
    fn parses_capture_with_process_name_and_options() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from([
            "rwp",
            "capture",
            "--presentmon",
            r"C:\Tools\PresentMon.exe",
            "--process-name",
            "Game.exe",
            "--duration-seconds",
            "60",
            "--interval-ms",
            "50",
            "--no-process-probing",
            "--output-dir",
            "sessions/live",
        ])?;
        assert!(matches!(cli.command, super::Command::Capture { .. }));
        Ok(())
    }

    #[test]
    fn parses_capture_stop_existing_session() -> Result<(), Box<dyn std::error::Error>> {
        let cli = Cli::try_parse_from([
            "rwp",
            "capture",
            "--presentmon",
            r"C:\Tools\PresentMon.exe",
            "--process-id",
            "42",
            "--duration-seconds",
            "10",
            "--stop-existing-session",
            "--output-dir",
            "sessions/live",
        ])?;
        assert!(matches!(
            cli.command,
            super::Command::Capture {
                stop_existing_session: true,
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn capture_rejects_both_process_selectors() {
        let result = Cli::try_parse_from([
            "rwp",
            "capture",
            "--presentmon",
            r"C:\Tools\PresentMon.exe",
            "--process-id",
            "42",
            "--process-name",
            "Game.exe",
            "--duration-seconds",
            "10",
            "--output-dir",
            "sessions/live",
        ]);
        assert!(result.is_err(), "PID and name are mutually exclusive");
    }

    #[test]
    fn capture_validation_rejects_bad_durations_and_intervals() {
        assert!(super::validate_capture_duration(0).is_err());
        assert!(super::validate_capture_duration(3601).is_err());
        assert!(super::validate_capture_duration(10).is_ok());
        assert!(super::validate_capture_interval_ms(9).is_err());
        assert!(super::validate_capture_interval_ms(5001).is_err());
        assert!(super::validate_capture_interval_ms(50).is_ok());
    }

    #[test]
    fn capture_validation_rejects_empty_selectors() {
        assert!(super::resolve_capture_target(None, None).is_err());
        assert!(super::resolve_capture_target(Some(0), None).is_err());
        assert!(super::resolve_capture_target(Some(42), None).is_ok());
        assert!(super::resolve_capture_target(None, Some(String::new())).is_err());
        assert!(super::resolve_capture_target(None, Some(String::from("Game.exe"))).is_ok());
        assert!(super::resolve_capture_target(Some(42), Some(String::from("Game.exe"))).is_err());
    }

    #[test]
    fn session_errors_map_to_live_platform_code() {
        // Pure taxonomy mapping, testable on every platform: the Windows
        // builder exercises the invalid-configuration arm, other platforms
        // the unsupported-platform arm; both must stay RWP-E5100/exit 51.
        #[cfg(windows)]
        let error = anyhow::Error::from(rwp_session::SessionError::InvalidConfiguration("probe"));
        #[cfg(not(windows))]
        let error = anyhow::Error::from(rwp_session::SessionError::UnsupportedPlatform);
        let report = super::classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E5100");
        assert_eq!(report.descriptor.exit_code, 51);
    }

    #[test]
    fn etw_hint_appears_only_for_killed_stops() -> Result<(), Box<dyn std::error::Error>> {
        let hint = super::etw_kill_hint_for_outcome(Some(rwp_capture::StopOutcome::Killed))
            .ok_or("a killed stop must carry the ETW-hygiene hint")?;
        assert!(hint.contains("logman query -ets"));
        assert!(hint.contains("--stop-existing-session"));
        assert_eq!(
            super::etw_kill_hint_for_outcome(Some(rwp_capture::StopOutcome::Graceful)),
            None
        );
        assert_eq!(super::etw_kill_hint_for_outcome(None), None);
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn dropped_rows_notice_reports_parser_warning() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(super::dropped_rows_notice(&[]), None);
        let unrelated = vec![rwp_session::TelemetryWarning {
            message: String::from("GlobalMemoryStatusEx failed"),
        }];
        assert_eq!(super::dropped_rows_notice(&unrelated), None);
        let warnings = vec![rwp_session::TelemetryWarning {
            message: String::from(
                "dropped 2 rows with backwards QPC timestamps; source CSV retained, counts recorded here",
            ),
        }];
        let notice = super::dropped_rows_notice(&warnings).ok_or("dropped rows must warn")?;
        assert!(notice.starts_with("note: "));
        assert!(notice.contains('2'));
        Ok(())
    }

    #[test]
    fn interrupted_capture_keeps_exit_zero_with_partial_notice() {
        let dir = std::path::PathBuf::from("sessions/live");
        assert_eq!(
            super::capture_completion_notice(true, &dir),
            Some(String::from(
                "interrupted: partial session saved at sessions/live"
            ))
        );
        assert_eq!(super::capture_completion_notice(false, &dir), None);
    }

    #[cfg(windows)]
    fn seam_fixture_session(
        root: &std::path::Path,
        name: &str,
        interrupted: bool,
    ) -> Result<(std::path::PathBuf, rwp_session::SessionResult), Box<dyn std::error::Error>> {
        use rwp_analysis::{StatisticsConfig, StutterConfig};
        let frames = vec![
            rwp_core::FrameSample::new(0, 8.0)?,
            rwp_core::FrameSample::new(8_000_000, 8.1)?,
        ];
        let document = rwp_report::CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames,
        };
        let report = rwp_report::build_report_with_stutter_analysis(
            &document,
            StatisticsConfig::default(),
            rwp_report::StutterAnalysisConfig::V1(StutterConfig::default()),
            interrupted,
        )?;
        let diagnostics =
            rwp_diagnostics::DiagnosticEngine::new(rwp_diagnostics::RuleThresholds::default())
                .diagnose(rwp_diagnostics::DiagnosisInput {
                    statistics: &report.frame_statistics,
                    frames: &document.frames,
                    stutters: &report.stutter_events,
                    system_samples: &[],
                })?;
        let report = rwp_report::with_diagnostics(report, diagnostics.clone());
        let coverage = rwp_session::CoverageSummary {
            frame_seconds: None,
            telemetry_seconds: None,
            intersection_seconds: None,
        };
        let source_csv = root.join("source.csv");
        std::fs::write(&source_csv, b"seam-source")?;
        let session_dir = root.join(name);
        let frame_type_counts: Vec<(String, usize)> = Vec::new();
        let input = rwp_session::PersistedSessionInput {
            process_id: 42,
            application: "SeamGame.exe",
            interrupted,
            display_refresh_rate_hz: None,
            dominant_swap_chain_address: None,
            discarded_frame_count: 0,
            frame_type_counts: &frame_type_counts,
            telemetry_warning_count: 0,
            coverage: &coverage,
            capture: &document,
            report: &report,
            system_samples: &[],
            source_csv: &source_csv,
            provenance: rwp_session::SessionProvenance {
                rwp_version: String::from("0.1.0-test"),
                git_sha: None,
                build_target: None,
                build_profile: None,
                os: None,
                cpu: None,
                gpu: None,
                driver: None,
                presentmon_version: None,
                presentmon_sha256: None,
                qpc_frequency_hz: None,
                sample_interval_ms: None,
                exclude_dropped: None,
                stop_existing_session: None,
                no_process_probing: None,
            },
        };
        rwp_session::persist_session(&session_dir, &input)?;
        rwp_session::verify_manifest(&session_dir)?;
        let result = rwp_session::SessionResult {
            process_id: 42,
            display_refresh_rate: None,
            display_refresh_warning: None,
            dominant_swap_chain_address: None,
            discarded_frame_count: 0,
            frame_type_counts: Vec::new(),
            application: String::from("SeamGame.exe"),
            system_samples: Vec::new(),
            report,
            telemetry_warnings: Vec::new(),
            diagnostics,
            coverage,
            stop_outcome: interrupted.then_some(rwp_capture::StopOutcome::Killed),
        };
        Ok((session_dir, result))
    }

    #[cfg(windows)]
    fn seam_validation_root(name: &str) -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "rwp-cli-capture-seam-{name}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        Ok(root)
    }

    #[cfg(windows)]
    #[test]
    fn capture_seam_persists_session_with_manifest() -> Result<(), Box<dyn std::error::Error>> {
        let root = seam_validation_root("persist")?;
        let (session_dir, result) = seam_fixture_session(&root, "seam", false)?;
        let config =
            rwp_session::SessionConfig::new("PresentMon.exe", root.join("temporary.csv"), 42, 10)?;
        let temp_csv = root.join("temporary.csv");
        std::fs::write(&temp_csv, b"temporary")?;
        let cancellation = std::sync::atomic::AtomicBool::new(false);
        let outcome =
            super::run_capture_session(&config, &temp_csv, &cancellation, |_, _| Ok(result))?;
        assert!(!outcome.report.interrupted);
        assert!(
            !temp_csv.exists(),
            "the seam removes the temporary CSV on success"
        );
        let manifest_bytes = std::fs::read(session_dir.join("manifest.json"))?;
        let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;
        assert_eq!(manifest["process_id"], 42);
        assert_eq!(manifest["application"], "SeamGame.exe");
        assert_eq!(manifest["interrupted"], false);
        rwp_session::verify_manifest(&session_dir)?;
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn capture_seam_interrupted_result_reports_partial_notice()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = seam_validation_root("interrupted")?;
        let (session_dir, result) = seam_fixture_session(&root, "seam", true)?;
        let config =
            rwp_session::SessionConfig::new("PresentMon.exe", root.join("temporary.csv"), 42, 10)?;
        let temp_csv = root.join("temporary.csv");
        std::fs::write(&temp_csv, b"temporary")?;
        let cancellation = std::sync::atomic::AtomicBool::new(false);
        let outcome =
            super::run_capture_session(&config, &temp_csv, &cancellation, |_, _| Ok(result))?;
        assert!(outcome.report.interrupted);
        let notice = super::capture_completion_notice(true, &session_dir)
            .ok_or("an interrupted capture must produce the partial-session notice")?;
        assert_eq!(
            notice,
            format!(
                "interrupted: partial session saved at {}",
                session_dir.display()
            )
        );
        let manifest_bytes = std::fs::read(session_dir.join("manifest.json"))?;
        let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes)?;
        assert_eq!(manifest["interrupted"], true);
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[cfg(windows)]
    #[test]
    fn capture_seam_propagates_runner_errors_and_cleans_temp()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = seam_validation_root("error")?;
        let config =
            rwp_session::SessionConfig::new("PresentMon.exe", root.join("temporary.csv"), 42, 10)?;
        let temp_csv = root.join("temporary.csv");
        std::fs::write(&temp_csv, b"temporary")?;
        let cancellation = std::sync::atomic::AtomicBool::new(false);
        let error = super::run_capture_session(&config, &temp_csv, &cancellation, |_, _| {
            Err(rwp_session::SessionError::InvalidConfiguration("injected"))
        })
        .err()
        .ok_or("runner errors must propagate through the seam")?;
        assert!(
            !temp_csv.exists(),
            "the seam removes the temporary CSV on failure too"
        );
        let report = super::classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E5100");
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[cfg(not(windows))]
    #[test]
    fn capture_on_unsupported_platform_uses_stable_machine_code() {
        let result = super::run_capture(
            std::path::PathBuf::from("PresentMon.exe"),
            None,
            None,
            10,
            50,
            false,
            false,
            None,
            std::path::PathBuf::from("sessions/live"),
        );
        let error = match result {
            Ok(()) => panic!("capture must fail off Windows"),
            Err(error) => error,
        };
        let report = super::classify(&error);
        assert_eq!(report.descriptor.code, "RWP-E5100");
        assert_eq!(report.descriptor.exit_code, 51);
    }
}
