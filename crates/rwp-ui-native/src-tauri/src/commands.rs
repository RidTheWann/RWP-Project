//! Tauri IPC commands for the live capture session.

use std::{
    fs,
    io::Read,
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard,
        atomic::{AtomicBool, Ordering},
    },
};

use rwp_analysis::{ComparisonConfig, ComparisonInput, compare};
use rwp_report::{PerformanceReport, from_json};
use rwp_session::{SessionConfig, SessionResult, SessionRunner};
use rwp_ui::DashboardModel;
use serde::{Deserialize, Serialize};
use tauri::State;

const MAX_SESSION_ID_LENGTH: usize = 96;

/// Upper bound for numbered session-directory probes before falling back to a
/// timestamp suffix.
const MAX_SESSION_ID_PROBES: usize = 10_000;

#[derive(Debug, Deserialize)]
pub struct SessionStartRequest {
    /// Absolute path to PresentMon.exe.
    pub presentmon_executable: String,
    /// Absolute path for the generated PresentMon CSV.
    pub output_file: String,
    /// Target process identifier.
    pub process_id: u32,
    /// Maximum capture duration in seconds.
    pub duration_seconds: u64,
    /// Windows telemetry sampling interval in milliseconds.
    #[serde(default)]
    pub sample_interval_ms: Option<u64>,
    /// Exclude dropped presents from the capture.
    #[serde(default)]
    pub exclude_dropped: bool,
    /// Explicitly allow stopping an existing PresentMon trace session.
    #[serde(default)]
    pub stop_existing_session: bool,
    /// Disable RWP's additional process/thread telemetry probing.
    #[serde(default)]
    pub no_process_probing: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "status")]
pub enum SessionSnapshot {
    /// No session has been started.
    Idle,
    /// A session is actively collecting capture and telemetry.
    Running {
        /// Target process identifier.
        process_id: u32,
    },
    /// A session completed and produced deterministic analysis.
    Completed {
        /// Target process identifier.
        process_id: u32,
        /// Active-display refresh rate captured at session start, when available.
        display_refresh_rate_hz: Option<f64>,
        /// Display-device name used for the refresh-rate observation, when available.
        display_refresh_device: Option<String>,
        /// Non-fatal refresh-rate resolver warning.
        display_refresh_warning: Option<String>,
        /// Dominant PresentMon swap-chain address, when available.
        dominant_swap_chain_address: Option<u64>,
        /// Number of source rows discarded as non-dominant swap-chain frames.
        discarded_frame_count: usize,
        /// Frame-type distribution retained after swap-chain filtering.
        frame_type_counts: Vec<(String, usize)>,
        /// Symmetric frame/telemetry observation coverage.
        coverage: rwp_session::CoverageSummary,
        /// Application name reported by PresentMon.
        application: String,
        /// Number of normalized frame observations.
        frame_count: usize,
        /// Number of system telemetry observations.
        system_sample_count: usize,
        /// Deterministic performance report.
        report: Box<PerformanceReport>,
        /// Renderer-neutral dashboard model.
        dashboard: Box<DashboardModel>,
    },
    /// A session failed before producing a complete result.
    Failed {
        /// Human-readable failure cause.
        message: String,
    },
}

#[derive(Debug, Clone, Serialize)]
pub struct SessionHistoryEntry {
    /// Stable session directory name shown in the UI.
    pub session_id: String,
    /// Application name from the verified manifest.
    pub application: String,
    /// Target process identifier.
    pub process_id: u32,
    /// Number of normalized frame observations.
    pub frame_count: usize,
    /// Whether the capture ended by interruption.
    pub interrupted: bool,
    /// Whether all persisted file hashes verified successfully.
    pub verified: bool,
}

const MAX_SESSION_REPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SESSION_TELEMETRY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_SESSION_TELEMETRY_SAMPLES: usize = 5_000_000;

fn read_bounded_file(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|error| error.to_string())?;
    let mut bytes = Vec::new();
    file.take(limit.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!(
            "session file exceeds the configured limit ({limit} bytes): {}",
            path.display()
        ));
    }
    Ok(bytes)
}

