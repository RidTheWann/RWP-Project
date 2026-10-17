# Product Specification

RidTheWann Performance (rwp) is an evidence-based gaming performance analyzer for Windows 11.

Tagline: Measure. Diagnose. Prove.

Capabilities ship in this order; the order is a product invariant (a later
capability may only be added after the earlier ones ship):

Measure -> Analyze -> Diagnose -> Compare -> Explain -> Recommend -> Optimize

Shipped today ends at Explain: Recommend and Optimize are future milestones
with no outputs in the current product.

M1 scope:
- platform-neutral normalized data contracts;
- deterministic statistics;
- explicit low-FPS definitions;
- robust local-median stutter detection;
- property, golden, snapshot, and benchmark tests.

Implemented through M8 foundation:
- platform-neutral normalized data contracts;
- deterministic statistics and robust local-median stutter detection;
- out-of-process PresentMon capture;
- Windows QPC/CPU/RAM/NVML/thread/background/PDH telemetry;
- deterministic report.v1 serialization and CLI analysis;
- deterministic diagnostic rules with contradiction-aware heuristic evidence;
- single-run descriptive comparison;
- matched run-level P99 inference with explicit non-causal limits;
- sample-coverage comparability and practical-equivalence gating;
- shared-QPC live session orchestration;
- Tauri session IPC and native dashboard integration.

Out of scope until later milestones:
- recommendation and optimization outputs (the shipped pipeline ends at Explain);
- streaming-specific attribution (no encoder telemetry exists; `streaming_overhead` never fires);
- causal claims from single captures or uncontrolled comparisons;
- automated process discovery and richer Windows capture configuration UI;
- installer/release packaging.

Core guarantees:
- local analysis works without a network;
- capture is out-of-process;
- no DLL injection, hooks, or overlay;
- missing telemetry is never fabricated;
- evidence score is never presented as a probability;
- Unknown is a valid outcome.

Reference observer budget for later capture validation (RC-05 wording):
- average total CPU overhead at or below 1.0% over a steady capture at the default 50 ms cadence;
- p99 single-sample sampler duration at or below 2 ms at the default interval;
- no unbounded allocation growth in steady-state capture.
