# ADR-0023 — Persisted CLI analysis, paired comparison, and A/A calibration

- Status: accepted
- Date: 2026-10-09
- Scope: WP4 / AUD-16

## Context

The CLI needs an end-to-end workflow over persisted sessions so imports, analysis, comparisons, calibration, and reports can be reproduced without a live capture.

## Decision

1. Route CLI import/analyze/report/compare/calibrate operations through the persisted-session layout and manifest verification.
2. Keep comparisons on the deterministic comparison engine and persist their configuration/provenance.
3. Provide A/A calibration as an explicit operation; do not interpret its output as causal evidence or a probability.
4. Redact human-readable report identity fields by default, with explicit opt-in to reveal them.
5. Keep fixture E2E coverage for import → analyze → compare → report.

## Consequences

- CLI output can be regenerated from the same persisted inputs and configuration.
- A/A calibration and real-data acceptance are distinct from synthetic fixture validation.
- Report schema changes require explicit versioning.

## References

- `docs/autopilot/MASTER_PLAN.md`, RC-04 and RC-06
- `docs/autopilot/STATE.md`, AUD-16
