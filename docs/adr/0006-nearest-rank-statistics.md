# ADR-0006 — Nearest-rank statistics

- Status: Accepted (reconstructed)
- Context: Percentile interpolation can introduce values not present in the observed frame-time sample and can vary between implementations.
- Decision: Use nearest-rank percentiles for all v1 frame-time percentiles, including P0.1, P1, P5, P10, P95, P99, and P99.9. No interpolation is performed.
- Consequences: Percentiles are deterministic and always correspond to an observed sample.
