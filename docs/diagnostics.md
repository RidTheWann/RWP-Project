# Diagnosis thresholds

All diagnosis thresholds are explicit configuration and are serialized into the diagnostic parameters. Evidence scores are heuristic evidence strengths, not probabilities.

## CPU/GPU-bound, memory, and background thresholds

The current engineering defaults are:

| Signal | Default | Evidence interpretation |
|---|---:|---|
| Minimum frame observations | 60 | Required before frame-based CPU, GPU, and pacing rules run. |
| Minimum system observations | 10 | Required before telemetry-based rules emit a diagnosis. |
| CPU/GPU frame-time share | 0.80 | Mean normalized frame share that adds CPU or GPU evidence. |
| Target-thread utilization | 85% | Mean peak target-thread utilization that adds CPU evidence. |
| Target GPU utilization | 90% | Mean target-process GPU utilization that adds GPU evidence. |
| System RAM utilization | 90% | Mean RAM utilization that adds memory-pressure evidence. |
| Target VRAM utilization | 90% | Mean VRAM utilization that adds video-memory-pressure evidence. |
| Background CPU utilization | 10% | Per-process CPU activity threshold aligned to a stutter event window. |
| Background alignment fraction | 0.50 | Minimum fraction of stutter events that must overlap a background burst before low alignment is treated as contradicting evidence. |
| Diagnosis activation score | 60 | Minimum heuristic score for a non-Unknown classification. |

CPU-bound uses CPU frame share and target-thread utilization as separate supporting
signals; a high GPU frame share is contradictory evidence. GPU-bound uses PresentMon
`GPUBusy` frame share and target-process GPU utilization; a high CPU frame share is
contradictory evidence. Memory-pressure rules use mean RAM/VRAM utilization, with an
additional stutter-overlap signal when high use is observed in the event window.
Background-interference requires at least one stutter window aligned with background CPU
activity at or above its threshold; missing activity is not treated as zero evidence. When
fewer than the configured alignment fraction of stutter events overlap a burst, the rule
records `background.low_alignment` as contradicting evidence and lowers the score. The rule
never activates when no stutter event aligns, regardless of the activation score.

These defaults are explicit engineering thresholds, not hardware-derived boundaries or
probability estimates. The source contract and regression cases establish deterministic
behavior; reference-hardware calibration remains outstanding. Changing these thresholds
or their interpretation requires updated provenance and regression tests.

## Stutter rule selection

When an unambiguous active-display refresh rate is available, `stutter.v2.refresh_aware`
uses the threshold `max(2.0 * local P50 baseline, local P50 baseline + refresh period)`,
with a 15-frame half-window, at least 7 valid neighbors, and a 50 ms merge gap. The
refresh period is `1000 / refresh_rate_hz`. If refresh rate is unavailable or ambiguous,
the report explicitly falls back to the versioned `stutter.v1.absolute` contract
documented in `docs/statistics.md`; it does not synthesize a refresh rate.

## Thermal — thermal.v2

The rule requires two co-observed conditions:

- GPU temperature reaches gpu_temperature_c (default 85 C).
- GPU clock drops by at least gpu_clock_drop_percent (default 15%) while the GPU is at or above the temperature threshold.

The clock-drop percentage is computed against the maximum observed GPU clock and the minimum GPU clock observed during samples meeting the thermal temperature threshold. A high temperature by itself therefore does not activate the rule.

The 15% clock-drop threshold is a conservative engineering default, not an empirical claim. Hardware calibration remains required under WP6/RC-05.

The rule emits two supporting evidence items and uses rule id thermal.v2.

## Frame pacing — frame_pacing.v2

The rule requires both persistent event-rate evidence and a long-tail frame-time signal:

- At least frame_pacing_min_stutter_events events (default 3).
- At least frame_pacing_min_events_per_minute events/minute (default 6).
- P99/P50 frame-time ratio at or above frame_pacing_ratio (default 1.75).

Event rate is derived from the capture frame timestamp span. Missing or zero-duration frame spans do not produce a synthetic rate.

The minimum event count prevents a single isolated stutter from being promoted solely because a short capture would make its per-minute rate appear large.