fn load_persisted_report(path: &Path) -> Result<PerformanceReport, String> {
    let bytes = read_bounded_file(path, MAX_SESSION_REPORT_BYTES)?;
    let input = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    from_json(input).map_err(|error| error.to_string())
}

fn count_persisted_telemetry_samples(path: &Path) -> Result<usize, String> {
    let bytes = read_bounded_file(path, MAX_SESSION_TELEMETRY_BYTES)?;
    let input = std::str::from_utf8(&bytes).map_err(|error| error.to_string())?;
    let mut count = 0_usize;
    for _ in input.lines().filter(|line| !line.trim().is_empty()) {
        count = count.saturating_add(1);
        if count > MAX_SESSION_TELEMETRY_SAMPLES {
            return Err(format!(
                "telemetry sample count exceeds the configured limit ({MAX_SESSION_TELEMETRY_SAMPLES})"
            ));
        }
    }
    Ok(count)
}

fn history_root() -> Result<PathBuf, String> {
    let root = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .ok_or_else(|| {
            "LOCALAPPDATA is unavailable; session history cannot be persisted".to_owned()
        })?;
    Ok(root.join("RidTheWann Performance").join("sessions"))
}

fn session_id_from_output(output: &Path) -> String {
    let raw = output
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("session");
    let mut id = raw
        .chars()
        .map(|value| {
            if value.is_ascii_alphanumeric() || matches!(value, '-' | '_' | '.') {
                value
            } else {
                '_'
            }
        })
        .collect::<String>();
    id.truncate(80);
    while id.ends_with('.') {
        id.truncate(id.len() - 1);
    }
    if id.is_empty() || id == "." || id == ".." || is_windows_reserved_device_name(&id) {
        String::from("session")
    } else {
        id
    }
}

/// Returns a session identifier that does not collide with an existing history
/// directory, so a repeated capture file name never discards a finished result.
fn unique_session_id(root: &Path, base: &str) -> String {
    let mut normalized = base.to_owned();
    if normalized.len() > MAX_SESSION_ID_LENGTH {
        normalized.truncate(MAX_SESSION_ID_LENGTH);
    }
    while normalized.ends_with('.') {
        normalized.truncate(normalized.len() - 1);
    }
    if normalized.is_empty() {
        normalized = String::from("session");
    }
    if !root.join(&normalized).exists() {
        return normalized;
    }
    for index in 2..=MAX_SESSION_ID_PROBES {
        let candidate = format!("{normalized}-{index}");
        if candidate.len() > MAX_SESSION_ID_LENGTH {
            break;
        }
        if !root.join(&candidate).exists() {
            return candidate;
        }
    }
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    let mut fallback = format!("{normalized}-{seconds}");
    if fallback.len() > MAX_SESSION_ID_LENGTH {
        fallback.truncate(MAX_SESSION_ID_LENGTH);
    }
    while fallback.ends_with('.') {
        fallback.truncate(fallback.len() - 1);
    }
    fallback
}

/// Builds one history entry from an already-verified manifest, skipping
/// sessions whose report cannot be parsed instead of failing the listing.
fn history_entry_from_verified_report(
    session_id: &str,
    manifest: &rwp_session::SessionManifest,
    report_path: &Path,
) -> Option<SessionHistoryEntry> {
    let report = load_persisted_report(report_path).ok()?;
    Some(SessionHistoryEntry {
        session_id: session_id.to_owned(),
        application: manifest.application.clone(),
        process_id: manifest.process_id,
        frame_count: report.frame_count,
        interrupted: manifest.interrupted,
        verified: true,
    })
}

