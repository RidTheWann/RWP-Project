# ADR-0033 — Capture-wide overhead protocol and single-device repeat (DRAFT)

- Status: DRAFT (not accepted; lock-vs-revise verdict explicitly deferred
  pending reference-hardware data — see Decision below)
- Date: 2026-10-15
- Scope: RC-05 follow-up / AUD-10 overhead continuation

## Context

ADR-0027 baselined sampler-tick cost on one reference box and locked the
50 ms cadence while leaving RC-05 open (full probing: mean 18.855 ms, p99
28.352 ms, 23.75% of one core — far from the ≤1%-total-CPU / ≤2 ms-p99
target). The remaining question is capture-wide cost (PresentMon process +
sampler + GUI) on a reference workload. This draft records the protocol, a
same-box repeat of the sampler legs plus a new GUI-idle leg, and what stays
blocked.

## Method (honest single-device label)

- Box: same reference box as ADR-0027 (Windows 11 Build 26200 x64, AMD
  Ryzen 5 5600G 6C/12T, 16 GiB RAM; non-admin, so ETW/cycle-time legs stay
  unavailable). No concurrent gate/build load was issued during the windows;
  background OS activity was otherwise uncontrolled.
- Temporary release-profile probe (measurement only, removed after
  collection, same precedent as ADR-0027): `WindowsSystemSampler` ticks at a
  fixed 50 ms cadence against the live `explorer.exe` PID, using the public
  `sampler.stats()` (`SamplerStats` mean/rolling-p99) plus own-process CPU
  from `GetProcessTimes` start/end deltas (`cpu_pct_one_core =
  100 * cpu_seconds / wall_seconds`); per-tick observation means from
  `SystemSample.process_threads/background_activity` lens.
- Full-probing leg: 2,400 ticks (~120 s; shorter than ADR-0027's 12,000, so
  the rolling-p99 window of 4096 never fills — p99 values are NOT directly
  comparable across the two runs).
- System-only leg: 1,200 ticks (~60 s).
- GUI-idle leg: debug `target/debug/rwp-ui-native.exe` (fresh `cargo build
  --locked -p rwp-ui-native`), window open, no capture; 11 × 5 s samples of
  `TotalProcessorTime` deltas (`% of one core`) plus working set.
- NOT run here: PresentMon process cost (no `PresentMon.exe` on this
  machine — verified absent via `Get-Command`, no `C:\Tools` bundle) and any
  reference-workload app (`tools/rwp-testapp` does not exist in tree).

## Results (this box, 2026-10-15)

| leg | profile | ticks | mean ms | p99 ms | CPU % one core | obs/tick |
|---|---|---|---|---|---|---|
| full probing (ADR-0027) | release | 12,000 | 18.855 | 28.352 | 23.75 | 210.66 thr / 56.44 bg |
| full probing repeat | release | 2,400 | 22.072 | 23.757 | 43.75 | 169.08 thr / 70.97 bg |
| full probing repeat | debug | 2,400 | 23.201 | 25.288 | 46.09 | (not recorded) |
| system-only (ADR-0027) | release | 2,400 | 0.016 | 0.088 | ~0 | — |
| system-only repeat | release | 1,200 | 0.019 | 0.073 | 0.08 | 0 / 0 |
| GUI idle (new leg) | debug | 11 × 5 s | — | — | ~0.11 mean (max sample 0.93) | WS 37.4 MB |

Reading (no new claims beyond the table): full probing stays the same order
of magnitude across runs but varies substantially run to run (mean ±20%, CPU
24%→44% with different thread/background populations), so single-point
numbers must not drive target changes. The cheap legs reproduce tightly
(system-only 0.016→0.019 ms; GUI idle ≈ zero). In every run the tick p99
sits comfortably under the 50 ms budget.

## Owner protocol (blocked legs, for the hardware batch)

1. Install PresentMon v2 (record version + SHA-256) and provide or point at a
   reference workload with a stable thread population.
2. With the workload steady, run a timed `rwp capture` (or GUI capture) and
   sample, at 5 s cadence: PresentMon `% Processor Time` (per-process),
   the RWP sampler tick stats (probe recipe above against the same target),
   and the RWP UI process CPU while its window idles.
3. Time CTRL_BREAK-to-exit on real PresentMon across several runs including
   one loaded run (Batch 7 measurement step in `docs/acceptance/smoke-test.md`).
4. Paste all samples plus manifest hashes into STATE before any target talk.

## Decision (deferred — no verdict without reference data)

- Keep RC-05 OPEN and the 50 ms default LOCKED (ADR-0027 stands; nothing
  above overturns it: ticks still fit budget, full probing still misses the
  release target by an order of magnitude, system-only stays the compliant
  option).
- Lock-vs-revise options for the owner once blocked legs report: (a) keep
  RC-05 as the release bar and require system-only/low-probing modes for
  compliance; (b) revise RC-05 to a tiered target (e.g. separate budgets for
  full-probing diagnostics vs lightweight capture). No revision is proposed
  here — that would be a baseless decision.
- ETW/admin legs remain an honest boundary (non-admin box).

## Consequences

- No product behavior, threshold, version, schema, or gate changes.
- Re-running needs only the recipe above; the temporary probe file was
  removed after collection per repo precedent.
