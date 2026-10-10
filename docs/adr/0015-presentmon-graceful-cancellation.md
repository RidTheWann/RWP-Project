# ADR-0015 — PresentMon graceful cancellation and fallback

- Status: Accepted
- Date: 2026-10-08
- Scope: PresentMon capture process lifecycle

## Context

A cancelled capture must preserve a valid partial CSV when possible. Hard-killing PresentMon can leave the final CSV row incomplete and can also leave child capture activity behind.

## Decision

PresentMon is started with Windows `CREATE_NEW_PROCESS_GROUP`. Cancellation first requests `CTRL+BREAK` for that process group through the Windows system boundary. The session waits up to a bounded timeout for the process to exit. If the control request is unavailable or the process does not exit before the timeout, the existing hard-kill path is used as a containment fallback.

The CSV parser has an explicit `ParseMode::TolerateTruncatedTail` used only for cancellation. Normal parsing remains strict. A successful partial cancellation is represented in report.v2 with `interrupted=true`.

Unsafe Windows API access remains isolated to `rwp-system` and carries a `// SAFETY:` comment.

## Consequences

- Cancellation can finish PresentMon cleanly when console-control delivery is available.
- GUI environments without a suitable console remain safe because timeout + kill is retained.
- Interrupted captures are distinguishable from failed captures.
- The fallback does not claim graceful shutdown when Windows could not deliver the control event.
- End-to-end verification on the reference Windows environment remains required before AUD-03 is marked VERIFIED.
