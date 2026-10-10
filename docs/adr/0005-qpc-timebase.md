# ADR-0005 — QPC canonical timebase

- Status: Accepted (reconstructed)
- Context: PresentMon and Windows telemetry must be aligned deterministically despite differing wall-clock representations.
- Decision: Use QueryPerformanceCounter (QPC) relative to capture start as the canonical integer nanosecond timebase. Wall-clock timestamps are metadata only.
- Consequences: Alignment is monotonic and platform-neutral; conversion and rounding rules are part of the data contract.
