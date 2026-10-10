# Testing Strategy

## M1

M1 covers deterministic platform-neutral analysis.

- Unit tests validate input contracts and edge cases.
- Property tests validate percentile monotonicity, observed-value membership, and duplication invariance.
- Golden traces validate stable frame pacing, isolated spikes, and distribution-sensitive statistics.
- Insta snapshot coverage locks a representative output projection.
- Criterion measures the 10,000-frame and 1,000,000-frame statistics paths. The one-million-frame case is the WP6 analyzer budget; it is not run by the default `cargo test` gate.

## Diagnostic golden traces

The planned labeled diagnostic corpus is:

- CPU game-thread bound
- GPU bound
- frame pacing
- thermal throttling
- VRAM pressure
- background interference
- streaming overhead
- mixed CPU/GPU
- contradictory evidence
- insufficient evidence / Unknown

The diagnosis corpus is consumed by the diagnostic engine in M5. M1 deliberately does not assign these labels.

## Windows integration

Hardware, PresentMon, ETW, PDH, NVML, OBS, and DXGI tests run only behind Windows-specific gates and ignored/integration suites once those adapters are implemented.

## RC-03 line coverage gate

The `coverage` CI job (Ubuntu only) collects line coverage with pinned
`cargo-llvm-cov 0.9.1 --locked` over the platform-neutral crates and enforces
`tools/check_coverage.py coverage.json` at a minimum of 85% lines per target:

- `rwp-analysis`, `rwp-diagnostics`, `rwp-report`: whole crate.
- `rwp-capture`: parser scope only (`src/csv.rs` and `src/command.rs`).

The gate fails with the crate name and its actual percentage when a target
drops below the threshold; raise coverage with new tests rather than lowering
the threshold (lowering needs an ADR). Windows-only blocks are compiled out on
the Linux job, while `#[cfg(unix)]` stub-process tests cover the runner paths
there. Reproduce locally with the same pinned toolchain plus the
`llvm-tools-preview` component, then run the gate script against the produced
JSON report.

## RC-03 mutation and fuzz baselines

Two non-gating workflows record long-running quality evidence as artifacts;
neither gates pushes, cancels pinned CI, or requires the `Deep-Audit` trailer:

- `mutation.yml` (RC-03 step 2): pinned `cargo-mutants 27.1.0 --locked` over
  `rwp-analysis` and `rwp-diagnostics`, per-package, `fail-fast: false`.
- `fuzz-scheduled.yml` (RC-03 step 3 / RC-08): pinned `cargo-fuzz 0.13.2
  --locked` on a nightly toolchain over the `fuzz/` libFuzzer workspace. Four
  targets cover the untrusted-input boundaries: the strict and tolerant
  PresentMon CSV parsers (`csv_strict`, `csv_tolerant`), the report JSON reader
  (`report_json`), and the session manifest JSON reader (`session_json`).

`fuzz/` is a self-contained cargo-fuzz workspace (`[workspace] members = ["."]`)
and is listed in the root workspace `exclude`; the stable 1.99.0 format, Clippy,
test, and coverage gates never compile it. Each target ships a seed input under
`fuzz/corpus/<target>/` (the minimal PresentMon CSV for the parser targets and
the report/session manifest goldens for the JSON targets) so a fresh run starts
from a valid grammar. Run a target locally with a nightly toolchain and the
pinned tool: `cargo install cargo-fuzz --version 0.13.2 --locked` then
`cargo fuzz run csv_strict`. Crashers land in `fuzz/artifacts/` and are uploaded
only on failure.
