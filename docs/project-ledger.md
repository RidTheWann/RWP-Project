# Project Ledger (milestone history; authoritative live status is docs/autopilot/STATE.md)

## Snapshot (0.1.0 ALPHA; for live WP/AUD/RC status see docs/autopilot/STATE.md)

- Milestone history ends at M10 below; current alpha state (Fase 0-7 done, hardware/sign-off blocked) lives in docs/autopilot/STATE.md.
- Workspace version: 0.1.0 (alpha; stays 0.1.0 per docs/local-gates.md §5)
- MSRV: Rust 1.91.0
- Development toolchain: Rust 1.99.0 (pinned; every gate uses --locked)
- Primary target: x86_64-pc-windows-msvc

## ADR index (titles match docs/adr/*.md filenames; supplements documented in docs/adr/README.md; numbers never renumbered)

- ADR-0001 workspace boundaries: accepted
- ADR-0002 external PresentMon capture: accepted
- ADR-0003 version-tolerant PresentMon CSV adapter: accepted
- ADR-0004 CPU per-thread telemetry source selection: accepted
- ADR-0005 QPC canonical timebase: accepted
- ADR-0006 nearest-rank statistics: accepted
- ADR-0007 schema and benchmark identity: accepted
- ADR-0008 live-session metrics: accepted
- ADR-0009 run-level A/B inference and equivalence margins: accepted
- ADR-0010 native GUI technology: accepted
- ADR-0011 performance classification: accepted
- ADR-0012 deterministic report serialization: accepted
- ADR-0013 release artifact provenance: accepted
- ADR-0014 live session orchestration and shared QPC timebase: accepted
- ADR-0015 PresentMon graceful cancellation and fallback: accepted
- ADR-0016 PresentMon version matrix: accepted
- ADR-0017 refresh-aware stutter rule v2: accepted
- ADR-0018 active-display refresh-rate source: accepted
- ADR-0019 thermal and frame-pacing evidence rules: accepted
- ADR-0020 target GPU adapter selection and vendor-neutral GPU telemetry: accepted
- ADR-0021 sampler overhead measurement instrumentation (+ supplement): accepted
- ADR-0022 deterministic session persistence and manifest provenance (+ supplement): accepted
- ADR-0023 persisted CLI analysis, paired comparison, and A/A calibration (+ supplement): accepted
- ADR-0024 GUI session history and provenance manifest (+ supplement): accepted
- ADR-0025 opt-out for process and thread telemetry probing: accepted
- ADR-0026 background-interference alignment contradiction: accepted
- ADR-0027 sampler overhead single-device baseline and cadence lock: accepted
- ADR-0028 PresentMon distribution and verification: accepted
- ADR-0029 WebView2 runtime install mode: accepted
- ADR-0030 no automatic updates: accepted

## M1 completion

Implemented:
- normalized FrameSample, SystemSample, ProcessThreadSample, BackgroundActivity, StutterEvent;
- Diagnostic and evidence contracts;
- mean, average FPS, median, sample variance, standard deviation;
- nearest-rank P0.1/P1/P5/P10/P50/P95/P99/P99.9;
- two explicit low-FPS definitions;
- robust local-median stutter detection with event merging and parameter provenance;
- unit, property, snapshot, and benchmark tests.

Validated by Windows CI:
- Rust 1.99 formatting and Clippy;
- workspace tests;
- MSRV 1.91 compile and tests;
- cargo-deny;
- cargo-audit.

## M2 progress

Implemented in `rwp-capture`:
- external PresentMon process runner with no in-process injection or hooks;
- explicit `--v2_metrics` + `--qpc_time` capture contract;
- process-name and process-ID targeting;
- timed and process-exit capture modes;
- output-file collision protection;
- lightweight CSV parser for the selected v2 schema;
- optional telemetry preservation using `None` for absent values;
- single-process identity enforcement;
- monotonic QPC validation;
- QPC-to-capture-relative-nanoseconds normalization;
- conversion into the existing platform-neutral `FrameSample` contract.

M2 intentionally leaves Windows QPC frequency discovery to `rwp-system` in M3 and leaves diagnosis, reporting, A/B inference, and optimization outside this milestone.

