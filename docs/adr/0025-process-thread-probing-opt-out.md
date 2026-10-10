# ADR-0025 — Opt-out for process and thread telemetry probing

- Status: accepted
- Date: 2026-10-09
- Scope: WP3 / AUD-09 / AUD-10

## Context

Live capture currently samples target-process threads, background process activity,
process-selected GPU telemetry, and the target process's display refresh rate. This
additional inspection can increase observer overhead and may be undesirable with software
that restricts process inspection. PresentMon remains an out-of-process dependency for
frame capture regardless of the option.

## Decision

1. Add a `no_process_probing` capture option, defaulting to `false` so existing callers keep
   their current behavior.
2. When enabled, construct the system-only Windows sampler. Do not enumerate process/thread
   activity, initialize target-process NVML/PDH GPU telemetry, or resolve the target process's
   display refresh rate.
3. Keep system CPU/RAM sampling and out-of-process PresentMon frame capture active.
4. Represent omitted process/thread/GPU observations as absent, not zero.
5. Expose the option in the native GUI and persist it as optional provenance in
   `rwp.session.manifest.v2`. The provenance field is optional and defaults when absent so
   existing v2 manifests remain readable.
6. Describe this as reducing RWP's extra probing, not as an anti-cheat compatibility
   guarantee: PresentMon still captures the selected PID.

## Consequences

- Users can turn off RWP's additional process inspection without disabling frame capture.
- CPU-bound/background-interference evidence and target-GPU fields may be unavailable for
  those sessions; diagnostic rules must preserve the existing `None`/Unknown semantics.
- The selected mode is visible in persisted provenance for reproducibility.
- The option does not replace reference-machine overhead or anti-cheat acceptance.
