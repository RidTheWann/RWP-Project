# ADR-0019 — Thermal and frame-pacing evidence rules

- Status: accepted
- Date: 2026-10-08
- Scope: WP3 / AUD-11

## Context

The original M5 rules could classify thermal pressure from a single high GPU temperature observation and could classify frame pacing from one stutter event. Those predicates are too weak for a diagnostic claim and can produce false positives on short or transient observations.

The normalized telemetry contract already exposes GPU temperature and GPU clock observations. nvml-wrapper 0.13.0 also exposes current NVML throttle reasons, including thermal and power-related slowdown reasons, but the cross-vendor SystemSample contract does not yet persist a throttle-reason field. The current WP3 change therefore uses co-observed temperature plus clock reduction as the persisted NVIDIA telemetry proxy.

## Decision

1. Version the thermal rule as thermal.v2.
2. Require GPU temperature at or above gpu_temperature_c and a GPU clock reduction at or above gpu_clock_drop_percent during the hot samples.
3. Compute clock reduction from the maximum observed clock to the minimum clock observed while the temperature condition holds.
4. Version the frame-pacing rule as frame_pacing.v2.
5. Require a minimum stutter-event count and event rate in events/minute, plus the configured P99/P50 tail ratio.
6. Use separate evidence items for event-rate and tail-ratio signals so one dimension alone cannot reach the default activation score.
7. Keep all thresholds explicit and serialized. Treat threshold values as engineering defaults pending WP6 calibration; do not present them as empirical measurements.

## Consequences

- A single temperature spike or isolated stutter no longer emits a positive diagnosis.
- The rule behavior changes are reproducibly identified by rule id, preserving V1 behavior for previously produced reports.
- Thermal attribution is conservative until throttle reasons are represented in the normalized cross-vendor telemetry contract.
- Short captures with zero-duration frame spans do not receive a fabricated event rate.
- Hardware calibration is still required before claiming production-grade threshold accuracy.

## References

- docs/diagnostics.md
- nvml-wrapper 0.13.0 Device::current_throttle_reasons
- https://docs.rs/nvml-wrapper/0.13.0/nvml_wrapper/device/struct.Device.html