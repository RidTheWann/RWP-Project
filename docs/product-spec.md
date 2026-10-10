# Product Specification

RidTheWann Performance (rwp) is an evidence-based gaming and streaming performance analyzer for Windows 11.

Tagline: Measure. Diagnose. Prove.

Capability order is a product invariant:

Measure -> Analyze -> Diagnose -> Compare -> Explain -> Recommend -> Optimize

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
- optimization behavior;
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

Reference observer budget for later capture validation:
- average CPU overhead below 1.0% over a steady 60-second capture;
- P99 instantaneous overhead below 2.0%;
- no unbounded allocation growth in steady-state capture.
