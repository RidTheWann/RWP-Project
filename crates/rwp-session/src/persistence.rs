//! Deterministic on-disk session persistence and provenance.

use std::{
    fs,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};

use rwp_core::SystemSample;
use rwp_report::{CaptureDocument, PerformanceReport};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::CoverageSummary;

/// Current persisted-session manifest schema.
pub const SESSION_MANIFEST_SCHEMA: &str = "rwp.session.manifest.v2";

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
const MAX_JSON_NESTING_DEPTH: usize = 64;
const MAX_CAPTURE_OR_REPORT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TELEMETRY_BYTES: u64 = 256 * 1024 * 1024;
const MAX_TELEMETRY_SAMPLES: usize = 5_000_000;
const MAX_PRESENTMON_CSV_BYTES: u64 = 512 * 1024 * 1024;
const MAX_TEMP_FILE_ATTEMPTS: u32 = 8;

static TEMP_FILE_SEQUENCE: AtomicU64 = AtomicU64::new(0);
const SESSION_DATA_FILES: [&str; 4] = [
    "capture.json",
    "report.json",
    "telemetry.jsonl",
    "presentmon.csv",
];

/// Files written for one persisted session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionManifestFile {
    /// Relative path below the session directory.
    pub path: String,
    /// Exact file length in bytes.
    pub size_bytes: u64,
    /// Lowercase SHA-256 digest of the file bytes.
    pub sha256: String,
}

/// Deterministic provenance manifest for a persisted session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionManifest {
    /// Build and machine provenance captured for this session.
    pub provenance: SessionProvenance,
    /// Manifest schema identifier.
    pub schema_version: String,
    /// Target process identifier.
    pub process_id: u32,
    /// Application name observed by PresentMon.
    pub application: String,
    /// Whether capture ended by explicit interruption.
    pub interrupted: bool,
    /// Active display refresh rate, when available.
    pub display_refresh_rate_hz: Option<f64>,
    /// Dominant swap-chain address, when available.
    pub dominant_swap_chain_address: Option<u64>,
    /// Number of discarded source frames.
    pub discarded_frame_count: usize,
    /// Persisted frame-type distribution.
    pub frame_type_counts: Vec<(String, usize)>,
    /// Non-fatal telemetry warnings observed during capture.
    pub telemetry_warning_count: usize,
    /// Symmetric frame/telemetry coverage.
    pub coverage: crate::CoverageSummary,
    /// Persisted files and their content hashes, in lexical path order.
    pub files: Vec<SessionManifestFile>,
}

/// Build, hardware, and PresentMon provenance for one persisted session.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SessionProvenance {
    /// RWP package version used to create the session.
    pub rwp_version: String,
    /// Git commit used to build the producer, when available.
    pub git_sha: Option<String>,
    /// Cargo build target triple, when available.
    pub build_target: Option<String>,
    /// Cargo build profile, when available.
    pub build_profile: Option<String>,
    /// Runtime operating-system identity.
    pub os: Option<String>,
    /// CPU identity, when available from the producer.
    pub cpu: Option<String>,
    /// Target GPU identity, when available from the producer.
    pub gpu: Option<String>,
    /// Target GPU driver identity, when available from the producer.
    pub driver: Option<String>,
    /// PresentMon executable version, when the executable was available to the producer.
    pub presentmon_version: Option<String>,
    /// SHA-256 of the PresentMon executable, when the executable was available to the producer.
    pub presentmon_sha256: Option<String>,
    /// QPC frequency used to normalize frame timestamps.
    pub qpc_frequency_hz: Option<u64>,
    /// Capture configuration that materially affects persisted observations.
    pub sample_interval_ms: Option<u64>,
    /// Whether PresentMon excluded dropped presents, when specified.
    pub exclude_dropped: Option<bool>,
    /// Whether PresentMon was allowed to stop an existing trace session, when specified.
    pub stop_existing_session: Option<bool>,
    /// Whether RWP process/thread telemetry probing was disabled, when specified.
    #[serde(default)]
    pub no_process_probing: Option<bool>,
}

