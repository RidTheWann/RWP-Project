# Slim STATE proposal (2026-10-17, Batch 1) — PROPOSAL, owner may correct

Status: this file is a proposal only; `docs/autopilot/STATE.md` is unchanged by it.
To apply: replace STATE.md body with the text inside the fence below, keeping the
Batch 1 entry newest. Omitted Log entries live in git history plus
`docs/autopilot/archive/STATE-log-2026-10-12-to-2026-10-15.md` (sha256 in Batch 1 entry).

```markdown
---
plan_version: 1
last_updated: 2026-10-17T02:48:00+07:00
repo_ref: "main @ 914662a; Windows CI gates + debug build PASS in run 38957035347"
active_wp: "UX-REDESIGN-0.5.0-beta (current main gates + debug build PASS; v0.5.0-beta tag/release BLOCKED_ON_TOOL; local gate and packaged smoke UNVERIFIED)"
---

## Status WP (product code targets 0.5.0-beta; historical WP/RC entries and their evidence remain preserved; UX redesign implementation/documentation committed; current main @ 914662a passed Windows gates + debug build in run 38957035347; v0.5.0-beta tag/release BLOCKED_ON_TOOL; packaged smoke UNVERIFIED and hardware acceptance remains BLOCKED_ON_USER)

| WP | status (DONE/IN_PROGRESS/BLOCKED_ON_USER/UNVERIFIED) | catatan |
|---|---|---|
| WP0 | DONE | Local gate authority + Cargo.lock + --locked verified 2026-10-11 (fmt 0, clippy 0, test 298 passed, deny ok, audit 0 errors/2 allowed, schemas ok, docs ok, unit 10/10, links 68 docs ok) on pinned 1.99.0. CI (`ci.yml` gates + debug build) runs as supplement since 2026-10-16; local gate remains the authority (`docs/local-gates.md`). |
| WP1 | DONE | AUD-01..07 code fixes + regression tests in tree; local gate green 2026-10-11. AUD-03 graceful-stop runtime remains UNVERIFIED (constant pinned by test only). |
| WP2 | DONE | Dominant-SwapChain/FrameType filtering, capped stderr (64 KiB) + preflight, symmetric coverage in tree; fixture E2E green. Real PresentMon v2 fixtures remain BLOCKED_ON_USER. |
| WP3 | DONE (alpha) | Refresh-aware stutter v2 + goldens, thermal/frame-pacing v2, GPUBusy v2, target-GPU selection, alpha-derived calibration, `no_process_probing` opt-out, 10 ms minimum, ADR-0027 baseline (full-probing mean 18.855 ms, p99 28.352 ms; system-only 0.016 ms). ETW/cycle-time cross-checks + multi-device repeats BLOCKED_ON_USER. |
| WP4 | DONE (alpha) | CLI import-presentmon/analyze/report/compare/doctor/calibrate/migrate + GUI capture/history/compare + fixture E2E (import -> analyze -> compare -> report) green 2026-10-11. Real-data acceptance BLOCKED_ON_USER. |
| WP5 | DONE (alpha) | Dep direction fixed, Tauri hardening (path validation, Job KILL_ON_JOB_CLOSE, AppManifest parity), rolling logger, RWP-Exxxx codes, `docs/threat-model.md` draft. Owner threat-model sign-off BLOCKED_ON_USER (PENDING). |
| WP6 | DONE (alpha) | Fase 1 coverage PASS 4/4 (analysis 91.1%, diagnostics 94.6%, report 92.7%, capture parser 86.6%); Fase 2 mutants 492/504 = 97.6% vs >=70% gate; determinism/serde round-trip + stutter/percentile properties; 3 deterministic 256-case smoke tests (libFuzzer execution Linux-deferred). `streaming_overhead` deferred by design; real fixtures BLOCKED_ON_USER. |
| WP7 | DONE (alpha, unsigned) | SemVer/Changelog discipline (0.1.0), SBOM 10 files/517 components + licenses 104 crates + provenance.json (Fase 6), NSIS offline bundle re-verified Step 4 (219,086,102 bytes, SHA256 3d587dc1c9ed8c00e547cab45afa5c957a9abd05e03a6dfe0745b1936bb89f7b; Fase 6 also built default 2.11 MiB bootstrapper), SmartScreen documented, RC-10 evidence recorded, `rwp migrate` shipped. Signing BLOCKED_ON_USER. |
| WP8 | DONE (alpha) | Fase 7: methodology/limitations/troubleshooting/privacy + README measured example + CLI-ref from binary + CoC/templates + SECURITY channel + CONTRIBUTING rewrite; check_docs + check_links (68 docs) green 2026-10-11. |
| WP9 | BLOCKED_ON_USER | Hardware acceptance matrix (WP9/RC-07/RC-12) requires real PresentMon fixtures + >=2 machines; not run. |

## Status AUD (code DONE = fix + regression test in tree and local gate green 2026-10-11; runtime/hardware = UNVERIFIED or BLOCKED_ON_USER as noted)

| AUD | status (OPEN/FIXED/STALE/WONTFIX) | bukti (tes/laporan/STATE) |
|---|---|---|
| AUD-01 | FIXED | Supporting/contradicting helpers separated; polarity regression green locally 2026-10-11 (diagnostics 40 passed). |
| AUD-02 | FIXED | BackgroundInterference v2/v3 alignment + burst evidence with aligned/total provenance; rule-matrix regressions green locally. |
| AUD-03 | FIXED; verified against fake child for console and no-console parents; real PresentMon BLOCKED_ON_USER | Console-sharing spawn/stop plus the no-console attach-then-signal path (flag recorded at spawn, `StopOutcome::{Graceful, Killed}`, 2 s bound unchanged) with `TolerateTruncatedTail` + `interrupted=true` provenance; proven against the fake child from console parents (Batch 1, via `terminate_gracefully` delegating to `terminate_with_outcome`) and detached no-console drivers (Batch 5: graceful exits 0 with 3 rows, ignoring child killed after ~2 s with 2 rows tolerated). Real PresentMon runtime still BLOCKED_ON_USER. |
| AUD-04 | FIXED | SystemSampler seam; non-essential errors -> None + TelemetryWarning; QPC-invalid-config fatal; intermittent-failure test green. |
| AUD-05 | FIXED | catch_unwind -> Failed; regression green. |
| AUD-06 | FIXED | Default minimum_pairs=6 + minimum_pairs_for_alpha validation; brute-force sign-test property green. |
| AUD-07 | FIXED (refined ADR-0036) | Non-decreasing frame timestamps; equal accepted; isolated backwards-QPC rows dropped and counted (live 2.6.0 evidence) instead of failing the capture. |
| AUD-08 | FIXED | stutter.v2.refresh_aware + V1 provenance + 60/144/240 Hz goldens + display-refresh sourcing; stutter suite 20 passed locally. |
| AUD-09 | FIXED (code) / BLOCKED_ON_USER (cross-checks) | THREAD_QUERY_LIMITED_INFORMATION + bracketing timestamps (ADR-0004); local duty-cycle 26.14/51.32/76.23/99.80% (max err 1.32 pp). ETW/cycle-time + other-hardware repeats BLOCKED_ON_USER. |
| AUD-10 | FIXED (instrumentation + baseline) / RC-05 OPEN | SamplerStats + ADR-0021/0027: 12,000 ticks full-probing mean 18.855 ms, p99 28.352 ms; system-only 0.016 ms; probe CPU 23.75% of one core. RC-05 target not met; capture-wide + repeats BLOCKED_ON_USER. |
| AUD-11 | FIXED | thermal.v2 (hot + clock-drop + sample minimum) + frame_pacing.v2 (count/rate + tail); synthetic regressions green. |
| AUD-12 | FIXED (code) / BLOCKED_ON_USER (correlation) | Process-matched adapter + PDH GPU Engine per-PID (1 Hz) + None-when-absent; live probe returned observations once. Multi-adapter/vendor correlation BLOCKED_ON_USER. |
| AUD-13 | FIXED (code) / BLOCKED_ON_USER (fixtures) | Dominant SwapChain + FrameType tracking; real v2 fixture acceptance BLOCKED_ON_USER. |
| AUD-14 | FIXED (code) / BLOCKED_ON_USER (runtime) | stderr capped 64 KiB + drain + lossy + truncation flag; boundary tests green. Real PresentMon runtime BLOCKED_ON_USER. |
| AUD-15 | FIXED | Symmetric frame/telemetry coverage metadata; tests green. |
| AUD-16 | FIXED (alpha) / BLOCKED_ON_USER (real-data) | capture.v1/report.v2, session dirs + manifest v2, CLI doctor/import/analyze/compare/calibrate/report/migrate, redacted MD/HTML, GUI history/compare; fixture E2E green. Real-data acceptance BLOCKED_ON_USER. |
| AUD-17 | FIXED | `rwp-session` -> `rwp-ui` dep removed; app layer builds DashboardModel; gates green. |
| AUD-18 | FIXED (code) / BLOCKED_ON_USER (sign-off) | Path validation, Job KILL_ON_JOB_CLOSE (runtime-verified 0.06 s non-admin), AppManifest parity, no-deep-clone status; threat-model sign-off PENDING (owner). |
| AUD-19 | FIXED | Cargo.lock committed; every gate uses --locked (verified Step 1 2026-10-11). |
| AUD-20 | FIXED (alpha) | Ledger/ADR/schema foundations + Fase 7 doc sync; check_docs + check_links green 2026-10-11. |
| AUD-21 | IN_PROGRESS | Rule-matrix + Unknown/absent coverage expanded (diagnostics 40, system 25, capture 50, cli 32+3 e2e); coverage/mutants DONE (alpha); `streaming_overhead` deferred by design (never fires, no encoder telemetry); real fixtures + libFuzzer execution BLOCKED_ON_USER (Linux-deferred, smoke tests green). |
| AUD-22 | FIXED (code) / UNVERIFIED (runtime) | Console-aware spawn/stop (`CREATE_NO_WINDOW` + attach-signal-free for console-less callers, direct delivery otherwise, `StopOutcome` reporting, 2 s bound kept) with fake-child proof for console and no-console parents; real-PresentMon GUI cancel stays BLOCKED_ON_USER. |
| AUD-23 | FIXED (code) / UNVERIFIED (runtime) | Killed stops emit a `tracing` event (GUI allowlist records it) and `rwp capture` prints an ETW-hygiene hint; `--stop-existing-session` (default off) is wired on the CLI and documented with the `logman` recovery in troubleshooting; leftover-ETW on a real kill stays BLOCKED_ON_USER. |
| AUD-24 | FIXED (code) / UNVERIFIED (runtime) | `rwp capture` installs `rwp_system::ConsoleStopGuard`: first Ctrl+C/Break flips the shared stop flag into `SessionRunner`, the partial session persists with `interrupted=true`, exit 0 with an `interrupted: partial session saved at <dir>` stderr notice (owner decision); repeats keep default handling so the process still ends and the Job kills PresentMon. Probe-tested with real CTRL+BREAK; live-Ctrl+C on real PresentMon stays BLOCKED_ON_USER. |
| AUD-25 | FIXED (code) / UNVERIFIED (runtime) | CLI seam `run_capture_session` (injected runner closure; zero `rwp-session` API change, ~40 non-test lines) tested with fakes persisting through the real `persist_session` (manifest fields verified, interrupted notice asserted, errors propagate with temp cleanup); RWP-E5100 mapping tested on every platform; duration reuses rwp-capture's own timed validation (`RWP-E2000`). Live-run evidence stays BLOCKED_ON_USER. |
| AUD-26 | FIXED | `tools/check_links.py` now enumerates through `collect_documents()` with a deterministic skip set (`target`, `.git`, `smoke-out`, `node_modules`, `__pycache__`), no git dependency; `tools/test_check_links.py` pins skipping, counting, broken-link and bad-anchor detection (unit checker 13/13). |

## Status RC (alpha 0.1.0: DONE = alpha evidence in tree + local gate green; v1.0 hardware/sign-off stays BLOCKED as noted)

| RC | status | evidence |
|---|---|---|
| RC-01 | IN_PROGRESS | P0/P1 code fixes + regressions green locally 2026-10-11; AUD-03 runtime UNVERIFIED; hardware-dependent AUD-09/10/12 + fixtures BLOCKED_ON_USER. |
| RC-02 | DONE (alpha) | Cargo.lock present; every gate uses --locked (Step 1 2026-10-11 green on 1.99.0). CI runs as supplement since 2026-10-16; Windows local gate is the authority per local-gates.md. |
| RC-03 | DONE (alpha) | Fase 1 coverage 4/4 PASS (>=85%); Fase 2 mutants 492/504 = 97.6% vs >=70% gate; determinism/serde + smoke mutation tests green. libFuzzer execution Linux-deferred. |
| RC-04 | IN_PROGRESS | Fire/no-fire/contradiction/data-kurang matrix green for shipped rules; `streaming_overhead` deferred by design (never fires). |
| RC-05 | IN_PROGRESS (target not met) | ADR-0027: full-probing mean 18.855 ms, p99 28.352 ms at 50 ms cadence; probe CPU 23.75% of one core — does not meet <=1% total / <=2 ms p99. System-only mode (0.016 ms) is the documented low-overhead option. Repeats + capture-wide BLOCKED_ON_USER. |
| RC-06 | DONE (alpha, fixture) | Fixture E2E import -> analyze -> compare -> report + manifest verification green; GUI history/load/compare wired. Real-data acceptance BLOCKED_ON_USER. |
| RC-07 | BLOCKED_ON_USER | Real capture/hardware evidence required (>=6 captures, calibration A/A). |
| RC-08 | DONE (alpha, sign-off pending) | Size/line/UTF-8/depth/count bounds + no-panic properties + threat-model draft in tree; gates green. Owner threat-model sign-off PENDING (BLOCKED_ON_USER). |
| RC-09 | DONE (alpha, unsigned) | Fase 6: NSIS 2 bundles + SHA256 + SBOM/licenses/provenance + SmartScreen doc + migrate; Step 4 re-verify 2026-10-12: offline bundle 219,086,102 bytes SHA256 3d587dc1..., per-user silent install/uninstall OK, app start OK, user data preserved. Signing BLOCKED_ON_USER (certificate/service). |
| RC-10 | DONE (alpha) | Fase 6: zero-socket code inventory + Windows dep-graph zero HTTP-client (reqwest/hyper only non-Windows edges) + CLI proxy-off run OK; 20 s app run zero connections per PID. |
| RC-11 | DONE (alpha) | Fase 7 docs + check_docs + unit 10/10 + check_links 70 docs green 2026-10-11. |
| RC-12 | BLOCKED_ON_USER | Hardware acceptance matrix required; not run. |

## Log (terbaru di atas; CI-era entries archived — see docs/autopilot/archive/STATE-ci-history.md)

- 2026-10-17 BATCH 1 document reconciliation (docs-only; no code, no behavior change): MASTER_PLAN §0/WP0-4/WP7-1/template-§6/WP9/execution-rule + CONTRIBUTING now state git/tag/CI-as-supplement per docs/local-gates.md (commit/push/tag on explicit owner order, main only; behavior claims via file reads + pasted gates, never log/diff); §2 baseline → 0.5.0-beta; ledger snapshot + Workspace version → 0.5.0-beta; cli-reference header regenerated from rebuilt binary (`rwp --version` = `rwp 0.5.0-beta`, top `--help` block byte-identical); STATE WP0/RC-02 rows drop "CI retired"; 16 pre-2026-10-16 Log entries moved verbatim to docs/autopilot/archive/STATE-log-2026-10-12-to-2026-10-15.md (sha256 3c1eceeb1544d154b9d46e7fbf76ac2c5ea5e2ea664e95405791f44f2eb16cef; 16/16 byte-identical vs HEAD, 0 leftover); new tools/test_check_docs.py (4 tests, RED-first AttributeError on missing check_versions) + check_docs version pin (ledger/cli-reference/README vs Cargo.toml workspace version) now enforced; slim-STATE proposal (≤20 KB) for owner correction at docs/autopilot/archive/STATE-slim-proposal-2026-10-17.md. Uncommitted (rule #1).


- 2026-10-17 BATCH 0 local baseline on `main @ 5782d46638c02c58743c6379a740332f11fa18ea` (pinned toolchain 1.99.0, `--locked`): all 10 local gates PASS — fmt exit 0; clippy exit 0; test 359 passed / 0 failed; deny ok (advisories/bans/licenses/sources ok); audit exit 0 (2 allowed warnings RUSTSEC-2024-0370 proc-macro-error unmaintained, RUSTSEC-2024-0429 glib unsound); workflow-security PASS; schemas PASS; docs PASS; Python tools 23/23 OK; links 84 docs PASS. Local checkout started behind at `1216ac9` (0.4.0-beta) and was fast-forwarded to `origin/main 5782d46` (no history rewrite); tree clean before and after gates; no code changed. Local tag `v0.5.0-beta` absent (only `v0.4.0-beta`); NSIS/install/smoke, real-capture and overhead-repeat evidence remain UNVERIFIED/BLOCKED_ON_USER. Uncommitted (rule #1).


- 2026-10-17 release workflow/docs sync verified: current head `914662ab13e6d3ea457109a963b37e0bfff02d3a` passed Windows gates and the debug workspace build in run `38957035347`. This run covers the 0.5.0-beta release workflow’s corrected beta examples/display name, release README status update, and the STATE synchronization. Earlier code-head verification remains run `38956343967` (359 Rust tests, Python 23/23, all dependency/security/schema/doc/link gates and `cargo build --locked --workspace` pass). This is CI evidence only; no local pinned-toolchain gate, NSIS bundle/SHA256SUMS, install/uninstall or installed GUI smoke has been run in this session. No `v0.5.0-beta` tag or GitHub release exists; the connector provides no tag/release write action, so publication cannot be completed from these tools. Hardware acceptance RC-05/RC-07/RC-12 remains blocked, and the unsigned-package limitation remains.


- [pointer] Older verification entries omitted here for size; pre-2026-10-16 entries are in
  docs/autopilot/archive/STATE-log-2026-10-12-to-2026-10-15.md (sha256 in Batch 1 entry);
  2026-10-16/17 entries remain in git history until the owner approves a second archive move.

## NEXT

Publication is the remaining tool blocker: the exact `v0.5.0-beta` tag is absent, and the available GitHub connector has no operation to create/push tags or publish releases. Do not create a branch or claim a simulated release. To finish publication, an owner/local checkout must tag the verified `main @ 914662a` and push the tag; the existing release workflow then performs its full gate, NSIS build, SHA256SUMS and pre-release publish. Before calling the release complete, verify the tagged run’s result/assets and perform local pinned-toolchain gates, installer install/uninstall and packaged GUI smoke. Real PresentMon/hardware acceptance RC-05/RC-07/RC-12 and signing remain explicitly unverified/blocked.
## Keputusan terbuka

- PresentMon graceful stop — default: process-group console control with immediate kill fallback when delivery fails; bounded wait only after successful request.
- PresentMon v2 fixtures — default: provide at least six anonymized real captures.
- CPU per-thread accuracy harness — default: run prescribed 25/50/75/100% duty-cycle harness and retain raw evidence.
- Sampler overhead — default: measure before changing 50 ms default.
- Code signing — default: use certificate/service when available; otherwise document SmartScreen limitation.
- PresentMon distribution — default: user-supplied executable for v1.0.
- WebView2 install mode — default: offline-capable documented mode.
- Auto-update — default OFF for v1.0.

## BLOCKED_ON_USER

- At least 6 real PresentMon v2 CSV fixtures — needed for WP2/WP6/RC-07/RC-08.
- ETW/QueryThreadCycleTime cross-check and repeat CPU per-thread measurements on other hardware — needed to complete WP3/AUD-09.
- Repeated sampler-overhead measurements on the reference workload and capture-wide CPU evidence — needed before changing the 50 ms default for WP3/AUD-10/RC-05.
- Authenticode certificate/signing service — needed for signed release; otherwise SmartScreen limitation must be documented.
- Hardware acceptance matrix — needed for WP9/RC-07/RC-12.
- Hardware matrix verdict: DEFERRED-EXPLICIT (owner verdict 2026-10-15: alpha definition + gamer load); the smoke flow still runs when the reference box is ready.
- Threat-model sign-off — APPROVED for alpha scope 2026-10-15 (revertible by owner with one word); re-review required before v1.0 RC freeze (see docs/threat-model.md).
```
