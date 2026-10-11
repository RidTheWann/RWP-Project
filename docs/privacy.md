# Privacy

RWP measures your machine for you. Nothing it collects leaves the machine on
its own.

## Offline-first

Normal operation makes no network requests: no telemetry upload, no update
check, no license ping. Dependency and runtime evidence for this is recorded
under RC-10 (no HTTP-client crate in the Windows build graph, no socket APIs
in product code, monitored runs with zero connections). The WebView2 runtime
itself is outside RWP's accounting.

## Redaction by default

Human-readable reports (Markdown/HTML) redact identifying fields unless the
operator explicitly opts out (`--redact false` in the CLI is an intentional
action):

- application names are escaped and marked redacted;
- process IDs are omitted entirely (never fabricated as zero);
- raw process inventories stay out of shared output.

JSON reports remain machine-readable and versioned; treat them as sensitive
and redact before sharing.

## Application logs

The native app writes best-effort JSONL logs under
`%LOCALAPPDATA%\RidTheWann\Performance\logs`. The event layer emits only an
allowlisted static event identifier plus severity, target, and timestamp —
never messages, paths, PIDs, process names, usernames, or raw errors. Files
are capped at 5 MiB with three archives retained; oversized records are
dropped. Set `RWP_LOG_LEVEL` to control verbosity; logging failure never
blocks startup. These logs are local diagnostics, not outbound telemetry.

## Sessions and bundles

Persisted sessions live under the operator-chosen directory (GUI history under
`%LOCALAPPDATA%`) with SHA-256 manifests that detect corruption, not identity.
`rwp doctor --bundle` writes a redacted diagnostic bundle safe to attach to a
report. Before sharing anything, re-check for private paths and unredacted
process data; see `docs/troubleshooting.md` (getting help) and `SECURITY.md`.
