# ADR-0014: Live session orchestration and shared QPC timebase

## Status

Accepted.

## Decision

Introduce a dedicated `rwp-session` orchestration boundary for live Windows capture.

A session owns the lifecycle of:
- the external PresentMon process;
- Windows system/thread/background telemetry;
- shared QPC timebase establishment;
- deterministic report construction;
- deterministic diagnosis;
- framework-neutral dashboard mapping.

The session creates one absolute QPC origin and reuses it for both Windows telemetry and PresentMon CSV normalization. This is required for reliable event-window correlation.

The Tauri layer is an IPC adapter only. It does not launch PresentMon itself and does not implement analysis rules.

## Cancellation and early exit

A timed session ends when its configured duration elapses. A target-process exit is observed through PresentMon completion and terminates the telemetry loop early.

A manual stop is modeled as an interrupted capture. The orchestrator terminates PresentMon, waits for the child, and attempts to normalize the CSV already produced. If the partial CSV cannot be parsed, the session fails instead of fabricating a result.

## Consequences

- Capture remains out-of-process and injection-free.
- Analysis remains deterministic and offline-first.
- QPC-relative timestamps from independent producers share one origin.
- Session lifecycle state is isolated from immutable analysis results.
- The first M8 UI can use vanilla JavaScript through Tauri's global API without a frontend framework.
- Richer process discovery, graceful interruption, persistent session storage, and release packaging remain later milestones.
