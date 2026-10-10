# ADR-0024 — GUI session history and provenance manifest

- Status: accepted
- Date: 2026-10-09
- Scope: WP4 / AUD-16 and WP5 / AUD-18

## Context

The native GUI must expose historical sessions and deterministic comparisons without bypassing manifest verification or allowing the webview to select arbitrary executables or filesystem capabilities.

## Decision

1. Store immutable session history under the application-local data directory.
2. Verify each manifest before listing or loading a historical session.
3. Use the same bounded report deserializer for history, loading, and comparison.
4. Expose only the explicitly registered session IPC commands through the Tauri capability boundary.
5. Validate the PresentMon executable path and contain the child process in a Windows Job Object configured with `KILL_ON_JOB_CLOSE`.

## Consequences

- GUI history is local-first and can be verified before use.
- Filesystem and process-launch authority remains in the native application layer.
- Windows compile/runtime and manual GUI acceptance remain required before release.

## References

- `docs/threat-model.md`
- `docs/acceptance/gui.md`
- `docs/autopilot/MASTER_PLAN.md`, RC-08 and RC-12
