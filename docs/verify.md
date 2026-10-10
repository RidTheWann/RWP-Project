# [VERIFY] Register

The following must be resolved against the exact release/build before the related milestone is marked complete:

1. PresentMon release, CLI flags, CSV schema, dropped/repeated-frame semantics, and timestamp units.
2. ETW session/provider privilege requirements on supported Windows 11 builds.
3. CPU per-thread source: GetThreadTimes vs PDH Thread vs ETW sampling, selected by measured overhead and accuracy.
4. NVML Rust wrapper support for utilization, memory, clock, power, temperature, and encoder metrics.
5. OBS WebSocket v5 field names, units, update behavior, and timestamp/alignment semantics.
6. Win32/DXGI active display refresh-rate reporting.
7. GPU/driver-specific telemetry support gaps and fallback semantics.
