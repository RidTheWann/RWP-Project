# ADR-0022 — Deterministic Session Persistence and Provenance

- Status: Accepted
- Date: 2026-10-09
- Scope: WP4 / AUD-16 / RC-06

## Decision

Persist one completed session as four deterministic payload files plus a manifest:

- `presentmon.csv` — raw PresentMon bytes copied from the capture source.
- `capture.json` — normalized `rwp.capture.v1` frame document.
- `telemetry.jsonl` — one canonical JSON object per normalized system sample, in sample order.
- `report.json` — deterministic `rwp.report.v2` output, including diagnostics.
- `manifest.json` — `rwp.session.manifest.v1` provenance containing session metadata, coverage, file sizes, and SHA-256 hashes.

The manifest lists files in lexical path order. Hashes are SHA-256 over exact persisted bytes and are lowercase hexadecimal. No wall-clock timestamp or nondeterministic identifier is inserted into the manifest.

Persistence is opt-in through `SessionConfig::with_persistence_dir`. A populated persistence directory is never silently overwritten; this prevents an accidental rerun from changing provenance in place.

## Integrity

`verify_manifest` recomputes every listed file hash and rejects a mismatch or unsupported manifest schema.

Payload writes use a temporary file followed by rename. Existing destination files are rejected before writing, so a completed persisted session is immutable through this API.

## Compatibility

Report v2 retains `#[serde(default)]` for diagnostics, allowing older report JSON without that field to remain readable.

## Follow-up

WP4 will wire CLI import/analyze/report and the GUI to this persisted session layout. Real capture acceptance remains dependent on the real PresentMon fixtures and hardware evidence listed in STATE.md.
