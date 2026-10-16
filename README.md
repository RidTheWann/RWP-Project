# RidTheWann Performance — 0.5.0-beta (candidate; publication pending)

**Measure. Diagnose. Prove.**

> Release scope (`0.5.0-beta` candidate; GitHub pre-release publication pending; `0.1.0-alpha`
> through `0.4.0-beta` records frozen): single-device
> validated only (see `docs/methodology.md` §7 and `docs/limitations.md`);
> installer is unsigned (expect a SmartScreen "unknown publisher" warning);
> PresentMon.exe is user-supplied (live capture is Windows-only via the GUI or
> the `rwp capture` CLI). Release notes: `docs/release/0.5.0-beta.md` (candidate), `docs/release/0.4.0-beta.md`,
> `docs/release/0.3.0-alpha.md`, `docs/release/0.2.0-alpha.md`,
> `docs/release/0.1.0-alpha.md` (frozen).

RidTheWann Performance (rwp) is an evidence-based gaming and streaming performance analyzer for Windows 11. It correlates frame timing with system state, detects reproducible failure patterns, and explains evidence without treating a heuristic score as probability.

> Measured example — single device, labeled (Windows 11 Build 26200 x64,
> Ryzen 5 5600G, 16 GiB, non-admin, release profile, 50 ms cadence, ~211
> target threads, 12,000 ticks):
>
> Sampler mean 18.855 ms/tick, rolling p99 28.352 ms/tick; system-only mode
> 0.016 ms mean. Probe-process CPU 23.75% of one core.
>
> This describes one box, not hardware in general. Method and validation
> basis: `docs/methodology.md`. What RWP cannot claim: `docs/limitations.md`.

## Product boundary

rwp follows this capability order:

Measure -> Analyze -> Diagnose -> Compare -> Explain -> Recommend -> Optimize

The engine is deterministic and offline-first. Capture is out-of-process. There is no DLL injection, game hook, overlay dependency, account, or background telemetry requirement.

## Quickstart — CLI

```powershell
rwp doctor --presentmon C:\Tools\PresentMon.exe
rwp capture --presentmon C:\Tools\PresentMon.exe --process-id 1234 --duration-seconds 60 --output-dir sessions/live
rwp import-presentmon capture.csv --process-id 42 --qpc-frequency-hz 10000000 --output-dir sessions/run
rwp analyze --session sessions/run --output-dir sessions/run-analyzed
rwp report --session sessions/run-analyzed --format md
rwp compare --baseline sessions/a --candidate sessions/b --same-scenario
rwp migrate --session sessions/legacy --output-dir sessions/migrated
```

Legacy reports or sessions go through `rwp migrate` (see `docs/troubleshooting.md`).
Full command reference: `docs/cli-reference.md`. Error codes: `docs/cli-errors.md`.

## Quickstart — GUI

Open RWP and follow the Home readiness checks. PresentMon remains user-supplied:
when prompted, choose a supported `PresentMon.exe` from its official source;
RWP does not bundle or download it. Open your game, select **Refresh**, choose
the running application, and select **Start measuring**. The default preparation
countdown is 5 seconds and the measurement duration is 60 seconds. Duration,
telemetry interval, and other technical controls stay under **Settings →
Advanced**. Results, verified local history, Before/After comparison, and
failure remediation are built in. Manual Windows/PresentMon acceptance remains
tracked in `docs/acceptance/gui.md`.

## Project status

The authoritative milestone, release-gate, audit, and verification status is maintained in docs/project-ledger.md and the active autopilot state in docs/autopilot/STATE.md. The release plan is docs/autopilot/MASTER_PLAN.md.

## Engineering constraints

- No DLL injection, hooks, or in-process overlay.
- No network access in normal operation (RC-10 evidence recorded).
- Missing measurements are represented as absence, never fabricated zeroes.
- Diagnostic evidence scores are not probabilities.
- Unknown is a first-class result.
- Local verification gates are mandatory: fmt, Clippy, tests, cargo-deny, cargo-audit, schema and docs checks (see `docs/local-gates.md`).

## License

Licensed under either the MIT License or Apache License 2.0 at your option.
