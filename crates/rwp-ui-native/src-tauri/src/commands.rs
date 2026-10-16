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
use tauri::{AppHandle, Emitter, Manager, Runtime, State};
use tauri_plugin_dialog::{DialogExt, FilePath};

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
    /// Disable the instant-stability toast notification. Notifications are on
    /// by default; the in-app banner always renders regardless of this flag.
    #[serde(default)]
    pub disable_stable_notification: bool,
    /// Language used by native notifications; unknown values fall back to English.
    #[serde(default = "default_notification_language")]
    pub notification_language: String,
}

fn default_notification_language() -> String {
    String::from("en")
}

/// Inputs for starting a session with an application-managed output path.
#[derive(Debug, Deserialize)]
pub struct ManagedSessionStartRequest {
    /// Absolute path to the user-supplied PresentMon executable.
    pub presentmon_executable: String,
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
    /// Disable the post-completion Stable notification.
    #[serde(default)]
    pub disable_stable_notification: bool,
    /// Native notification locale ("id" or "en").
    #[serde(default = "default_notification_language")]
    pub notification_language: String,
}

/// Permission/component/process readiness used by the Home preflight.
#[derive(Debug, Clone, Serialize)]
pub struct SessionPreflight {
    /// Whether Windows permits PresentMon real-time capture.
    pub permission_ready: bool,
    /// Whether the configured PresentMon path currently points to an executable file.
    pub component_ready: bool,
    /// Whether the selected process identifier exists in the current process snapshot.
    pub process_ready: bool,
}

