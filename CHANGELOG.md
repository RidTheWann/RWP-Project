# Changelog

Versioning follows Semantic Versioning; the tree is at `0.2.0` after the
post-alpha console-stop work (tags `v0.1.0-alpha` and `v0.2.0-alpha` published
as GitHub pre-releases; their notes stay frozen under their sections below).
Entry format: one bullet per user-facing change under
`Added`/`Changed`/`Fixed`/`Security`; process-only work is recorded in
`docs/autopilot/STATE.md`, never here (CI-era process entries moved to
`docs/autopilot/archive/CHANGELOG-ci-era.md`).

## Unreleased

No user-facing changes yet since `0.3.0-alpha`.

## [0.3.0-alpha] - 2026-10-16

### Fixed
- Accept PresentMon executables without a Windows version resource (observed on genuine PresentMon 2.6.0) as version-unknown instead of refusing capture: `start()` proceeds with a logged warning, the session records a telemetry warning with the executable SHA-256 in the manifest, and `rwp doctor` reports the version check as advisory; detected non-2.x versions are still refused and non-v2 CSV output is still rejected by the schema gate.
- Fix GUI IPC denial ("Command ... not allowed by ACL" with a stuck IPC ERROR badge): the main capability referenced a `default` permission that the Tauri runtime resolves through the default permission *set*, which did not exist, so every session command was denied; the capability now references the `session-commands` permission that actually lists the 11 registered commands.

## [0.2.0-alpha] - 2026-10-16

