# Measurement and Analysis Methodology

This document defines what RWP measures, how it analyzes, and what validates
each step. Every number cited here comes from a recorded local measurement
(STATES entries referenced inline); nothing here is a hardware claim.

## 1. Timebase

All timestamps are normalized to Windows QPC nanoseconds through one shared
origin per session (`QpcTimebase`). PresentMon QPC ticks and telemetry
timestamps share this base, so frame/telemetry overlap is computed on one
clock. See ADR-0005.

## 2. Frame statistics

Percentiles use the nearest-rank method over sorted frame times
(`percentile * n`, ceiling, 1-indexed). The 1% low is `1000 / P99` and the
0.1% low is `1000 / P99.9`. Mean FPS of the worst 1% (0.1%) averages the
reciprocal FPS of the slowest 1% (0.1%) of frames, minimum one frame.
Definitions: `docs/statistics.md`.

## 3. Stutter detection

Two versioned detectors; reports record which one ran:

- `stutter.v1.absolute`: a frame is a candidate when it exceeds both
  `multiplier * local-median-baseline` and `absolute_threshold_ms`
  (defaults: multiplier 2.0, 15-frame half-window, minimum 7 neighbors,
  50 ms merge gap, 30 ms absolute threshold).
- `stutter.v2.refresh_aware`: threshold
  `max(multiplier * baseline, baseline + refresh_period_ms)` with
  `refresh_period_ms = 1000 / refresh_rate_hz`. Used only when an unambiguous
  active-display refresh rate is available; otherwise the report falls back to
  V1 explicitly and never synthesizes a rate.

Candidates within `merge_gap_ms` fuse into one event carrying start/end
timestamps, peak frame time, baseline, and candidate count. See ADR-0017.

## 4. Diagnosis rules and thresholds

Rules, defaults, and evidence semantics: `docs/diagnostics.md` (single source
of truth, serialized into every diagnostic's parameters). Headlines:

- Rules are versioned (`rule_id`); behavior changes require a new version so
  old results stay reproducible.
- Evidence scores are heuristic weights (support minus contradiction, capped
  0..100), not probabilities.
- Absent telemetry stays absent (`None`); nothing is fabricated as zero.
- Insufficient evidence yields `Unknown` as a first-class result.
- Every default is an engineering threshold, not a hardware-derived boundary;
  reference-hardware calibration is outstanding (see `docs/limitations.md`).

## 5. Comparison and paired inference

A/B comparison is descriptive-only: metric deltas with a change threshold
(default 1.0%) and direction (Improved/Regressed/Unchanged); different
scenarios are explicitly inconclusive, never adjusted.

Paired inference uses an exact two-sided sign test over informative pairs
(outside the equivalence margin, default 1.0%). The minimum-pair requirement
derives from the chosen alpha (`minimum_pairs_for_alpha`; 6 at alpha 0.05),
so an underpowered design reports Inconclusive instead of a false verdict.
A/A calibration recommends the margin from same-scenario variability. See
ADR-0009.

## 6. Determinism contract

Same inputs plus same configuration produce byte-identical JSON: stable
ordering, explicit schema/rule versions, no wall-clock or unordered maps in
persisted content. Covered by round-trip and two-run determinism tests.

## 7. Sampler overhead (measured)

Single-device baseline (Windows 11 Build 26200 x64, Ryzen 5 5600G, 16 GiB,
non-admin, release profile, 50 ms cadence, ~211 target threads): full-probing
mean 18.855 ms with rolling p99 28.352 ms over 12,000 ticks; system-only mode
0.016 ms mean. Probe-process CPU was 23.75% of one core. Snapshot creation is
~2.2 ms of a full sample; the rest is handle opens plus enumeration. A prior
independent run on another box reported the same order (27.10 ms mean,
29.97 ms p99). The 50 ms default is locked on this data (ADR-0027); the
release overhead target (RC-05) stays open.

## 8. Validation basis

- Synthetic golden traces (60/144/240 Hz) and property tests (ordering,
  non-overlap, merge gap, percentile-vs-naive-reference, determinism).
- Line coverage ≥85% per gate target; mutation kill rate 97.6% against a ≥70%
  gate with call-site-triaged equivalents.
- Seeded input robustness: corpus-backed no-panic smoke tests over the CSV
  and JSON boundaries (libFuzzer execution itself is Linux-deferred).
- Open: reference-hardware accuracy, multi-device validation, real PresentMon
  fixtures, ETW/cycle-time cross-checks (need an admin session).
