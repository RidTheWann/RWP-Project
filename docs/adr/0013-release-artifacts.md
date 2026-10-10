# ADR-0013 — Release artifact provenance

- Status: Accepted (reconstructed)
- Context: Release artifacts must be traceable to a tested commit and protected from accidental mutation.
- Decision: CI builds the Windows release artifact from the locked source tree, verifies dependency policy and audit gates, generates checksums, and publishes artifacts only from the release workflow/tag path defined by the master plan.
- Consequences: A release is not considered valid merely because an installer exists; the complete CI provenance and checksum gate must pass.
