---
plan_version: 1
last_updated: 2026-10-12T02:00:00+07:00
repo_ref: USANG sejak 2026-10-10 — git tak dipakai lagi (nilai terakhir: main @ 902c512; jejak tertulis pindah ke Log + laporan)
active_wp: WP7 (alpha release prep)
---

## Status WP (alpha scope 0.1.0; v1.0 hardware acceptance stays BLOCKED_ON_USER; local gate 2026-10-11 green: fmt/clippy/test 298/deny/audit/schemas/docs/unit-10/links-68)

| WP | status (DONE/IN_PROGRESS/BLOCKED_ON_USER/UNVERIFIED) | catatan |
|---|---|---|
| WP0 | DONE | Local gate authority + Cargo.lock + --locked verified 2026-10-11 (fmt 0, clippy 0, test 298 passed, deny ok, audit 0 errors/2 allowed, schemas ok, docs ok, unit 10/10, links 68 docs ok) on pinned 1.99.0. CI retired. |
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
| AUD-03 | FIXED (code) / UNVERIFIED (runtime) | Process-group + bounded graceful wait + kill fallback + TolerateTruncatedTail + interrupted provenance in tree; constant-value test only — production call/runtime UNVERIFIED. |
| AUD-04 | FIXED | SystemSampler seam; non-essential errors -> None + TelemetryWarning; QPC-invalid-config fatal; intermittent-failure test green. |
| AUD-05 | FIXED | catch_unwind -> Failed; regression green. |
| AUD-06 | FIXED | Default minimum_pairs=6 + minimum_pairs_for_alpha validation; brute-force sign-test property green. |
| AUD-07 | FIXED | Non-decreasing frame timestamps; equal-accept/backwards-reject tests green. |
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

## Status RC (alpha 0.1.0: DONE = alpha evidence in tree + local gate green; v1.0 hardware/sign-off stays BLOCKED as noted)

| RC | status | evidence |
|---|---|---|
| RC-01 | IN_PROGRESS | P0/P1 code fixes + regressions green locally 2026-10-11; AUD-03 runtime UNVERIFIED; hardware-dependent AUD-09/10/12 + fixtures BLOCKED_ON_USER. |
| RC-02 | DONE (alpha) | Cargo.lock present; every gate uses --locked (Step 1 2026-10-11 green on 1.99.0). CI retired; Linux neutral gates retired with CI — Windows local gate is the authority per local-gates.md. |
| RC-03 | DONE (alpha) | Fase 1 coverage 4/4 PASS (>=85%); Fase 2 mutants 492/504 = 97.6% vs >=70% gate; determinism/serde + smoke mutation tests green. libFuzzer execution Linux-deferred. |
| RC-04 | IN_PROGRESS | Fire/no-fire/contradiction/data-kurang matrix green for shipped rules; `streaming_overhead` deferred by design (never fires). |
| RC-05 | IN_PROGRESS (target not met) | ADR-0027: full-probing mean 18.855 ms, p99 28.352 ms at 50 ms cadence; probe CPU 23.75% of one core — does not meet <=1% total / <=2 ms p99. System-only mode (0.016 ms) is the documented low-overhead option. Repeats + capture-wide BLOCKED_ON_USER. |
| RC-06 | DONE (alpha, fixture) | Fixture E2E import -> analyze -> compare -> report + manifest verification green; GUI history/load/compare wired. Real-data acceptance BLOCKED_ON_USER. |
| RC-07 | BLOCKED_ON_USER | Real capture/hardware evidence required (>=6 captures, calibration A/A). |
| RC-08 | DONE (alpha, sign-off pending) | Size/line/UTF-8/depth/count bounds + no-panic properties + threat-model draft in tree; gates green. Owner threat-model sign-off PENDING (BLOCKED_ON_USER). |
| RC-09 | DONE (alpha, unsigned) | Fase 6: NSIS 2 bundles + SHA256 + SBOM/licenses/provenance + SmartScreen doc + migrate; Step 4 re-verify 2026-10-12: offline bundle 219,086,102 bytes SHA256 3d587dc1..., per-user silent install/uninstall OK, app start OK, user data preserved. Signing BLOCKED_ON_USER (certificate/service). |
| RC-10 | DONE (alpha) | Fase 6: zero-socket code inventory + Windows dep-graph zero HTTP-client (reqwest/hyper only non-Windows edges) + CLI proxy-off run OK; 20 s app run zero connections per PID. |
| RC-11 | DONE (alpha) | Fase 7 docs + check_docs + unit 10/10 + check_links 68 docs green 2026-10-11. |
| RC-12 | BLOCKED_ON_USER | Hardware acceptance matrix required; not run. |

## Log (terbaru di atas; CI-era entries archived — see docs/autopilot/archive/STATE-ci-history.md)