/// Prevents unconfirmed native window-close requests during active measurement.
#[derive(Default)]
pub struct WindowCloseGuard {
    pub(crate) allow_close: std::sync::atomic::AtomicBool,
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
        /// Requested capture duration in seconds, for live progress.
        duration_seconds: u64,
        /// Unix timestamp in milliseconds when the worker started.
        started_unix_ms: u64,
    },
    /// A session completed and produced deterministic analysis.
    Completed {
        /// Target process identifier.
        process_id: u32,
        /// Whether the capture stopped before its natural duration completed.
        interrupted: bool,
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

/// Backend session lifecycle event emitted to the webview.
///
/// The existing 500 ms status poll remains the fallback; events only make
/// state transitions prompt. Payloads stay small and contain no telemetry.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case", tag = "event")]
pub enum SessionEvent {
    /// A capture worker started.
    Started {
        /// Target process identifier.
        process_id: u32,
        /// Requested capture duration in seconds.
        duration_seconds: u64,
    },
    /// A capture worker finished and persisted analysis.
    Completed {
        /// Target process identifier.
        process_id: u32,
        /// Number of normalized frame observations.
        frame_count: usize,
    },
    /// A capture worker failed before producing a complete result.
    Failed {
        /// Human-readable failure cause.
        message: String,
    },
}

fn emit_session_event<R: Runtime>(app: Option<&AppHandle<R>>, event: &SessionEvent) {
    if let Some(app) = app {
        let _ = app.emit("rwp-session-event", event);
    }
}

/// Returns generic native notification copy; never includes process identity.
fn completion_notification_content(language: &str) -> (&'static str, &'static str) {
    if language.eq_ignore_ascii_case("id") {
        ("Pengukuran selesai", "Hasil Anda siap dilihat.")
    } else {
        ("Measurement complete", "Your results are ready to view.")
    }
}

/// Best-effort native delivery; Windows host policy may suppress notifications.
fn show_native_notification<R: Runtime>(
    app: &Option<&AppHandle<R>>,
    title: &str,
    body: &str,
) {
    use tauri_plugin_notification::NotificationExt;
    let Some(app) = app else { return };
    let _notified = app
        .notification()
        .builder()
        .title(title.to_owned())
        .body(body.to_owned())
        .show()
        .is_ok();
}

fn now_unix_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u128::from(u64::MAX)) as u64)
        .unwrap_or(0)
}

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
        interrupted: manifest.interrupted,
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
        let interrupted = result.stop_outcome.is_some();
        state.snapshot = SessionSnapshot::Completed {
            process_id: result.process_id,
            interrupted,
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

fn run_session_worker<F, R>(
    manager: SessionManager,
    app: Option<&AppHandle<R>>,
    operation: F,
    notify_on_stable: bool,
    notification_language: &str,
) where
    F: FnOnce() -> Result<SessionResult, rwp_session::SessionError>,
    R: Runtime,
{
    match panic::catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(result)) => {
            let event = SessionEvent::Completed {
                process_id: result.process_id,
                frame_count: result.report.frame_count,
            };
            let stable_notification = if notify_on_stable {
                rwp_ui::stable_notification_content_for_language(
                    &result.diagnostics,
                    notification_language,
                )
            } else {
                None
            };
            if manager.finish(result).is_ok() {
                emit_session_event(app, &event);
                let (title, body) = completion_notification_content(notification_language);
                show_native_notification(&app, title, body);
                if let Some((title, body)) = stable_notification {
                    show_native_notification(&app, &title, &body);
                }
                tracing::info!(target: "rwp", event = "session_completed");
            }
        }
        Ok(Err(error)) => {
            let message = error.to_string();
            emit_session_event(
                app,
                &SessionEvent::Failed {
                    message: message.clone(),
                },
            );
            tracing::warn!(target: "rwp", event = "session_failed");
            let _ = manager.fail(message);
        }
        Err(_) => {
            emit_session_event(
                app,
                &SessionEvent::Failed {
                    message: String::from("capture session worker panicked"),
                },
            );
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

/// Redacted live-process identity returned to the target picker: PID plus the
/// executable name only, matching the sampler's existing observation boundary.
#[derive(Debug, Clone, Serialize)]
pub struct ProcessEntry {
    /// Live process identifier.
    pub process_id: u32,
    /// Executable basename reported by the snapshot.
    pub name: String,
}

/// Lists currently present processes for GUI target selection.
///
/// Async so the snapshot never blocks the webview main thread. Only identity
/// is returned; utilization stays inside the sampler.
#[tauri::command(async)]
pub fn session_processes() -> Result<Vec<ProcessEntry>, String> {
    rwp_system::list_processes()
        .map(|processes| {
            processes
                .into_iter()
                .map(|entry| ProcessEntry {
                    process_id: entry.process_id,
                    name: entry.name,
                })
                .collect()
        })
        .map_err(|error| error.to_string())
}

fn dialog_selection(selection: Option<FilePath>) -> Option<String> {
    selection.map(|path| match path {
        FilePath::Path(path) => path.to_string_lossy().into_owned(),
        FilePath::Url(url) => url.to_string(),
    })
}

/// Opens a native file picker for the PresentMon executable.
///
/// Returns None when the operator dismisses the dialog. Dialogs cannot run
/// headless, so this thin wrapper stays untested by unit tests; picked paths
/// still flow through the regular session configuration validation.
#[tauri::command]
pub fn session_pick_presentmon_file(app: AppHandle) -> Result<Option<String>, String> {
    Ok(dialog_selection(
        app.dialog()
            .file()
            .add_filter("PresentMon executable", &["exe"])
            .blocking_pick_file(),
    ))
}

/// Opens a native save picker for the PresentMon CSV output file.
///
/// Returns None when the operator dismisses the dialog; see
/// `session_pick_presentmon_file` for the headless-testing note.
#[tauri::command]
pub fn session_pick_output_file(app: AppHandle) -> Result<Option<String>, String> {
    Ok(dialog_selection(
        app.dialog()
            .file()
            .add_filter("PresentMon CSV", &["csv"])
            .blocking_save_file(),
    ))
}

/// Starts a capture while RWP manages the temporary CSV/output location.
#[tauri::command]
pub fn session_start_managed(
    app: AppHandle,
    request: ManagedSessionStartRequest,
    manager: State<'_, SessionManager>,
) -> Result<SessionSnapshot, String> {
    let root = history_root()?;
    let pending_root = root.join(".capture");
    fs::create_dir_all(&pending_root).map_err(|error| {
        format!("managed capture output directory could not be prepared: {error}")
    })?;
    let base = format!("session-{}", now_unix_ms());
    let session_id = unique_session_id(&root, &base);
    let output_file = pending_root.join(format!("{session_id}.csv"));
    start_session_inner(
        app,
        SessionStartRequest {
            presentmon_executable: request.presentmon_executable,
            output_file: output_file.to_string_lossy().into_owned(),
            process_id: request.process_id,
            duration_seconds: request.duration_seconds,
            sample_interval_ms: request.sample_interval_ms,
            exclude_dropped: request.exclude_dropped,
            stop_existing_session: request.stop_existing_session,
            no_process_probing: request.no_process_probing,
            disable_stable_notification: request.disable_stable_notification,
            notification_language: request.notification_language,
        },
        manager,
        true,
    )
}

/// Performs best-effort preflight without exposing executable paths to the UI.
#[tauri::command]
pub fn session_preflight(
    presentmon_executable: Option<String>,
    process_id: Option<u32>,
) -> SessionPreflight {
    let permission_ready = capture_permission_ready();
    let component_ready = presentmon_executable.as_deref().is_some_and(|value| {
        let path = Path::new(value);
        path.is_absolute()
            && path.file_name().is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("PresentMon.exe"))
            && path.metadata().is_ok_and(|metadata| metadata.is_file())
            && path.canonicalize().is_ok()
    });
    let process_ready = process_id.is_some_and(|target| {
        target > 0
            && rwp_system::list_processes()
                .is_ok_and(|entries| entries.iter().any(|entry| entry.process_id == target))
    });
    SessionPreflight { permission_ready, component_ready, process_ready }
}

#[cfg(windows)]
fn capture_permission_ready() -> bool {
    rwp_system::preflight_presentmon_permissions().is_ok()
}

#[cfg(not(windows))]
fn capture_permission_ready() -> bool {
    false
}

/// Returns a stable UI error kind for the existing set of known English errors.
#[tauri::command]
pub fn session_error_kind(message: String) -> String {
    error_kind_for_message(&message).to_owned()
}

fn error_kind_for_message(message: &str) -> &'static str {
    let marker = [
        ("Performance Log Users", "permission"),
        ("permission", "permission"),
        ("between 10 ms and 5 s", "interval"),
        ("duration", "duration"),
        ("greater than zero", "target"),
        ("already running", "session_active"),
        ("worker", "worker"),
        ("unsupported PresentMon version", "version"),
        ("PresentMon.exe", "presentmon_name"),
    ];
    marker.iter()
        .find_map(|(needle, kind)| message.contains(needle).then_some(*kind))
        .unwrap_or("unknown")
}

