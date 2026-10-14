# Architecture Decision Records

Accepted and reconstructed decisions currently relevant to implementation:

- ADR-0001: workspace boundaries
- ADR-0002: external PresentMon capture
- ADR-0003: version-tolerant PresentMon CSV adapter
- ADR-0004: CPU per-thread telemetry source selection
- ADR-0005: QPC canonical timebase
- ADR-0006: nearest-rank statistics
- ADR-0007: JSON schema and benchmark identity
- ADR-0008: live-session metrics
- ADR-0009: run-level A/B inference and equivalence margins
- ADR-0010: native GUI technology
- ADR-0011: performance classification
- ADR-0012: deterministic report serialization
- ADR-0013: release artifact provenance
- ADR-0014: live-session orchestration and shared QPC timebase
- ADR-0015: PresentMon graceful cancellation and fallback
- ADR-0016: PresentMon version matrix
- ADR-0017: refresh-aware stutter rule v2
- ADR-0018: active-display refresh-rate source
- ADR-0019: thermal and frame-pacing evidence rules
- ADR-0020: target GPU adapter selection and vendor-neutral telemetry
- ADR-0021: sampler overhead measurement instrumentation
- ADR-0022: deterministic session persistence and manifest provenance
- ADR-0023: persisted CLI analysis, paired comparison, and A/A calibration
- ADR-0024: GUI session history and provenance manifest
- ADR-0025: opt-out for process and thread telemetry probing
- ADR-0026: background-interference alignment contradiction
- ADR-0027: sampler overhead single-device baseline and cadence lock
- ADR-0028: PresentMon distribution and verification
- ADR-0029: WebView2 runtime install mode
- ADR-0030: no automatic updates
- ADR-0031: `rwp capture` CLI on the shared session orchestrator
- ADR-0032: graceful PresentMon stop without a caller console (amends ADR-0015)

ADR-0001..0008 and ADR-0011..0013 are reconstructed from the project ledger and implementation contracts; reconstruction status is explicit in each record.

Supplement detail records share the same ADR number prefix: each of ADR-0021, ADR-0022, ADR-0023, and ADR-0024 has two files (one canonical indexed summary plus one `-supplement-*` detail file preserving the original design text): `0021-sampler-overhead-supplement-*`, `0022-session-persistence-supplement-*`, `0023-cli-analysis-supplement-*`, and `0024-gui-history-supplement-*`; the shorter `0021-*`, `0022-*`, `0023-*`, `0024-*` files are the canonical indexed summaries. Numbers are never renumbered.
