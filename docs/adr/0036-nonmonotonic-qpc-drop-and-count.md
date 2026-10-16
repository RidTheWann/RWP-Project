# ADR-0036 — Drop isolated out-of-order QPC rows (refines AUD-07)

- **Status:** accepted
- **Date:** 2026-10-16
- **Scope:** WP2 / AUD-07
- **Refines:** AUD-07 monotonic policy, ADR-0003 (CSV adapter tolerance)

## Context

AUD-07 set one monotonic policy: equal timestamps accepted, backwards
timestamps rejected — the rejection failed the whole capture
(`NonMonotonicQpc`). The first live capture against genuine PresentMon 2.6.0
(dwm target) hit this at row 136: `538411452922 -> 538411058509` (backwards
by ~39 ms at 10 MHz). Same-process rows passed every other check, so the
inversion reads as out-of-order emission from multi-threaded presents, not
a broken clock. Failing the session on one isolated row makes live capture
unusable on real data. A second defect surfaced in the same run: the CLI
classified the session-wrapped CSV failure as `RWP-E5100` (wrong
remediation) because `classify` never looked inside `SessionError`.

## Decision

1. The parser drops a row whose `CPUStartQPC` moves backwards, counts it in
   a new `PresentMonCapture.dropped_nonmonotonic_rows` field (kept separate
   from the swap-chain `discarded_frame_count`, which is schema-visible),
   and keeps the previous high-water mark so later rows still validate.
   Equal timestamps stay accepted. A capture that loses every row still
   fails with `EmptyCapture`.
2. The count surfaces without any schema change: one `TelemetryWarning`
   with stable shared wording in live sessions (counted in the manifest),
   plus a one-line stderr notice on `rwp capture` and `rwp import-presentmon`.
3. `PresentMonCsvError::NonMonotonicQpc` is removed (now unreachable);
   genuinely corrupt input still fails via the remaining variants.
4. `classify` now refines `SessionError`: `CaptureCsv` (and
   `Capture(Csv)`) keep `RWP-E2200`, `CaptureConfig` keeps `RWP-E2000`,
   other lifecycle failures keep `RWP-E2100`; the rest stays `RWP-E5100`.
5. No drop-fraction threshold is introduced: with a single field data
   point, any cutoff would be invented. The count is explicit precisely so
   a threshold can be set from data later.

## Consequences

- Live captures survive isolated out-of-order rows with visible provenance
  instead of failing; error codes point at the true boundary again.
- Rejected alternative: sorting the whole capture by QPC (larger semantic
  change, fabricates adjacency for genuine clock artifacts) and keeping the
  hard failure (product unusable on observed real output).

## References

- AUD-07 (monotonic policy), ADR-0003 (CSV adapter), ADR-0011
  (Unknown stays first-class; absence is recorded, not fabricated)