fn list_history(root: &Path) -> Result<Vec<SessionHistoryEntry>, String> {
    let mut entries = Vec::new();
    if !root.exists() {
        return Ok(Vec::new());
    }
    for item in fs::read_dir(root).map_err(|error| error.to_string())? {
        let item = item.map_err(|error| error.to_string())?;
        let session_id = item.file_name().to_string_lossy().into_owned();
        if !valid_session_id(&session_id) {
            continue;
        }
        if !item
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let path = match canonical_session_path_within_root(root, &item.path()) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let manifest = match rwp_session::verify_manifest(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Some(entry) =
            history_entry_from_verified_report(&session_id, &manifest, &path.join("report.json"))
        {
            entries.push(entry);
        }
    }
    entries.sort_by(|left, right| left.session_id.cmp(&right.session_id));
    Ok(entries)
}

/// Returns verified persisted sessions stored by the native GUI.
///
/// Async so manifest hashing and report parsing over hundreds of megabytes
/// never block the webview main thread.
#[tauri::command(async)]
pub fn session_history() -> Result<Vec<SessionHistoryEntry>, String> {
    list_history(&history_root()?)
}

fn is_windows_reserved_device_name(session_id: &str) -> bool {
    let base_name = match session_id.split('.').next() {
        Some(value) => value,
        None => return false,
    };
    let base_name = base_name.to_ascii_uppercase();
    if matches!(base_name.as_str(), "CON" | "PRN" | "AUX" | "NUL") {
        return true;
    }

    ["COM", "LPT"].into_iter().any(|prefix| {
        base_name.strip_prefix(prefix).is_some_and(|number| {
            number.len() == 1
                && number
                    .as_bytes()
                    .first()
                    .is_some_and(|digit| (b'1'..=b'9').contains(digit))
        })
    })
}

fn valid_session_id(session_id: &str) -> bool {
    !session_id.is_empty()
        && session_id.len() <= MAX_SESSION_ID_LENGTH
        && session_id != "."
        && session_id != ".."
        && !session_id.ends_with('.')
        && session_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && !is_windows_reserved_device_name(session_id)
}

fn canonical_session_path_within_root(root: &Path, candidate: &Path) -> Result<PathBuf, String> {
    let canonical_root = root
        .canonicalize()
        .map_err(|_| "session history root is unavailable".to_owned())?;
    let canonical_candidate = candidate
        .canonicalize()
        .map_err(|_| "session history entry is unavailable".to_owned())?;
    if !canonical_candidate.starts_with(&canonical_root) {
        return Err("session history path resolves outside the history root".to_owned());
    }
    Ok(canonical_candidate)
}

fn history_session_path(session_id: &str) -> Result<PathBuf, String> {
    if !valid_session_id(session_id) {
        return Err("invalid session history identifier".to_owned());
    }
    let root = history_root()?;
    canonical_session_path_within_root(&root, &root.join(session_id))
}

/// Loads one verified historical session into the same dashboard snapshot used by live capture.
///
/// Async so manifest hashing and report parsing never block the webview main thread.
#[tauri::command(async)]
pub fn session_history_load(session_id: String) -> Result<SessionSnapshot, String> {
    let path = history_session_path(&session_id)?;
    let manifest = rwp_session::verify_manifest(&path).map_err(|error| error.to_string())?;
    let report = load_persisted_report(&path.join("report.json"))?;
    let system_sample_count = count_persisted_telemetry_samples(&path.join("telemetry.jsonl"))?;
    let diagnostics = report.diagnostics.clone();
    let dashboard = rwp_ui::build_dashboard_model(&report, &diagnostics, None);
    Ok(SessionSnapshot::Completed {
        process_id: manifest.process_id,
        display_refresh_rate_hz: manifest.display_refresh_rate_hz,
        display_refresh_device: None,
        display_refresh_warning: None,
        dominant_swap_chain_address: manifest.dominant_swap_chain_address,
        discarded_frame_count: manifest.discarded_frame_count,
        frame_type_counts: manifest.frame_type_counts,
        coverage: manifest.coverage,
        application: manifest.application,
        frame_count: report.frame_count,
        system_sample_count,
        report: Box::new(report),
        dashboard: Box::new(dashboard),
    })
}

struct SessionState {
    snapshot: SessionSnapshot,
    cancellation: Option<Arc<AtomicBool>>,
}

#[derive(Clone)]
/// Thread-safe owner of one active capture session.
pub struct SessionManager {
    state: Arc<Mutex<SessionState>>,
}

impl Default for SessionManager {
    fn default() -> Self {
        Self {
            state: Arc::new(Mutex::new(SessionState {
                snapshot: SessionSnapshot::Idle,
                cancellation: None,
            })),
        }
    }
}

impl SessionManager {
    fn lock(&self) -> Result<MutexGuard<'_, SessionState>, String> {
        self.state
            .lock()
            .map_err(|_| "session state mutex was poisoned".to_owned())
    }

    fn snapshot(&self) -> Result<SessionSnapshot, String> {
        Ok(self.lock()?.snapshot.clone())
    }

    fn finish(&self, result: SessionResult) -> Result<(), String> {
        let mut state = self.lock()?;
        state.snapshot = SessionSnapshot::Completed {
            process_id: result.process_id,
            display_refresh_rate_hz: result.display_refresh_rate.as_ref().map(|value| value.hz),
            display_refresh_device: result
                .display_refresh_rate
                .as_ref()
                .map(|value| value.device_name.clone()),
            display_refresh_warning: result.display_refresh_warning,
            dominant_swap_chain_address: result.dominant_swap_chain_address,
            discarded_frame_count: result.discarded_frame_count,
            frame_type_counts: result.frame_type_counts,
            coverage: result.coverage,
            application: result.application,
            frame_count: result.report.frame_count,
            system_sample_count: result.system_samples.len(),
            dashboard: Box::new(rwp_ui::build_dashboard_model(
                &result.report,
                &result.diagnostics,
                None,
            )),
            report: Box::new(result.report),
        };
        state.cancellation = None;
        Ok(())
    }

    fn fail(&self, error: impl Into<String>) -> Result<(), String> {
        let mut state = self.lock()?;
        state.snapshot = SessionSnapshot::Failed {
            message: error.into(),
        };
        state.cancellation = None;
        Ok(())
    }
}

