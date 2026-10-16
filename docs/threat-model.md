# Threat Model

## Scope

This document covers the trust boundaries and untrusted inputs used by RWP v1.0 capture, persistence, analysis, and reporting.

## Trust boundaries

| Boundary | Trusted side | Untrusted side | Required property |
|---|---|---|---|
| Tauri webview → backend | Rust application commands | Web UI input | Capability-scoped IPC; validate every path and numeric field server-side. |
| PresentMon executable | RWP process supervisor | User-supplied external executable | Require a canonical regular file named `PresentMon.exe`; verify supported-or-unknown version before capture (unknown recorded with SHA-256, CSV schema gate applies); contain the child in a kill-on-close Job Object. |
| PresentMon CSV → parser | Normalized internal frame model | CSV file contents, quoting, headers, metrics, timestamps | Strict parsing, monotonic timestamp validation, bounded resource use, no panic on arbitrary input. |
| Persisted session → loader | Verified manifest and file hashes | Session directory contents | Verify manifest hashes before consuming capture/report/telemetry data. |
| JSON report → renderer | Typed report model | JSON file contents | Deserialize through the versioned model; unknown or missing observations remain absent rather than sentinel values. |
| Filesystem paths | Application-controlled destination roots | CLI/webview path arguments | Reject traversal and unsafe executable paths; never trust a path merely because it came from the UI. |

## Input handling

### CSV

PresentMon CSV is treated as hostile input. The parser rejects malformed structure, missing required columns, invalid metrics, process identity changes, and timestamp conversion overflow; isolated backwards-QPC rows are dropped and counted with a warning instead of failing the session. Property tests must preserve the no-panic invariant.

### Persisted JSON

Session files are not trusted until manifest verification succeeds. A report or telemetry file that does not match the manifest is rejected rather than partially consumed. GUI history/load/compare paths reapply bounded reads after manifest verification to limit time-of-check/time-of-use replacement risk; reports pass through the versioned report parser rather than direct unbounded deserialization.

Persistence writes use a bounded number of unique, same-directory temporary filenames created with exclusive creation. Completed temporary files are published with a hard link, which refuses to replace an existing destination; the writer fails closed if the destination filesystem does not support this operation rather than falling back to a check-then-rename sequence. A failed multi-file session write may leave a partial session directory without `manifest.json`; loaders fail closed on that directory, and persistence does not recursively remove it automatically.

Manifest SHA-256 entries detect corruption or inconsistency between listed file bytes and the manifest. They do not prove authenticity or prevent deliberate tampering: the manifest is stored beside the data at the same trust level, so a writer able to modify the session can also rewrite the manifest.

### Paths

Executable paths are canonicalized before launch and must resolve to a regular file whose basename is exactly `PresentMon.exe` (case-insensitive on Windows). Session-history identifiers reject path traversal.

## Resource limits

The CSV parser enforces a 512 MiB maximum file size and a 10,000,000-frame maximum. These are resource-safety bounds, not analysis thresholds; valid captures below the limits retain the same parsing semantics.

Persisted-session I/O is bounded before allocating an untrusted file in memory:
- manifest.json: 1 MiB.
- capture.json and report.json: 64 MiB each.
- telemetry.jsonl: 256 MiB and at most 5,000,000 serialized samples.
- presentmon.csv: 512 MiB when copied into a session.
- Manifest file entries must be the four expected session data filenames, each exactly once; traversal paths and duplicate entries are rejected before hashing.

Bounded readers consume at most limit + 1 bytes, so a file that grows after an initial metadata check cannot bypass the limit. Before deserializing persisted-session manifest, capture, and report JSON, a string/escape-aware scanner rejects nesting deeper than 64 levels; the same guard applies to each non-empty telemetry JSONL record. The JSON parser remains responsible for validating syntax and matching delimiters. The public report JSON deserializer independently enforces a 64 MiB input limit and the same 64-level depth policy. GUI history/load/compare uses the same bounded report parser and caps telemetry counting at 256 MiB / 5,000,000 records. The current static inventory found no remaining direct report deserialization in the GUI path; compile/runtime verification is still pending.

## Webview security

The native Tauri capability grants only the registered session commands. No filesystem, shell, HTTP, updater, or other plugin permission is granted. The application CSP is `default-src 'self'`.

The webview is not a trusted source of executable paths, output paths, process IDs, or capture options; backend validation remains mandatory.

## Data and privacy

RWP is offline-first. The analysis engine must not make network requests or emit telemetry externally. Human-readable reports default to redaction of identifying fields. Diagnostic bundles must be redacted before release.

### Application logs

The native desktop application writes best-effort JSONL logs beneath `%LOCALAPPDATA%\RidTheWann\Performance\logs`. The event layer emits only a fixed target, severity, timestamp, and a static event identifier from an allowlist; arbitrary message fields, paths, process IDs, process names, usernames, and raw error strings are never serialized. The level can be set with `RWP_LOG_LEVEL` (`trace`, `debug`, `info`, `warn`, `error`, or `off`; invalid values fall back to `info`). Each active/archive file is limited to 5 MiB, three archives are retained, and each event is bounded to 16 KiB. Failure to initialize logging must not prevent application startup. These application logs are local diagnostics, not outbound telemetry.

## Residual risks

- PresentMon binary authenticity is currently constrained by filename/path/version checks plus SHA-256 recording and an advisory offline Authenticode report (`doctor`); signature enforcement remains a signing-phase decision (see ADR-0028).
- Windows Job Object kill-on-close behavior is verified by a local runtime test (unprivileged); lifecycle of an attached live PresentMon child still needs live-capture confirmation.
- Fuzz/property gates and Windows runtime verification of resource-boundary behavior remain outstanding.
- Hardware acceptance and real PresentMon fixture validation remain external acceptance work.
- P2: migrated sessions carry no provenance marker, so a migrated session is indistinguishable from a natively verified one. Options: stamp migration provenance and document it, or verify-first when the source hashes are intact. Tracked as follow-up, not a release blocker.

## Review status

Drafted during WP5 audit on 2026-10-09. Owner review is required before RC freeze.

Sign-off: APPROVED for alpha scope (single-device, unsigned) — 2026-10-15, by Roadmap-auditor under owner full-handoff order (revertible by owner with one word). Residuals accepted as documented; re-review required before v1.0 RC freeze.
