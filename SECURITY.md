# Security Policy

RidTheWann Performance is intended to be an offline-first Windows performance analyzer.

## Reporting

Do not publish sensitive telemetry, credentials, private paths, or unredacted process inventories in public issues.

Report suspected vulnerabilities privately through the repository's GitHub security reporting mechanism when available.

## Design security boundary

- Capture is out-of-process.
- No DLL injection or game-process hooks.
- Network access is opt-in, not a runtime requirement for local analysis.
- Shareable reports redact usernames, full paths, and raw process inventories by default.
