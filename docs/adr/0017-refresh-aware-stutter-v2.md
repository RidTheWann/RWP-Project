# ADR-0017 — Refresh-aware stutter rule v2

- **Status:** accepted
- **Date:** 2026-10-08
- **Scope:** WP3 / AUD-08

## Context

The legacy stutter rule requires an absolute 30 ms frame-time floor. That suppresses shorter hitches on high-refresh displays even when the hitch is materially larger than the display frame period.

Behavioral changes must be versioned and old rule behavior must remain replayable.

## Decision

Add a separate refresh-aware detector configuration and API:

- rule id: `stutter.v2.refresh_aware`;
- threshold: `max(k * baseline, baseline + refresh_period_ms)`;
- `refresh_period_ms = 1000 / refresh_rate_hz`;
- refresh rate is required input for v2 and is never synthesized when unavailable;
- the legacy detector remains available as `stutter.v1.absolute`.

The v2 detector records its rule id, refresh rate, refresh period, multiplier, baseline-window settings, and merge gap in deterministic detector provenance.

Golden traces cover 60 Hz, 144 Hz, and 240 Hz using a fixed deterministic seed.

## Consequences

- At higher refresh rates, shorter hitches can be detected without lowering the relative multiplier.
- Existing V1 results remain reproducible from the legacy configuration.
- Live-session integration must wait for a verified active-display refresh-rate source; missing refresh data remains None.

## Acceptance

The synthetic seeded 20 ms hitch must be rejected at 60 Hz and detected at 144 Hz and 240 Hz.

