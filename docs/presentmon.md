# PresentMon Integration

RidTheWann Performance uses PresentMon as an external process. The analyzer does not inject DLLs, install a graphics overlay, or hook the target application as part of capture.

## CLI contract

The M2 runner explicitly selects the PresentMon v2 metrics path with:

- `--process_name` or `--process_id`
- `--output_file`
- `--no_console_stats`
- `--qpc_time`
- `--v2_metrics`
- `--terminate_on_proc_exit`
- optional `--timed N` + `--terminate_after_timed`
- optional `--exclude_dropped`

The parser requires these v2 frame columns:

- `Application`
- `ProcessID`
- `CPUStartQPC`
- `FrameTime`
- `CPUBusy`
- `CPUWait`
- `GPUTime`
- `GPUBusy`
- `GPUWait`

`DisplayLatency` is optional.

The current upstream PresentMon release used for M2 validation is v2.6.0. PresentMon documents the command-line options and the v2 CSV metrics, including `CPUStartQPC` and frame timing fields. 

## Schema policy

The adapter does not silently reinterpret a different CSV schema. In particular, it does not map newer/default names such as `TimeInMs` or `MsBetweenPresents` into the canonical `FrameTime` field.

Optional values such as empty strings, `NA`, and `N/A` become `None`. Missing telemetry is never represented as zero.

Each capture must contain one process identity. Process-name matching is case-insensitive and accepts either the bare executable name or the `.exe` form. A PresentMon row with `<unknown>` application name is accepted when PID matching is otherwise valid.

## Time normalization

PresentMon emits an absolute performance-counter value in `CPUStartQPC`. M2 can normalize from the first observed QPC value. Coordinated M8 sessions instead provide a shared absolute QPC origin so PresentMon frames and Windows telemetry use one canonical capture-relative timebase.

The QPC frequency is deliberately not discovered in `rwp-capture`. The future `rwp-system` layer owns Windows-specific discovery so the normalized contract remains platform-neutral.

## Capture lifecycle

1. Build `PresentMonCaptureConfig`.
2. Start PresentMon with `PresentMonRunner`.
3. Use a finite timed run or stop an ongoing process explicitly.
4. Finish the process with `PresentMonProcess::finish`.
5. Parse and normalize the output with `PresentMonCsvParser`.
6. Pass the resulting `FrameSample` values to `rwp-analysis`.

The command runner protects against accidentally overwriting an existing output file and keeps the capture process outside the target application's process space.
