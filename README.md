# RidTheWann Performance

**Measure. Diagnose. Prove.**

RidTheWann Performance (rwp) is an evidence-based gaming and streaming performance analyzer for Windows 11. It correlates frame timing with system state, detects reproducible failure patterns, and explains evidence without treating a heuristic score as probability.

> Illustrative report — not a live measurement.
>
> Evidence score: 82/100 (heuristic evidence score, not probability).
>
> These numbers are illustrative only. They do not claim a live measurement, causal proof, or a tuning recommendation.

## Product boundary

rwp follows this capability order:

Measure -> Analyze -> Diagnose -> Compare -> Explain -> Recommend -> Optimize

The engine is deterministic and offline-first. Capture is out-of-process. There is no DLL injection, game hook, overlay dependency, account, or background telemetry requirement.

## Project status

The authoritative milestone, release-gate, audit, and verification status is maintained in docs/project-ledger.md and the active autopilot state in docs/autopilot/STATE.md. The release plan is docs/autopilot/MASTER_PLAN.md.

## CLI

    rwp analyze --input capture.json --pretty
    rwp analyze --input capture.json --output report.json
    rwp compare --baseline baseline.json --candidate candidate.json --same-scenario --pretty

Without --same-scenario, the result is explicitly marked inconclusive for A/B interpretation.

## Engineering constraints

- No DLL injection, hooks, or in-process overlay.
- No telemetry sent anywhere by default.
- Missing measurements are represented as absence, never fabricated zeroes.
- Diagnostic evidence scores are not probabilities.
- Unknown is a first-class result.
- Locked CI gates are mandatory: fmt, Clippy, tests, cargo-deny, cargo-audit, release build, and NSIS packaging.

## License

Licensed under either the MIT License or Apache License 2.0 at your option.
