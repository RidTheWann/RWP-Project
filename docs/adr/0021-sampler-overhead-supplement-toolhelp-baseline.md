# ADR-0021 — Sampler overhead instrumentation before backend replacement

Status: accepted  
Date: 2026-10-09  
WP: WP3 / AUD-10

## Context

The Windows sampler defaults to a 50 ms interval. The current target-process path performs one system-wide
TH32CS_SNAPTHREAD snapshot and one system-wide TH32CS_SNAPPROCESS snapshot per sample. It then opens
each target thread to read GetThreadTimes and each enumerated background process to read GetProcessTimes.

Microsoft documents that TH32CS_SNAPPROCESS includes all processes and TH32CS_SNAPTHREAD includes all
threads in the system, so these are system-wide snapshots even though the resulting data is filtered to the
target process. GetThreadTimes accepts THREAD_QUERY_LIMITED_INFORMATION; GetProcessTimes accepts
PROCESS_QUERY_LIMITED_INFORMATION.

At 50 ms, the two ToolHelp snapshots alone imply 20 process snapshots/second plus 20 thread snapshots/second.
The handle/query count is workload-dependent and therefore cannot be represented as a fixed CPU-cost claim.

## Decision

1. Add bounded runtime SamplerStats instrumentation to WindowsSystemSampler.
2. Record successful sample duration with Instant.
3. Report:
   - total successful sample count;
   - mean duration across all recorded successful samples;
   - rolling p99 duration over the latest 4096 samples.
4. Do not change the 50 ms default until hardware measurements exist.
5. Do not introduce NtQuerySystemInformation(SystemProcessInformation) in this batch.
6. Treat NtQuerySystemInformation as a follow-up optimization candidate only after the measured ToolHelp
   baseline is collected and compared on the reference machine.

## Consequences

- The sampler can now expose empirical duration data without retaining an unbounded history.
- The rolling p99 window adds bounded memory and makes the metric suitable for long-running sessions.
- The instrumentation itself is part of the measured path, so acceptance measurements must be collected with
  the same build and configuration that will be evaluated.
- Current measurements remain UNVERIFIED until collected on the reference Windows hardware.
- No rule semantics, serialized report fields, sampling interval, or None semantics change.

## Evidence basis

- Microsoft Learn: CreateToolhelp32Snapshot documents that TH32CS_SNAPPROCESS includes all processes
  and TH32CS_SNAPTHREAD includes all threads in the system.
- Microsoft Learn: GetThreadTimes documents THREAD_QUERY_LIMITED_INFORMATION as an accepted access right.
- Microsoft Learn: GetProcessTimes documents PROCESS_QUERY_LIMITED_INFORMATION as an accepted access right.

## Verification plan

On the reference Windows machine, collect at least 10 minutes at the default 50 ms interval and record:

- sampler sample count;
- mean sampler duration;
- p99 sampler duration;
- total capture CPU utilization;
- target process thread count and background-process enumeration scale.

Compare the baseline against any future NtQuerySystemInformation implementation using the same workload.
