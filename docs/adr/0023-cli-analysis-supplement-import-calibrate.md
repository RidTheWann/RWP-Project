# ADR-0023 — Persisted CLI Analysis, Paired Comparison, and A/A Calibration

- Status: Accepted
- Date: 2026-10-09
- Scope: WP4 / AUD-16 / RC-06

## Decision

The CLI operates on immutable persisted session directories.

### Import

`rwp import-presentmon` parses a PresentMon CSV with an explicit QPC frequency and writes a complete initial session directory. The import report is deterministic and contains no fabricated telemetry; telemetry remains an empty JSONL stream.

### Analyze

`rwp analyze --session <dir> --output-dir <dir>` reads and verifies the input manifest, rebuilds the deterministic report, runs the diagnosis engine against persisted telemetry, and writes a new immutable analyzed session. It does not mutate the source session.

### Report

`rwp report --session <dir> --format json|md|html` reads only verified persisted data. Markdown and HTML are self-contained and contain no external assets or network references.

### Compare

Single-session comparison uses the existing deterministic `compare` API. Paired comparison uses:

`<pairs>/<pair-id>/baseline/<session files>`

and

`<pairs>/<pair-id>/candidate/<session files>`.

Each session manifest is verified before its report contributes to `paired_infer`.

### Calibrate

A/A calibration uses the same pair layout. For each pair, the absolute relative P99 frame-time difference is computed. The empirical P95 is used as the observed variability bound, with a 1% floor:

`equivalence_margin_percent = max(1%, empirical P95)`.

The CLI recommends the minimum informative (non-tie) pair count that can reach the requested two-sided alpha under an exact sign test. At alpha 0.05 this is six pairs; changing alpha updates the recommendation. Practical-equivalence ties and other ineligible pairs can require more total runs. This is a statistical eligibility floor, not a guarantee that the experiment is adequately powered or a hardware-wide claim.

## Provenance

Compare and calibrate consume only manifest-verified sessions. Pair IDs are taken from sorted directory names and passed unchanged to the deterministic paired inference engine. No timestamps or random identifiers are introduced into results.

## Limitations

Calibration does not establish causality, cross-machine equivalence, or statistical power beyond the configured paired inference contract. Real hardware acceptance and sufficient A/A runs remain release criteria and are tracked in STATE.md.
