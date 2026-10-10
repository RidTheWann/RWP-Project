# ADR-0003 — Version-tolerant PresentMon CSV adapter

- Status: Accepted (reconstructed)
- Context: PresentMon CSV columns can vary by version and invocation while required semantics must remain trustworthy.
- Decision: Detect headers and map known fields by semantic column name. Unknown optional columns are ignored; missing required semantics fail closed. Parsing is bounded and does not fabricate absent measurements.
- Consequences: New PresentMon columns do not silently change analysis, while unsupported required schemas are rejected with an actionable error.