/// Platform-neutral input for one persisted analyzed session.
#[derive(Debug, Clone, PartialEq)]
pub struct PersistedSessionInput<'a> {
    /// Target process identifier.
    pub process_id: u32,
    /// Application name.
    pub application: &'a str,
    /// Whether capture ended by explicit interruption.
    pub interrupted: bool,
    /// Active display refresh rate, when available.
    pub display_refresh_rate_hz: Option<f64>,
    /// Dominant swap-chain address, when available.
    pub dominant_swap_chain_address: Option<u64>,
    /// Number of discarded source frames.
    pub discarded_frame_count: usize,
    /// Frame-type distribution.
    pub frame_type_counts: &'a [(String, usize)],
    /// Number of non-fatal telemetry warnings.
    pub telemetry_warning_count: usize,
    /// Symmetric frame/telemetry coverage.
    pub coverage: &'a CoverageSummary,
    /// Normalized capture document.
    pub capture: &'a CaptureDocument,
    /// Deterministic performance report.
    pub report: &'a PerformanceReport,
    /// Normalized system telemetry.
    pub system_samples: &'a [SystemSample],
    /// Original PresentMon CSV path.
    pub source_csv: &'a Path,
    /// Build/hardware/PresentMon provenance for the persisted session.
    pub provenance: SessionProvenance,
}

/// Persistence errors raised while writing or verifying a session.
#[derive(Debug, thiserror::Error)]
pub enum PersistenceError {
    /// Session persistence filesystem or serialization failure.
    #[error("session persistence failed: {0}")]
    Message(String),
}

/// Persists one analyzed session beneath a caller-selected directory.
pub fn persist_session(
    session_dir: &Path,
    input: &PersistedSessionInput<'_>,
) -> Result<SessionManifest, PersistenceError> {
    if session_dir.as_os_str().is_empty() {
        return Err(PersistenceError::Message(
            "session directory must not be empty".to_string(),
        ));
    }
    fs::create_dir_all(session_dir).map_err(|error| io_error("create session directory", error))?;

    let capture_json = serde_json::to_vec(input.capture)
        .map_err(|error| PersistenceError::Message(format!("serialize capture: {error}")))?;
    let report_json = serde_json::to_vec(input.report)
        .map_err(|error| PersistenceError::Message(format!("serialize report: {error}")))?;
    let telemetry_jsonl = serialize_jsonl(input.system_samples)?;
    ensure_size(
        "capture.json",
        capture_json.len() as u64,
        MAX_CAPTURE_OR_REPORT_BYTES,
    )?;
    ensure_size(
        "report.json",
        report_json.len() as u64,
        MAX_CAPTURE_OR_REPORT_BYTES,
    )?;
    ensure_size(
        "telemetry.jsonl",
        telemetry_jsonl.len() as u64,
        MAX_TELEMETRY_BYTES,
    )?;
    let source_csv_bytes = read_bounded_file(
        input.source_csv,
        MAX_PRESENTMON_CSV_BYTES,
        "read PresentMon CSV",
    )?;

    write_atomic(&session_dir.join("capture.json"), &capture_json)?;
    write_atomic(&session_dir.join("report.json"), &report_json)?;
    write_atomic(&session_dir.join("telemetry.jsonl"), &telemetry_jsonl)?;
    write_atomic(&session_dir.join("presentmon.csv"), &source_csv_bytes)?;

    let mut files = Vec::new();
    for relative in [
        "capture.json",
        "report.json",
        "telemetry.jsonl",
        "presentmon.csv",
    ] {
        files.push(hash_file(&session_dir.join(relative), relative)?);
    }
    files.sort_by(|left, right| left.path.cmp(&right.path));

    let manifest = SessionManifest {
        schema_version: String::from(SESSION_MANIFEST_SCHEMA),
        process_id: input.process_id,
        application: input.application.to_owned(),
        interrupted: input.interrupted,
        display_refresh_rate_hz: input.display_refresh_rate_hz,
        dominant_swap_chain_address: input.dominant_swap_chain_address,
        discarded_frame_count: input.discarded_frame_count,
        frame_type_counts: input.frame_type_counts.to_vec(),
        telemetry_warning_count: input.telemetry_warning_count,
        coverage: input.coverage.clone(),
        provenance: input.provenance.clone(),
        files,
    };
    let manifest_json = serde_json::to_vec(&manifest)
        .map_err(|error| PersistenceError::Message(format!("serialize manifest: {error}")))?;
    ensure_size(
        "manifest.json",
        manifest_json.len() as u64,
        MAX_MANIFEST_BYTES,
    )?;
    write_atomic(&session_dir.join("manifest.json"), &manifest_json)?;
    Ok(manifest)
}

