# Contributing

## Repository branch policy

The repository has exactly two canonical branches:

- `development`: the only working branch. All implementation, fixes, tests, documentation, and release preparation are performed here.
- `main`: the release branch. Promote only the audited, CI-green `development` lineage to `main`.

Never create a new branch for project work. Do not create `feature/*`, `fix/*`, `task/*`, `autopilot/*`, or any other temporary branch. Do not continue development directly on `main`.

When a batch is ready, finish and verify it on `development`, then promote `development` to `main`. This repository policy applies equally to local Git work and GitHub.

## Development and CI verification

Use the pinned toolchain from `rust-toolchain.toml` when working with Rust. Local builds and tests are optional developer tools, not a project policy gate; contributors are not required to build or run the full verification suite locally before CI.

All project work stays directly on `development`; never create a task branch. Keep the product invariants intact: out-of-process capture, no DLL injection/hooks/overlay, offline-first analysis with no outbound telemetry, deterministic output, and no production `unwrap`/`expect`/`todo!`.

Do not claim local verification passed unless the relevant commands were actually run. CI remains responsible for configured build, test, schema, dependency, and platform gates.

## Hard CI trigger gate

Deep audit remains mandatory before intentionally triggering CI:

- Re-audit the current diff against `docs/autopilot/MASTER_PLAN.md`, applicable AUD findings, and the product invariants.
- Check the affected call sites, tests, schemas, documentation, and workflow effects; add or update regression tests for behavior changes.
- Fix known issues identified by the audit before triggering CI. Do not knowingly use CI as a substitute for reviewing the change.
- Until the audit is complete, development-state commits must include `[skip ci]`.
- The first commit intentionally allowed to trigger CI must include the trailer `Deep-Audit: PASS`, only after the audit above has actually passed.
- Do not require a `Local-Verification: PASS` trailer or local build/test execution as a prerequisite for triggering CI.
- After a CI failure, inspect the failing evidence, audit the root cause, and correct it; do not weaken tests or lint gates. Trigger a new CI run after the correction has been reviewed.
- The same audit-before-trigger rule applies to `main`; `main` remains release-only and must never be used to bypass audit.
- Manual workflow dispatch is reserved for explicitly authorized final validation/release work, not routine debugging.

## Commit messages

Use Conventional Commits:

    feat(analysis): add deterministic stutter detector
    fix(core): reject non-finite frame samples

Keep library crates explicit about errors and avoid unwrap/expect in production code.

## Verification workflow

Keep changes consolidated into a coherent batch on `development`, then deep-audit the complete diff before intentionally triggering CI. Local builds/tests are optional and are not a CI prerequisite. CI is responsible for the configured pinned-toolchain, formatting, lint, test, schema, dependency, installer, and documentation gates. If CI fails, inspect the failing evidence and correct the root cause; never weaken tests or lint gates.

### Optional local format guard (reduces CI churn)

CI pins Rust 1.99.0 (see `rust-toolchain.toml`); local `cargo fmt` must use the same pinned toolchain or you will generate layout churn. To catch `cargo fmt --all -- --check`, `git diff --check`, and `tools/check_docs.py` failures before pushing, enable the optional advisory hook once per clone:

    git config core.hooksPath .githooks

The hook in `.githooks/pre-commit` only reports; it never rewrites files. It is not a policy gate and must not replace the mandatory deep audit before triggering CI.

CI policy follows the two-branch topology: pushes to `development` run audited development gates; pull-request validation targets `main` and may only originate from `development`; release validation runs on `main`. Do not create another branch merely to satisfy CI or release workflow requirements.