fn run_session_worker<F>(manager: SessionManager, operation: F)
where
    F: FnOnce() -> Result<SessionResult, rwp_session::SessionError>,
{
    match panic::catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(result)) => {
            if manager.finish(result).is_ok() {
                tracing::info!(target: "rwp", event = "session_completed");
            }
        }
        Ok(Err(error)) => {
            tracing::warn!(target: "rwp", event = "session_failed");
            let _ = manager.fail(error.to_string());
        }
        Err(_) => {
            tracing::error!(target: "rwp", event = "session_failed");
            let _ = manager.fail("capture session worker panicked");
        }
    }
}

fn fail_session_worker_spawn(
    manager: &SessionManager,
    error: std::io::Error,
) -> Result<SessionSnapshot, String> {
    let message = format!("failed to start capture session worker: {error}");
    manager.fail(message)?;
    tracing::error!(target: "rwp", event = "session_failed");
    manager.snapshot()
}

/// Compares two verified persisted sessions for the GUI Compare tab.
///
/// Async so manifest hashing and report parsing never block the webview main thread.
#[tauri::command(async)]
pub fn session_compare(
    baseline_session_id: String,
    candidate_session_id: String,
    same_scenario: bool,
) -> Result<rwp_ui::ComparisonView, String> {
    let baseline_path = history_session_path(&baseline_session_id)?;
    let candidate_path = history_session_path(&candidate_session_id)?;
    rwp_session::verify_manifest(&baseline_path).map_err(|error| error.to_string())?;
    rwp_session::verify_manifest(&candidate_path).map_err(|error| error.to_string())?;
    let baseline = load_persisted_report(&baseline_path.join("report.json"))?;
    let candidate = load_persisted_report(&candidate_path.join("report.json"))?;
    let result = compare(
        ComparisonInput {
            baseline: &baseline.frame_statistics,
            candidate: &candidate.frame_statistics,
            baseline_stutter_count: baseline.stutter_events.len(),
            candidate_stutter_count: candidate.stutter_events.len(),
            same_scenario,
        },
        ComparisonConfig::default(),
    )
    .map_err(|error| error.to_string())?;
    let dashboard =
        rwp_ui::build_dashboard_model(&candidate, &candidate.diagnostics, Some(&result));
    dashboard
        .comparison
        .ok_or_else(|| "comparison view was not produced".to_owned())
}