/// Verifies every persisted file against its manifest hash.
pub fn verify_manifest(session_dir: &Path) -> Result<SessionManifest, PersistenceError> {
    let input = read_bounded_file(
        &session_dir.join("manifest.json"),
        MAX_MANIFEST_BYTES,
        "read manifest",
    )?;
    ensure_json_depth(&input, MAX_JSON_NESTING_DEPTH)?;
    let manifest: SessionManifest = serde_json::from_slice(&input)
        .map_err(|error| PersistenceError::Message(format!("parse manifest: {error}")))?;
    if manifest.schema_version != SESSION_MANIFEST_SCHEMA {
        return Err(PersistenceError::Message(format!(
            "unsupported session manifest schema: {}",
            manifest.schema_version
        )));
    }
    if manifest.files.len() != SESSION_DATA_FILES.len() {
        return Err(PersistenceError::Message(
            "manifest must list each session data file exactly once".to_string(),
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for entry in &manifest.files {
        if !SESSION_DATA_FILES.contains(&entry.path.as_str()) || !seen.insert(entry.path.as_str()) {
            return Err(PersistenceError::Message(format!(
                "manifest contains an invalid or duplicate file path: {}",
                entry.path
            )));
        }
        let actual = hash_file(&session_dir.join(&entry.path), &entry.path)?;
        if actual != *entry {
            return Err(PersistenceError::Message(format!(
                "manifest hash mismatch for {}",
                entry.path
            )));
        }
    }
    Ok(manifest)
}

fn serialize_jsonl(samples: &[SystemSample]) -> Result<Vec<u8>, PersistenceError> {
    ensure_size(
        "telemetry sample count",
        samples.len() as u64,
        MAX_TELEMETRY_SAMPLES as u64,
    )?;
    let mut output = Vec::new();
    for sample in samples {
        let line = serde_json::to_string(sample)
            .map_err(|error| PersistenceError::Message(format!("serialize telemetry: {error}")))?;
        let next_len = output.len().saturating_add(line.len()).saturating_add(1);
        ensure_size("telemetry.jsonl", next_len as u64, MAX_TELEMETRY_BYTES)?;
        output.extend_from_slice(line.as_bytes());
        output.push(b'\n');
    }
    Ok(output)
}

fn ensure_json_depth(bytes: &[u8], limit: usize) -> Result<(), PersistenceError> {
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
                    return Err(PersistenceError::Message(format!(
                        "JSON nesting depth exceeds the configured limit ({depth} > {limit})"
                    )));
                }
            }
            b'}' | b']' => {
                depth = depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    Ok(())
}

fn ensure_size(label: &str, actual: u64, limit: u64) -> Result<(), PersistenceError> {
    if actual > limit {
        return Err(PersistenceError::Message(format!(
            "{label} exceeds the configured limit ({actual} > {limit} bytes)"
        )));
    }
    Ok(())
}

fn read_bounded_file(
    path: &Path,
    limit: u64,
    operation: &str,
) -> Result<Vec<u8>, PersistenceError> {
    let file = fs::File::open(path).map_err(|error| io_error(operation, error))?;
    let read_limit = limit.saturating_add(1);
    let mut bytes = Vec::new();
    file.take(read_limit)
        .read_to_end(&mut bytes)
        .map_err(|error| io_error(operation, error))?;
    ensure_size(operation, bytes.len() as u64, limit)?;
    Ok(bytes)
}

fn persisted_file_limit(relative: &str) -> Result<u64, PersistenceError> {
    match relative {
        "capture.json" | "report.json" => Ok(MAX_CAPTURE_OR_REPORT_BYTES),
        "telemetry.jsonl" => Ok(MAX_TELEMETRY_BYTES),
        "presentmon.csv" => Ok(MAX_PRESENTMON_CSV_BYTES),
        _ => Err(PersistenceError::Message(format!(
            "unsupported persisted file path: {relative}"
        ))),
    }
}

fn hash_file(path: &Path, relative: &str) -> Result<SessionManifestFile, PersistenceError> {
    let limit = persisted_file_limit(relative)?;
    let bytes = read_bounded_file(path, limit, "read persisted file")?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(SessionManifestFile {
        path: String::from(relative),
        size_bytes: bytes.len() as u64,
        sha256: format!("{:x}", hasher.finalize()),
    })
}

fn create_unique_temporary(path: &Path) -> Result<(fs::File, PathBuf), PersistenceError> {
    let file_name = path.file_name().ok_or_else(|| {
        PersistenceError::Message(format!(
            "persisted path has no file name: {}",
            path.display()
        ))
    })?;
    let parent = match path.parent() {
        Some(parent) if !parent.as_os_str().is_empty() => parent,
        _ => Path::new("."),
    };

    for _ in 0..MAX_TEMP_FILE_ATTEMPTS {
        let sequence = TEMP_FILE_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{}.{}.{}.tmp",
            file_name.to_string_lossy(),
            std::process::id(),
            sequence
        ));
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
        {
            Ok(file) => return Ok((file, temporary)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(io_error("create persistence temporary file", error)),
        }
    }

    Err(PersistenceError::Message(format!(
        "could not allocate a unique temporary file for {}",
        path.display()
    )))
}

fn refuse_overwrite(path: &Path) -> PersistenceError {
    PersistenceError::Message(format!(
        "refusing to overwrite persisted file: {}",
        path.display()
    ))
}

fn publish_without_overwrite(temporary: &Path, path: &Path) -> Result<(), PersistenceError> {
    match fs::hard_link(temporary, path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => Err(refuse_overwrite(path)),
        Err(error) => Err(io_error("commit persisted file", error)),
    }
}

fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), PersistenceError> {
    let (mut file, temporary) = create_unique_temporary(path)?;
    let write_result = file
        .write_all(bytes)
        .map_err(|error| io_error("write persistence temporary file", error))
        .and_then(|()| {
            file.sync_all()
                .map_err(|error| io_error("flush persistence temporary file", error))
        });
    drop(file);

    let result = write_result.and_then(|()| publish_without_overwrite(&temporary, path));
    let _ = fs::remove_file(&temporary);
    result
}

