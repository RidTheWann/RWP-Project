# ADR-0004 — CPU per-thread telemetry source selection

- Status: Accepted (reconstructed)
- Context: Per-thread CPU attribution can be collected through multiple Windows mechanisms with different accuracy, privileges, and observer cost.
- Decision: Treat GetThreadTimes, thread cycle counters, and ETW as candidates. Select the lowest-overhead source that satisfies the validated accuracy contract using the WP3 harness; until then, preserve missing data as None and record the source/provenance.
- Consequences: The implementation is measurement-driven rather than assuming one API is universally authoritative.


## Current implementation contract

The Windows implementation currently uses `GetThreadTimes` through `OpenThread(THREAD_QUERY_LIMITED_INFORMATION)` rather than the broader `THREAD_QUERY_INFORMATION` right. Thread CPU deltas are normalized against wall-clock intervals timestamped immediately around the thread-read phase. This narrows access requirements and avoids attributing the full system-sample preamble to the thread interval. The prescribed 25/50/75/100% duty-cycle harness remains required before selecting this source as the validated v1 accuracy contract.