/// Allows the main window to close only after capture has reached a terminal state.
#[tauri::command]
pub fn session_close_confirmed(
    app: AppHandle,
    manager: State<'_, SessionManager>,
    close_guard: State<'_, WindowCloseGuard>,
) -> Result<(), String> {
    {
        let state = manager.lock()?;
        if matches!(state.snapshot, SessionSnapshot::Running { .. }) {
            return Err("capture session is still running".to_owned());
        }
    }
    close_guard.allow_close.store(true, Ordering::Release);
    let window = app.get_webview_window("main")
        .ok_or_else(|| "main window is unavailable".to_owned())?;
    if let Err(error) = window.close() {
        close_guard.allow_close.store(false, Ordering::Release);
        return Err(error.to_string());
    }
    Ok(())
}

/// Maps a session failure message to actionable remediation guidance.
///
/// Pure substring rules over backend error text so the GUI and future callers
/// share one remediation source. Returns None when no rule matches.
pub fn remediation_for(message: &str) -> Option<&'static str> {
    let rules: &[(&str, &str)] = &[
        (
            "PresentMon.exe",
            "Use the official PresentMon executable; the file name must be exactly PresentMon.exe.",
        ),
        (
            "Performance Log Users",
            "Add your account to the Performance Log Users group or run elevated, then retry.",
        ),
        (
            "permission",
            "Run RWP elevated or add your account to the Performance Log Users group, then retry.",
        ),
        (
            "between 10 ms and 5 s",
            "Set the telemetry interval between 10 ms and 5000 ms.",
        ),
        (
            "duration",
            "Set the capture duration between 1 and 3600 seconds.",
        ),
        (
            "greater than zero",
            "Pick a live target process from the process list; PID 0 is never valid.",
        ),
        (
            "already running",
            "Stop or reset the active session before starting a new capture.",
        ),
        (
            "worker",
            "The capture worker failed to start; restart the application and retry.",
        ),
        (
            "unsupported PresentMon version",
            "Use PresentMon 2.x; a version-unknown binary (no version resource) is accepted with its SHA-256 recorded, but a detected non-2.x version is refused.",
        ),
    ];
    rules
        .iter()
        .find_map(|(marker, remediation)| message.contains(marker).then_some(*remediation))
}

