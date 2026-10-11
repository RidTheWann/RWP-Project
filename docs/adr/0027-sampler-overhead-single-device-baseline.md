# ADR-0027 — Sampler overhead single-device baseline and cadence lock

- Status: accepted
- Date: 2026-10-10
- Scope: Fase 3 / AUD-10 / RC-05

## Context

AUD-10 requires measured sampler overhead before any low-overhead claim.
ADR-0021 fixed the instrumentation (`SamplerStats`: count, mean, rolling p99
over 4096 samples) and kept the 50 ms default pending reference-hardware
measurement. A prior 10-minute probe (other machine, 69-thread target) reported
mean 27.10 ms, rolling p99 29.97 ms, and 53.74% of one core. Fase 3 re-baselines
on one local reference box and evaluates the NtQuery-single-call alternative
against the current ToolHelp-two-snapshot design.

## Method (honest single-device label)

- Box: Windows 11 Pro Build 26200 x64, AMD Ryzen 5 5600G (6C/12T), 16 GiB RAM;
  non-admin (ETW and cycle-time cross-checks unavailable).
- Temporary release-profile probe (measurement only, removed after collection):
  full-probing `WindowsSystemSampler` against existing `explorer` (208 threads
  at selection) for 12,000 ticks at 50 ms cadence, then a 2,400-tick
  system-only comparison, then 200-iteration ToolHelp snapshot-syscall timings.
- Default interval unchanged at 50 ms throughout.

## Results

| metric | value |
|---|---|
| full-probing samples | 12,000 |
| full mean sampler duration | 18.855 ms |
| full rolling p99 sampler duration | 28.352 ms |
| system-only mean / p99 | 0.016 ms / 0.088 ms |
| mean target-thread observations per tick (max) | 210.66 (218) |
| mean background-activity observations per tick (max) | 56.44 (69) |
| probe-process CPU | 225.7 s over 950.5 s wall = 23.75% of one core |
| snapshot-syscall process mean / p99 | 1.106 ms / 1.666 ms |
| snapshot-syscall thread mean / p99 | 1.086 ms / 1.296 ms |

Snapshot creation (~2.2 ms combined) is ~12% of a full sample; the remaining
~17 ms is per-thread/per-process handle opens plus enumeration. System-only
sampling is three orders of magnitude cheaper (0.016 ms) because it skips
process/thread enumeration entirely.

## Decision

1. LOCK the 50 ms default cadence on current data: mean (18.9 ms) and p99
   (28.4 ms) stay below the tick budget with no overrun on this box, and the
   numbers are the same order as the prior independent baseline. No default
   change in Fase 3.
2. DEFER the NtQuery-single-call redesign. It could remove both snapshots and
   the per-thread open calls, but it needs hand-rolled `ntdll` bindings for a
   semi-documented API (`SYSTEM_PROCESS_INFORMATION` layout is
   version-sensitive), a new `SAFETY` surface, and fallback logic — a product
   change that belongs to a later bump-plus-ADR batch, not to measurement-only
   Fase 3.
3. RC-05 stays open: full probing on a heavy target does not meet the
   ≤1%-total-CPU / ≤2 ms-p99 release target on this box; the system-only mode
   (0.016 ms) is the documented low-overhead option until reference-hardware
   and capture-wide measurements say otherwise.

## Consequences

- The 50 ms default is now data-backed on one device instead of assumed.
- No product behavior, threshold, version, or gate changes in this batch.
- Re-running this protocol on reference hardware only needs the deleted probe
  recipe above; ETW/cycle-time cross-checks still require an admin session.
