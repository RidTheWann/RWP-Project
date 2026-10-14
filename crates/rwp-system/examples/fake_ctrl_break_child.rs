//! Fake console child for AUD-03 graceful-stop runtime proof.
//!
//! Built from this repo with `cargo build --locked -p rwp-system --examples`.
//! No new dependencies (only `std` plus the workspace `windows-sys` console
//! API already used by `rwp-system`). Never shipped in the installer and never
//! part of release artifacts: Cargo examples are local test helpers only.
//!
//! Protocol: `--mode graceful|ignore --csv <path> --ready <path>`.
//! The child writes a minimal PresentMon v2-style CSV (PID 42, QPC 1000/1100),
//! installs a console handler, then creates the ready file. Parents poll for
//! the ready file (generous 15 s bound in tests) before requesting
//! `CTRL+BREAK`, so no signal races the handler installation.
//!
//! - `graceful`: on `CTRL+BREAK` (or `CTRL+C`) appends one final complete row
//!   (QPC 1200), flushes, and exits 0. Proves the production
//!   `request_process_group_break` + bounded-wait path can finish cleanly
//!   with no kill and a tolerantly parsable CSV.
//! - `ignore`: installs a handler that swallows `CTRL+BREAK`/`CTRL+C` and
//!   writes a truncated final line (no newline, incomplete fields) up front,
//!   then sleeps. The parent must fall back to kill after the bounded wait;
//!   the CSV retains 2 complete rows plus a truncated tail that strict
//!   parsing rejects and `TolerateTruncatedTail` drops.
//!
//! Both modes exit 2 on internal timeout (60 s without the expected signal)
//! so a hung test cannot orphan the child forever.

#![allow(unsafe_code)]

use std::io::Write;
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(windows)]
static BREAK_RECEIVED: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
static IGNORE_BREAK: AtomicBool = AtomicBool::new(false);

#[cfg(windows)]
unsafe extern "system" fn ctrl_handler(ctrl_type: u32) -> i32 {
    use windows_sys::Win32::System::Console::{CTRL_BREAK_EVENT, CTRL_C_EVENT};
    const TRUE: i32 = 1;
    const FALSE: i32 = 0;
    if ctrl_type == CTRL_BREAK_EVENT || ctrl_type == CTRL_C_EVENT {
        if IGNORE_BREAK.load(Ordering::SeqCst) {
            return TRUE;
        }
        BREAK_RECEIVED.store(true, Ordering::SeqCst);
        return TRUE;
    }
    FALSE
}

fn main() {
    if let Err(error) = run() {
        eprintln!("fake_ctrl_break_child: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let mut mode: Option<String> = None;
    let mut csv: Option<String> = None;
    let mut ready: Option<String> = None;
    let mut index = 1_usize;
    while index < args.len() {
        let arg = args[index].as_str();
        if let Some(value) = arg.strip_prefix("--mode=") {
            mode = Some(value.to_owned());
        } else if let Some(value) = arg.strip_prefix("--csv=") {
            csv = Some(value.to_owned());
        } else if let Some(value) = arg.strip_prefix("--ready=") {
            ready = Some(value.to_owned());
        } else if arg == "--mode" {
            index = index.saturating_add(1);
            let value = args
                .get(index)
                .ok_or("missing value for --mode (expected graceful|ignore)")?;
            mode = Some(value.clone());
        } else if arg == "--csv" {
            index = index.saturating_add(1);
            let value = args.get(index).ok_or("missing value for --csv")?;
            csv = Some(value.clone());
        } else if arg == "--ready" {
            index = index.saturating_add(1);
            let value = args.get(index).ok_or("missing value for --ready")?;
            ready = Some(value.clone());
        } else {
            return Err(format!("unexpected argument: {arg}").into());
        }
        index = index.saturating_add(1);
    }
    let mode = mode.ok_or("missing --mode (expected graceful|ignore)")?;
    let csv = csv.ok_or("missing --csv <path>")?;
    let ready = ready.ok_or("missing --ready <path>")?;
    if mode != "graceful" && mode != "ignore" {
        return Err(format!("unsupported --mode '{mode}' (expected graceful|ignore)").into());
    }

    let header = "Application,ProcessID,CPUStartQPC,FrameTime,CPUBusy,CPUWait,GPUTime,GPUBusy,GPUWait,DisplayLatency";
    let row_one = "Game.exe,42,1000,8.0,5.0,1.0,7.0,6.0,1.0,9.0";
    let row_two = "Game.exe,42,1100,8.0,5.0,1.0,7.0,6.0,1.0,9.0";
    let final_row = "Game.exe,42,1200,8.0,5.0,1.0,7.0,6.0,1.0,9.0";
    let truncated_tail = "Game.exe,42,1200,8.0,5.0,1.0";

    let mut file = std::fs::File::create(csv.as_str())?;
    writeln!(file, "{header}")?;
    writeln!(file, "{row_one}")?;
    if mode == "graceful" {
        writeln!(file, "{row_two}")?;
    } else {
        writeln!(file, "{row_two}")?;
        write!(file, "{truncated_tail}")?;
    }
    file.flush()?;
    file.sync_all()?;

    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::SetConsoleCtrlHandler;
        IGNORE_BREAK.store(mode == "ignore", Ordering::SeqCst);
        // SAFETY: handler is a plain function pointer with the documented
        // `unsafe extern "system" fn(u32) -> i32` shape; TRUE (1) claims the
        // event and suppresses default termination, FALSE (0) passes it on.
        // No heap or lock is touched inside the handler, only two atomics.
        let installed = unsafe { SetConsoleCtrlHandler(Some(ctrl_handler), 1) };
        if installed == 0 {
            return Err("SetConsoleCtrlHandler failed".into());
        }
    }

    std::fs::write(ready.as_str(), b"ready")?;

    if mode == "graceful" {
        #[cfg(windows)]
        {
            let deadline = Instant::now() + Duration::from_secs(60);
            loop {
                if BREAK_RECEIVED.load(Ordering::SeqCst) {
                    writeln!(file, "{final_row}")?;
                    file.flush()?;
                    file.sync_all()?;
                    return Ok(());
                }
                if Instant::now() >= deadline {
                    return Err("timed out waiting for CTRL+BREAK in graceful mode".into());
                }
                std::thread::sleep(Duration::from_millis(25));
            }
        }
        #[cfg(not(windows))]
        {
            let _ = final_row;
            std::thread::sleep(Duration::from_secs(60));
            return Err("graceful mode requires Windows console control".into());
        }
    } else {
        std::thread::sleep(Duration::from_secs(60));
        Err("timed out waiting to be killed in ignore mode".into())
    }
}
