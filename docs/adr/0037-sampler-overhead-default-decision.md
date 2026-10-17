# ADR-0037 — Sampler overhead default decision (option A accepted)

- Status: ACCEPTED (option A: default system-only, full probing opt-in) —
  owner-approved 2026-10-17; implementation rides the next versioned change
  (see Decision)
- Date: 2026-10-17
- Scope: RC-05 / AUD-10 follow-up to ADR-0027 (baseline) and ADR-0033 (protocol)

## Context

RC-05 requires sampler overhead within ≤1% total CPU and p99 single-sample
duration ≤2 ms at the default cadence. The GUI default keeps full process +
thread probing on (`no_process_probing=false`), while system-only mode is the
documented low-overhead option (ADR-0025, `docs/diagnostics.md`). Three
single-device measurement legs now exist on the same reference box, all against
the 50 ms cadence; a per-N-tick probing mode does not exist in the tree
(verified by search: no decimation/stride option in `rwp-system`/`rwp-session`),
so it can only be reasoned about, not measured. Game-load and PresentMon-cost
legs remain blocked (non-elevated shell, no installed game).

## Method (Batch 4 leg, same reference box)

- Box: AMD Family 25 Model 80 (Zen 3, Ryzen 5 5600G class) 6C/12T, 16 GiB RAM,
  NVIDIA RTX 3060 driver 617.42, OS build 26200, non-admin shell. Same box as
  ADR-0027/ADR-0033; background load uncontrolled (browser + agent host active).
- Temporary release-profile probe (measurement only, removed after collection,
  same precedent): `WindowsSystemSampler::new_for_process(pid)` (full) and
  `new_system_only_with_timebase(QpcTimebase::new())` (system-only) against live
  `dwm.exe` (PID 14072, SYSTEM-owned compositor); fixed 50 ms cadence via
  deadline sleep; `sampler.stats()` mean/rolling-p99 plus own-process CPU from
  `GetProcessTimes` start/end deltas; observation means from
  `SystemSample.process_threads/background_activity` lens.
- Full leg: 2,400 ticks (~120 s). System-only leg: 1,200 ticks (~60 s). Rolling
  p99 window (4096) never fills on these legs — same caveat as ADR-0033.

## Results (all legs, one table)

| leg | date | target | ticks | mean ms | p99 ms | CPU % one core | obs/tick |
|---|---|---|---|---|---|---|---|
| full probing (ADR-0027) | 2026-10-10 | explorer | 12,000 | 18.855 | 28.352 | 23.75 | 210.66 thr / 56.44 bg |
| full probing repeat (ADR-0033) | 2026-10-15 | explorer | 2,400 | 22.072 | 23.757 | 43.75 | 169.08 thr / 70.97 bg |
| full probing, dwm (this draft) | 2026-10-17 | dwm (SYSTEM) | 2,400 | 43.532 | 49.372 | 84.95 | 0.00 thr / 127.95 bg |
| system-only (ADR-0027) | 2026-10-10 | — | 2,400 | 0.016 | 0.088 | ~0 | — |
| system-only repeat (ADR-0033) | 2026-10-15 | — | 1,200 | 0.019 | 0.073 | 0.08 | 0 / 0 |
| system-only, dwm (this draft) | 2026-10-17 | — | 1,200 | 0.046 | 0.137 | 0.00 | 0 / 0 |
| per-N-tick probing | — | — | — | BLOCKED (mode unimplemented; no data, not projected) | — | — | — |
| game-load full probing | — | — | — | BLOCKED_ON_USER (no installed game; elevated runbook in `docs/acceptance/2026-10-17.md`) | — | — | — |
| PresentMon process cost | — | — | — | BLOCKED_ON_USER (ETW needs elevation; protocol in ADR-0033) | — | — | — |

Reading (no claims beyond the table):