- 2026-10-12 Alpha prep Step 6 final gate (pinned 1.99.0, --locked, after docs + installer): fmt EXIT 0; clippy EXIT 0 (-D warnings); test EXIT 0, 298 passed / 0 failed; deny EXIT 0 (duplicate-version warnings only); audit EXIT 0 (1296 advisories, 498 deps, 2 allowed RUSTSEC-2024-0370/0429); validate_schemas PASS; check_docs PASS; unit checker 10/10 PASS; check_links PASS (70 documents, 0 external). Zero source changes in alpha-prep batch (docs + release notes + STATE only; installer rebuilt from unchanged sources).
- 2026-10-12 Alpha prep Step 5 smoke test: BLOCKED_ON_USER — no `PresentMon.exe` on this machine (`Get-Command PresentMon.exe` empty, no `C:\Tools` bundle, no repo copy), so no real doctor -> import/capture -> analyze -> report -> compare flow was run and nothing was faked. CLI sanity only: `cargo run --locked -p rwp-cli -- doctor` EXIT 0 with `presentmon: null`, `offline.engine ok:true` (no network). Owner manual checklist: (1) install Intel PresentMon, note version + SHA-256; (2) `rwp doctor --presentmon <PATH>` must pass; (3) GUI live capture 60 s vs a real target (or `import-presentmon` a real CSV); (4) `analyze` -> `report --format md` -> two-session `compare --same-scenario`; (5) paste outputs + session manifest hashes into STATE before calling RC-06/RC-07 done.
- 2026-10-12 Alpha prep Step 4 installer (from current tree, binaries NOT committed): `cargo tauri build --bundles nsis` in `crates/rwp-ui-native/src-tauri` EXIT 0 (release build 43.80 s, makensis OK, 208.94 MiB). Bundle `target/release/bundle/nsis/RidTheWann Performance_0.1.0_x64-setup.exe`: 219,086,102 bytes, SHA256 `3d587dc1c9ed8c00e547cab45afa5c957a9abd05e03a6dfe0745b1936bb89f7b` (SHA256SUMS generated in temp dir outside repo, not committed). Silent per-user install `...setup.exe /S` OK (files at `%LOCALAPPDATA%/RidTheWann Performance/rwp-ui-native.exe` + `uninstall.exe`); app start OK (`rwp-ui-native` PID 12624 observed); silent uninstall `uninstall.exe /S` OK (app dir removed); user data preserved (`%LOCALAPPDATA%/RidTheWann` still present). Config is `offlineInstaller` (ADR-0029); default bootstrapper bundle not rebuilt in this batch.
- 2026-10-12 Alpha prep Step 3 release notes: created `docs/release/0.1.0-alpha.md` (what/works/install/quickstart/KNOWN ISSUES from docs facts only, ADR-0027 numbers reused, no new numbers) + linked from `docs/release/README.md`; check_docs PASS; check_links PASS (70 documents, 0 external).
- 2026-10-12 Alpha prep Step 2 docs sync: WP/AUD/RC tables rewritten to match Fase 0-7 Log + local gate evidence (WP7/WP8 DONE-alpha, WP9 BLOCKED, RC-10 DONE-alpha, WP6 DONE-alpha after Fase 2); CI-era entries moved to docs/autopilot/archive/STATE-ci-history.md; retired references (`docs/audit/2026-10-10-deep-audit.md`, `TODO-NEXT.md`, exchange folder) marked RETIRED (verified absent); ledger ADR index + README/CHANGELOG alpha updates follow in the same batch. check_docs + check_links re-run after edits (see Step 2 evidence).
- 2026-10-12 First push to RWP-Project: prior GitHub lineage retired; history below is engineering record. Batches aboard: Fase 2 quality gates, Fase 3 overhead baseline + ADR-0027, Fase 4 migrate + GUI, Fase 5 hardening + ADR-0028, Fase 6 release prep + ADR-0029/0030, Fase 7 docs/UX. Pending owner: threat-model sign-off, repo decisions already taken (name RWP-Project), instant-stable-notify thresholds (N/minimum, margin, notification default).
- 2026-10-12 Fase 7 DONE: 4 dokumen (methodology/limitations/troubleshooting/privacy, angka hanya DATA terukur) + README contoh nyata berlabel + quickstart CLI asli + CLI-ref generate dari biner + CoC/template di docs/community + SECURITY kanal privat + check_links.py hijau (68 dokumen) + CONTRIBUTING ditulis ulang + bedah CI MASTER_PLAN (§0/WP0/WP6-9/§4-6/template/execution-rule) + hook tanpa-git. Threat-model sign-off tetap PENDING milik owner. RC-11 setara-lokal. Nol kode/threshold/versi; nol klaim hardware.
- 2026-10-12 Fase 6 DONE: SemVer/Changelog formal (tetap 0.1.0 alpha, format 1-baris user-facing) + SBOM 10 file/517 komponen (cyclonedx 0.5.9) + licenses 104 crate dalam policy (about 0.9.2, about.toml baru) + provenance.json + NSIS 2 bundle (default 2.11MiB #00167A61 + offline 208.94MiB #B5368E0B, install/uninstall senyap per-user OK, app 20s NOL koneksi/PID x2, data user dipertahankan) + ADR-0029/0030 + SmartScreen doc + RC-10 (kode nol socket, graf Windows nol HTTP-client — lock reqwest/hyper hanya edge non-Windows; CLI + proxy-mati OK). Akuntansi 305-vs-298: 7 uji platform-gated (unix/not-windows) tak terkompilasi di Windows. DoD: biner tak dikomit. Gate 7/7 hijau. Nol signing/HW/versi-bump.
- 2026-10-12 Fase 5 DONE: Authenticode advisory (`verify_presentmon_authenticode` WinVerifyTrustEx offline, Signed/NotSigned/VerifyFailed; doctor `presentmon.signature` + bundle field; enforcement deferred) + ADR-0028 distribusi (user-supplied tetap v1.0) + Job KILL_ON_JOB_CLOSE runtime-verified (0.06s, non-admin) + threat-model sign-off PENDING + P2 marker-migrasi + ADR konsolidasi (0015 masuk indeks, 5 judul selaras isi) + schema-v1 diarsipkan. Tes +5 (picker, job, 2xauthenticode, doctor-signature); total 298 hijau. Gate 7/7 (fmt/clippy/test/deny/audit-2-warnings/schemas/docs/unit-10). Koreksi: klaim Fase 4 "system +1 picker" prematur (picker mendarat di Fase 5); total Fase 4 seharusnya dibaca dari log per-crate. Nol threshold/rule/versi; fitur windows-sys +2 tanpa crate baru.
- 2026-10-10 Keputusan owner: gate mutants >=70% DIADOPSI mengikat (baseline Fase 2 97.6%; 1 baris di `docs/local-gates.md` §6).
- 2026-10-10 Fase 2 DONE (locally-verified, pinned 1.99.0): mutants `cargo-mutants 27.1.0 --locked` — rwp-analysis 337 tested: baseline 94 missed/235 caught/8 unviable (71.4%) -> pasca-triase 11 missed/318 caught (96.7% viable); rwp-diagnostics 194 tested: baseline 60/115/19 (65.7%) -> 1 missed/174 caught (99.4%); gabungan 492/504 = 97.6% (gate >=70%). 12 survivor ekuivalen (call-site). Fuzz: 4 harness COMPILE nightly + 4 seed ada; EKSEKUSI libFuzzer terblokir di Windows -> Linux-deferred; mitigasi lokal: 3 smoke-test mutasi deterministik 256-kasus. Total workspace 229->284 tes, gate 7/7 hijau. Hanya 10 file uji tersentuh; nol threshold/rule/versi.
- 2026-10-10 Fase 1 DONE (locally-verified, `cargo-llvm-cov 0.9.1 --locked`): coverage PASS 4/4 — rwp-analysis 845/928 = 91.1%, rwp-diagnostics 1622/1714 = 94.6%, rwp-report 661/713 = 92.7%, rwp-capture parser-scope 1113/1285 = 86.6% (min 85.0%). Artefak `coverage.json` dihapus, tidak di-commit.
- 2026-10-10 Fase 0 DONE (locally-verified, pinned 1.99.0): `docs/local-gates.md` mengikat sebagai otoritas pengganti CI; versi tetap `0.1.0`. Gate: fmt PASS (exit 0); clippy PASS (exit 0); test PASS (229 passed); deny 0.20.2 PASS; audit 0.22.2 PASS (2 allowed warnings RUSTSEC-2024-0370/0429); schemas PASS; docs PASS; unit checker 10/10 PASS.
- 2026-10-10 Local-only cutover: GitHub remote deleted by owner (verified dead), `.github/` removed, single branch renamed `development` -> `main`. No promotion/merge-back/freeze/trailer rituals from here on; no CI — per-batch local gates on pinned 1.99.0 are the verification. History intact as the rebuild base.
- Archived CI-era entries (runs/PRs/branches, dead links): moved to docs/autopilot/archive/STATE-ci-history.md on 2026-10-12. Retired references: `docs/audit/2026-10-10-deep-audit.md` (RETIRED, never in tree), `TODO-NEXT.md` (RETIRED, never in tree), exchange folder (RETIRED, no exchange folder in tree).

## NEXT

alpha release prep: finish Step 2 doc sync (ledger/README/CHANGELOG) -> write docs/release/0.1.0-alpha.md -> build + verify NSIS bundle(s) with SHA256SUMS -> smoke test or BLOCKED_ON_USER checklist -> final gate.

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
- Threat-model sign-off — owner decision required before RC freeze (see docs/threat-model.md).
