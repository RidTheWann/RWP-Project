# ADR-0001 — Workspace boundaries

- Status: Accepted (reconstructed)
- Context: The product combines platform-neutral contracts, Windows capture/telemetry, deterministic analysis, reporting, CLI orchestration, and a separate GUI process.
- Decision: Keep ten workspace crates with dependency direction from platform-neutral rwp-core outward; rwp-analysis and rwp-diagnostics remain pure; Windows unsafe stays in rwp-system; rwp-cli is the application boundary.
- Consequences: Platform-specific code cannot leak into the analysis layer, and deterministic analysis can be tested without Windows runtime state.
