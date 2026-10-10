# ADR-0021 — Sampler overhead measurement instrumentation

- Status: accepted
- Date: 2026-10-08
- Scope: WP3 / AUD-10

## Context

The Windows sampler enumerates process/thread telemetry on a periodic interval. Calling it low-overhead without measurement would be an unsupported performance claim, and instrumentation itself must remain bounded.

## Decision

1. Record successful sample count and mean sample duration.
2. Track a rolling p99 sample duration using a fixed-size window of 4096 observations.
3. Keep the default sample interval at 50 ms pending reference-hardware measurements.
4. Document the baseline and measurement protocol before claiming that the sampler meets the release overhead target.
5. Keep performance measurements local; do not add telemetry upload or network dependencies.

## Consequences

- Reports can expose sampler timing evidence without inventing measurements.
- The rolling window has a bounded memory cost.
- Passing the release overhead target remains unverified until the prescribed reference-hardware harness is run.

## References

- `docs/autopilot/MASTER_PLAN.md`, RC-05
- `docs/autopilot/STATE.md`, AUD-10