/// Starts one live capture session in a background worker.
#[tauri::command]
pub fn session_start(
    request: SessionStartRequest,
    manager: State<'_, SessionManager>,
) -> Result<SessionSnapshot, String> {
    let mut state = manager.lock()?;
    if matches!(state.snapshot, SessionSnapshot::Running { .. }) {
        return Err("a capture session is already running".to_owned());
    }

    let mut config = SessionConfig::new(
        request.presentmon_executable,
        request.output_file,
        request.process_id,
        request.duration_seconds,
    )
    .map_err(|error| error.to_string())?;

    if let Some(interval_ms) = request.sample_interval_ms {
        config = config
            .with_sample_interval(std::time::Duration::from_millis(interval_ms))
            .map_err(|error| error.to_string())?;
    }
    config = config
        .with_exclude_dropped(request.exclude_dropped)
        .with_stop_existing_session(request.stop_existing_session)
        .with_no_process_probing(request.no_process_probing);
    let persistence_dir = history_root().map(|root| {
        let base = session_id_from_output(Path::new(&config.output_file));
        root.join(unique_session_id(&root, &base))
    })?;
    config = config.with_persistence_dir(persistence_dir);

    let cancellation = Arc::new(AtomicBool::new(false));
    state.snapshot = SessionSnapshot::Running {
        process_id: config.process_id,
    };
    state.cancellation = Some(cancellation.clone());
    drop(state);
    tracing::info!(target: "rwp", event = "session_start_requested");

    let state_handle = manager.inner().state.clone();
    match std::thread::Builder::new()
        .name("rwp-session-worker".to_owned())
        .spawn(move || {
            tracing::info!(target: "rwp", event = "session_worker_started");
            let manager = SessionManager {
                state: state_handle,
            };
            run_session_worker(manager, || {
                let runner = SessionRunner;
                runner.run(&config, &cancellation)
            });
        }) {
        Ok(_worker) => manager.snapshot(),
        Err(error) => fail_session_worker_spawn(&manager, error),
    }
}

/// Requests cancellation of the active capture session.
#[tauri::command]
pub fn session_stop(manager: State<'_, SessionManager>) -> Result<SessionSnapshot, String> {
    let state = manager.lock()?;
    if let Some(cancellation) = &state.cancellation {
        tracing::info!(target: "rwp", event = "session_stop_requested");
        cancellation.store(true, Ordering::Release);
    }
    Ok(state.snapshot.clone())
}

/// Returns the latest session state.
#[tauri::command]
pub fn session_status(manager: State<'_, SessionManager>) -> Result<SessionSnapshot, String> {
    manager.snapshot()
}

/// Resets a non-running session to the idle state.
#[tauri::command]
pub fn session_reset(manager: State<'_, SessionManager>) -> Result<SessionSnapshot, String> {
    let mut state = manager.lock()?;
    if matches!(state.snapshot, SessionSnapshot::Running { .. }) {
        return Err("cannot reset a running capture session".to_owned());
    }
    state.snapshot = SessionSnapshot::Idle;
    state.cancellation = None;
    Ok(state.snapshot.clone())
}

#[cfg(test)]
mod audit_regressions {
    use super::{SessionManager, SessionSnapshot, fail_session_worker_spawn, run_session_worker};

    #[test]
    fn session_start_defaults_process_probing_on_for_compatibility()
    -> Result<(), Box<dyn std::error::Error>> {
        let request: super::SessionStartRequest = serde_json::from_str(
            r#"{"presentmon_executable":"PresentMon.exe","output_file":"capture.csv","process_id":42,"duration_seconds":10}"#,
        )?;
        assert!(!request.no_process_probing);
        Ok(())
    }

    #[test]
    fn session_id_validation_rejects_path_traversal() {
        assert!(super::valid_session_id("2026-run"));
        assert!(!super::valid_session_id(""));
        assert!(!super::valid_session_id("../other"));
        assert!(!super::valid_session_id("a\\b"));
    }

    #[test]
    fn session_id_validation_allows_safe_local_names() {
        assert!(super::valid_session_id("capture_01.v2"));
    }

    #[test]
    fn session_id_validation_rejects_unsafe_windows_names_and_characters() {
        for value in [
            "C:relative",
            "capture:stream",
            "CON",
            "con.json",
            "AUX",
            "NUL.txt",
            "PRN",
            "COM1.csv",
            "LPT9",
            "capture.",
            "capture ",
            "capture name",
            "capture/child",
            "capture\\child",
            ".",
            "..",
        ] {
            assert!(
                !super::valid_session_id(value),
                "accepted unsafe id: {value}"
            );
        }
    }