- Full probing cost is population/load-dependent and varies >2× across legs
  (19 → 22 → 44 ms mean; 24% → 44% → 85% of one core). The dwm leg is the
  worst yet: a SYSTEM-owned target yields **zero** target-thread observations
  (thread opens denied at medium integrity) while background enumeration
  (127.95 obs/tick) still bills the full cost.
- The dwm-leg p99 (49.372 ms) sits ~0.6 ms under the 50 ms tick budget:
  overruns on busier machines are plausible, not proven.
- System-only reproduces tightly across all three campaigns (0.016 → 0.019 →
  0.046 ms) and is the only mode inside RC-05 as-shipped.
- No single point may drive targets (ADR-0033 reading stands).

## Evidence lost per mode (code-verified, see `docs/diagnostics.md`)

| evidence | full probing | system-only (`no_process_probing`) | per-N-tick (projected, unimplemented) |
|---|---|---|---|
| `cpu.target_thread` (peak target-thread util) | present | LOST → `cpu_bound.v1` loses 35-weight support | stale between probe ticks; needs staleness semantics |
| background activity (alignment bursts) | present | LOST → `background_interference.v2` cannot fire | stale/aliased; burst attribution weakens |
| `gpu.utilization` (per-PID engine) | present (1 Hz PDH) | LOST (no target-GPU queries) → `gpu_bound.v2` loses 35-weight support | stale at probe cadence |
| NVML temp/clock/power/VRAM | present (matched adapter) | LOST → `thermal.v2`, `VramPressure` cannot fire | stale; throttle detection lags |
| frame shares, GPUBusy, stutter, pacing, stability | present (PresentMon CSV) | KEPT (capture path unchanged) | KEPT (capture path unchanged) |
| display refresh resolve | present | skipped → refresh-aware stutter falls back | unchanged from full |

Absent stays absent (None) in every mode; no rule fires on missing data.

## Options

- **A. Default system-only; full probing opt-in.** RC-05 met as-shipped; full
  depth stays one toggle away per session with provenance recorded. Cost: the
  default diagnosis loses CPU/thread/GPU/thermal attribution until opted in;
  UI must surface the trade-off honestly at the toggle.
- **B. Probing thread at a lower frequency (e.g. every Nth tick).** Keeps some
  attribution at reduced cost. Cost: new mode + staleness/aliasing semantics +
  versioned `analysis_config` change + tests; savings unmeasured until built.
- **C. Single `NtQuerySystemInformation` call replacing ToolHelp snapshots.**
  Attacks the ~2.2 ms snapshot floor and possibly the per-thread opens. Cost:
  semi-documented API surface, hand-rolled bindings, `SAFETY` review, fallback
  logic; needs a research spike before any estimate (ADR-0027 deferred this).
- **D. Keep the default and relax RC-05 via a data-backed ADR.** Cheapest
  mechanically. Cost: weakens the release bar without new evidence; still
  leaves the near-budget p99 and population variance unaddressed.

## Decision (accepted 2026-10-17: option A)

- Owner selected **A** (2026-10-17, full-handoff approval of the Batch 0–6
  consolidation report): default system-only, full probing opt-in per
  session. Rationale stands: A is the only option meeting RC-05 with shipped
  code, needs no new unsafe/API surface, and matches the already-shipped
  per-session opt-out (same toggle, flipped default, versioned properly).
  C is a spike, not a plan; D spends credibility without fixing cost. **B**
  remains the follow-up if opt-in friction proves real in practice.
- Implementation is not part of the 0.5.0-beta candidate; it rides the next
  versioned change: `analysis_config` bump + tests (fire/no-fire per affected
  rule in the new default, provenance recording, UI toggle copy with the
  honest evidence-loss trade-off). Until that ships and is validated: 50 ms
  cadence stays LOCKED, `no_process_probing` defaults to false, and RC-05
  stays OPEN.

## Consequences

- No product behavior, threshold, version, schema, IPC, or gate changes in
  this batch — the default flip ships with the next versioned change and will
  carry its own tests and provenance.
- Re-running needs only the recipe in Method; the temporary probe file was
  removed after collection per repo precedent (tree verified clean).
