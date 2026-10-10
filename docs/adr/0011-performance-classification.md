# ADR-0011 — Performance classification

- Status: Accepted (reconstructed)
- Context: Classification must distinguish measured evidence from unknown conditions.
- Decision: Classifications are rule-versioned deterministic outcomes. Insufficient evidence yields Unknown rather than a synthetic pass/fail or zero-valued metric.
- Consequences: Rule changes require a new rule/config version and identical inputs remain byte-stable.