    #[test]
    fn session_id_validation_enforces_a_maximum_length() {
        assert!(super::valid_session_id(&"a".repeat(96)));
        assert!(!super::valid_session_id(&"a".repeat(97)));
    }

    #[test]
    fn session_id_generation_does_not_emit_windows_device_names() {
        let generated = super::session_id_from_output(std::path::Path::new("CON.csv"));
        assert!(super::valid_session_id(&generated));
        assert_ne!(generated, "CON");
    }

    #[test]
    fn session_path_rejects_outside_root() -> Result<(), Box<dyn std::error::Error>> {
        let temp_root =
            std::env::temp_dir().join(format!("rwp-session-containment-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_root);
        let root = temp_root.join("history");
        let inside = root.join("capture");
        let outside = temp_root.join("outside");
        std::fs::create_dir_all(&inside)?;
        std::fs::create_dir_all(&outside)?;

        let resolved = super::canonical_session_path_within_root(&root, &inside)?;
        assert_eq!(resolved, inside.canonicalize()?);
        assert!(super::canonical_session_path_within_root(&root, &outside).is_err());

        std::fs::remove_dir_all(temp_root)?;
        Ok(())
    }

    #[test]
    fn unique_session_id_keeps_free_base() -> Result<(), Box<dyn std::error::Error>> {
        let root =
            std::env::temp_dir().join(format!("rwp-session-unique-free-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        assert_eq!(super::unique_session_id(&root, "capture"), "capture");
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn unique_session_id_numbers_existing_directories() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "rwp-session-unique-collision-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("capture"))?;
        std::fs::create_dir_all(root.join("capture-2"))?;
        let third = super::unique_session_id(&root, "capture");
        assert_eq!(third, "capture-3");
        assert!(super::valid_session_id(&third));
        std::fs::create_dir_all(root.join(&third))?;
        let fourth = super::unique_session_id(&root, "capture");
        assert_eq!(fourth, "capture-4");
        assert!(super::valid_session_id(&fourth));
        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    fn test_manifest() -> rwp_session::SessionManifest {
        rwp_session::SessionManifest {
            provenance: rwp_session::SessionProvenance {
                rwp_version: String::from("0.1.0"),
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
            },
            schema_version: String::from("rwp.session.manifest.v2"),
            process_id: 42,
            application: String::from("Game.exe"),
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
        }
    }

    const TEST_REPORT_JSON: &str = r#"{"schema_version":"rwp.report.v2","frame_count":3,"frame_statistics":{"sample_count":3,"mean_frame_time_ms":16.7,"median_frame_time_ms":16.7,"average_fps":59.9,"variance_ms2":null,"stdev_ms":null,"percentiles":{"p0_1_ms":16.7,"p1_ms":16.7,"p5_ms":16.7,"p10_ms":16.7,"p50_ms":16.7,"p95_ms":16.7,"p99_ms":16.7,"p99_9_ms":16.7},"low_fps":{"percentile_1_low_fps":59.9,"percentile_0_1_low_fps":59.9,"mean_fps_of_worst_1pct_frames":59.9,"mean_fps_of_worst_0_1pct_frames":59.9},"p999_reliable":false},"stutter_events":[],"analysis_config":{"p999_reliability_threshold":1,"stutter_rule_id":"stutter.v1.absolute","stutter_refresh_rate_hz":null,"stutter_refresh_period_ms":null,"stutter_rolling_half_window_frames":15,"stutter_multiplier":2.0,"stutter_absolute_threshold_ms":30.0,"stutter_minimum_baseline_samples":7,"stutter_merge_gap_ms":50.0},"diagnostics":[],"interrupted":false}"#;

    #[test]
    fn history_entry_skips_unparseable_report() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("rwp-history-entry-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root)?;
        let manifest = test_manifest();

        let broken = root.join("broken.json");
        std::fs::write(&broken, b"{not a report")?;
        assert!(super::history_entry_from_verified_report("broken", &manifest, &broken).is_none());

        let valid = root.join("valid.json");
        std::fs::write(&valid, TEST_REPORT_JSON)?;
        let entry = super::history_entry_from_verified_report("valid", &manifest, &valid)
            .ok_or("valid report must produce a history entry")?;
        assert_eq!(entry.session_id, "valid");
        assert_eq!(entry.application, "Game.exe");
        assert_eq!(entry.process_id, 42);
        assert_eq!(entry.frame_count, 3);
        assert!(entry.verified);

        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    #[test]
    fn list_history_skips_invalid_entries() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!("rwp-history-list-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert!(super::list_history(&root)?.is_empty());

        std::fs::create_dir_all(root.join("broken"))?;
        std::fs::write(root.join("broken").join("manifest.json"), b"{invalid")?;
        std::fs::write(root.join("stray.txt"), b"not a session")?;
        assert!(super::list_history(&root)?.is_empty());

        std::fs::remove_dir_all(&root)?;
        Ok(())
    }

    fn quoted_items_after(source: &str, marker: &str) -> Vec<String> {
        let Some((_, rest)) = source.split_once(marker) else {
            return Vec::new();
        };
        let Some((items, _)) = rest.split_once(']') else {
            return Vec::new();
        };
        items
            .lines()
            .filter_map(|line| {
                let item = line.trim().trim_end_matches(',');
                if item.len() >= 2 && item.starts_with('"') && item.ends_with('"') {
                    Some(item[1..item.len() - 1].to_owned())
                } else {
                    None
                }
            })
            .collect()
    }

    fn registered_app_commands(source: &str) -> Vec<String> {
        let Some((_, rest)) = source.split_once("tauri::generate_handler![") else {
            return Vec::new();
        };
        let Some((handler, _)) = rest.split_once("])") else {
            return Vec::new();
        };
        handler
            .lines()
            .filter_map(|line| {
                line.split_once("commands::")
                    .map(|(_, name)| name.trim().trim_end_matches(',').to_owned())
            })
            .collect()
    }

    fn sorted(mut commands: Vec<String>) -> Vec<String> {
        commands.sort();
        commands
    }

    #[test]
    fn app_manifest_and_capability_allow_only_registered_application_commands() {
        let registered = sorted(registered_app_commands(include_str!("main.rs")));
        let manifest = sorted(quoted_items_after(
            include_str!("../build.rs"),
            "const APPLICATION_COMMANDS: &[&str] = &[",
        ));
        let capability = sorted(quoted_items_after(
            include_str!("../permissions/default.toml"),
            "commands.allow = [",
        ));

        assert_eq!(
            manifest, registered,
            "build.rs AppManifest command set must match invoke_handler"
        );
        assert_eq!(
            capability, registered,
            "main capability permission set must match invoke_handler"
        );
    }

    #[test]
    fn bounded_session_reader_rejects_oversized_files() -> Result<(), Box<dyn std::error::Error>> {
        let path = std::env::temp_dir().join("rwp-tauri-bounded-session-test.json");
        std::fs::write(&path, b"12345")?;
        assert!(super::read_bounded_file(&path, 4).is_err());
        std::fs::remove_file(path)?;
        Ok(())
    }

    #[test]
    fn worker_panic_transitions_session_to_failed() {
        let manager = SessionManager::default();
        run_session_worker(
            manager.clone(),
            || -> Result<_, rwp_session::SessionError> {
                panic!("synthetic worker panic");
            },
        );
        assert!(matches!(
            manager.snapshot().ok(),
            Some(SessionSnapshot::Failed { message })
                if message == "capture session worker panicked"
        ));
    }

    #[test]
    fn worker_spawn_failure_transitions_running_session_to_failed() -> Result<(), String> {
        let manager = SessionManager::default();
        {
            let mut state = manager
                .state
                .lock()
                .map_err(|_| "session state mutex was poisoned".to_owned())?;
            state.snapshot = SessionSnapshot::Running { process_id: 42 };
            state.cancellation = Some(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
                false,
            )));
        }

        let snapshot =
            fail_session_worker_spawn(&manager, std::io::Error::other("thread quota reached"))?;

        assert!(matches!(
            snapshot,
            SessionSnapshot::Failed { message }
                if message == "failed to start capture session worker: thread quota reached"
        ));
        assert!(
            manager
                .state
                .lock()
                .map_err(|_| "session state mutex was poisoned".to_owned())?
                .cancellation
                .is_none()
        );
        Ok(())
    }
}
