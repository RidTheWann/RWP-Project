# ADR-0012 — Deterministic report serialization

- Status: Accepted (reconstructed)
- Context: Release evidence must be reproducible byte-for-byte.
- Decision: Canonicalize report inputs, use stable ordering, explicit schema/rule versions, and exclude volatile timestamps and environment noise from deterministic content. Any intentional behavior change increments the applicable version identifier.
- Consequences: The same evidence and configuration produce identical report bytes.
