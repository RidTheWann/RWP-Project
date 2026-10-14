//! No-console stop-path driver for AUD-03b.
//!
//! The test spawns THIS driver with `DETACHED_PROCESS`, so it runs without a
//! console exactly like the release GUI (`windows_subsystem = "windows"`).
//! The driver spawns the fake PresentMon-style child, stops it through the
//! REAL production path (`PresentMonProcess::terminate_with_outcome` with the
//! production 2 s bound), parses the CSV tolerantly, and reports a result file.
//! Exit 0 means the protocol completed (whatever the stop outcome); exit 1
//! means a driver-internal error (bad args, spawn failure, ...).
//!
//! Cargo examples are local test helpers only: never shipped in the installer
//! and never part of release artifacts.
//!
//! Protocol:
//! `no_console_stop_driver --helper <exe> --mode graceful|ignore
//! --csv <path> --ready <path> --result <path>`
//!
//! Result file lines (all present on exit 0):
//! `detected_no_console`, `outcome` (`graceful`|`killed`),
//! `child_success`, `child_exit_code`, `frames`, `strict_failed`,
//! `signal_error` (`none` or the terminate error for diagnosis).

use std::io::Write;
use std::time::{Duration, Instant};

#[cfg(windows)]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    use std::os::windows::process::CommandExt;
    use std::process::{Command, Stdio};

    use rwp_capture::{
        ParseMode, PresentMonCsvParser, PresentMonProcess, PresentMonTarget, StopOutcome,
    };

    let args: Vec<String> = std::env::args().collect();
    let mut helper: Option<String> = None;
    let mut mode: Option<String> = None;
    let mut csv: Option<String> = None;
    let mut ready: Option<String> = None;
    let mut result: Option<String> = None;
    let mut index = 1_usize;
    while index < args.len() {
        let arg = args[index].as_str();
        if let Some(value) = arg.strip_prefix("--helper=") {
            helper = Some(value.to_owned());
        } else if let Some(value) = arg.strip_prefix("--mode=") {
            mode = Some(value.to_owned());
        } else if let Some(value) = arg.strip_prefix("--csv=") {
            csv = Some(value.to_owned());
        } else if let Some(value) = arg.strip_prefix("--ready=") {
            ready = Some(value.to_owned());
        } else if let Some(value) = arg.strip_prefix("--result=") {
            result = Some(value.to_owned());
        } else if arg == "--helper" {
            index = index.saturating_add(1);
            let value = args.get(index).ok_or("missing value for --helper")?;
            helper = Some(value.clone());
        } else if arg == "--mode" {
            index = index.saturating_add(1);
            let value = args.get(index).ok_or("missing value for --mode")?;
            mode = Some(value.clone());
        } else if arg == "--csv" {
            index = index.saturating_add(1);
            let value = args.get(index).ok_or("missing value for --csv")?;
            csv = Some(value.clone());
        } else if arg == "--ready" {
            index = index.saturating_add(1);
            let value = args.get(index).ok_or("missing value for --ready")?;
            ready = Some(value.clone());
        } else if arg == "--result" {
            index = index.saturating_add(1);
            let value = args.get(index).ok_or("missing value for --result")?;
            result = Some(value.clone());
        } else {
            return Err(format!("unexpected argument: {arg}").into());
        }
        index = index.saturating_add(1);
    }
    let helper = helper.ok_or("missing --helper <exe>")?;
    let mode = mode.ok_or("missing --mode graceful|ignore")?;
    let csv = csv.ok_or("missing --csv <path>")?;
    let ready = ready.ok_or("missing --ready <path>")?;
    let result = result.ok_or("missing --result <path>")?;
    if mode != "graceful" && mode != "ignore" {
        return Err(format!("unsupported --mode '{mode}'").into());
    }

    // Production spawn policy: the same creation flags
    // `PresentMonRunner::start` uses, selected from the caller's console
    // attachment, so a detached driver spawns exactly what the GUI spawns.
    let mut child = Command::new(helper.as_str())
        .arg("--mode")
        .arg(mode.as_str())
        .arg("--csv")
        .arg(csv.as_str())
        .arg("--ready")
        .arg(ready.as_str())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(rwp_capture::presentmon_child_creation_flags())
        .spawn()
        .map_err(|error| format!("spawn fake child failed: {error}"))?;

    let ready_path = std::path::Path::new(ready.as_str());
    let ready_deadline = Instant::now() + Duration::from_secs(15);
    loop {
        if ready_path.exists() {
            break;
        }
        if Instant::now() >= ready_deadline {
            let _ = child.kill();
            return Err("fake child never signaled readiness within 15 s".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let detected_no_console = !rwp_system::caller_has_console();
    let target = PresentMonTarget::ProcessId(42);
    let mut process = PresentMonProcess::from_child_for_testing(
        child,
        std::path::PathBuf::from(csv.as_str()),
        target.clone(),
    );
    // Production bound: the documented graceful-stop wait from the stop path.
    let outcome = match process.terminate_with_outcome(rwp_capture::GRACEFUL_STOP_TIMEOUT) {
        Ok(outcome) => outcome,
        Err(error) => {
            let _ = process.wait();
            return Err(format!("production terminate failed: {error}").into());
        }
    };
    let status = process
        .wait()
        .map_err(|error| format!("wait for fake child failed: {error}"))?;

    let parser = PresentMonCsvParser::new(1_000_000)?;
    let strict_failed = parser
        .parse_path(std::path::Path::new(csv.as_str()), &target)
        .is_err();
    let capture = parser.parse_path_with_mode(
        std::path::Path::new(csv.as_str()),
        &target,
        ParseMode::TolerateTruncatedTail,
    )?;
    let outcome_text = match outcome {
        StopOutcome::Graceful => "graceful",
        StopOutcome::Killed => "killed",
    };
    let exit_code = match status.code() {
        Some(code) => code.to_string(),
        None => String::from("none"),
    };
    let mut output = String::new();
    output.push_str(&format!("detected_no_console={detected_no_console}\n"));
    output.push_str(&format!("outcome={outcome_text}\n"));
    output.push_str(&format!("child_success={}\n", status.success()));
    output.push_str(&format!("child_exit_code={exit_code}\n"));
    output.push_str(&format!("frames={}\n", capture.frames.len()));
    output.push_str(&format!("strict_failed={strict_failed}\n"));
    output.push_str("signal_error=none\n");
    let mut file = std::fs::File::create(result.as_str())?;
    file.write_all(output.as_bytes())?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

#[cfg(windows)]
fn main() {
    if let Err(error) = run() {
        eprintln!("no_console_stop_driver: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("no_console_stop_driver: windows-only test helper");
    std::process::exit(2);
}
