//! Redacted, bounded, rolling JSON logs for the native desktop application.

use std::{
    ffi::OsStr,
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};

use tracing::{
    Event, Subscriber,
    field::{Field, Visit},
};
use tracing_subscriber::{
    filter::{LevelFilter, Targets},
    layer::{Context, Layer, SubscriberExt},
    util::SubscriberInitExt,
};

const MAX_LOG_FILE_BYTES: u64 = 5 * 1024 * 1024;
const MAX_LOG_ARCHIVES: usize = 3;
const MAX_LOG_RECORD_BYTES: usize = 16 * 1024;
const SAFE_EVENTS: &[&str] = &[
    "logging_initialized",
    "application_started",
    "session_start_requested",
    "session_worker_started",
    "session_stop_requested",
    "session_completed",
    "session_failed",
];

/// Configures redacted JSONL logging in the user's local application data directory.
///
/// Logging setup is best-effort: failure to resolve the directory or open a log file
/// never prevents the application from starting. The layer emits only allowlisted event
/// identifiers, a fixed target, level, and timestamp. It deliberately discards message
/// fields and every other caller-supplied field so paths, PIDs, process names, usernames,
/// and raw operating-system errors cannot leak through arbitrary tracing events.
pub fn initialize() {
    let root = match std::env::var_os("LOCALAPPDATA") {
        Some(root) => root,
        None => return,
    };
    let path = log_path_for_root(&root);
    let sink = match RollingFile::open(path, MAX_LOG_FILE_BYTES, MAX_LOG_ARCHIVES) {
        Ok(sink) => Arc::new(Mutex::new(sink)),
        Err(_) => return,
    };
    let layer = RedactedJsonLayer { sink }
        .with_filter(Targets::new().with_target("rwp", configured_level()));
    if tracing_subscriber::registry()
        .with(layer)
        .try_init()
        .is_ok()
    {
        tracing::info!(target: "rwp", event = "logging_initialized");
    }
}

fn log_path_for_root(local_app_data: &OsStr) -> PathBuf {
    PathBuf::from(local_app_data)
        .join("RidTheWann")
        .join("Performance")
        .join("logs")
        .join("rwp.jsonl")
}

fn configured_level() -> LevelFilter {
    let value = std::env::var("RWP_LOG_LEVEL").ok();
    parse_level_filter(value.as_deref())
}

fn parse_level_filter(value: Option<&str>) -> LevelFilter {
    let normalized = match value {
        Some(value) => value.to_ascii_lowercase(),
        None => return LevelFilter::INFO,
    };
    match normalized.as_str() {
        "trace" => LevelFilter::TRACE,
        "debug" => LevelFilter::DEBUG,
        "info" => LevelFilter::INFO,
        "warn" | "warning" => LevelFilter::WARN,
        "error" => LevelFilter::ERROR,
        "off" => LevelFilter::OFF,
        _ => LevelFilter::INFO,
    }
}

fn safe_event(value: &str) -> Option<&'static str> {
    SAFE_EVENTS
        .iter()
        .copied()
        .find(|allowed| *allowed == value)
}

struct SafeEventVisitor {
    event: Option<&'static str>,
}

impl Visit for SafeEventVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "event" && self.event.is_none() {
            self.event = safe_event(value);
        }
    }

    fn record_debug(&mut self, _field: &Field, _value: &dyn std::fmt::Debug) {
        // Discard arbitrary caller-provided debug values, including message fields.
    }
}

struct RedactedJsonLayer {
    sink: Arc<Mutex<RollingFile>>,
}

impl<S> Layer<S> for RedactedJsonLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _context: Context<'_, S>) {
        let metadata = event.metadata();
        if metadata.target() != "rwp" {
            return;
        }
        let mut visitor = SafeEventVisitor { event: None };
        event.record(&mut visitor);
        let Some(event_name) = visitor.event else {
            return;
        };
        let elapsed = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(elapsed) => elapsed,
            Err(_) => return,
        };
        let timestamp_unix_ms = elapsed
            .as_secs()
            .saturating_mul(1_000)
            .saturating_add(u64::from(elapsed.subsec_millis()));
        let record = serde_json::json!({
            "event": event_name,
            "level": metadata.level().as_str(),
            "target": "rwp",
            "timestamp_unix_ms": timestamp_unix_ms,
        })
        .to_string();
        let mut sink = match self.sink.lock() {
            Ok(sink) => sink,
            Err(poisoned) => poisoned.into_inner(),
        };
        let _ = sink.write_record(record.as_bytes());
    }
}

struct RollingFile {
    path: PathBuf,
    file: Option<File>,
    current_size: u64,
    max_file_bytes: u64,
    max_archives: usize,
}

