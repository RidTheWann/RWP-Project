# ADR-0007 — Schema and benchmark identity

- Status: Accepted (reconstructed)
- Context: Reports must remain machine-readable and reproducible while volatile runtime state must not change benchmark identity.
- Decision: Version report JSON with JSON Schema 2020-12 and derive benchmark identity from canonical, non-volatile identity JSON using SHA-256. Store the full digest and a compact display ID.
- Consequences: Schema evolution is explicit, and identical benchmark identity inputs produce identical IDs independent of runtime timestamps or background inventory.