fn io_error(operation: &str, error: io::Error) -> PersistenceError {
    PersistenceError::Message(format!("{operation}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{PersistedSessionInput, SessionManifest, persist_session, verify_manifest};
    use crate::CoverageSummary;
    use rwp_analysis::{StatisticsConfig, StutterConfig, compute_statistics};
    use rwp_core::{FrameSample, SystemSample};
    use rwp_report::{AnalysisConfigSummary, CaptureDocument, PerformanceReport};
    use std::{
        fs,
        path::PathBuf,
        sync::atomic::{AtomicUsize, Ordering},
    };

    static TEMP_DIR_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

    const MANIFEST_SEED_JSON: &str =
        include_str!("../../../fuzz/corpus/session_json/session.manifest.v2.golden.json");

    #[test]
    fn seeded_manifest_mutations_do_not_panic() {
        assert!(serde_json::from_str::<SessionManifest>(MANIFEST_SEED_JSON).is_ok());
        let mut state = 0x8BAD_F00D_5EED_1234_u64;
        let mut next = || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            (state >> 56) as u8
        };
        let seed_bytes = MANIFEST_SEED_JSON.as_bytes();
        for _ in 0..256 {
            let mut input = seed_bytes.to_vec();
            let operations = usize::from(next() % 4);
            for _ in 0..=operations {
                let index = usize::from(next()) % (input.len() + 1);
                match next() % 3 {
                    0 => {
                        if index < input.len() {
                            input[index] ^= next() | 1;
                        }
                    }
                    1 => {
                        if index < input.len() {
                            input.remove(index);
                        }
                    }
                    _ => {
                        if input.len() < 4096 {
                            input.insert(index.min(input.len()), next());
                        }
                    }
                }
            }
            if let Ok(text) = std::str::from_utf8(&input) {
                let _ = serde_json::from_str::<SessionManifest>(text);
            }
        }
    }

    fn temp_dir() -> PathBuf {
        let sequence = TEMP_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        std::env::temp_dir().join(format!(
            "rwp-session-persistence-test-{}-{sequence}",
            std::process::id()
        ))
    }

    fn fixture<'a>(
        capture: &'a CaptureDocument,
        report: &'a PerformanceReport,
        samples: &'a [SystemSample],
        csv: &'a std::path::Path,
    ) -> PersistedSessionInput<'a> {
        PersistedSessionInput {
            process_id: 42,
            application: "test",
            interrupted: false,
            display_refresh_rate_hz: None,
            dominant_swap_chain_address: None,
            discarded_frame_count: 0,
            frame_type_counts: &[],
            telemetry_warning_count: 1,
            coverage: &CoverageSummary {
                frame_seconds: None,
                telemetry_seconds: None,
                intersection_seconds: None,
            },
            capture,
            report,
            system_samples: samples,
            source_csv: csv,
            provenance: super::SessionProvenance {
                rwp_version: "0.2.0".to_owned(),
                git_sha: None,
                build_target: None,
                build_profile: None,
                os: None,
                cpu: None,
                gpu: None,
                driver: None,
                presentmon_version: None,
                presentmon_sha256: None,
                qpc_frequency_hz: Some(1),
                sample_interval_ms: None,
                exclude_dropped: None,
                stop_existing_session: None,
                no_process_probing: None,
            },
        }
    }

    #[test]
    fn v2_provenance_without_process_probe_field_remains_readable()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy = r#"{
            "rwp_version":"0.1.0",
            "git_sha":null,
            "build_target":null,
            "build_profile":null,
            "os":null,
            "cpu":null,
            "gpu":null,
            "driver":null,
            "presentmon_version":null,
            "presentmon_sha256":null,
            "qpc_frequency_hz":10000000,
            "sample_interval_ms":50,
            "exclude_dropped":false,
            "stop_existing_session":false
        }"#;
        let provenance: super::SessionProvenance = serde_json::from_str(legacy)?;
        assert_eq!(provenance.no_process_probing, None);
        Ok(())
    }

    fn report() -> Result<PerformanceReport, Box<dyn std::error::Error>> {
        Ok(PerformanceReport {
            schema_version: String::from("rwp.report.v2"),
            frame_count: 1,
            frame_statistics: compute_statistics(
                &[16.7],
                StatisticsConfig {
                    p999_reliability_threshold: 1,
                },
            )?,
            stutter_events: Vec::new(),
            analysis_config: AnalysisConfigSummary {
                p999_reliability_threshold: 1,
                stutter_rule_id: String::from("stutter.v1.absolute"),
                stutter_refresh_rate_hz: None,
                stutter_refresh_period_ms: None,
                stutter_rolling_half_window_frames: 2,
                stutter_multiplier: 2.5,
                stutter_absolute_threshold_ms: StutterConfig::default().absolute_threshold_ms,
                stutter_minimum_baseline_samples: 3,
                stutter_merge_gap_ms: 25.0,
            },
            diagnostics: Vec::new(),
            interrupted: false,
        })
    }

    #[test]
    fn manifest_is_self_verifying_and_stable() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)?;
        let csv = root.join("source.csv");
        fs::write(&csv, b"header\n")?;
        let session_dir = root.join("session");
        let second_dir = root.join("session-second");
        let capture = CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames: vec![FrameSample::new(0, 16.7)?],
        };
        let report = report()?;
        let samples = vec![SystemSample::new(1)];
        let input = fixture(&capture, &report, &samples, &csv);

        let first = persist_session(&session_dir, &input)?;
        let first_manifest = fs::read(session_dir.join("manifest.json"))?;
        let second = persist_session(&second_dir, &input)?;
        let second_manifest = fs::read(second_dir.join("manifest.json"))?;

        assert_eq!(first, second);
        assert_eq!(first_manifest, second_manifest);
        assert_eq!(verify_manifest(&session_dir)?, first);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn manifest_preserves_no_process_probing_provenance() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = temp_dir();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)?;
        let csv = root.join("source.csv");
        fs::write(&csv, b"header\n")?;
        let capture = CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames: vec![FrameSample::new(0, 16.7)?],
        };
        let report = report()?;
        let samples = vec![SystemSample::new(1)];
        let mut input = fixture(&capture, &report, &samples, &csv);
        input.provenance.no_process_probing = Some(true);
        let session_dir = root.join("session");

        let manifest = persist_session(&session_dir, &input)?;
        assert_eq!(manifest.provenance.no_process_probing, Some(true));
        assert_eq!(
            verify_manifest(&session_dir)?.provenance.no_process_probing,
            Some(true)
        );

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn manifest_rejects_path_traversal_before_hashing() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)?;
        let csv = root.join("source.csv");
        fs::write(&csv, b"header\n")?;
        let session_dir = root.join("session");
        let capture = CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames: vec![FrameSample::new(0, 16.7)?],
        };
        let report = report()?;
        let samples = vec![SystemSample::new(1)];
        let input = fixture(&capture, &report, &samples, &csv);
        let mut manifest = persist_session(&session_dir, &input)?;
        manifest.files[0].path = String::from("../source.csv");
        fs::write(
            session_dir.join("manifest.json"),
            serde_json::to_vec(&manifest)?,
        )?;

        assert!(verify_manifest(&session_dir).is_err());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn json_depth_guard_ignores_brackets_inside_strings() -> Result<(), Box<dyn std::error::Error>>
    {
        assert!(super::ensure_json_depth(br#"{ "value": "[[[[[[[[" }"#, 1).is_ok());
        Ok(())
    }

    #[test]
    fn json_depth_guard_rejects_excessive_nesting() -> Result<(), Box<dyn std::error::Error>> {
        assert!(super::ensure_json_depth(b"[[[0]]]", 2).is_err());
        assert!(super::ensure_json_depth(br#"{"escaped":"\\\"["}"#, 1).is_ok());
        Ok(())
    }

    #[test]
    fn bounded_reader_rejects_oversized_input() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)?;
        let path = root.join("oversized.json");
        fs::write(&path, b"12345")?;

        assert!(super::read_bounded_file(&path, 4, "test read").is_err());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn manifest_hash_mismatch_rejects_file_corruption() -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)?;
        let csv = root.join("source.csv");
        fs::write(&csv, b"header\n")?;
        let session_dir = root.join("session");
        let capture = CaptureDocument {
            schema_version: String::from("rwp.capture.v1"),
            frames: vec![FrameSample::new(0, 16.7)?],
        };
        let report = report()?;
        let samples = vec![SystemSample::new(1)];
        let input = fixture(&capture, &report, &samples, &csv);
        persist_session(&session_dir, &input)?;
        fs::write(session_dir.join("report.json"), b"tampered")?;

        assert!(verify_manifest(&session_dir).is_err());
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn write_atomic_refuses_overwrite_and_leaves_no_temporary_file()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)?;
        let target = root.join("report.json");
        fs::write(&target, b"original")?;

        assert!(super::write_atomic(&target, b"replacement").is_err());
        assert_eq!(fs::read(&target)?, b"original");
        assert_eq!(fs::read_dir(&root)?.count(), 1);

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn write_atomic_leaves_no_temporary_file_after_success()
    -> Result<(), Box<dyn std::error::Error>> {
        let root = temp_dir();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)?;
        let target = root.join("report.json");

        super::write_atomic(&target, b"payload")?;

        assert_eq!(fs::read(&target)?, b"payload");
        assert_eq!(fs::read_dir(&root)?.count(), 1);
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[cfg(unix)]
    #[test]
    fn write_atomic_does_not_follow_legacy_temporary_symlink()
    -> Result<(), Box<dyn std::error::Error>> {
        use std::os::unix::fs::symlink;

        let root = temp_dir();
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root)?;
        let target = root.join("report.json");
        let victim = root.join("victim.txt");
        let legacy_temporary = target.with_extension("tmp");
        fs::write(&victim, b"preserve victim")?;
        symlink(&victim, &legacy_temporary)?;

        super::write_atomic(&target, b"new payload")?;

        assert_eq!(fs::read(&victim)?, b"preserve victim");
        assert_eq!(fs::read(&target)?, b"new payload");
        assert!(
            fs::symlink_metadata(&legacy_temporary)?
                .file_type()
                .is_symlink()
        );
        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn session_manifest_round_trips_and_is_byte_identical() -> Result<(), Box<dyn std::error::Error>>
    {
        let manifest = super::SessionManifest {
            provenance: super::SessionProvenance {
                rwp_version: String::from("0.2.0"),
                git_sha: Some(String::from("abcdef1")),
                build_target: Some(String::from("x86_64-pc-windows-msvc")),
                build_profile: Some(String::from("release")),
                os: Some(String::from("Windows 11")),
                cpu: Some(String::from("AMD Ryzen 7")),
                gpu: Some(String::from("NVIDIA RTX 4070")),
                driver: Some(String::from("550.00")),
                presentmon_version: Some(String::from("2.0.0")),
                presentmon_sha256: Some(String::from(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                )),
                qpc_frequency_hz: Some(10_000_000),
                sample_interval_ms: Some(50),
                exclude_dropped: Some(true),
                stop_existing_session: Some(false),
                no_process_probing: Some(false),
            },
            schema_version: String::from(super::SESSION_MANIFEST_SCHEMA),
            process_id: 1234,
            application: String::from("Game.exe"),
            interrupted: false,
            display_refresh_rate_hz: Some(144.0),
            dominant_swap_chain_address: Some(0x1000),
            discarded_frame_count: 0,
            frame_type_counts: vec![(String::from("Hardware Composed"), 100)],
            telemetry_warning_count: 0,
            coverage: CoverageSummary {
                frame_seconds: Some(10.0),
                telemetry_seconds: Some(10.0),
                intersection_seconds: Some(10.0),
            },
            files: vec![super::SessionManifestFile {
                path: String::from("report.json"),
                size_bytes: 1024,
                sha256: String::from(
                    "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
                ),
            }],
        };
        let json1 = serde_json::to_string(&manifest)?;
        let json2 = serde_json::to_string(&manifest)?;
        assert_eq!(
            json1, json2,
            "manifest serialization must be deterministic and byte-identical"
        );
        let decoded: super::SessionManifest = serde_json::from_str(&json1)?;
        assert_eq!(decoded, manifest, "manifest must round-trip through serde");
        Ok(())
    }
}
