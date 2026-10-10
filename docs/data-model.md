# Normalized Data Model

M1 separates observations, derived events, diagnoses, and benchmark identity. Analysis receives normalized values only.

## FrameSample

- timestamp_ns: capture-relative QPC-derived timestamp.
- frame_time_ms: required finite positive frame interval.
- cpu_busy_ms, cpu_wait_ms, gpu_time_ms, gpu_busy_ms, gpu_wait_ms, display_latency_ms: optional telemetry.

Missing telemetry is represented by absence, never by zero.

## SystemSample

A timestamped system observation containing optional CPU/GPU/memory/clock/power/temperature metrics plus per-thread target-process observations and redacted background activity.

## StutterEvent

A derived event containing start/end timestamp, peak frame time, robust local baseline, candidate count, and the detector parameters used.

A StutterEvent never claims causality by itself.

## Diagnostic

A rule-engine result containing category, rule identifier, evidence score, supporting and contradicting evidence, parameters, sample count, and minimum sample requirement.

Evidence score is a heuristic score from 0 to 100, not a statistical probability.

## Serialization invariants

- Timestamps are non-negative.
- Required frame time is finite and greater than zero.
- Optional measurements use absence instead of sentinel zeroes.
- Ordered vectors, not unordered map iteration, determine canonical output order.
