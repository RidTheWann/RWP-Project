# ADR-0020 — Target GPU adapter selection and vendor-neutral GPU telemetry

- Status: accepted
- Date: 2026-10-09
- Scope: WP3 / AUD-12

## Context

The previous Windows sampler selected NVIDIA NVML device index 0 regardless of the target process. NVML documents that device-index ordering has no stable cross-reboot/API meaning, so index 0 is not evidence that the captured process used that adapter.

The project also needs a vendor-neutral per-PID GPU signal. Windows exposes per-process GPU Engine instances through performance counters, with instance names containing the process ID and adapter LUID/physical index. Microsoft documents wildcard counter-array retrieval for such instance sets.

## Decision

1. For a target process, enumerate `\GPU Engine(*)\Utilization Percentage` through PDH and filter returned instances by `pid_<targetPid>`.
2. Group matching engine observations by the LUID/physical-index adapter portion of the instance name and select the adapter group with the highest aggregate utilization. Tie-breaking is deterministic by adapter key.
3. If no valid adapter observation exists, leave GPU utilization absent. Do not synthesize zero.
4. Rate-limit PDH GPU Engine collection to 1 Hz. The system sampler remains a 50 ms scheduler, but GPU Engine observations are intentionally sparser because Microsoft documents PDH as unsuitable for high-frequency collection above roughly once per second.
5. For NVIDIA-specific telemetry, enumerate NVML devices and select the unique device whose running graphics-process list contains the target PID. If zero or multiple NVIDIA devices match, leave NVIDIA-specific fields absent rather than selecting an arbitrary device.
6. Target-process `gpu_utilization` comes from the vendor-neutral GPU Engine path. NVIDIA-specific clock, temperature, power, and VRAM fields come only from the process-selected NVIDIA adapter.
7. Keep the existing `gpu_bound.v2` rule ID for this telemetry-source correction because the rule's evidence contract already names `GPUBusy` as its primary frame-level signal; the SystemSample GPU utilization is supporting telemetry, not a new rule meaning.

## Consequences

- Multi-GPU systems no longer silently attribute target-process NVIDIA telemetry to NVML device 0.
- AMD/Intel systems can expose target-process GPU utilization through the Windows GPU Engine path when the counter provider is available.
- GPU utilization may be `None` on the first PDH sample, when the counter provider is unavailable, or when adapter attribution is ambiguous.
- Hardware validation is still required to verify counter availability, adapter-instance semantics, and correlation against PresentMon/Task Manager on reference systems.

## Evidence

- Microsoft Learn: `PdhEnumObjectItemsW` documents enumeration of performance-counter instance names.
- Microsoft Learn: `PdhGetFormattedCounterArrayW` documents wildcard instance-array retrieval.
- Microsoft Learn: `About Performance Counters` cautions that PDH is not intended for high-frequency collection.
- Microsoft Learn Q&A, Microsoft staff response: GPU Engine instances should be enumerated rather than guessed from PID/process-name paths.
- nvml-wrapper 0.13 documentation: `Device::index` has no guaranteed stable ordering, while `running_graphics_processes` identifies graphics processes on a device.