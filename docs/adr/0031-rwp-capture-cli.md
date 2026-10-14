# ADR-0031 — `rwp capture` CLI on the shared session orchestrator

- Status: accepted
- Date: 2026-10-12
- Scope: WP4 / `rwp capture` live CLI

## Context

Live capture was GUI-only; MASTER_PLAN WP4 lists `rwp capture`. The GUI runs
`rwp_session::SessionRunner::run(&SessionConfig, &AtomicBool)`, which owns the
PresentMon lifecycle (`rwp_capture::PresentMonRunner`), telemetry sampling
(`rwp_system`, `SystemSampler` seam), tolerant interrupted-tail parsing, and
deterministic persistence (`persist_session`, manifest v2). A CLI must reuse
that path without duplicating capture/analysis logic, must never orphan
PresentMon on interrupt/crash, and must stay Windows-only with the existing
`RWP-Exxxx` taxonomy and no new dependencies.

Seam inspection (no large refactor):

- Reusable orchestrator: yes. `SessionRunner`, `SessionConfig`
  (`crates/rwp-session/src/windows.rs`), already used by
  `rwp-ui-native/src-tauri/src/commands.rs::session_start`.
- Sampler seam: yes. `SystemSampler` trait with deterministic fakes
  (`collect_telemetry_sample`, `collect_capture` are generic).
- Runner seam: no new trait. `SessionRunner::run` hardcodes
  `PresentMonRunner::start`; CLI validation tests do not need a runner seam
  (live runs stay `BLOCKED_ON_USER`). No runner refactor was undertaken.
- Safety: the kill-on-close Job Object lives in `rwp-system::PresentMonJob`
  and is assigned by `rwp-capture::PresentMonRunner::start` (shared by GUI and
  CLI), not in `rwp-ui-native`. No unsafe duplication was needed.

## Decision

1. Add `rwp capture --presentmon <path> (--process-id <pid> |
   --process-name <name>) --duration-seconds <1-3600> [--interval-ms
   <10-5000, default 50>] [--no-process-probing] [--refresh-rate-hz <hz>]
   `--output-dir <dir>`. Clap enforces mutual exclusion; validation reuses
   `MAX_TIMED_SECONDS`, the 10 ms–5 s session bound, and the refresh-aware
   detector contract (no new thresholds).
2. `--process-name` resolves to a PID in the CLI via the existing
   `rwp_system::list_processes` (fails closed on zero/multiple matches); the
   session stays PID-based, so no orchestration change was needed there.
3. Add one optional `SessionConfig::display_refresh_rate_hz` override
   (validated by `RefreshAwareStutterConfig::new`, default `None` preserves all
   existing callers). Explicit CLI `--refresh-rate-hz` beats auto-detect and
   also applies under `--no-process-probing` (otherwise V1); provenance records
   the effective Hz.
4. Call the same `SessionRunner::run` + `persist_session` as the GUI (same
   session layout and manifest v2). The CLI writes PresentMon CSV to a unique
   temp file and sets `--output-dir` as the persistence dir, then removes the
   temp file best-effort after the run.
5. Safety: rely on the shared `KILL_ON_JOB_CLOSE` Job Object. PresentMon runs
   in its own process group, so Ctrl+C never reaches it; a CLI interrupt or
   crash closes the job handle and the OS terminates PresentMon. ~~Ctrl+C ends
   the CLI without persisting a partial session (documented; rerun for a clean
   capture).~~ Struck by the Batch 6 amendment below: the first Ctrl+C now
   persists the partial session (exit 0 with a stderr notice). No new unsafe
   code was added for this path.
6. Windows-only: non-Windows returns `SessionError::UnsupportedPlatform`
   (`RWP-E5100`, exit 51). No new error codes. Duration failures reuse
   rwp-capture's timed validation and classify as `RWP-E2000`; session-layer
   failures classify as `RWP-E5100`; interval/selector validation failures
   surface as `RWP-E9000` through the existing classifier.

## Consequences

- GUI and CLI produce identical session dirs and manifests; analysis/compare/
  report paths are unchanged.
- CLI arg validation is unit-tested (bounds, mutual exclusion, refresh
  contract, non-Windows code); live PresentMon runs, multi-process name
  collisions on real machines, and hardware acceptance stay `BLOCKED_ON_USER`.
- ~~Ctrl+C graceful persistence is deferred;~~ the mandatory no-orphan property
  holds via the Job Object (proven by the existing job kill-on-close runtime
  test plus owner Task Manager confirmation in `docs/acceptance/smoke-test.md`);
  graceful Ctrl+C persistence landed in the Batch 6 amendment below.

## Amended by Batch 6 (no new ADR number)

- Ctrl+C / Ctrl+Break handling landed in `rwp-system` (`ConsoleStopGuard`
  plus the pure, platform-neutral `decide_console_signal` core): the first
  signal flips a shared stop flag passed to `SessionRunner::run`, the partial
  session persists with `interrupted=true`, and the CLI exits 0 with an
  `interrupted: partial session saved at <dir>` stderr notice (OWNER DECISION,
  documented in `docs/cli-errors.md`); repeats keep default handling so the
  process still ends. All new unsafe stays in `rwp-system`.
- CLI seam: `run_capture_session` takes an injected runner closure (production
  passes the shared `SessionRunner`; tests inject fakes that persist fixture
  data through the real `persist_session`). No `rwp-session` API change was
  needed (all fabricated fields are public); changed production lines stay
  well under the ~150-line budget.
- Duration validation now calls rwp-capture's existing timed validation
  (failures classify as `RWP-E2000`); only the interval bound stays
  CLI-specific. Exit policy, Ctrl+C drill, and the console requirement are
  documented in `cli-reference.md`, `cli-errors.md`, `limitations.md`, and
  `docs/acceptance/smoke-test.md`.