/// Returns remediation guidance for one session failure message, if known.
#[tauri::command]
pub fn session_error_remediation(message: String) -> Option<String> {
    remediation_for(&message).map(str::to_owned)
}

/// Starts one live capture session in a background worker.
#[tauri::command]
pub fn session_start(
    app: AppHandle,
    request: SessionStartRequest,
    manager: State<'_, SessionManager>,
) -> Result<SessionSnapshot, String> {
    start_session_inner(app, request, manager, false)
}

fn start_session_inner(
    app: AppHandle,
    request: SessionStartRequest,
    manager: State<'_, SessionManager>,
    cleanup_output_on_finish: bool,
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
    let duration_seconds = request.duration_seconds;
    let process_id = config.process_id;
    state.snapshot = SessionSnapshot::Running {
        process_id,
        duration_seconds,
        started_unix_ms: now_unix_ms(),
    };
    state.cancellation = Some(cancellation.clone());
    drop(state);
    tracing::info!(target: "rwp", event = "session_start_requested");

    let state_handle = manager.inner().state.clone();
    let event_app = app.clone();
    let cleanup_output = cleanup_output_on_finish.then(|| config.output_file.clone());
    match std::thread::Builder::new()
        .name("rwp-session-worker".to_owned())
        .spawn(move || {
            tracing::info!(target: "rwp", event = "session_worker_started");
            let manager = SessionManager {
                state: state_handle,
            };
            run_session_worker(
                manager,
                Some(&event_app),
                || {
                    let runner = SessionRunner;
                    let result = runner.run(&config, &cancellation);
                    if let Some(path) = cleanup_output {
                        if let Err(error) = fs::remove_file(&path) {
                            if error.kind() != std::io::ErrorKind::NotFound {
                                tracing::warn!(target: "rwp", event = "managed_output_cleanup_failed");
                            }
                        }
                    }
                    result
                },
                !request.disable_stable_notification,
                &request.notification_language,
            );
        }) {
        Ok(_worker) => {
            emit_session_event(
                Some(&app),
                &SessionEvent::Started {
                    process_id,
                    duration_seconds,
                },
            );
            manager.snapshot()
        }
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
    fn default_notification_language_is_backward_compatible() -> Result<(), Box<dyn std::error::Error>> {
        let request: super::SessionStartRequest = serde_json::from_str(
            r#"{"presentmon_executable":"PresentMon.exe","output_file":"capture.csv","process_id":42,"duration_seconds":10}"#,
        )?;
        assert_eq!(request.notification_language, "en");
        Ok(())
    }

    #[test]
    fn completion_notification_is_static_and_localized() {
        assert_eq!(
            super::completion_notification_content("id"),
            ("Pengukuran selesai", "Hasil Anda siap dilihat.")
        );
        assert_eq!(
            super::completion_notification_content("en"),
            ("Measurement complete", "Your results are ready to view.")
        );
    }

    #[test]
    fn error_kinds_cover_known_cases_and_unknown_fallback() {
        assert_eq!(super::error_kind_for_message("invalid PresentMon.exe path"), "presentmon_name");
        assert_eq!(super::error_kind_for_message("permission denied"), "permission");
        assert_eq!(super::error_kind_for_message("an unrecognized failure"), "unknown");
    }

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

    fn granted_commands(permissions: &str, identifier: &str) -> Vec<String> {
        let Some(block) = permissions.split("[[permission]]").skip(1).find(|block| {
            block.lines().any(|line| {
                line.trim()
                    .strip_prefix("identifier = \"")
                    .and_then(|rest| rest.strip_suffix('"').map(|name| name == identifier))
                    == Some(true)
            })
        }) else {
            return Vec::new();
        };
        quoted_items_after(block, "commands.allow = [")
    }

    /// Pins the Tauri runtime ACL resolution rule
    /// (`acl::resolved::get_permissions` in the pinned tauri-utils): a
    /// capability entry literally named `default` resolves through the
    /// manifest's default permission *set*, never through a `[[permission]]
    /// identifier = "default"` entry, so such an entry grants nothing and
    /// every command is denied at runtime with "not allowed by ACL".
    #[test]
    fn capability_grants_resolve_to_registered_commands() -> Result<(), Box<dyn std::error::Error>>
    {
        let capability: serde_json::Value =
            serde_json::from_str(include_str!("../capabilities/default.json"))?;
        let entries: Vec<String> = capability
            .get("permissions")
            .and_then(|value| value.as_array())
            .ok_or("capability must list permissions")?
            .iter()
            .filter_map(|value| value.as_str().map(str::to_owned))
            .collect();

        let permissions = include_str!("../permissions/default.toml");
        let has_default_set = permissions.lines().any(|line| line.trim() == "[default]");
        let permission_identifiers: Vec<String> = permissions
            .split("[[permission]]")
            .skip(1)
            .filter_map(|block| {
                block.lines().find_map(|line| {
                    line.trim()
                        .strip_prefix("identifier = \"")
                        .and_then(|rest| rest.strip_suffix('"'))
                        .map(str::to_owned)
                })
            })
            .collect();

        for entry in &entries {
            if entry.contains(':') {
                continue; // plugin-scoped entries resolve in their own manifest
            }
            if entry == "default" {
                assert!(
                    has_default_set,
                    "capability entry \"default\" resolves through the default permission set, which permissions/default.toml does not define"
                );
            } else {
                assert!(
                    permission_identifiers.iter().any(|id| id == entry),
                    "capability entry {entry:?} has no matching [[permission]] identifier"
                );
            }
        }

        // Every command the frontend invokes must be granted by one of the
        // resolved permissions; otherwise it fails with "not allowed by ACL".
        let frontend = include_str!("../../web/app.js");
        let mut invoked = Vec::new();
        let mut rest = frontend;
        while let Some((_, tail)) = rest.split_once("invoke(\"") {
            if let Some((name, tail)) = tail.split_once('"') {
                invoked.push(name.to_owned());
                rest = tail;
            } else {
                break;
            }
        }
        assert!(
            !invoked.is_empty(),
            "frontend invoke scan must find commands"
        );
        let granted: Vec<String> = permission_identifiers
            .iter()
            .flat_map(|id| granted_commands(permissions, id))
            .collect();
        for command in invoked {
            assert!(
                granted.iter().any(|grant| grant == &command),
                "frontend invokes {command:?} but no resolved permission grants it"
            );
        }
        Ok(())
    }

    #[test]
    fn stable_notification_defaults_on_and_deserializes() -> Result<(), Box<dyn std::error::Error>>
    {
        // Notifications are ON unless explicitly disabled; the in-app banner
        // renders regardless. Missing-field tolerance keeps older webviews
        // working against this backend.
        let request: super::SessionStartRequest = serde_json::from_str(
            r#"{"presentmon_executable":"a","output_file":"b","process_id":1,"duration_seconds":60}"#,
        )?;
        assert!(!request.disable_stable_notification);
        let request: super::SessionStartRequest = serde_json::from_str(
            r#"{"presentmon_executable":"a","output_file":"b","process_id":1,"duration_seconds":60,"disable_stable_notification":true}"#,
        )?;
        assert!(request.disable_stable_notification);
        Ok(())
    }

    #[test]
    fn frontend_grants_no_notification_permission() {
        // The toast is sent from Rust through the plugin backend; the webview
        // must never gain notification IPC. Any future grant fails closed here
        // and needs a threat-model re-review first.
        for file in [
            include_str!("../capabilities/default.json"),
            include_str!("../permissions/default.toml"),
        ] {
            assert!(
                !file.contains("notification"),
                "notification IPC must stay ungranted to the webview"
            );
        }
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
            None::<&tauri::AppHandle<tauri::Wry>>,
            || -> Result<_, rwp_session::SessionError> {
                panic!("synthetic worker panic");
            },
            false,
            "en",
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
            state.snapshot = SessionSnapshot::Running {
                process_id: 42,
                duration_seconds: 60,
                started_unix_ms: 0,
            };
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

    #[test]
    fn process_picker_lists_sorted_live_identities() -> Result<(), String> {
        let entries = super::session_processes()?;
        assert!(!entries.is_empty());
        assert!(entries.iter().all(|entry| entry.process_id > 0));
        assert!(entries.iter().all(|entry| !entry.name.is_empty()));
        assert!(
            entries
                .windows(2)
                .all(|pair| pair[0].process_id < pair[1].process_id)
        );
        Ok(())
    }

    #[test]
    fn remediation_rules_cover_known_failures() {
        let cases = [
            (
                "presentmon path must be named PresentMon.exe",
                "exactly PresentMon.exe",
            ),
            (
                "missing permission: join the Performance Log Users group",
                "Performance Log Users",
            ),
            (
                "sample_interval must be between 10 ms and 5 s",
                "between 10 ms and 5000 ms",
            ),
            (
                "duration_seconds must be greater than zero",
                "between 1 and 3600 seconds",
            ),
            (
                "process_id must be greater than zero",
                "PID 0 is never valid",
            ),
            (
                "unsupported PresentMon version 1.9.0; supported major version is 2.x",
                "PresentMon 2.x",
            ),
            ("a capture session is already running", "Stop or reset"),
            (
                "failed to start capture session worker: thread quota reached",
                "restart the application",
            ),
        ];
        for (message, fragment) in cases {
            let remediation = super::remediation_for(message)
                .unwrap_or_else(|| panic!("no remediation for: {message}"));
            assert!(
                remediation.contains(fragment),
                "remediation for {message:?} misses {fragment:?}"
            );
        }
        assert!(super::remediation_for("some unknown failure").is_none());
        assert_eq!(
            super::session_error_remediation(String::from("a capture session is already running")),
            Some(String::from(
                "Stop or reset the active session before starting a new capture."
            ))
        );
    }

    #[test]
    fn session_events_serialize_with_stable_tags() -> Result<(), Box<dyn std::error::Error>> {
        let started = serde_json::to_value(super::SessionEvent::Started {
            process_id: 42,
            duration_seconds: 60,
        })?;
        assert_eq!(started["event"], "started");
        assert_eq!(started["process_id"], 42);
        let running = serde_json::to_value(super::SessionSnapshot::Running {
            process_id: 42,
            duration_seconds: 60,
            started_unix_ms: 1_700_000_000_000,
        })?;
        assert_eq!(running["status"], "running");
        assert_eq!(running["duration_seconds"], 60);
        assert_eq!(running["started_unix_ms"], 1_700_000_000_000_u64);
        Ok(())
    }

    #[test]
    fn session_compare_rejects_unknown_sessions() {
        assert!(
            super::session_compare(
                String::from("no-such-baseline"),
                String::from("no-such-candidate"),
                true,
            )
            .is_err()
        );
    }
}
