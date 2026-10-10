# ADR-0022 — Deterministic session persistence and manifest provenance

- Status: accepted
- Date: 2026-10-09
- Scope: WP4 / AUD-16

## Context

Session analysis must survive process exit and support later verification, reporting, and comparison. Persisted artifacts must be deterministic and verifiable, while unavailable observations must remain absent rather than becoming fabricated sentinel values.

## Decision

1. Persist session artifacts under a versioned manifest that records relative artifact paths, sizes, and SHA-256 hashes.
2. Verify manifest paths and artifact hashes before accepting persisted sessions.
3. Record build, target, hardware, PresentMon, QPC, and capture-option provenance when available; represent unavailable values as `None`.
4. Bound persisted input sizes and JSON nesting before deserialization.
5. Keep persisted data local and do not add outbound telemetry.

## Consequences

- Session files can be checked for accidental modification and compared reproducibly.
- Schema/version evolution must be explicit.
- Real PresentMon fixture and hardware acceptance remain separate release requirements.

## References

- `docs/threat-model.md`
- `docs/autopilot/MASTER_PLAN.md`, RC-06 and RC-08
