# Windows Telemetry Foundation

M3 establishes the Windows-specific time and system-state boundary while preserving the platform-neutral contracts in rwp-core.

## QPC

WindowsSystemSampler calls Windows QueryPerformanceFrequency once at construction and establishes a QPC origin. Each sample converts the current counter delta to integer nanoseconds. The authoritative frequency is exposed so rwp-capture can use the same source without discovering it itself.

## CPU

System CPU utilization is computed from successive GetSystemTimes snapshots. Kernel and user time form total processor time; idle time is removed from that total. The first sample has no utilization percentage because a delta does not yet exist.

## Memory

Physical RAM used is derived from GlobalMemoryStatusEx as total physical memory minus available physical memory. There is no sentinel-zero representation for unavailable telemetry.

## NVIDIA

The NVML backend initializes once per sampler. Initialization failure is treated as unavailable GPU telemetry rather than a fatal system-sampling error. When available, the sampler records GPU utilization, used VRAM, graphics clock, power usage, and GPU temperature. NVIDIA's Rust NVML wrapper documents dynamic NVML loading and per-metric errors for unsupported or unavailable functions.

## Shared session timebase

M8 can create one `QpcTimebase` and pass it to `WindowsSystemSampler::new_for_process_with_timebase`. PresentMon normalization can use the same frequency and absolute QPC origin through `PresentMonCsvParser::new_with_qpc_origin`. This makes frame-event and system-telemetry timestamps directly comparable within a session.

## Boundary

This layer owns OS-specific APIs only. It emits SystemSample and does not perform diagnosis. ETW, PDH, and per-thread process telemetry remain explicit follow-up backends rather than being represented as guessed values.
