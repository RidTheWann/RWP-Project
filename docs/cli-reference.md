# CLI Reference (`rwp 0.1.0`)

Generated from the built binary, never hand-written:

```powershell
cargo build -p rwp-cli --locked            # pinned toolchain 1.99.0
./target/debug/rwp --help
./target/debug/rwp <COMMAND> --help        # per subcommand below
```

Regenerate this file when CLI surface changes; the generator output is
authoritative over this text.

```text
Evidence-based Windows gaming and streaming performance analyzer

Usage: rwp.exe [OPTIONS] <COMMAND>

Commands:
  import-presentmon  Import a PresentMon CSV into a persisted session
  analyze            Analyze a persisted session and write a new immutable analyzed session
  report             Render a persisted report
  compare            Compare two persisted session reports
  doctor             Run local preflight diagnostics for RWP and an optional PresentMon executable
  calibrate          Calibrate A/A paired sessions and recommend an equivalence margin
  migrate            Migrate a legacy report or session to the current schema version
  help               Print this message or the help of the given subcommand(s)

Options:
      --json     Emit machine-readable JSON for command failures
  -h, --help     Print help
  -V, --version  Print version
```

## import-presentmon

```text
Usage: rwp.exe import-presentmon [OPTIONS] --process-id <PROCESS_ID> --qpc-frequency-hz <QPC_FREQUENCY_HZ> --output-dir <OUTPUT_DIR> <CSV>

Arguments:
  <CSV>  PresentMon CSV path

Options:
      --json
          Emit machine-readable JSON for command failures
      --process-id <PROCESS_ID>
          Target process identifier
      --qpc-frequency-hz <QPC_FREQUENCY_HZ>
          PresentMon QPC frequency in Hz
      --output-dir <OUTPUT_DIR>
          Destination session directory
      --refresh-rate-hz <REFRESH_RATE_HZ>
          Display refresh rate in Hz for refresh-aware stutter analysis
  -h, --help
          Print help
```

## analyze

```text
Usage: rwp.exe analyze [OPTIONS] --session <SESSION> --output-dir <OUTPUT_DIR>

Options:
      --json                     Emit machine-readable JSON for command failures
      --session <SESSION>        Input session directory
      --output-dir <OUTPUT_DIR>  Destination analyzed session directory
      --pretty                   Emit indented human-readable JSON to stdout after persistence
  -h, --help                     Print help
```

## report

```text
Usage: rwp.exe report [OPTIONS] --session <SESSION>

Options:
      --json               Emit machine-readable JSON for command failures
      --session <SESSION>  Persisted session directory
      --format <FORMAT>    Output format [default: json] [possible values: json, md, html]
      --redact <REDACT>    Redact identifying session fields from human-readable report output [default: true] [possible values: true, false]
      --output <OUTPUT>    Optional output file. Defaults to stdout
  -h, --help               Print help
```

## compare

```text
Usage: rwp.exe compare [OPTIONS]

Options:
      --baseline <BASELINE>
          Baseline session directory
      --json
          Emit machine-readable JSON for command failures
      --candidate <CANDIDATE>
          Candidate session directory
      --pairs <PAIRS>
          Root containing <pair-id>/baseline and <pair-id>/candidate session directories
      --same-scenario
          Treat both reports as the same intended workload/scenario
      --minimum-sample-ratio <MINIMUM_SAMPLE_RATIO>
          Minimum smaller/larger sample ratio for normal comparability [default: 0.8]
      --change-threshold-percent <CHANGE_THRESHOLD_PERCENT>
          Minimum relative metric change, in percent, before classifying direction [default: 1]
      --output <OUTPUT>
          Optional output file. Defaults to stdout
  -h, --help
          Print help
```

## doctor

```text
Usage: rwp.exe doctor [OPTIONS]

Options:
      --json                     Emit machine-readable JSON for command failures
      --presentmon <PRESENTMON>  Optional PresentMon executable to validate
      --bundle <BUNDLE>          Optional path for a redacted deterministic JSON diagnostic bundle
  -h, --help                     Print help
```

## calibrate

```text
Usage: rwp.exe calibrate [OPTIONS] --pairs <PAIRS>

Options:
      --json             Emit machine-readable JSON for command failures
      --pairs <PAIRS>    Root containing <pair-id>/baseline and <pair-id>/candidate A/A session directories
      --alpha <ALPHA>    Two-sided alpha used for the minimum-pair recommendation [default: 0.05]
      --output <OUTPUT>  Optional output file. Defaults to stdout
  -h, --help             Print help
```

## migrate

```text
Usage: rwp.exe migrate [OPTIONS]

Options:
      --json                     Emit machine-readable JSON for command failures
      --report <REPORT>          Standalone report JSON file using a legacy `rwp.report` schema
      --session <SESSION>        Persisted session directory using legacy schemas
      --output <OUTPUT>          Destination file for a migrated standalone report
      --output-dir <OUTPUT_DIR>  Destination directory for a migrated session
  -h, --help                     Print help
```

Failure taxonomy: `docs/cli-errors.md`. Troubleshooting walkthrough:
`docs/troubleshooting.md`.
