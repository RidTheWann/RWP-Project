# ADR-0016 — PresentMon version matrix

- **Status:** accepted
- **Date:** 2026-10-08
- **Scope:** WP2 / AUD-14

## Context

The capture engine invokes PresentMon with the v2 metrics contract. A binary outside that tested major-version contract must not be accepted silently because column semantics and lifecycle flags can change between major versions.

## Decision

The capture runner reads the Windows file-version resource from the configured PresentMon executable before spawning it. The supported matrix is **PresentMon major version 2.x**. The full four-component version is retained in the detection result for later manifest/provenance work.

Versions with a major component other than `2` are rejected before process creation.

Minor, patch, and build values are not widened into an acceptance claim until real anonymized PresentMon v2 fixtures are available and the acceptance matrix is expanded.

## Consequences

- Unsupported major versions fail fast with a deterministic error.
- Version detection does not depend on a `--version` command-line flag.
- The real fixture matrix remains the acceptance authority for minor/patch compatibility.

## References

- Microsoft Win32 version-information API: <https://learn.microsoft.com/en-us/windows/win32/api/winver/>
- PresentMon v2 metrics are selected explicitly by the capture command.