### Added
- Add the `stability.v1` diagnostic rule: stutter-free captures at or above the minimum-frame bound with split-half mean drift inside the shared 1% equivalence margin now carry an explicit Stable verdict (with threshold provenance) that the instant-stability notification keys on; below-minimum observations still yield no stability verdict.
- Expose instant-stability in the native GUI: a `Notify when stable` checkbox (checked by default; unchecking suppresses only the toast, never the in-app banner) and an always-on stability banner (`Stable` with observation counts, `Measuring stability…` while capturing, `Stability unknown`/`Not stable` otherwise, never a stale verdict); toast and banner payloads carry counts only, never identities, and a refused toast falls back silently to the banner.
- Add `rwp capture` for live Windows capture (`--presentmon`, `--process-id`/`--process-name`, `--duration-seconds` 1–3600, `--interval-ms` 10–5000 default 50, `--no-process-probing`, `--refresh-rate-hz`, `--output-dir`): it runs the same session orchestration, persistence layout, and manifest as the GUI, reuses the kill-on-close Job Object so PresentMon never orphans on interrupt/crash, and returns `RWP-E5100` off Windows. The first Ctrl+C/Ctrl+Break stops PresentMon gracefully and persists the partial session with `interrupted=true` (exit 0 with an `interrupted: partial session saved at <dir>` stderr notice); a repeat signal still ends the process. `rwp capture` also accepts `--stop-existing-session` (default off, also stops another tool's session of the same name), and a killed stop prints an ETW-hygiene hint pointing at `logman query -ets` and the troubleshooting entry. Live PresentMon validation on reference hardware stays outstanding.

### Fixed
- Request a graceful PresentMon stop when cancelling from the console-less release GUI (attach-then-signal with no visible window) instead of always falling back to a hard kill; validated with a test helper only, real PresentMon behavior stays outstanding.

## [0.1.0-alpha] - 2026-10-12

### Added
- Add `rwp migrate` for legacy inputs: `--report` normalizes one standalone `rwp.report` JSON file (v1/v2) to the current report schema, and `--session` rewrites one persisted session with a fresh `rwp.session.manifest.v2` manifest and recomputed hashes while preserving the authored report content and provenance (migration never recomputes diagnostics; use `rwp analyze` for that).
- Extend the native GUI without hardware: live process picker, native file/save dialogs for PresentMon paths, live capture progress with backend session events, a stutter-peak chart, two-session Compare, and remediation guidance for session failures.
- Seed the fuzz corpora from existing fixtures: the minimal PresentMon CSV for `csv_strict`/`csv_tolerant`, and the report/session manifest JSON goldens for `report_json`/`session_json`, so the scheduled runs start from valid grammar instead of an empty corpus.
- Complete WP6 properties: serde round-trip across all public types (`rwp-core`, `rwp-analysis`, `rwp-report`, `rwp-session`) and two-run byte-identical JSON determinism for `compare`, `paired_infer`, and `SessionManifest`.

### Fixed
- Validate persisted session content on load: capture frames must carry positive finite timing with non-decreasing timestamps, and telemetry samples must carry in-range percentages, non-negative power, finite temperature, and ordered timestamps; `SystemSample::new` stays guard-free for programmatic use.
- Harden the report JSON reader: `from_json` now rejects stutter events with inverted windows or non-positive magnitudes, non-positive frame statistics, negative variance, and invalid detector settings, closing the deserialization bypass around the constructor contracts.
- Raise the minimum telemetry sampling interval from 1 ms to 10 ms (owner bound): the backend, GUI input minimum, and GUI validation message now agree on 10–5000 ms, since each tick pays two ToolHelp snapshots plus per-thread/per-process handle opens. The default stays 50 ms and the Tauri command passes the value through unchanged.
- Reject report JSON whose diagnostics carry `evidence_score` above 100 in `from_json` (deserialization bypasses the `Diagnostic::new` constructor check); previously-passing out-of-range input is now refused, engine behavior is unchanged.
- Report the actual V2 stutter provenance in `PerformanceReport.analysis_config`: `stutter_minimum_baseline_samples` and `stutter_merge_gap_ms` now reflect the active refresh-aware configuration instead of V1 defaults, and `stutter_absolute_threshold_ms` carries the explicit not-applicable sentinel `0.0` since the V2 rule derives its threshold from the local baseline plus the display refresh period. V1 reports are byte-identical.
- Harden the PresentMon CSV reader: bound input size on the reader path (previously only enforced for file inputs), reject single lines beyond 1 MiB instead of reading them whole, and drop an undecodable truncated tail in tolerant mode instead of failing an interrupted capture.
- Run the GUI history/load/compare IPC commands asynchronously so manifest hashing and report parsing over hundreds of megabytes never block the webview main thread, and skip sessions whose report cannot be parsed instead of failing the entire history listing with one broken entry.
- Give repeated GUI captures unique history folders (`<base>-2`, `<base>-3`, …) instead of failing persistence when the CSV file name is reused, so a finished capture result is never discarded at the end of a long session.
- Preserve the refresh-aware stutter detector and interruption status in `rwp analyze`: analysis now selects V2 when the input manifest records a display refresh rate and writes the manifest interruption flag into the rebuilt report, so re-analysis no longer silently downgrades V2 sessions to V1. `rwp import-presentmon` accepts `--refresh-rate-hz` for refresh-aware imports; non-finite or non-positive rates are rejected.
- Rename legacy `docs/adr/ADR-*.md` records to the indexed `NNNN-*.md` convention as `-supplement-*` detail files alongside the canonical summaries; no decision content was removed and `tools/check_docs.py` still passes.
- Version BackgroundInterference as `background_interference.v3`: a capture where fewer than `background_min_aligned_fraction` (default 0.50) of stutter events overlap background bursts now records `background.low_alignment` contradicting evidence (weight 40) and reports the reduced score instead of the previous maximum; zero alignment still never activates.
- Evaluate `thermal.v2` only after both the frame-sample and documented system-sample minima are met, matching other telemetry-based rules; a short hot/clock-drop trace no longer activates from the frame-sample path alone.
- Reject alpha values at or outside `(0, 1)` consistently in paired inference and A/A CLI calibration; zero was admitted by Rust's lower-inclusive range check and overflowed the minimum-pair recommendation.
- Add property-based stutter invariants for ordered, non-overlapping events and merge-gap enforcement across generated hitch patterns.
- Derive the A/A calibration minimum-pair recommendation from its selected alpha instead of always reporting six; expose and test the same exact-sign-test eligibility calculation used by paired inference.
- Add a GUI opt-out for RWP's extra process/thread telemetry probing; PresentMon frame capture and system CPU/RAM sampling remain active, unavailable process/GPU observations stay absent, and the setting is persisted in manifest provenance.
- Use a named session worker thread and transition the session to `Failed` with an explicit error if the operating system cannot create the worker, rather than leaving the session stuck in `Running`.
- Apply the exact Rust 1.99.0 rustfmt layout for the no-clobber hard-link error arm and the Unix symlink-regression assertion after CI #596 rejected those two formatting hunks on both OS jobs; no behavior changed.
- Publish persisted session files through unique `create_new` temporary files and a no-clobber hard-link step; clean temporary files on write/publish failures and avoid following a planted legacy `.tmp` symlink.
- Describe manifest SHA-256 as corruption/inconsistency detection rather than tamper-proof authenticity, and document that failed multi-file session writes may leave an untrusted partial directory.
- Correct the Markdown escape helper's backslash character literals after CI #593 rejected the invalid multi-backslash Rust `char` literals; the literals now encode one backslash each.
- Escape unredacted Markdown application names (syntax punctuation and entities) and normalize line breaks so captured process names cannot alter report structure.
- Document the existing 1–3,600 second timed-capture contract and test that stderr exactly at its byte limit is not marked truncated.
- Apply the exact Rust 1.99.0 rustfmt output after CI #591 reported three layout-only hunks in the new PresentMon duration/stderr regression tests; behavior and assertions are unchanged.
- Bound PresentMon stderr retention to 64 KiB, drain the remainder to avoid blocking the child, decode invalid UTF-8 lossily, and explicitly mark truncated diagnostics.
- Enforce the existing 1–3600 second UI timed-capture contract in the capture backend, including configurations that set the public duration field directly.
- Omit Process ID from redacted Markdown/HTML output instead of fabricating PID 0; render the redaction marker safely/visibly, and extend CLI fixture E2E coverage.
- Replace the tautological process-group test with a regression check for the module-level Win32 `CREATE_NEW_PROCESS_GROUP` value used by the production spawn path.

### Security hardening
- Apply the exact Rust 1.99.0 rustfmt output to session-history validation and canonical-root regression tests after CI #587 reported four formatting-only hunks; runtime behavior and assertions are unchanged.
- Configure `tauri_build::AppManifest::commands` for the exact seven commands registered by the native application, generating explicit command allow/deny permissions. The build script uses `try_build` and fails the build if permission metadata generation fails.
- Restrict persisted-session identifiers to 1–96 ASCII letters/digits/dots/underscores/hyphens, reject Windows DOS device names and trailing dots, and normalize generated identifiers so they remain valid. Canonicalize session paths and reject entries resolving outside the canonical history root; the history listing skips unsafe names and out-of-root targets. Add regression coverage for reserved names, ADS/colon input, length limits, generated IDs, and root containment.

### System telemetry
- Fix PDH GPU Engine array handling to use the successful fill-call item count, validate it against allocated slots, zero-initialize backing storage, and re-query on `PDH_MORE_DATA` with a three-attempt bound. Treat a zero-size/zero-item sizing result as absent telemetry (`None`), and add regression coverage for shrinking instance counts and out-of-bounds reported counts. Hardware-specific counter behavior remains subject to reference-machine validation.

### Operational diagnostics
- Apply the exact pinned Rust 1.99.0 rustfmt layout for the production `sync::{Arc, Mutex}` import after the test-only atomic imports were removed; formatting only, no runtime behavior change.
- Remove test-only atomic imports from the production logger scope after Windows Clippy reported `unused_imports`; the test module retains its own imports. Logging behavior and limits are unchanged.
- Add a Tauri JSONL rolling logger under the user's local application data directory. The subscriber records only an allowlisted static event identifier, severity, fixed target, and timestamp; arbitrary message/field values are discarded. Logs are controlled by `RWP_LOG_LEVEL`, capped at 5 MiB per active/archive file, retain three archives, reject oversized records, and discard pre-existing active/archive files already above the limit. Logger setup failure does not block application startup. The rolling writer counts its optional JSONL terminator explicitly under the event/file byte limits.
- Apply the pinned Rust 1.99.0 rustfmt layout to the new logging module after CI #583 reported formatting-only diffs on Linux and Windows; runtime logic is unchanged.

### Diagnostic test coverage
- Add rule-matrix coverage that misaligned background bursts do not activate Background Interference, that a low-alignment capture records the contradicting alignment evidence at score 60, and that frame-based and telemetry-based rules stay inactive below their documented minimum sample counts.
- Add CPU-bound contradiction coverage: high GPU frame share records `gpu.strong_share` with contradicting polarity and reduces the score to 30, symmetric with the existing GPU-bound case.
- Add a thermal no-fire case below `minimum_system_samples` and require ten system observations in the thermal fire golden so the fixture matches the documented telemetry gate.
- Add a well-formed PresentMon CSV property that parsed frame times match the serialized input, and a report JSON property that arbitrary text does not panic `from_json`.
- Extend the Criterion statistics bench with a one-million-frame path for the WP6 analyzer budget; default `cargo test` still does not run benches.
- Add regression coverage that adequate frame/system sample counts with absent optional telemetry produce a deterministic `Unknown` result with evidence score 0, rather than fabricated rule activations.

### Test reliability
- Applied the pinned Rust 1.99.0 rustfmt layout to the persistence report fixture after CI #581 reported a format-only failure on Linux and Windows; fixture semantics are unchanged.
- Give each persistence test a unique temporary directory to prevent Windows parallel-test collisions.
- Populate persistence test reports with valid non-empty frame statistics and a matching capture frame; `compute_statistics` correctly rejects empty input. Use the same P99.9 reliability threshold in the test report and computed statistics.

### Diagnostic test coverage
- Expanded the WP6 rule matrix with positive fire cases for aligned background CPU bursts and frame-pacing tail/event-rate evidence, plus CPU and RAM/VRAM no-fire cases for low or absent evidence.
- Corrected the GPU-bound no-fire fixture to set `gpu_busy_ms = None` explicitly while keeping `gpu_time_ms` high, so the regression actually tests the intended missing-observation contract. Rule behavior and thresholds are unchanged.

### Operational diagnostics
- Added `rwp doctor --presentmon <PATH> --bundle <PATH>` with deterministic, redacted preflight output for executable validity, Windows capture permissions, PresentMon version support, and offline operation.
- Hardened bundle creation with create-new semantics, overwrite protection, write/flush error cleanup, and no-path error messages; added byte-determinism and path-redaction regression tests.
- Distinguishes skipped/unavailable preflight checks with `None`, and returns stable exit code `RWP-E6000` when an executed doctor check fails.
- Added the versioned `rwp.doctor.v1` diagnostic bundle format without introducing a new runtime dependency.

### CLI reliability
- Added stable RWP CLI error codes, deterministic exit codes, actionable remediation text, and `--json` machine-readable failures. The taxonomy is defined in `docs/cli-errors.md`; nested PresentMon CSV failures retain CSV classification.

### Build and lint
- Added crate-level documentation to the `rwp-session` and `rwp-cli` build scripts so pinned Clippy runs satisfy the workspace `missing_docs` lint without suppressing warnings or weakening CI.

### Build reproducibility
- Reconciled `Cargo.lock` workspace-package dependency lists with current manifests: removed the stale `rwp-ui` dependency from `rwp-session` and added the declared `rwp-analysis` dependency to `rwp-ui-native`. This addresses CI #562's `--locked` lockfile-update failure without changing registry package versions or weakening any gate.

### Autopilot — WP5
- Added a 64-level explicit JSON nesting guard before persisted manifest deserialization, plus bounded manifest serialization and regression tests for nested arrays and bracket characters inside strings.
- Bounded CLI loading of manifest, capture, report, and telemetry JSONL; applied the 64-level depth guard to capture/report and each telemetry record, and enforced the telemetry sample-count limit before appending each sample.
- Hardened the public report JSON deserializer with a 64 MiB input limit and a 64-level nesting guard, including regression coverage for deeply nested JSON and brackets inside strings.
- Closed the GUI persisted-session JSON bypass: history listing/loading and Compare now use bounded report reads and the validated report deserializer; telemetry counting is bounded to 256 MiB and 5,000,000 samples, including post-manifest-verification file replacement races.
- Added a test-first regression for the GUI bounded session-file reader; Rust verification remains pending because the current automation environment has no local checkout/toolchain.
- Added PresentMon CSV resource bounds: 512 MiB maximum input size and 10,000,000 frame rows, with explicit actionable parser errors and regression coverage.
- Added `docs/threat-model.md` covering Tauri/webview, PresentMon, CSV/JSON, filesystem trust boundaries, residual risks, and explicit resource limits for hostile input.
- Added `docs/acceptance/gui.md` as the manual GUI acceptance checklist for capture lifecycle, persistence, compare/reporting, IPC security, and evidence recording.
- Removed the `rwp-session` → `rwp-ui` dependency; live session orchestration now returns analysis state and the application layer builds the dashboard model.
- Corrected the session persistence regression test attributes while re-auditing AUD-17.
- Hardened PresentMon executable launch validation to require a canonical regular file named `PresentMon.exe`; path rejection is explicit and remains offline/local.
- Added a Tauri v2 capability/permission boundary exposing only the seven registered session IPC commands to the main window; no filesystem, shell, HTTP, updater, or other plugin permission is granted.
- Added a Windows Job Object with `KILL_ON_JOB_CLOSE` around the PresentMon child; failure to establish containment aborts capture rather than leaving an unowned child process.

### Autopilot — WP4
- Added native GUI Compare using two manifest-verified persisted sessions and the deterministic comparison engine.
- Added default-redacted human-readable report output with explicit `--redact` control.

### Autopilot — WP4
- Added immutable native GUI session history under the local application-data directory, with manifest verification before listing/loading historical sessions.
- Added deterministic build, hardware, PresentMon, QPC, and capture-option provenance fields to persisted session manifests; unavailable observations remain `None`.
- Added a real CLI fixture E2E covering `import-presentmon → analyze → compare → report` and asserting persisted provenance.
- Added build-script metadata embedding for RWP Git SHA, Cargo target, and build profile.

### Autopilot — WP3
- WP4 foundation: `PerformanceReport` v2 now persists deterministic diagnostics produced by live session analysis, while `#[serde(default)]` preserves compatibility with older report JSON that has no diagnostics field.

- Added bounded `SamplerStats` instrumentation for mean and rolling p99 sample duration; retained the 50 ms default pending reference-hardware measurement and documented the ToolHelp baseline in ADR-0021.
- Replaced target-process NVIDIA device-0 attribution with process-matched adapter selection; added vendor-neutral Windows GPU Engine per-PID utilization at a documented 1 Hz cadence, preserving `None` when unavailable or ambiguous.
- Hardened CPU per-thread sampling to use `THREAD_QUERY_LIMITED_INFORMATION` and timestamp the thread-read interval around enumeration; accuracy validation remains hardware-dependent.
- Versioned GPU-bound evidence as `gpu_bound.v2`, using PresentMon `GPUBusy` as the frame-level GPU-work signal; retained `None` semantics when GPUBusy is unavailable.
- Added arbitrary-input property coverage for the PresentMon CSV parser.
- Versioned Thermal and FramePacing as v2 rules: Thermal now requires hot-temperature plus co-observed GPU clock reduction; FramePacing requires persistent stutter rate plus P99/P50 tail evidence, with explicit serialized thresholds.

- Added versioned refresh-aware stutter rule v2 with deterministic 60/144/240 Hz golden coverage; legacy stutter rule v1 remains available with explicit provenance.
- Added Windows active-display refresh-rate discovery with explicit missing/ambiguous semantics and wired V2/V1 selection plus refresh provenance into live session reports; corrected Windows API compatibility for windows-sys 0.61.2 and default refresh-rate sentinel handling.

### Autopilot — WP2

- Captured PresentMon stderr without blocking the capture process and included it in non-zero exit errors.
- Added explicit `--stop_existing_session` control, disabled by default, and exposed it through the session configuration.

### Autopilot — WP1

- Corrected contradiction evidence polarity and versioned BackgroundInterference as background_interference.v2 with aligned-event provenance.
- Raised default paired-inference minimum to 6 and validate minimum_pairs against alpha.
- Standardized frame timestamps to a non-decreasing policy.
- Added explicit PresentMon interrupted-tail parsing and report.v2 interruption provenance.
- PresentMon cancellation now uses a Windows process group, with immediate kill fallback when `CTRL+BREAK` delivery is unavailable and a bounded graceful wait otherwise.
- Downgraded non-essential Windows telemetry failures to absence plus deterministic session warnings; QPC initialization and invalid process configuration remain fatal.
- Added a panic boundary around the native Tauri session worker so worker panics transition the session to Failed.

### Autopilot — WP0

- Established docs/autopilot/STATE.md as the release-work tracking source.
- Split CI into Windows full gates and Linux platform-neutral checks.
- Removed CI lockfile generation; Cargo gates now require the committed Cargo.lock.
- Added locked release build and NSIS installer artifact/checksum gates.
- Pinned GitHub Actions used by CI to immutable commit SHAs.
- Added Dependabot configuration and documentation consistency checks.
- Made docs/project-ledger.md the single milestone-status source referenced by README.

### M0

- Established product specification and milestone acceptance criteria.
- Established workspace crate boundaries and engineering constraints.
- Established initial ADR set, statistical definitions, diagnosis-rule contract, JSON Schema v1 draft, test strategy, and risk register.
- Added Windows CI scaffold for formatting, Clippy, tests, dependency policy, and RustSec auditing.

### M1

- Implemented normalized core sample and diagnostic contracts.
- Implemented deterministic frame-time statistics using nearest-rank percentiles.
- Added both percentile-frame-time and worst-frame-average low-FPS definitions.
- Added robust local-median stutter detection with event merging and parameter provenance.
- Added unit tests, property tests, golden scenarios, deterministic snapshots, and Criterion benchmark coverage.

### M2

- Implemented an out-of-process PresentMon runner with process-name and process-ID targeting.
- Added explicit v2 metrics and QPC capture arguments with timed and process-exit lifecycle options.
- Added a deterministic CSV adapter that validates schema, process identity, monotonic QPC timestamps, and frame-time invariants.
- Normalized PresentMon QPC timestamps into the platform-neutral capture-relative nanosecond contract used by rwp-core.
- Preserved absent optional telemetry using None instead of sentinel zeroes.

### M3

- Added authoritative Windows QPC timing, system CPU/RAM telemetry, and optional NVIDIA NVML sampling.
- Added target-process per-thread CPU deltas from ToolHelp/GetThreadTimes.
- Added privacy-preserving background-process CPU activity with redacted per-run IDs.
- Added PDH total CPU backend with explicit collection semantics.

### M4

- Added capture.v1 JSON input and report.v1 deterministic JSON output contracts.
- Added serializable frame statistics and stutter-event results.
- Added explicit analysis configuration metadata to reports.
- Added rwp analyze CLI orchestration with stdout or file output and optional pretty JSON.

### M5

- Added deterministic evidence-based diagnosis rules for CPU-bound, GPU-bound, frame-pacing, thermal, and background interference.
- Added explicit threshold provenance and contradiction-aware heuristic scoring.
- Added deterministic Unknown fallback for insufficient evidence.

### M6

- Added deterministic single-run baseline/candidate comparison with explicit same-scenario and sample-coverage gates.
- Added paired P99 run-level inference with practical-equivalence margin, minimum-pair eligibility, exact two-sided sign test, and non-causal interpretation boundary.
- Added invalid-pair and empty-run validation plus serialized comparison configuration provenance.
- Added rwp compare CLI orchestration for deterministic report comparisons.

### M7

- Added a framework-neutral dashboard view model for Overview, Stutters, Diagnostics, and Compare.
- Added deterministic metric cards, stutter rows, diagnostic evidence cards, and comparison summaries.
- Kept UI navigation state separate from immutable analysis/report data.

### M8

- Added shared-QPC live session orchestration across PresentMon and Windows telemetry.
- Added early PresentMon process-exit handling and cancellation-aware session lifecycle.
- Added deterministic report, diagnosis, and dashboard generation after live capture.
- Added Tauri start/stop/status/reset IPC and native live-session controls.
- Added ADR-0014 documenting live-session ownership, shared timebase, and interruption semantics.

### M9/M10

- Added normalized RAM/VRAM telemetry and deterministic memory/VRAM pressure rules.
- Added Windows NSIS bundling configuration and release packaging documentation.
- Preserved PresentMon as an external, user-supplied executable.
