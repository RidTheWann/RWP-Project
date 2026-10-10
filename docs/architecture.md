# Architecture

Data flow:

Capture + telemetry + system state
-> normalization to QPC-relative time
-> deterministic analysis
-> diagnostic rules
-> report

M1 implements the normalized contracts and deterministic statistics/stutter stages only.

## Boundaries

rwp-core: platform-neutral types and validation.

rwp-analysis: pure functions over normalized inputs; no Windows APIs, no file/network I/O.

rwp-capture: PresentMon process and CSV boundary, M2.

rwp-system: Win32, PDH, ETW, NVML boundary, M3.

rwp-diagnostics: deterministic rules, M5.

rwp-report: JSON/Markdown/HTML, M4.

rwp-cli: application orchestration, M4.

rwp-ui: separate-process GUI, M7.

## Determinism

The analysis layer uses fixed formulas, stable sorting with total_cmp, explicit rule/parameter ordering, and no process-global random state.

## Time

QPC-derived capture-relative integer nanoseconds are canonical. Wall-clock time is metadata only.

## Observer isolation

GUI is never part of the measurement hot path. Capture remains out-of-process and contains no DLL injection, hook, or overlay mechanism.
