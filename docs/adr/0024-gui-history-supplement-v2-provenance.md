# ADR-0024 — GUI Session History and Provenance Manifest

- Status: Accepted
- Date: 2026-10-09
- Scope: WP4 / AUD-16 / RC-06

## Context

The native GUI previously kept completed analysis only in process memory. A restart therefore lost the session, while the persisted-session layer did not record enough producer provenance to support deterministic "Prove" workflows.

## Decision

The native GUI persists each completed session under:

`%LOCALAPPDATA%/RidTheWann Performance/sessions/<session-id>/`

The session ID is derived from the user-selected PresentMon output filename using a bounded filesystem-safe transformation. Persistence remains immutable: an existing session file is never overwritten.

The GUI exposes two local IPC operations:

- `session_history` lists only sessions whose manifest and file hashes verify.
- `session_history_load` verifies the selected session again and reconstructs the same renderer-neutral dashboard model used by live capture.

The manifest is versioned as `rwp.session.manifest.v2` and carries explicit provenance fields:

- RWP package version, Git SHA when available, build target, and build profile;
- runtime OS, CPU identity when available, target GPU and driver when available;
- PresentMon version and executable SHA-256 when available;
- QPC frequency;
- capture options that materially affect observations.

Unavailable provenance is represented as `None`, never as a sentinel value.

The CLI and live Windows session producers embed build metadata from their build scripts. The live Windows session also records PresentMon executable version and SHA-256 when those observations are available.

## Consequences

- GUI history survives application restarts without introducing a database or network dependency.
- Corrupted or partially written sessions are excluded from the history list rather than treated as valid evidence.
- Manifest provenance is explicit and backward-incompatible manifest changes require a schema version change.
- Some hardware identity fields remain absent on platforms or drivers where the producer cannot observe them; absence is preserved rather than guessed.
- Session IDs are user-output-name derived, so reusing the same output name intentionally collides with immutable persistence instead of silently replacing evidence.