## Verification status

M1 is merged to `main` at `4cfacc58d03f61fd6caefc80f408518d0bb36821`.

M2 verification is performed by the same Windows CI gate before merge. The parser is tested against deterministic in-memory CSV samples; live PresentMon runtime validation remains an acceptance test for a Windows environment with a real PresentMon executable and target process.

## Reproducibility

The M1 Windows CI generated and uploaded a verified `Cargo.lock` artifact. M2 introduces no new external dependencies, so the lock graph is unchanged. The source-tree lockfile remains a release-readiness task once the generated lock can be committed through a normal repository write path.

## M3 progress

Implemented in `rwp-system`:
- authoritative Windows QPC frequency and capture-relative nanosecond clock;
- system CPU utilization from GetSystemTimes delta accounting;
- physical RAM usage from GlobalMemoryStatusEx;
- optional NVIDIA telemetry via NVML for GPU utilization, VRAM usage, graphics clock, power draw, and GPU temperature;
- absence-preserving behavior when NVML is unavailable or an individual NVML metric is unsupported.

M3 Windows telemetry foundation is merged and validated.
- target-process per-thread CPU telemetry via ToolHelp + GetThreadTimes;
- redacted background-process CPU activity via ToolHelp + GetProcessTimes;
- total CPU PDH backend with explicit error/absence semantics;
- no process names, paths, command lines, or window titles are retained by the background-activity contract.

ETW remains deliberately unimplemented until provider/session semantics are validated independently.

## M4 progress

Implemented and merged:
- capture.v1 JSON input contract;
- report.v1 deterministic JSON output;
- complete serialized frame-time statistics;
- serialized stutter events with detector-parameter provenance;
- explicit analysis configuration metadata in every report;
- rwp analyze --input <capture.json> [--output <report.json>] [--pretty].

M4 does not perform diagnosis, A/B inference, recommendation, or optimization.

## Verification status

M1-M4 are validated by the Windows CI gate:
- Rust 1.99 formatting and Clippy;
- workspace tests;
- MSRV 1.91 compile and tests;
- cargo-deny;
- cargo-audit.

## M5 progress

Implemented and merged:
- deterministic CPU-bound, GPU-bound, frame-pacing, thermal, and background-interference rules;
- explicit RuleThresholds with serialized provenance;
- contradiction-aware heuristic scoring;
- Unknown fallback when evidence is insufficient;
- deterministic score/category ordering.

M5 intentionally does not claim causal proof or probability. Memory-pressure and VRAM-pressure rules are implemented from mean normalized utilization with stutter-overlap support; the streaming-overhead category remains deferred until normalized encoder telemetry exists.

## Verification status

M1-M5 are validated by the Windows CI gate:
- Rust 1.99 formatting and Clippy;
- workspace tests;
- MSRV 1.91 compile and tests;
- cargo-deny;
- cargo-audit.

## M6 progress

Implemented on the M6 branch:
- deterministic single-run descriptive metric comparison;
- matched run-level P99 inference;
- explicit practical-equivalence margin;
- exact two-sided sign test;
- minimum-pair gate;
- Improved, Regressed, Equivalent, and Inconclusive outcomes;
- duplicate-pair and invalid-metric validation;
- ADR-0009 documenting the non-causal inference boundary.

The M6 engine never labels a single-run delta as causal evidence.

## M6 progress

Implemented and merged:
- deterministic single-run baseline/candidate metric deltas;
- explicit same-scenario gate;
- sample-count comparability for descriptive comparison;
- validated report.v1 deserialization for comparison inputs;
- matched P99 run-level inference;
- per-pair sample-coverage eligibility with excluded-pair accounting;
- practical-equivalence margin and exact two-sided sign test;
- duplicate-pair, invalid-metric, empty-run, and sample-count validation;
- configuration provenance in serialized comparison results;
- `rwp compare` CLI command;
- ADR-0009 documenting the non-causal inference boundary.

M6 does not claim that a single-run delta is causal evidence.

## M7 progress

