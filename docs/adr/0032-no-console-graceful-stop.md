# ADR-0032 — Graceful PresentMon stop without a caller console

- Status: accepted
- Date: 2026-10-15
- Scope: AUD-03b / PresentMon stop path for console-less parents (release GUI)
- Amends: ADR-0015 (console-sharing precondition below)

## Context

ADR-0015 delivers `CTRL+BREAK` with a direct `GenerateConsoleCtrlEvent` call.
That API only reaches processes sharing the caller's console, but the release
GUI sets `windows_subsystem = "windows"` and has no console, and the spawn
path passed only `CREATE_NEW_PROCESS_GROUP`. GUI cancel therefore always took
the hard-kill fallback. The Batch 1 runtime proof ran under a console (cargo
test, CLI), so it covered console parents only. A detached-driver test
reproduced the gap: from a console-less parent the graceful fake child was
killed (`outcome=killed`, final row missing) instead of exiting cleanly.

## Decision

1. At spawn, record whether the child shares the caller's console
   (`rwp_system::caller_has_console`, via `GetConsoleProcessList`).
   Console callers keep exactly the historical flags
   (`CREATE_NEW_PROCESS_GROUP`); console-less callers spawn with
   `CREATE_NEW_PROCESS_GROUP | CREATE_NO_WINDOW`, so the child keeps a
   (hidden) console but no visible window ever appears.
2. The stop path is selected from the recorded flag, never from return-code
   heuristics. Console path: the direct `GenerateConsoleCtrlEvent` call as
   before. No-console path: `AttachConsole(child_pid)` →
   `GenerateConsoleCtrlEvent(CTRL_BREAK_EVENT, child_pid)` → always
   `FreeConsole`, even on failure; an already-exited child counts as stopped.
   Attach/free pairs are serialized by a process-wide `Mutex` because console
   attachment is process-global. All unsafe stays in `rwp-system` behind safe
   functions with `// SAFETY:` comments; raw Win32 codes are preserved in
   `DetachedBreakError` for diagnosis.
3. Add `StopOutcome::{Graceful, Killed}` via `terminate_with_outcome()`.
   `terminate()` and `terminate_gracefully()` keep their signatures and
   behavior (including the 2 s bound, unchanged) and delegate to it.
4. Test helpers stay Cargo examples (never shipped, never release artifacts):
   the no-console driver plus the `from_child_for_testing` constructor. The
   workspace gate (`cargo test --workspace --all-features --locked`) builds
   every example before running any test (verified by deleting both helper
   binaries and watching `--no-run` rebuild them), so no manual step is
   needed there; package-selected runs pre-build once with `cargo build
   --locked --examples -p rwp-capture -p rwp-system`. A nested `cargo build`
   from inside a test was tried and rejected: the outer run links example
   binaries concurrently, and two linkers on the same `.exe` fail
   deterministically with `LNK1104` under full-workspace runs.

## Consequences

- GUI cancel now requests a graceful PresentMon stop; the kill fallback,
  bounded wait, tolerant tail parsing, and `interrupted=true` provenance are
  unchanged.
- Verified against the fake child for console and no-console parents;
  real-PresentMon behavior stays `BLOCKED_ON_USER`.
- Residual risks: a hard kill can still leave a PresentMon ETW session open
  (tracked for Batch 7: stderr hint, `--stop_existing_session` on the CLI,
  troubleshooting); Ctrl+C graceful persistence for `rwp capture` is Batch 6.
