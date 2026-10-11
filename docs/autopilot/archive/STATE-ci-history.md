# STATE CI history (archived 2026-10-12)

CI is retired (`.github/` removed with the remote; `docs/local-gates.md` is the
sole verification authority). Entries below were moved verbatim out of
`docs/autopilot/STATE.md` on 2026-10-12 during alpha prep so the active Log
keeps only Fase 0-7 plus NEXT. Run IDs, PR numbers, branch names, and
`docs/audit/2026-10-10-deep-audit.md` / `TODO-NEXT.md` / exchange-folder
references below are dead engineering record, not verification. The retired
audit-file and checklist references never existed in this tree (verified
missing on 2026-10-12) and are marked RETIRED where they appear.

## Archived entries (newest first, as in STATE.md before archival)

- 2026-10-10 Promotion complete: PR #44 squash-merged to `b360373`, synced back as `4232af4`. PR validation #628 success (full suite incl. packaging, ~16m). Post-merge push #629 fails by design at the audit gate in 8s (merge commit carries no trailer; no source conclusions). Local `main` branch removed (stale `42a748b`, unrelated to canonical history). Next: dispatch mutants + fuzz baselines (owner action), then overhead/RC-05.
- 2026-10-10 CI #626 FINAL VERIFIED: run `38035393199` concluded `success` on `90bdc69` (5m 15s, fast path, no packaging artifact by design). Input-validation batch verification record; promotion flow references retired with CI.
- 2026-10-10 CI #622 FINAL VERIFIED: run `38031558000` concluded `success` on `2251c76` (5m 9s): speed-split validated. Packaging-job validation via manual dispatch pending at the time.
- 2026-10-10 CI #621 FINAL VERIFIED: run `38029163237` concluded `success` on `9c4e975`. WP6 serde/determinism + fuzz-corpus + header-sync batch. Scope caveat about later docs commits NOT covered at the time.
- 2026-10-10 CI speed batch (local, UNVERIFIED): split release packaging out of per-push runs; build caching rejected. 1 file changed (`ci.yml`); validation waited for a quiet window after #621.
- 2026-10-10 WP6/RC-03 property test and fuzz corpus completion: seeded fuzz corpora from minimal PresentMon v2 CSV and JSON goldens; determinism and serde round-trip tests added; `cargo test -p rwp-report` (16 passed) and `cargo test -p rwp-session` (21 passed) green locally.
- 2026-10-10 CI #620 FINAL VERIFIED: run `38012229387` concluded `success` on `c0fac43`. Docs, schema, Linux, coverage, and Windows checks passed; canonical branch policy skipped on `development`.
- 2026-10-10 WP3/WP5 F-07: enforced the owner 10 ms minimum sampling interval in 3 layers; default 50 ms untouched. Test-first fix green locally; pinned CI validation pending at the time.
- 2026-10-10 WP4/WP6 F-06: `from_json` rejects diagnostics with `evidence_score > 100`; fix green locally; pinned CI validation pending at the time.
- 2026-10-10 WP4/WP6 F-05: `build_report_with_stutter_analysis` records per-variant provenance; fix green locally; pinned CI validation pending at the time.
- 2026-10-10 WP6/RC-03/AUD-21: added non-gating `mutation.yml` (pinned cargo-mutants 27.1.0, schedule + dispatch only). First scheduled baseline and >=70% gate pending at the time.
- 2026-10-10 Deep-Audit: PASS — cumulative F-05/F-06/F-07 + mutation workflow local gate validated; pinned CI trigger enqueued; next per Roadmap 0024: F-08 overhead work + mutation baseline CI scheduling.
- 2026-10-10 CI #616 FAILED (audit gate only, 8s): push tip merge commit carried no `Deep-Audit: PASS` trailer, so the gate exited before any source gate ran. No source or gate change caused the failure.
- 2026-10-10 WP6/RC-03/RC-08/AUD-21 fuzz gate (RC-03 step 3): added non-gating `fuzz-scheduled.yml` plus self-contained `fuzz/` workspace with four libFuzzer targets. Local compile evidence recorded; schedule-only workflow never exercised by push CI.
- 2026-10-10 CI investigation (#616-#619): #616/#617 failed only at the Deep audit gate (missing trailer); #618 passed source gates but coverage failed transiently (exit 101, ~28 s), not a source defect; #619 in progress at the time.
- 2026-10-10 CI #619 FINAL VERIFIED: run `38009093838` concluded `success` on `d8be0e2`. Verifies F-05/F-06/F-07 + mutation + fuzz-scheduled batch on the pinned run.
- 2026-10-10 CI #615 FINAL VERIFIED: run `37998454144` concluded `success` on `404f57f`. Verifies F-04 CSV reader/size/line/UTF-8 bounds.
- 2026-10-10 CI #614 FINAL VERIFIED: run `37996347903` concluded `success` on `2adfc0b`. Verifies F-03 async history IPC + skip-broken-report.
- 2026-10-10 CI #613 FINAL VERIFIED: run `37993867070` concluded `success` on `d39adb4`. Verifies the RC-03 line-coverage gate batch on pinned CI.
- 2026-10-10 RC-03 line-coverage gate: new `coverage` CI job (pinned cargo-llvm-cov 0.9.1) enforces 85% per target; `tools/test_check_coverage.py` covers the gate script.
- 2026-10-10 CI #612 FINAL VERIFIED: run `37988930325` concluded `success` on `8366918`. Verifies the F-01/F-02 batch on pinned CI. Filed the owner-relayed deep audit as `docs/audit/2026-10-10-deep-audit.md` with execution checklist `TODO-NEXT.md` — RETIRED: neither file exists in this tree (verified missing 2026-10-12); treated as retired CI-era references, not sources.
- 2026-10-10 Pinned validation trigger for the F-01/F-02 batch: cumulative source delta documented; local checks pass on pinned Rust 1.99.0; commit intentionally triggered the full pinned run.
- 2026-10-10 CI #611 FINAL VERIFIED: run `37984684917` concluded `success` on `faf2f27`. Scope caveat: run predates local F-01/F-02 and exchange commits.
- 2026-10-10 P0 header sync: `repo_ref` updated to match HEAD; docs-only commit carried `[skip ci]`; fresh pinned CI required for the new HEAD at the time.
- 2026-10-10 P0 stabilization before WP6 CI: renamed legacy `docs/adr/ADR-*.md` records to indexed `NNNN-*-supplement-*.md` files; ignored `__pycache__/`; committed proptest seed; added advisory `.githooks/pre-commit` guard. Pinned workspace CI remained the verification gate at the time.
- 2026-10-10 WP6/AUD-21 background-interference contradiction: versioned rule as `background_interference.v3`; ADR-0026 + diagnostics docs updated; streaming-overhead category explicitly deferred. Pinned workspace CI pending at the time; coverage/mutation/cargo-fuzz gates and real fixtures open.
- 2026-10-10 WP6/WP3 rule-matrix follow-up: gated `thermal.v2` on `minimum_system_samples`; added contradiction and property coverage plus 1M-frame Criterion path. Pinned workspace CI pending at the time.
- 2026-10-10 WP6 report determinism check: extended serde round-trip test to assert byte-identical compact JSON. Focused test passed.
- 2026-10-10 WP6 diagnostic contradiction regression: added GPU-bound case with contradicting polarity (score 30). Focused test passed.
- 2026-10-10 WP6 nearest-rank property check: added proptest comparing percentiles vs naive reference. Crate tests passed.
- 2026-10-10 main promotion and development sync: PR #43 squash-merged after CI #606; development synchronized in merge commit `3bbf468`. Post-merge CI #607 running at record time.
- 2026-10-09 WP3/AUD-10 local hardware baseline: release-profile `WindowsSystemSampler` ran 600.05 s at 50 ms cadence (11,930 samples; mean 27.098 ms, rolling p99 29.975 ms, probe-process CPU 53.74% of one core). Single-process/device baseline; does not replace reference-workload or capture-wide measurement.
- 2026-10-09 WP3/AUD-09 local hardware check: temporary worker-thread probe 20 s each at 25/50/75/100% duty cycle; measured 26.14/51.32/76.23/99.80% (max abs error 1.32 pp). Local GetThreadTimes path only; ETW/cycle-time and cross-device checks open.
- 2026-10-09 WP3/AUD-12 local hardware check: Windows GPU Engine PDH wildcard probe returned non-`None` GPU utilization twice in 3 s; multi-adapter/vendor correlation and hardware matrix pending.
- 2026-10-09 WP3/AUD-12 follow-up: accepted both documented PDH success statuses; regression coverage added. Full workspace CI and hardware validation pending at the time.
- 2026-10-09 CI #602 FINAL VERIFIED: run `37931284003` concluded `success`. Validates WP3 no-process-probing + alpha-dependent calibration + CLI calibration E2E + WP6 stutter properties. Hardware acceptance untested as requested.
- 2026-10-09 CI #601 alpha-boundary remediation deep audit PASS and failure/correction: `(0.0..1.0).contains(&0.0)` admitted 0.0 and overflowed the minimum-pair recommendation; changed to explicit finite/`alpha <= 0 || alpha >= 1` checks with expanded coverage. No test or gate weakened; fresh CI required at the time.
- 2026-10-09 CI #600 remediation deep audit PASS and failure/correction: `prop_assert!` format-string braces fixed by binding booleans first; ADR-0025 renamed to four-digit convention. No behavior/dependency/schema/workflow change; fresh CI required at the time.
- 2026-10-09 WP3/WP6 cumulative deep audit PASS: 21-path delta reviewed (GUI option flow, system-only sampler, absent-GPU handling, manifest compat, alpha-derived minimum formula, stutter properties, docs/ADR/STATE/CHANGELOG, exchange handoffs). No manifest/lockfile or workflow/gate changes. Rust compilation/tests awaited pinned CI at the time.
- 2026-10-09 WP3 logic advancement: added `no_process_probing` session/GUI option (default off, ADR-0025) plus alpha-derived A/A calibration minimum. Pinned CI validation pending at the time; hardware checks deferred per owner.
- 2026-10-09 CI #599 FINAL VERIFIED: run `37925943987` concluded `success`. Validates worker-spawn correction and prior WP5 source; predates local WP3/WP6 commits at the time.
- 2026-10-09 WP5 Clippy correction deep audit PASS and CI failure record: `MutexGuard` non-Send poison conversion fixed by mapping to fixed string; fresh pinned run required at the time.
- 2026-10-09 WP5 session-worker spawn-failure deep audit PASS and remediation: named `thread::Builder` worker; OS spawn failure records `Failed` with OS error. Local build/tests not run at the time; CI owned validation then.
- 2026-10-09 CI #597 FINAL VERIFIED: run `37920065639` concluded `success`. Validates persistence/Markdown follow-up batch and pinned Windows/Linux source gates.
- 2026-10-09 WP5 post-#596 format-fix deep audit PASS and CI #596 remediation: Rust 1.99 rustfmt-only hunks in `persistence.rs`; no behavior change; fresh pinned CI required at the time.
- 2026-10-09 WP5/WP4 persistence temp-file hardening (UNVERIFIED until pinned CI at the time): unique `create_new` temp files + hard-link no-clobber publish; renamed tamper-named test to corruption wording.
- 2026-10-09 Exchange 0005 re-audit: confirmed prior fixed-`.tmp` + `File::create` + `rename` behavior; rejected unsafe rename fallback (check-then-rename race); CI #594 cancelled by concurrency, #595 failed on missing audit trailer. Exchange-folder references RETIRED (no exchange folder in this tree).
- 2026-10-09 WP5 post-#593 correction deep audit PASS and CI #593 failure/fix: invalid multi-backslash `char` literals corrected to `'\\'`; fresh pinned CI required at the time.
- 2026-10-09 Post-#592 cumulative static deep audit PASS: seven-path follow-ups reviewed; AUD-03 runtime remains UNVERIFIED; fresh run required at the time.
- 2026-10-09 CI #592 FINAL VERIFIED: run `37915082888` concluded `success`. Scope caveat: predates exact-limit test, Markdown escaping helper, and capture contract docs.
- 2026-10-09 Exchange `0003` re-audit and residual fixes: 1-3600 s contract documented; exact-limit stderr regression; Markdown escaping fix. Process-group test pins the constant only; AUD-03 runtime UNVERIFIED.
- 2026-10-09 WP5 post-#591 format deep audit PASS and CI #591 remediation: three rustfmt-only hunks in `command.rs`; fresh CI required at the time.
- 2026-10-09 WP5/WP4 cumulative deep audit before CI trigger: delta limited to CHANGELOG, `rwp-capture`, `rwp-cli`, STATE, exchange files. Static audit PASS; Rust gates awaited pinned CI at the time.
- 2026-10-09 Exchange tree correction: restored exchange README + messages on `development`. Exchange-folder references RETIRED (no exchange folder in this tree).
- 2026-10-09 CI #588 FINAL VERIFIED: run `37910299013` concluded `success`. Scope caveat: predates later source batch.
- 2026-10-09 WP5/WP4 patch batch 1 applied: bounded 64 KiB stderr, 1-3600 s duration enforcement, redacted reports omit PID, `CREATE_NEW_PROCESS_GROUP` module constant. Source gates not yet run on the cumulative head at the time.
- 2026-10-09 WP5 CI #587 format-only remediation + workflow security regression evidence: four rustfmt-only hunks; docs/schema checks passed. Fresh CI required at the time.
- 2026-10-09 CI audit-gate failures (#37909445111, #37909460026): trigger commits omitted `Deep-Audit: PASS`; no compile/test/schema/package gate ran.
- 2026-10-09 CI #586 FINAL VERIFIED: run `37905767697` concluded `success`. Scope caveat: predates PDH/session/AppManifest/workflow-security changes.
