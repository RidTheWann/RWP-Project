# ADR-0008 — Live-session metrics

- Status: Accepted (reconstructed)
- Context: Live sessions need stable metric aggregation without changing offline evidence semantics.
- Decision: Keep live-session orchestration outside the deterministic analysis core; session state carries explicit missing values and provenance, and finalized evidence is passed through the same deterministic analysis contracts.
- Consequences: Live updates cannot silently invent zeroes or alter offline rule behavior.
