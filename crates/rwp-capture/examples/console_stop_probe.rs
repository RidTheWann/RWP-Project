//! Console-stop handler probe for the `rwp capture` Ctrl+C work.
//!
//! Installs the production [`rwp_system::ConsoleStopGuard`], signals
//! readiness, then waits until the shared stop flag flips (or a 30 s internal
//! bound lapses) and reports a result file. Exit 0 means the protocol
//! completed; exit 1 means a probe-internal error. The test delivers a real
//! CTRL+BREAK to this probe's process group and asserts the flag.
//!
//! Cargo examples are local test helpers only: never shipped in the installer
//! and never part of release artifacts.
//!
//! Protocol: `console_stop_probe --ready <path> --result <path>`; the result
//! file contains `stopping=true` when the flag was observed.

use std::io::Write;
use std::time::{Duration, Instant};

#[cfg(windows)]
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut ready: Option<String> = None;
    let mut result: Option<String> = None;
    let mut index = 1_usize;
    while index < args.len() {
        let arg = args[index].as_str();
        if let Some(value) = arg.strip_prefix("--ready=") {
            ready = Some(value.to_owned());
        } else if let Some(value) = arg.strip_prefix("--result=") {
            result = Some(value.to_owned());
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
    let ready = ready.ok_or("missing --ready <path>")?;
    let result = result.ok_or("missing --result <path>")?;

    let _guard = rwp_system::ConsoleStopGuard::install()
        .map_err(|error| format!("install console handler failed: {error}"))?;
    std::fs::write(ready.as_str(), b"ready")?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if rwp_system::capture_stop_flag().load(std::sync::atomic::Ordering::SeqCst) {
            break;
        }
        if Instant::now() >= deadline {
            return Err("stop flag never flipped within 30 s".into());
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    let mut file = std::fs::File::create(result.as_str())?;
    file.write_all(b"stopping=true\n")?;
    file.flush()?;
    file.sync_all()?;
    Ok(())
}

#[cfg(windows)]
fn main() {
    if let Err(error) = run() {
        eprintln!("console_stop_probe: {error}");
        std::process::exit(1);
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("console_stop_probe: windows-only test helper");
    std::process::exit(2);
}
