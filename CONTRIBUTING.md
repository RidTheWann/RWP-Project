# Contributing

## Working method (local-first, main only)

There is one working tree on `main`: no working branches, no promotion
flow, no trailers, no skip markers. CI (`.github/workflows/ci.yml` gates +
debug build, `release-alpha.yml` tag-only NSIS pre-release) runs as a
supplement on owner order — never as verification authority. Commit, push,
and tag only on explicit owner order. Every batch follows the same loop:

1. **Work** — implement in the tree, tests first for behavior changes.
2. **Gate** — run the full local gate in order (`docs/local-gates.md`,
   pinned toolchain 1.99.0 exactly).
3. **Audit** — the owner audits the batch against the plan and invariants.
4. **Record** — paste gate output plus numbers into `docs/autopilot/STATE.md`;
   add a CHANGELOG bullet only for user-facing changes.

Git is used for publish and tree-cleanliness checks only: commit/push/tag on
explicit owner order (`main` only), `git status` to guard a clean tree
before gates. Never use `git log`/`git diff` output as proof of behavior;
verify via direct file reads plus pasted gate output. Never remove the
`.git` directory without an explicit owner order.

## Product invariants (never break these)

- Out-of-process capture; no DLL injection, hooks, or overlay.
- Offline-first analysis with no outbound telemetry in normal operation.
- Deterministic output: no unordered maps or wall-clock in persisted content.
- No production `unwrap`/`expect`/`todo!`/`dbg!`; every `unsafe` block carries
  a `// SAFETY:` comment.
- Error taxonomy `RWP-E1000..9000` with stable codes, exit values, and
  remediation text.

## Change tiers

- **Detector, rule, threshold, or schema-semantic change**: bump the versioned
  identifier (`rule_id`, `analysis_config`, schema), keep old results
  reproducible, record the decision in a short ADR under `docs/adr/`, and
  update regression tests plus provenance. No silent semantic drift.
- **Everything else**: keep the change minimal, add or extend tests that pin
  the behavior, and update affected docs in the same batch.

## Tests and docs

- Bug fix → failing test first, then the fix.
- New behavior → regression tests plus golden updates where applicable.
- User-facing change → CHANGELOG bullet (one line per change, no process
  noise); process-only work goes to STATE only.
- Docs that describe behavior must match the tree; `tools/check_docs.py` and
  `tools/check_links.py` enforce consistency locally.

## What not to do

- Do not claim verification without pasting the actual command output.
- Do not weaken tests or lint gates to get green.
- Do not invent measurements, hardware results, or provenance.
- Do not commit binaries, secrets, private paths, or unredacted fixtures.
- Do not touch `.git` history, create working branches, or commit/push/tag
  without an explicit owner order; do not reference CI runs, PRs, or
  trailers as authority; per-batch local gates are the verification.

Local hygiene (optional): the advisory hook in `.githooks/` reports fmt and
docs-checker failures before they reach the gate. It never rewrites files
and never replaces gate runs.
