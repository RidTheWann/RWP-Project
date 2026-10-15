# CLI Error Codes

RWP command failures expose a stable error code on stderr. The code identifies the failure boundary; the detailed message remains specific to the observed error.

## Machine-readable output

Pass `--json` to emit exactly one JSON error object to stderr:

```json
{"error":{"code":"RWP-E2200","exit_code":22,"message":"PresentMon CSV contains no frame rows","remediation":"Validate the PresentMon CSV version, required columns, timestamps, target PID, and resource limits."}}
```

Successful command output is unchanged by `--json`; the flag only changes failure rendering.

## Stable taxonomy

| Code | Exit | Boundary | Typical remediation |
|---|---:|---|---|
| RWP-E1000 | 10 | Filesystem I/O | Check the referenced path, permissions, and storage. |
| RWP-E2000 | 20 | PresentMon configuration | Check executable path, target PID, output path, and capture options. |
| RWP-E2100 | 21 | PresentMon process lifecycle | Verify permissions, executable validity, lifecycle state, and stderr. A detected non-2.x PresentMon version is refused here; a version-unknown binary (no version resource) proceeds with a warning instead. |
| RWP-E2200 | 22 | PresentMon CSV normalization | Validate CSV schema, timestamps, target PID, and resource limits. |
| RWP-E3000 | 30 | Report/analysis serialization | Check report/capture schema compatibility. |
| RWP-E4000 | 40 | Diagnostic engine | Review thresholds and provide sufficient observations. |
| RWP-E5000 | 50 | Persisted-session integrity | Check manifest hashes, file limits, and filesystem permissions. |
| RWP-E5100 | 51 | Live-session platform/runtime | Use a supported platform and verify live-session configuration. |
| RWP-E6000 | 60 | Doctor preflight | Review failed doctor checks and address each reported issue. |
| RWP-E9000 | 90 | Unclassified/internal | Inspect the detailed error and session/input state. |

These codes and exit values are part of the CLI contract. Changing them requires a documented compatibility decision.

## Timed capture duration

Timed live captures accept whole-second durations from **1 through 3,600 seconds (one hour)**.
The native UI exposes this range, and the capture configuration enforces the same boundary
before starting PresentMon. The capture-layer validation also checks the public duration field,
so callers cannot bypass the limit by skipping the builder method.

The upper bound is a configuration contract, not a claim about measurement accuracy or a
benchmark result. Durations outside the supported range are rejected; they are not clamped.

## Interrupted live captures

A `capture` interrupted by Ctrl+C or Ctrl+Break is not a failure: the partial
session persists with `interrupted=true` and the command exits 0 with the
stderr notice `interrupted: partial session saved at <dir>`. No error code is
emitted. This exit policy is an OWNER DECISION (default): the partial session
is the successful product of an interrupted run. A second signal while a stop
is already in progress keeps default handling so the process still ends; the
kill-on-close Job Object prevents orphaned PresentMon processes either way.
Installing the console handler requires a console: on a console-less host the
command fails before spawning and surfaces as `RWP-E9000`.

## Privacy

Error messages are intended to stay actionable without adding new telemetry or network behavior. Existing path-bearing lower-level errors may still contain local filesystem details; user-facing bundles must apply the same redaction policy as reports.


## Verification boundary

The stable error-code contract is reviewed against the current tree with the
local gate (`cargo test` over the error taxonomy plus CLI E2E). No separate
infrastructure owns this contract; per-batch local verification is the
authority.
