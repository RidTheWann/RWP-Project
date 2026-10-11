# Troubleshooting

Stuck? Match the failure to its code or message, apply the fix, and retry.
Every CLI failure prints a stable `RWP-Exxxx` code plus a `Fix:` line; with
`--json` the same object goes to stderr machine-readable. Code table:
`docs/cli-errors.md`.

## Quick map

| Symptom | Code | Fix |
|---|---|---|
| Bad path / missing file / disk full | RWP-E1000 (10) | Check the path, permissions, and storage. |
| PresentMon path, PID, or options rejected | RWP-E2000 (20) | Executable must exist, be a regular file named `PresentMon.exe`; PID > 0; duration 1–3600 s; interval 10–5000 ms. |
| Capture dies at runtime (permissions, lifecycle, stderr) | RWP-E2100 (21) | Run elevated or join Performance Log Users; check the executable and stderr. |
| CSV rejected (schema, timestamps, PID, limits) | RWP-E2200 (22) | Validate PresentMon v2 columns, monotonic QPC, target PID, and the 512 MiB / 10 M-row bounds. |
| Report/capture schema rejected | RWP-E3000 (30) | Use a compatible RWP version; legacy files go through `rwp migrate`. |
| Diagnosis needs more observations | RWP-E4000 (40) | Provide enough frames/telemetry; review thresholds in `docs/diagnostics.md`. |
| Manifest/hash/session integrity | RWP-E5000 (50) | Re-import or re-analyze; check file limits and filesystem permissions. |
| Live session on wrong platform/config | RWP-E5100 (51) | Live capture is Windows-only; check the session configuration. |
| Doctor preflight red | RWP-E6000 (60) | Address each failed doctor check below. |
| Anything else | RWP-E9000 (90) | Inspect the message and input/session state, then retry. |

## `rwp doctor` first

Run `rwp doctor --presentmon <PATH>` before capturing. It checks the
executable, version support, permissions, and (advisory) Authenticode
signature without failing on unsigned binaries. `--bundle <file>` writes a
redacted JSON bundle safe to attach to a report.

## Common cases

- **"must be named PresentMon.exe"**: rename or pick the official binary;
  the GUI Browse button filters to `.exe` but the backend rule decides.
- **Permission failure**: elevate once to confirm, then prefer Performance
  Log Users membership for routine captures.
- **Empty/short CSV**: the target exited immediately or the wrong PID was
  given; confirm the process lived for the whole duration.
- **Manifest mismatch after editing files**: any hand edit breaks hashes by
  design; re-import or re-analyze instead of patching session files.
- **Legacy session rejected**: run `rwp migrate --session <dir>
  --output-dir <out>` (report-only: `migrate --report <file> --output
  <file>`); migration preserves evidence and never recomputes diagnostics.
- **GUI "already running"**: stop or reset the active session first.
- **GUI shows a failure**: the message line is followed by a `Fix:` hint
  from the same remediation table the CLI prints.

## Getting help

Redact before sharing: reports default to redacted output, and doctor
bundles are redacted by construction. Never attach unredacted bundles,
raw process inventories, or private paths. See `docs/privacy.md` and
`SECURITY.md` (suspected vulnerabilities are reported privately, never in
public issues).
