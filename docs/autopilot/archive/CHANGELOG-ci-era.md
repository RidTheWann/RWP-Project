# CHANGELOG CI-era entries (archived 2026-10-12)

CI workflows are gone (`.github/` absent, verified 2026-10-12); the GitHub
remote is used for publish only (branch `main` + tag `v0.1.0-alpha`) and local
gates in `docs/local-gates.md` are the sole verification authority. Entries
below were moved verbatim out of `CHANGELOG.md` on 2026-10-12 during the
post-alpha audit follow-up because they describe CI-internal process
(workflows, jobs, runs) rather than user-facing changes, per the CHANGELOG
header rule ("process-only work is recorded in `docs/autopilot/STATE.md`,
never here"). Kept as engineering record.

## Moved entries (verbatim, newest first as in CHANGELOG.md)

### Added

- Add a pinned `coverage` CI gate (Ubuntu, `cargo-llvm-cov 0.9.1 --locked`): per-crate line coverage must stay at or above 85% for `rwp-analysis`, `rwp-diagnostics`, `rwp-report`, and the `rwp-capture` parser scope (`src/csv.rs`, `src/command.rs`), enforced by `tools/check_coverage.py`.
- Add a non-gating `mutation.yml` baseline workflow (pinned `cargo-mutants 27.1.0 --locked`, schedule Monday 03:00 UTC + dispatch only): per-package matrix over `rwp-analysis` (337 mutants) and `rwp-diagnostics` (194 mutants), `fail-fast: false`, 360-minute timeout, `mutants.out/` artifacts retained 14 days. Results are recorded as artifacts; the >=70% gate follows only after a stable baseline.
- Add a non-gating `fuzz-scheduled.yml` workflow (pinned `cargo-fuzz 0.13.2 --locked`, nightly toolchain, schedule Monday 04:00 UTC + dispatch only): a self-contained `fuzz/` cargo-fuzz workspace with four libFuzzer targets (`csv_strict`, `csv_tolerant`, `report_json`, `session_json`), 300 s per target, `continue-on-error`, and crasher artifacts retained 14 days. `fuzz/` is excluded from the stable 1.99 workspace so nightly/libFuzzer dependencies never enter a gate.

### Changed

- Split CI release packaging out of per-push development runs to cut the ~30 minute cycle: the Windows job now runs only format, Clippy, and tests; `cargo-deny`/`cargo-audit` moved to a parallel Ubuntu job; release build plus Tauri CLI install, NSIS bundling, checksums, and artifact upload run on promotion PRs, `main`, the nightly schedule, and manual dispatch. No gate was removed and no action, pin, or permission changed; batches touching packaging paths must trigger a manual dispatch run before promotion.

### Fixed

- Ignore Python `__pycache__/` artifacts and add an optional advisory `.githooks/pre-commit` guard (`cargo fmt --check`, `git diff --check`, `check_docs.py`) enabled via `git config core.hooksPath .githooks`; CI remains the verification gate.
- Fix the generated stutter property test's assertion formatting after both CI Clippy jobs rejected closure expressions embedded in `prop_assert!`; place ADR-0025 under the four-digit filename convention required by the documentation checker.

### Security hardening

- Pass GitHub branch refs through step environment variables instead of interpolating them into shell scripts, reduce workflow token permissions to `contents: read`, and remove the unnecessary full-history checkout. Add a documentation-job regression checker for direct shell interpolation, required context mappings, token scopes, and checkout depth, with five unit tests covering safe and unsafe workflow fixtures.

### CI formatting remediation

- Applied the pinned Rust 1.99.0 rustfmt changes requested by CI #571 to the CLI doctor, CLI dispatch/tests, and diagnostic regression tests. This is formatting-only; diagnostic behavior and thresholds are unchanged.

### CI compile and lint remediation

- Restored the PresentMon CSV resource-limit constants as real Rust declarations and added boundary regression tests for the 512 MiB input-size and 10 million frame-row guards without allocating oversized fixtures.
- Removed unsupported derives from a configuration error carrying `std::io::Error`, imported the runner in its regression test module, passed the child process by reference into the Windows Job Object owner API, documented public error and persisted capture-provenance fields exposed by `missing_docs`, added `// SAFETY:` comments to Win32 last-error reads, and fixed Windows session borrow/unused-import errors. Added the existing workspace `serde` and `rwp-system` dependencies required by doctor JSON serialization and PresentMon preflight; reconciled `Cargo.lock`, fixed the report-format test import, and made the CLI E2E process helper fallible with documented test-crate metadata. Also preserved the stable CSV error code through `PresentMonCaptureError::Csv` wrapping after an existing regression test exposed the classifier's generic-capture precedence; the failing test remains unchanged; also added the explicit `PathBuf` import required by the Windows-only session test after Clippy exposed it. Fixed the telemetry line-count loop to ignore an unused item binding while preserving blank-line filtering and the existing sample cap. A subsequent pinned CI pass is pending.

### CI workflow policy

- Aligned the CI trigger gate with the documented policy: deep audit remains mandatory, local builds/tests are optional, and pushes to `development` run platform, schema, dependency, installer, and documentation gates.
- Fixed the first development CI run's formatting failures using the exact pinned-runner rustfmt output, restored ADR-0021..0024 files referenced by the ADR index, and aligned MASTER_PLAN with the optional-local-verification policy. Re-triggered CI from `development` with commit `18486d3681f152889506647f3ea2f3219c00bf58` and `Deep-Audit: PASS`; the connector has not exposed the new run result yet.