Implemented on the M7 branch:
- framework-neutral dashboard view model;
- stable Overview, Stutters, Diagnostics, and Compare tabs;
- deterministic metric-card mapping from report.v1;
- diagnostic evidence presentation mapping;
- single-run comparison presentation mapping;
- isolated navigation state so renderer interaction cannot mutate analysis results.

The M7 model layer intentionally adds no GUI framework dependency and preserves the Rust 1.91 MSRV. A native renderer remains a separate adapter.

## M7 progress

Implemented and validated on the M7 branch:
- framework-neutral dashboard model;
- Tauri 2 native renderer shell;
- stable Overview, Stutters, Diagnostics, and Compare navigation;
- restrained Windows-first visual design with no frontend framework;
- renderer isolated from analysis semantics and live capture.

ADR-0010 selects Tauri 2 for the native renderer. Current Tauri 2.12 documents Rust 1.90 MSRV, compatible with this repository's Rust 1.91 baseline. egui 0.36.2 was not selected because its documented MSRV is Rust 1.95.

## M8 progress

Implemented on the M8 branch:
- shared QPC timebase API for Windows telemetry;
- PresentMon CSV normalization against an explicit shared QPC origin;
- explicit error when PresentMon timestamps predate the session origin;
- early PresentMon process-exit detection;
- `rwp-session` orchestration joining PresentMon, Windows telemetry, deterministic report construction, diagnosis, and dashboard mapping;
- Tauri IPC commands for start, stop, status, and reset;
- vanilla UI controls for live session configuration and result rendering;
- one-session-at-a-time lifecycle state isolated from the analysis engine.

Manual stop is modeled as an interrupted capture: the orchestrator terminates PresentMon and attempts to parse the CSV already produced. No partial values are fabricated.

## Verification status

M7 CI is green. M8 is merged and its post-merge CI gate is green for Rust checks and MSRV 1.91.

## M9 progress

Implemented and merged:
- normalized RAM and VRAM utilization fields in `SystemSample`;
- Windows physical-memory utilization derived from committed/available memory totals;
- NVIDIA VRAM utilization derived from used/total adapter memory;
- deterministic `MemoryPressure` and `VramPressure` diagnostic rules using normalized utilization;
- correlation of memory-pressure signals with stutter windows;
- absence-preserving telemetry semantics and threshold provenance;
- deterministic regression coverage for high normalized RAM/VRAM utilization.

M9 deliberately preserves the existing non-causal diagnostic boundary: correlation is evidence, not causal proof.

## Verification status

M9 PR #19 was squash-merged after CI run #207 completed successfully against head `6205a69dce71da324ac9111721bafa4e93b963f0`. Both `Rust checks` and `MSRV 1.91.0` passed every reported step. (Arsip era-CI: nomor PR/run dan workflow adalah catatan sejarah, bukan otoritas verifikasi — otoritasnya `docs/local-gates.md`.)

Release-readiness debt remains explicit: the source-tree `Cargo.lock` and installer/release packaging are not part of the merged M9 implementation.

## M10 progress

Implemented and merged:
- enabled Tauri Windows NSIS bundling;
- configured the existing `icons/icon.ico` as the application/bundle icon;
- documented the release packaging contract and installer acceptance gates;
- kept PresentMon explicitly external and user-supplied, preserving the out-of-process capture boundary;
- documented remaining release gates: committed source-tree `Cargo.lock`, CI installer build/artifact retention, and Windows runtime acceptance.

M10 does not silently redistribute or download PresentMon and does not introduce automated process discovery.

## Verification status

M10 PR #21 was squash-merged after CI run #211 completed successfully. Both `Rust checks` and `MSRV 1.91.0` passed every reported step. (Arsip era-CI: nomor PR/run dan workflow adalah catatan sejarah, bukan otoritas verifikasi — otoritasnya `docs/local-gates.md`.)

## Next

- M11 source is not defined in the current repository roadmap; do not invent a milestone until an authoritative roadmap/spec entry exists.
- Preserve the product invariant: Measure -> Analyze -> Diagnose -> Compare -> Explain -> Recommend -> Optimize.
- Keep optimization behavior and causal claims out of scope until their evidence and product contracts are explicitly defined.
