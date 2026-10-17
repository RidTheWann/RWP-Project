//! End-to-end CLI fixture tests.

use std::{fs, process::Command};

const FIXTURE: &str = include_str!("fixtures/minimal-presentmon-v2.csv");

const REAL_CAPTURE_FIXTURE: &str =
    include_str!("../../rwp-capture/tests/fixtures/real-capture-v2-anonymized.csv");

fn run_rwp(arguments: &[&str]) -> Result<std::process::Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_rwp"))
        .args(arguments)
        .output()
}

#[test]
fn fixture_pipeline_import_analyze_compare_report() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("rwp-cli-e2e-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root)?;
    let csv = root.join("fixture.csv");
    let baseline = root.join("baseline");
    let candidate = root.join("candidate");
    let analyzed_baseline = root.join("baseline-analyzed");
    let analyzed_candidate = root.join("candidate-analyzed");
    let report = root.join("report.json");
    fs::write(&csv, FIXTURE)?;

    let baseline_output = run_rwp(&[
        "import-presentmon",
        csv.to_str().ok_or("invalid csv path")?,
        "--process-id",
        "42",
        "--qpc-frequency-hz",
        "1000000",
        "--output-dir",
        baseline.to_str().ok_or("invalid baseline path")?,
    ])?;
    assert!(baseline_output.status.success(), "{:?}", baseline_output);
    let candidate_output = run_rwp(&[
        "import-presentmon",
        csv.to_str().ok_or("invalid csv path")?,
        "--process-id",
        "42",
        "--qpc-frequency-hz",
        "1000000",
        "--output-dir",
        candidate.to_str().ok_or("invalid candidate path")?,
    ])?;
    assert!(candidate_output.status.success(), "{:?}", candidate_output);

    let baseline_analyze = run_rwp(&[
        "analyze",
        "--session",
        baseline.to_str().ok_or("invalid baseline path")?,
        "--output-dir",
        analyzed_baseline
            .to_str()
            .ok_or("invalid analyzed baseline path")?,
    ])?;
    assert!(baseline_analyze.status.success(), "{:?}", baseline_analyze);
    let candidate_analyze = run_rwp(&[
        "analyze",
        "--session",
        candidate.to_str().ok_or("invalid candidate path")?,
        "--output-dir",
        analyzed_candidate
            .to_str()
            .ok_or("invalid analyzed candidate path")?,
    ])?;
    assert!(
        candidate_analyze.status.success(),
        "{:?}",
        candidate_analyze
    );

    let comparison = run_rwp(&[
        "compare",
        "--baseline",
        analyzed_baseline
            .to_str()
            .ok_or("invalid analyzed baseline path")?,
        "--candidate",
        analyzed_candidate
            .to_str()
            .ok_or("invalid analyzed candidate path")?,
        "--same-scenario",
    ])?;
    assert!(comparison.status.success(), "{:?}", comparison);
    let comparison_json: serde_json::Value = serde_json::from_slice(&comparison.stdout)?;
    assert!(comparison_json.get("comparability").is_some());

    let report_output = run_rwp(&[
        "report",
        "--session",
        analyzed_candidate
            .to_str()
            .ok_or("invalid analyzed candidate path")?,
        "--format",
        "json",
        "--output",
        report.to_str().ok_or("invalid report path")?,
    ])?;
    assert!(report_output.status.success(), "{:?}", report_output);
    let report_json: serde_json::Value = serde_json::from_slice(&fs::read(&report)?)?;
    assert_eq!(report_json["schema_version"], "rwp.report.v2");
    assert_eq!(report_json["frame_count"], 4);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(analyzed_candidate.join("manifest.json"))?)?;
    assert_eq!(manifest["provenance"]["qpc_frequency_hz"], 1000000);
    assert_eq!(
        manifest["provenance"]["rwp_version"],
        env!("CARGO_PKG_VERSION")
    );
    assert!(
        manifest["files"]
            .as_array()
            .is_some_and(|files| files.len() == 4)
    );

    let pairs = root.join("pairs");
    let pair = pairs.join("fixture-pair");
    for (source, destination) in [
        (&analyzed_baseline, pair.join("baseline")),
        (&analyzed_candidate, pair.join("candidate")),
    ] {
        fs::create_dir_all(&destination)?;
        for entry in fs::read_dir(source)? {
            let entry = entry?;
            assert!(
                entry.file_type()?.is_file(),
                "analyzed fixture session contains a non-file entry"
            );
            fs::copy(entry.path(), destination.join(entry.file_name()))?;
        }
    }
    let calibration = run_rwp(&[
        "calibrate",
        "--pairs",
        pairs.to_str().ok_or("invalid pairs path")?,
        "--alpha",
        "0.25",
    ])?;
    assert!(calibration.status.success(), "{calibration:?}");
    let calibration_json: serde_json::Value = serde_json::from_slice(&calibration.stdout)?;
    assert_eq!(calibration_json["recommended_minimum_pairs"], 3);
    assert!(
        calibration_json["inference_limit"]
            .as_str()
            .is_some_and(|text| text.contains("informative, non-tie pairs"))
    );
    let invalid_calibration = run_rwp(&[
        "calibrate",
        "--pairs",
        root.join("missing-pairs")
            .to_str()
            .ok_or("invalid missing pairs path")?,
        "--alpha",
        "0",
    ])?;
    assert!(!invalid_calibration.status.success());
    assert!(
        String::from_utf8(invalid_calibration.stderr)?.contains("alpha must be between 0 and 1")
    );

    let session = analyzed_candidate
        .to_str()
        .ok_or("invalid analyzed candidate path")?;
    for (format, marker) in [("md", "<redacted>"), ("html", "&lt;redacted&gt;")] {
        let redacted = run_rwp(&["report", "--session", session, "--format", format])?;
        assert!(redacted.status.success(), "{redacted:?}");
        let redacted_text = String::from_utf8(redacted.stdout)?;
        assert!(redacted_text.contains(marker), "{redacted_text}");
        assert!(
            !redacted_text.contains("Process ID"),
            "redacted {format} report must omit the process ID: {redacted_text}"
        );

        let plain = run_rwp(&[
            "report",
            "--session",
            session,
            "--format",
            format,
            "--redact",
            "false",
        ])?;
        assert!(plain.status.success(), "{plain:?}");
        assert!(String::from_utf8(plain.stdout)?.contains("Process ID: 42"));
    }

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn migrate_session_and_report_flow() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("rwp-cli-e2e-migrate-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root)?;
    let csv = root.join("fixture.csv");
    let imported = root.join("imported");
    let migrated = root.join("migrated");
    fs::write(&csv, FIXTURE)?;

    let import = run_rwp(&[
        "import-presentmon",
        csv.to_str().ok_or("invalid csv path")?,
        "--process-id",
        "42",
        "--qpc-frequency-hz",
        "1000000",
        "--output-dir",
        imported.to_str().ok_or("invalid imported path")?,
    ])?;
    assert!(import.status.success(), "{import:?}");

    // Downgrade the schema stamps without touching content hashes: current
    // readers must reject the session, migration must accept it.
    let manifest_path = imported.join("manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path)?.replacen(
        "rwp.session.manifest.v2",
        "rwp.session.manifest.v1",
        1,
    );
    assert!(manifest_text.contains("rwp.session.manifest.v1"));
    fs::write(&manifest_path, manifest_text)?;
    let report_path = imported.join("report.json");
    let report_text = fs::read_to_string(&report_path)?.replacen(
        "\"schema_version\":\"rwp.report.v2\"",
        "\"schema_version\":\"rwp.report.v1\"",
        1,
    );
    assert!(report_text.contains("\"schema_version\":\"rwp.report.v1\""));
    fs::write(&report_path, report_text.as_bytes())?;

    let rejected = run_rwp(&[
        "report",
        "--session",
        imported.to_str().ok_or("invalid imported path")?,
        "--format",
        "json",
    ])?;
    assert!(!rejected.status.success(), "{rejected:?}");

    let migrate_session = run_rwp(&[
        "migrate",
        "--session",
        imported.to_str().ok_or("invalid imported path")?,
        "--output-dir",
        migrated.to_str().ok_or("invalid migrated path")?,
    ])?;
    assert!(migrate_session.status.success(), "{migrate_session:?}");

    let migrated_report = root.join("migrated-report.json");
    let report = run_rwp(&[
        "report",
        "--session",
        migrated.to_str().ok_or("invalid migrated path")?,
        "--format",
        "json",
        "--output",
        migrated_report.to_str().ok_or("invalid report path")?,
    ])?;
    assert!(report.status.success(), "{report:?}");
    let report_json: serde_json::Value = serde_json::from_slice(&fs::read(&migrated_report)?)?;
    assert_eq!(report_json["schema_version"], "rwp.report.v2");
    let migrated_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(migrated.join("manifest.json"))?)?;
    assert_eq!(
        migrated_manifest["schema_version"],
        "rwp.session.manifest.v2"
    );

    // Standalone report migration is idempotent on current-schema input.
    let standalone = root.join("standalone.json");
    let migrated_standalone = root.join("standalone-migrated.json");
    fs::write(&standalone, report_text.as_bytes())?;
    let migrate_report = run_rwp(&[
        "migrate",
        "--report",
        standalone.to_str().ok_or("invalid standalone path")?,
        "--output",
        migrated_standalone
            .to_str()
            .ok_or("invalid migrated standalone path")?,
    ])?;
    assert!(migrate_report.status.success(), "{migrate_report:?}");
    let standalone_json: serde_json::Value =
        serde_json::from_slice(&fs::read(&migrated_standalone)?)?;
    assert_eq!(standalone_json["schema_version"], "rwp.report.v2");

    // Unknown schemas and missing sources fail without writing output.
    let unknown = root.join("unknown.json");
    fs::write(&unknown, "{\"schema_version\":\"rwp.report.v9\"}")?;
    let unknown_out = root.join("unknown-out.json");
    let unknown_run = run_rwp(&[
        "migrate",
        "--report",
        unknown.to_str().ok_or("invalid unknown path")?,
        "--output",
        unknown_out.to_str().ok_or("invalid unknown out path")?,
    ])?;
    assert!(!unknown_run.status.success(), "{unknown_run:?}");
    assert!(!unknown_out.exists());
    let missing_source = run_rwp(&["migrate"])?;
    assert!(!missing_source.status.success(), "{missing_source:?}");

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn import_refresh_rate_and_analyze_preserve_v2_and_interruption()
-> Result<(), Box<dyn std::error::Error>> {
    let root = std::env::temp_dir().join(format!("rwp-cli-e2e-refresh-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root)?;
    let csv = root.join("fixture.csv");
    let imported = root.join("imported");
    let analyzed = root.join("analyzed");
    let interrupted = root.join("interrupted");
    fs::write(&csv, FIXTURE)?;

    let import = run_rwp(&[
        "import-presentmon",
        csv.to_str().ok_or("invalid csv path")?,
        "--process-id",
        "42",
        "--qpc-frequency-hz",
        "1000000",
        "--refresh-rate-hz",
        "144",
        "--output-dir",
        imported.to_str().ok_or("invalid imported path")?,
    ])?;
    assert!(import.status.success(), "{import:?}");

    let analyze = run_rwp(&[
        "analyze",
        "--session",
        imported.to_str().ok_or("invalid imported path")?,
        "--output-dir",
        analyzed.to_str().ok_or("invalid analyzed path")?,
    ])?;
    assert!(analyze.status.success(), "{analyze:?}");

    let report: serde_json::Value =
        serde_json::from_slice(&fs::read(analyzed.join("report.json"))?)?;
    assert_eq!(
        report["analysis_config"]["stutter_rule_id"],
        "stutter.v2.refresh_aware"
    );
    assert_eq!(report["analysis_config"]["stutter_refresh_rate_hz"], 144.0);
    assert_eq!(report["interrupted"], false);
    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(analyzed.join("manifest.json"))?)?;
    assert_eq!(manifest["display_refresh_rate_hz"], 144.0);
    assert_eq!(manifest["interrupted"], false);

    // Simulate an interrupted V2 live session: only manifest metadata changes,
    // so the persisted file hashes stay valid for verification.
    let mut source_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(imported.join("manifest.json"))?)?;
    source_manifest["interrupted"] = serde_json::Value::Bool(true);
    fs::write(
        imported.join("manifest.json"),
        serde_json::to_vec(&source_manifest)?,
    )?;

    let analyze_interrupted = run_rwp(&[
        "analyze",
        "--session",
        imported.to_str().ok_or("invalid imported path")?,
        "--output-dir",
        interrupted.to_str().ok_or("invalid interrupted path")?,
    ])?;
    assert!(
        analyze_interrupted.status.success(),
        "{analyze_interrupted:?}"
    );
    let interrupted_report: serde_json::Value =
        serde_json::from_slice(&fs::read(interrupted.join("report.json"))?)?;
    assert_eq!(interrupted_report["interrupted"], true);
    assert_eq!(
        interrupted_report["analysis_config"]["stutter_rule_id"],
        "stutter.v2.refresh_aware"
    );
    let interrupted_manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(interrupted.join("manifest.json"))?)?;
    assert_eq!(interrupted_manifest["interrupted"], true);

    // Non-positive and non-finite refresh rates are rejected before analysis.
    for rate in ["0", "NaN"] {
        let bad_dir = root.join(format!("bad-{rate}"));
        let bad = run_rwp(&[
            "import-presentmon",
            csv.to_str().ok_or("invalid csv path")?,
            "--process-id",
            "42",
            "--qpc-frequency-hz",
            "1000000",
            "--refresh-rate-hz",
            rate,
            "--output-dir",
            bad_dir.to_str().ok_or("invalid bad path")?,
        ])?;
        assert!(!bad.status.success(), "{rate}: {bad:?}");
    }

    fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn real_capture_window_pipeline_import_analyze_report() -> Result<(), Box<dyn std::error::Error>> {
    // Anonymized window of the 2026-10-17 live dwm.exe capture; see
    // docs/acceptance/2026-10-17.md and the rwp-capture parser regression.
    let root = std::env::temp_dir().join(format!("rwp-cli-e2e-real-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root)?;
    let csv = root.join("real-window.csv");
    let imported = root.join("imported");
    let analyzed = root.join("analyzed");
    let report = root.join("report.json");
    fs::write(&csv, REAL_CAPTURE_FIXTURE)?;

    let import = run_rwp(&[
        "import-presentmon",
        csv.to_str().ok_or("invalid csv path")?,
        "--process-id",
        "8852",
        "--qpc-frequency-hz",
        "10000000",
        "--output-dir",
        imported.to_str().ok_or("invalid imported path")?,
    ])?;
    assert!(import.status.success(), "{import:?}");
    let import_stderr = String::from_utf8(import.stderr)?;
    assert!(
        import_stderr.contains("dropped 32 rows with backwards QPC timestamps"),
        "{import_stderr}"
    );

    let analyze = run_rwp(&[
        "analyze",
        "--session",
        imported.to_str().ok_or("invalid imported path")?,
        "--output-dir",
        analyzed.to_str().ok_or("invalid analyzed path")?,
    ])?;
    assert!(analyze.status.success(), "{analyze:?}");

    let report_run = run_rwp(&[
        "report",
        "--session",
        analyzed.to_str().ok_or("invalid analyzed path")?,
        "--format",
        "json",
        "--output",
        report.to_str().ok_or("invalid report path")?,
    ])?;
    assert!(report_run.status.success(), "{report_run:?}");
    let report_json: serde_json::Value = serde_json::from_slice(&fs::read(&report)?)?;
    assert_eq!(report_json["schema_version"], "rwp.report.v2");
    assert_eq!(report_json["frame_count"], 255);
    assert_eq!(report_json["interrupted"], false);

    let manifest: serde_json::Value =
        serde_json::from_slice(&fs::read(imported.join("manifest.json"))?)?;
    assert_eq!(manifest["process_id"], 8852);
    assert_eq!(manifest["discarded_frame_count"], 33);
    assert_eq!(manifest["provenance"]["qpc_frequency_hz"], 10000000);
    let capture: serde_json::Value =
        serde_json::from_slice(&fs::read(imported.join("capture.json"))?)?;
    let frames = capture["frames"]
        .as_array()
        .ok_or("capture frames missing")?;
    assert_eq!(frames.len(), 255);
    assert!(
        frames
            .windows(2)
            .all(|pair| pair[0]["timestamp_ns"].as_u64() <= pair[1]["timestamp_ns"].as_u64())
    );

    fs::remove_dir_all(root)?;
    Ok(())
}