The rule emits separate event-rate and tail-ratio evidence with rule id frame_pacing.v2.

## Data-availability contract

Unavailable telemetry remains absent (None). A missing GPU clock, missing GPU temperature, insufficient stutter count, or unusable timestamp span cannot be converted into zero-valued evidence.

## Preliminary stability (in-memory live surfaces only)

`preliminary_stability` gives live capture surfaces an early verdict without
touching persistence: below the engine's minimum-frame bound (default 60) the
verdict is always insufficient data, never a stability claim. At or above the
bound, a stable verdict additionally requires zero stutter events and
first-half vs second-half mean frame-time drift within the shared
equivalence margin (default 1%, the A/A margin — one system-wide margin,
documented once here). The verdict is never written to sessions or reports,
and no refresh rate is synthesized for it. "Never Stable while
under-powered" is literal here: insufficient or degenerate evidence can only
ever yield insufficient data, never Stable and never Unstable-by-guess.

## Streaming overhead — deferred

The `streaming_overhead` category exists in the product contract, but no rule emits it yet. Normalized telemetry currently exposes target-process GPU utilization and redacted background CPU activity, but no encoder or video-engine signal that could attribute degradation to streaming or creator tooling; inferring the category from generic background activity alone would fabricate an attribution the evidence does not support. The category stays deferred until a normalized encoder observation exists.

## Validation status

Synthetic regression coverage verifies:

- hot GPU temperature with a stable clock does not activate Thermal;
- hot GPU temperature with a significant clock drop activates Thermal;
- Thermal does not activate when system observations are below the documented minimum, even if temperature and clock-drop evidence would otherwise be sufficient;
- a high GPU frame share contradicts CPU-bound evidence and reduces its score;
- a high CPU frame share contradicts GPU-bound evidence and reduces its score;
- a low-alignment background case records `background.low_alignment` contradicting evidence and reduces the score to 60;
- background activity outside every stutter window does not activate Background Interference;
- frame-based and telemetry-based rules stay inactive below their documented minimum sample counts;
- a single stutter does not activate Frame Pacing;
- persistent event rate combined with a high tail ratio activates Frame Pacing.

Hardware-dependent validation of telemetry accuracy and thresholds remains pending.

## GPU telemetry — vendor-neutral per-PID observation

For target-process sampling, rwp-system uses the Windows `GPU Engine` performance-counter object with the `Utilization Percentage` counter. The instance list is wildcard-enumerated; instance names are filtered by `pid_<targetPid>` rather than constructing a guessed PID counter path. The dominant adapter grouping is derived from the instance LUID/physical-index portion.

PDH collection is intentionally rate-limited to 1 Hz in the 50 ms system sampler. Microsoft documents PDH as an administrative/diagnostic collection interface and notes it is not designed for collection above once per second; therefore the sampler does not claim 20 Hz GPU-counter precision. Samples without a fresh GPU Engine observation retain `None`.

For NVIDIA target-process telemetry, NVML no longer assumes device index 0. The sampler selects an unambiguous NVIDIA adapter whose running graphics-process list contains the target PID. If no matching adapter exists or multiple NVIDIA adapters match, NVIDIA-specific fields remain absent rather than being attributed to an arbitrary device.

The vendor-neutral GPU Engine path is the source for target-process `gpu_utilization`; NVIDIA-specific clock, temperature, power, and VRAM fields are sourced only from the process-selected NVIDIA adapter.

Hardware validation remains required.

## Process/thread telemetry opt-out

The GUI offers an opt-out for RWP's additional process/thread probing, disabled by default
to preserve the existing capture behavior. When enabled, RWP does not enumerate process or
thread activity, query target-process GPU adapters, or resolve the target process's display
refresh rate. PresentMon still captures the selected PID out-of-process, while system CPU
and RAM telemetry continue. Per-thread/background activity and target-GPU fields remain
absent rather than being represented as zero. The selection is recorded in session
provenance; older v2 manifests without this field remain readable.

This reduces RWP's extra process inspection but is not a guarantee of anti-cheat
compatibility: PresentMon still observes the requested target process. Hardware and
anti-cheat acceptance remain pending.
