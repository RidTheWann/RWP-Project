# ADR-0009: Run-level A/B inference and equivalence margins

## Status

Accepted.

## Decision

RidTheWann Performance performs controlled A/B comparison at the run-pair level, not from a single baseline/candidate capture.

The primary inferential metric is P99 frame time. Each matched pair records baseline and candidate P99 values under the same intended workload and records both frame sample counts. The analysis uses:

- a configurable minimum number of comparable paired runs;
- a minimum smaller/larger sample-count ratio for pair eligibility;
- a practical equivalence margin expressed as relative percentage change;
- an exact two-sided sign test over informative pairs;
- median relative improvement as the descriptive effect size.

Pairs whose change falls inside the equivalence margin are practical ties and are excluded from the sign test.

The engine may return `Improved` when the candidate wins directionally and the exact sign-test p-value is at or below alpha; `Regressed` under the symmetric condition; `Equivalent` when every pair remains inside the practical-equivalence margin; or `Inconclusive` when there are too few pairs or directional evidence is not statistically supported.

## Interpretation limits

This is run-level evidence, not a causal proof. It does not control for every confounder and does not claim that an observed change was caused exclusively by a configuration change.

A single-run delta remains available as a descriptive comparison, but it is not labeled inferential.

## Rationale

The design prevents common false conclusions from one unusually good or bad capture and makes the practical significance threshold explicit.

## A/A calibration

The CLI's A/A calibration estimates a workload-specific starting equivalence margin from
matched runs of the same scenario. For each pair it computes the absolute relative P99
frame-time difference, then recommends `max(1%, empirical P95)` as the margin. For the
selected two-sided alpha, the CLI computes the smallest informative-pair count whose best
possible exact sign-test p-value can reach that alpha. This is the informative, non-tie pair
count; practical ties or other ineligible pairs can require more total runs. At the default
alpha of 0.05 the minimum is six pairs; changing alpha changes the recommendation.

These values are recommendations derived from the supplied A/A runs, not universal
hardware guarantees. They do not establish causality, cross-machine equivalence, or
adequate statistical power for every workload; users must preserve scenario and run
provenance when applying them.