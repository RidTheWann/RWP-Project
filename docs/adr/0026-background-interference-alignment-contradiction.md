# ADR-0026 — Background-interference alignment contradiction

- Status: accepted
- Date: 2026-10-10
- Scope: WP6 / RC-04

## Context

`background_interference.v2` emits its primary supporting evidence (weight 70) whenever at
least one detected stutter event overlaps background CPU activity at or above the configured
threshold, and records the aligned/total fraction only as provenance. A capture where one of
many stutter events aligned therefore received the same maximum heuristic score as a capture
where every event aligned, overstating the hypothesis that background load explains the
observed degradation.

## Decision

1. Version the rule as `background_interference.v3`.
2. Keep the v2 alignment-window construction and burst threshold unchanged.
3. When the fraction of stutter events that overlap a background burst is below
   `background_min_aligned_fraction` (default 0.5), emit `background.low_alignment` as
   contradicting evidence with weight 40.
4. Serialize the new threshold in the diagnostic parameters and validate it as a finite
   value in `(0, 1]`.
5. Keep the rule inactive when no stutter event aligns, regardless of the activation score.

## Consequences

- A low-alignment capture still activates Background Interference when at least one burst
  aligns, but with a reduced score (100 to 60 at default thresholds) and explicit
  contradicting evidence; callers that raise the activation score above the reduced score
  will not activate.
- The alignment provenance (`aligned_events` / `total_events`) remains serialized for audit.
- Previously produced reports remain reproducible under the v2 rule identifier.
- The 0.5 default is a conservative engineering default; reference-hardware calibration
  remains pending under WP6/RC-05.

## References

- docs/diagnostics.md
- crates/rwp-diagnostics/src/lib.rs (`evaluate_background_interference`)
