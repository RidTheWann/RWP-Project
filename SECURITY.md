# Security Policy

RidTheWann Performance is intended to be an offline-first Windows performance analyzer.

## Reporting

Do not publish sensitive telemetry, credentials, private paths, or unredacted process inventories in public issues.

Report suspected vulnerabilities privately via GitHub Security Advisories
(repository Security tab → "Report a vulnerability":
`https://github.com/RidTheWann/RWP-Project/security/advisories/new`).
Never file a public issue for a suspected vulnerability. Attach only
redacted bundles (`rwp doctor --bundle`, redaction on by construction).

## Design security boundary

- Capture is out-of-process.
- No DLL injection or game-process hooks.
- No network access in normal operation (RC-10 evidence recorded); there is
  no updater, telemetry upload, or license check to opt out of.
- Shareable reports redact usernames, full paths, and raw process inventories by default.