impl RollingFile {
    fn open(path: PathBuf, max_file_bytes: u64, max_archives: usize) -> io::Result<Self> {
        if max_file_bytes == 0 || max_archives == 0 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "rolling log limits must be greater than zero",
            ));
        }
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)?;
        }
        let file = OpenOptions::new().create(true).append(true).open(&path)?;
        let existing_size = file.metadata()?.len();
        let (file, current_size) = if existing_size > max_file_bytes {
            drop(file);
            remove_if_exists(&path)?;
            (OpenOptions::new().create(true).append(true).open(&path)?, 0)
        } else {
            (file, existing_size)
        };
        let mut writer = Self {
            path,
            file: Some(file),
            current_size,
            max_file_bytes,
            max_archives,
        };
        writer.remove_oversized_archives()?;
        if current_size == max_file_bytes {
            writer.rotate()?;
        }
        Ok(writer)
    }

    fn write_record(&mut self, record: &[u8]) -> io::Result<()> {
        let needs_newline = !record.ends_with(b"\n");
        let newline_bytes = if needs_newline { 1_usize } else { 0_usize };
        let additional_bytes = record.len().saturating_add(newline_bytes);
        if additional_bytes > MAX_LOG_RECORD_BYTES || additional_bytes as u64 > self.max_file_bytes
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "structured log event exceeds the configured limit",
            ));
        }
        if self.current_size > 0
            && self.current_size.saturating_add(additional_bytes as u64) > self.max_file_bytes
        {
            self.rotate()?;
        }
        let write_result = match self.file.as_mut() {
            Some(file) => match file.write_all(record) {
                Ok(()) if needs_newline => file.write_all(b"\n"),
                Ok(()) => Ok(()),
                Err(error) => Err(error),
            },
            None => Err(io::Error::other("rolling log file is unavailable")),
        };
        if let Err(error) = write_result {
            // Stop subsequent append attempts after a partial write so tracked size cannot drift.
            drop(self.file.take());
            return Err(error);
        }
        self.current_size = self.current_size.saturating_add(additional_bytes as u64);
        Ok(())
    }

    fn remove_oversized_archives(&self) -> io::Result<()> {
        for index in 1..=self.max_archives {
            let archive = self.archive_path(index);
            match fs::metadata(&archive) {
                Ok(metadata) if metadata.len() > self.max_file_bytes => remove_if_exists(&archive)?,
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error),
            }
        }
        Ok(())
    }

    fn rotate(&mut self) -> io::Result<()> {
        if let Some(mut file) = self.file.take() {
            file.flush()?;
            drop(file);
        }
        remove_if_exists(&self.archive_path(self.max_archives))?;
        for index in (1..self.max_archives).rev() {
            rename_if_exists(&self.archive_path(index), &self.archive_path(index + 1))?;
        }
        rename_if_exists(&self.path, &self.archive_path(1))?;
        self.file = Some(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(&self.path)?,
        );
        self.current_size = 0;
        Ok(())
    }

    fn archive_path(&self, index: usize) -> PathBuf {
        let mut path = self.path.as_os_str().to_os_string();
        path.push(format!(".{index}"));
        PathBuf::from(path)
    }
}

fn remove_if_exists(path: &Path) -> io::Result<()> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn rename_if_exists(source: &Path, destination: &Path) -> io::Result<()> {
    match fs::rename(source, destination) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_LOG_FILE_BYTES, MAX_LOG_RECORD_BYTES, RollingFile, SAFE_EVENTS, log_path_for_root,
        parse_level_filter, safe_event,
    };
    use std::{
        ffi::OsStr,
        fs, io,
        path::PathBuf,
        sync::{
            Arc, Mutex,
            atomic::{AtomicUsize, Ordering},
        },
    };
    use tracing_subscriber::filter::LevelFilter;
    use tracing_subscriber::prelude::*;

    static TEMP_DIR_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

    fn temp_dir() -> PathBuf {
        let sequence = TEMP_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rwp-native-logging-test-{}-{sequence}",
            std::process::id()
        ))
    }

    #[test]
    fn rolling_log_keeps_bounded_history() -> io::Result<()> {
        let root = temp_dir();
        let path = root.join("rwp.jsonl");
        let mut writer = RollingFile::open(path.clone(), 40, 2)?;
        for record in [
            b"aaaaaaaaaaaaaaaaaaa".as_slice(),
            b"bbbbbbbbbbbbbbbbbbb".as_slice(),
            b"ccccccccccccccccccc".as_slice(),
            b"ddddddddddddddddddd".as_slice(),
            b"eeeeeeeeeeeeeeeeeee".as_slice(),
        ] {
            writer.write_record(record)?;
        }

        let current = fs::read(&path)?;
        let newest_archive = fs::read(writer.archive_path(1))?;
        let oldest_archive = fs::read(writer.archive_path(2))?;
        assert_eq!(current.iter().filter(|byte| **byte == b'\n').count(), 1);
        assert_eq!(
            newest_archive.iter().filter(|byte| **byte == b'\n').count(),
            2
        );
        assert_eq!(
            oldest_archive.iter().filter(|byte| **byte == b'\n').count(),
            2
        );
        assert!(!writer.archive_path(3).exists());
        assert_eq!(fs::metadata(&path)?.len(), 20);

        drop(writer);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn rejects_a_record_larger_than_the_file_limit() -> io::Result<()> {
        let root = temp_dir();
        let path = root.join("rwp.jsonl");
        let mut writer = RollingFile::open(path.clone(), 8, 1)?;
        assert!(writer.write_record(b"record-too-large").is_err());
        assert_eq!(fs::metadata(&path)?.len(), 0);

        drop(writer);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn rejects_a_record_over_the_event_byte_limit() -> io::Result<()> {
        let root = temp_dir();
        let path = root.join("rwp.jsonl");
        let mut writer = RollingFile::open(path.clone(), MAX_LOG_FILE_BYTES, 1)?;
        let record = vec![b'x'; MAX_LOG_RECORD_BYTES + 1];

        assert!(writer.write_record(&record).is_err());
        assert_eq!(fs::metadata(&path)?.len(), 0);

        drop(writer);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn oversized_existing_log_is_not_preserved_as_an_archive() -> io::Result<()> {
        let root = temp_dir();
        fs::create_dir_all(&root)?;
        let path = root.join("rwp.jsonl");
        let mut archive_name = path.as_os_str().to_os_string();
        archive_name.push(".1");
        let oversized_archive = PathBuf::from(archive_name);
        fs::write(&path, b"123456789")?;
        fs::write(&oversized_archive, b"abcdefghijk")?;

        let writer = RollingFile::open(path.clone(), 8, 2)?;
        assert_eq!(fs::metadata(&path)?.len(), 0);
        assert!(!writer.archive_path(1).exists());
        assert!(!oversized_archive.exists());

        drop(writer);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn log_path_is_scoped_to_local_app_data() {
        let local_app_data = OsStr::new(r"C:\Users\test\AppData\Local");
        let path = log_path_for_root(local_app_data);
        assert_eq!(path.file_name(), Some(OsStr::new("rwp.jsonl")));
        assert!(path.starts_with(PathBuf::from(local_app_data)));
        assert!(
            path.to_string_lossy()
                .contains(r"RidTheWann\Performance\logs")
        );
    }

    #[test]
    fn log_level_configuration_accepts_known_levels_and_defaults_safely() {
        assert_eq!(parse_level_filter(Some("TRACE")), LevelFilter::TRACE);
        assert_eq!(parse_level_filter(Some("debug")), LevelFilter::DEBUG);
        assert_eq!(parse_level_filter(Some("warning")), LevelFilter::WARN);
        assert_eq!(parse_level_filter(Some("off")), LevelFilter::OFF);
        assert_eq!(parse_level_filter(Some("unknown")), LevelFilter::INFO);
        assert_eq!(parse_level_filter(None), LevelFilter::INFO);
    }

    #[test]
    fn event_allowlist_discards_unknown_or_sensitive_values() {
        assert!(safe_event("session_start_requested").is_some());
        assert!(safe_event("C:\\Users\\private\\capture.csv").is_none());
        assert!(safe_event("session_start_requested C:\\Users\\private").is_none());
        assert!(safe_event("unknown").is_none());
        assert_eq!(SAFE_EVENTS.len(), 7);
    }

    #[test]
    fn subscriber_discards_arbitrary_messages_and_fields() -> io::Result<()> {
        let root = temp_dir();
        let path = root.join("rwp.jsonl");
        let sink = Arc::new(Mutex::new(RollingFile::open(
            path.clone(),
            MAX_LOG_FILE_BYTES,
            1,
        )?));
        let subscriber = tracing_subscriber::registry().with(super::RedactedJsonLayer { sink });

        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(
                target: "rwp",
                event = "application_started",
                message = "C:\\Users\\private\\capture.csv",
                process_id = 4242
            );
            tracing::info!(
                target: "rwp",
                event = "unknown_private_event",
                path = "C:\\Users\\private\\capture.csv"
            );
            tracing::info!(
                target: "private.target",
                event = "application_started",
                message = "must not be recorded"
            );
        });

        let output = fs::read_to_string(&path)?;
        assert_eq!(output.lines().count(), 1);
        assert!(output.contains("application_started"));
        assert!(!output.contains("C:\\Users\\private"));
        assert!(!output.contains("process_id"));
        assert!(!output.contains("4242"));
        let record: serde_json::Value = serde_json::from_str(output.trim())?;
        assert_eq!(record["target"], "rwp");
        assert_eq!(record["event"], "application_started");

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn record_limit_is_bounded() {
        assert_eq!(MAX_LOG_FILE_BYTES, 5 * 1024 * 1024);
        assert_eq!(MAX_LOG_RECORD_BYTES, 16 * 1024);
    }
}
