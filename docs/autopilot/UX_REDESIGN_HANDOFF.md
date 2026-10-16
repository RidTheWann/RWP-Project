# RWP Desktop UX Redesign — Full Handoff for Muse

**Status:** approved design direction; implementation not started  
**Reviewed source baseline:** main at 1216ac9eea70e7cb011faa1aa21d161cf7e1e8c8 (2026-10-16)  
**Audience:** Muse coding agent. Follow the repository's direct-main/no-branches policy.  
**Scope:** UX specification, bilingual copy, implementation batches, acceptance criteria, and local verification plan.

This is a design handoff, not evidence that any implementation, build, test, or runtime behavior works. Muse must verify every implementation assumption against the current repository before changing code.

## 1. Repository facts verified for this handoff

The native UI is Tauri v2 with vanilla HTML/CSS/JavaScript. Relevant files:

- crates/rwp-ui-native/web/index.html
- crates/rwp-ui-native/web/app.js
- crates/rwp-ui-native/web/styles.css
- crates/rwp-ui-native/src-tauri/src/commands.rs
- crates/rwp-ui-native/src-tauri/src/main.rs
- crates/rwp-ui-native/src-tauri/tauri.conf.json

There is no frontend framework or Node build pipeline in this UI crate. The current CSP is default-src 'self'; do not introduce CDNs, remote fonts, scripts, images, packages, or build tooling. The configured window opens at 1360×860 with minimum 1120×720. The redesign target is a usable 900×600 minimum; changing the Tauri window minimum is a planned implementation change and must be validated at that size.

Current registered IPC commands: session_start, session_stop, session_status, session_reset, session_history, session_history_load, session_compare, session_processes, session_pick_presentmon_file, session_pick_output_file, session_error_remediation.

Verified implementation details that constrain the UX:

- session_processes returns process name and PID only. It does not prove which process is a game or which game is foreground.
- session_start requires PresentMon executable path, CSV output path, process ID and requested duration. Normal UI must not expose technical paths.
- session_status uses Idle, Running, Completed and Failed. Running exposes PID, requested duration and started_unix_ms. It exposes no live FPS, frame count or stability verdict.
- session_stop only sets a cancellation flag and can return Running. UI must keep polling until Completed or Failed.
- SessionSnapshot::Completed exposes application, frame count, telemetry sample count, report and dashboard fields, but no interrupted flag or actual-duration field. SessionHistoryEntry exposes application, session_id, process_id, frame_count, interrupted and verified.
- session_compare already verifies both manifests before comparison and accepts same_scenario. The comparison layer supplies comparability results.
- session_error_remediation uses English substring matching for nine known cases and returns no guidance when no substring matches.
- A Rust-generated native notification already exists for Stable via rwp_ui::stable_notification_content. Its text is hard-coded English and includes generic measurement statistics, not process identity. A generic completion notification does not yet exist.
- main.rs has no custom window-close confirmation handler.
- Current product/Tauri config version is 0.4.0. The checked-in docs/autopilot/STATE.md header still points to 353f284 / 0.3.0-era metadata. Reconcile stale state metadata when recording implementation progress; do not copy it as current fact.

## 2. Non-negotiable product contract

1. Capture remains out-of-process. No DLL injection, hooks or overlays.
2. Offline-first: no external telemetry, no network dependency in the analysis engine, no remote UI assets.
3. Missing data stays absent/unknown, never coerced to zero or sentinel. Evidence scores are heuristic evidence scores, not probabilities.
4. Identical inputs remain deterministic. Do not alter analysis rules, thresholds, rule versions, manifest schema or persisted data schema as part of UX work.
5. Diagnosis is authoritative backend output. The web UI must never recreate thresholds from metric numbers.
6. Preserve current command compatibility. IPC changes are additive only; keep CLI and persisted sessions compatible.
7. Keep Rust changes to the minimum native-UI integration required. No new dependencies unless separately justified under the project plan.
8. Every user-facing string has Indonesian and English. Raw technical backend statements appear only under Detail untuk ahli / Expert details.
9. Never report “smooth” merely because no diagnosis fired. Unknown always maps to Belum bisa disimpulkan / Inconclusive.
10. No branches. Work directly on the existing branch per repository policy. Do not publish a release for this UX task.

## 3. Decisions to use by default

### 3.1 PresentMon distribution and one-time setup

**Decision: do not bundle PresentMon, do not download it, and do not scan the entire disk.** ADR-0028 already requires user-supplied PresentMon for v1.0, with version validation and SHA-256 provenance.

1. If a previously selected PresentMon path is saved locally, validate it at startup and reuse it without displaying the path.
2. If missing or invalid, show guided one-time setup using the existing native picker, session_pick_presentmon_file. Explain that RWP needs the user's PresentMon component; keep the path under Settings → Advanced only.
3. Do not bundle, fetch or silently install executables. Do not recursively scan the drive. Only consider auto-detection for a documented, known location if verified for the supported distribution; otherwise show the picker.
4. Keep the path local and never include it in notifications or normal copy.

ADR: ../adr/0028-presentmon-distribution-and-verification.md.

### 3.2 Permissions and elevation

Preflight permissions before enabling Start and re-check immediately before capture. A permission check on Home is not a guarantee that permission remains available later.

**Security default:** never elevate silently. The repair flow explains the requirement and offers user-initiated administrator relaunch only after explicit confirmation/native UAC consent. Also explain the alternative of adding the account to the Windows Performance Log Users group. Mention the group name only in repair instructions, not ordinary Home status. If safe explicit UAC relaunch cannot be implemented without broadening capabilities, ship guided Windows instructions instead; do not weaken permission enforcement.

Elevation grants RWP and child processes a broader privilege boundary and increases the impact of path-validation/parsing defects. Never auto-restart elevated; preserve executable validation, Job Object containment and current Tauri hardening.

### 3.3 Game selection

Do not claim that a process was “detected as the game” based on name + PID alone. Label the list “Aplikasi yang sedang berjalan” / “Running applications”; let the player search and choose. If multiple instances share a name, show stable labels like “{application} (instance 1)” and “(instance 2)” in deterministic order. Keep PID hidden from normal UI and available under Expert details only.

Empty state: “Buka game Anda dulu, lalu Segarkan.” / “Open your game first, then select Refresh.” Keep manual selection from the process list as a first-class fallback.

An additive session_game_candidates command is optional if the current process list proves too noisy, but it must not pretend to know which process is a game or add a classifier. Prefer session_processes unless testing shows it is insufficient.

### 3.4 Managed output and minimal IPC additions

Normal UI must not ask the player to enter a raw CSV output path. Add a managed-session entry point which stores artifacts under the existing per-user session directory while preserving session_start.

Recommended additive adapters, subject to source verification:

- session_preflight: structured permission status, component readiness and whether the selected PID still exists. Do not return sensitive paths for display.
- session_start_managed: accepts process ID, duration, settings and hidden validated PresentMon path; backend chooses output file and persisted session directory. Reuse validation/persistence logic, not a second capture implementation.
- session_game_candidates: optional wrapper only if the current list is too noisy.
- session_error_details: optional structured error kind/code and safe remediation ID while retaining the technical message for Expert details. Prefer source-level typed errors or existing RWP error codes; do not invent codes or spread more substring matching.

All additions are IPC-only. Do not alter manifest JSON, report JSON, analysis config, CLI behavior or persisted schema.

### 3.5 Completion and Stable notifications

Notifications originate in Rust, not webview i18n. Add a generic completion notification after a Completed result; its copy is static and never includes application, PID, executable path or process identity. Stable notification stays conditional on a finished Stable verdict and the Stable preference. It is never an early-stop signal.

Best bilingual design: add an id/en notification-language field to the IPC start request with a backward-compatible default, and maintain a small static translation table in Rust or the renderer-neutral UI crate. Do not assume web-only i18n keys automatically reach Rust. No manifest schema change.

### 3.6 Window close during measurement

Add a Tauri close-request guard. While counting down or measuring, prevent closing until the player makes an explicit choice. Use an accessible dialog with “Tetap di RWP” and “Tutup dan hentikan pengukuran”.

Conservative copy: “Jika Anda menutup RWP sekarang, proses pengukuran akan berhenti dan hasil pengukuran ini tidak akan tersimpan.” Muse must verify actual stop/persistence behavior before finalizing that statement. If cancellation reliably persists an interrupted session, update the copy to describe the verified behavior. Never let a worker be silently killed by closing the window. On confirmed close, request stop and wait for worker shutdown/child cleanup before allowing the window to close; cover Completed/Failed races.

## 4. Information architecture: four primary views + Settings

- **Home:** first-run checks, choose running application, preparation time, Stable-notification preference and one primary Start action. No History card duplicated on every view.
- **Measure:** one view with exclusive states Countdown → Recording → Analyzing. Countdown and recording never appear together. Analyzing is derived in UI, not a new backend status.
- **Result:** completed / stopped early / failed. Friendly verdict, one evidence-backed reason/advice, expandable Expert details. Compare disabled until another verified session of the same application exists.
- **History & Compare:** one list and one Before/After selection mechanism. Use “Jadikan Sebelum” / “Jadikan Sesudah”; do not combine checkboxes and parallel dropdowns. Render only the three existing comparison statuses.
- **Settings:** language, 5/10/15 second preparation, Stable-notification preference and Advanced disclosure. Technical tuning, PresentMon path, output/diagnostic paths and detailed PID data live only in Advanced/Expert details.

Remove LOCAL SESSION, LIVE SESSION, Capture, and the technical “Measure. Diagnose. Prove.” tagline from the player-facing header. Branding may remain as a small RWP wordmark. Use “Ukur performa game Anda” and a benefit-oriented subtitle.

## 5. Interaction and state contracts

### First run / Home

1. Run permission and component preflight.
2. If PresentMon is not configured, guide the user through one-time selection and verification; then hide the path from ordinary UI.
3. Show permission as Ready / Needs attention before Start.
4. Load currently running processes. Never auto-select an application as a verified game solely from its name. Refresh is always available; show the empty-state copy when there are no candidates.
5. Start is enabled only when component, permissions and process choice are ready.
6. Preserve current requested-duration default of 60 seconds unless changed in Advanced. Preparation time is independent and defaults to 5 seconds.

### Countdown

- Use an absolute **monotonic deadline**, e.g. performance.now() + selectedPreparationSeconds × 1000, not a counter decremented by timer ticks. Recompute remaining time from the deadline on each paint tick and on visibilitychange/focus. It must continue correctly while RWP is minimized as the player switches to the game.
- Show “Pindah ke game Anda…” and the large countdown. Use concise aria-live="polite" announcements; do not flood screen readers every second.
- Do not invoke session_start until countdown reaches zero. Cancel before zero means no capture.
- Revalidate selected PID, permission and component immediately before start. If the game closed, show game-not-found. If permissions changed, show repair. If the component is missing/damaged, use the shared component error catalogue.

### Recording

- Compute elapsed/progress from session_status.started_unix_ms and requested duration. Recompute from timestamps, not accumulated ticks. Use role="progressbar" with accessible name, aria-valuemin, aria-valuemax and aria-valuenow; cap visual progress at 100%.
- Do not render intermediate FPS or “smoothness”; Running supplies no live result. Static text: “Sedang merekam. Mainkan seperti biasa; hasil muncul setelah selesai.”
- Primary action: “Hentikan pengukuran”. Confirm with generic copy, no sample thresholds: “Hasil mungkin belum cukup untuk menarik kesimpulan. Data yang tersedia tetap akan dianalisis sebagai hasil yang dihentikan lebih awal.”
- After confirmation, set local UI state to Analyzing and disable Stop. Call session_stop once, then keep polling until Completed or Failed; it can return Running.
- When requested duration elapses but status remains Running, UI may infer Analyzing and disable Stop, but must not call session_stop just to force a transition.
- A live frame count is optional backend enhancement only; it is not present in the Running snapshot and is not required.

### Results and diagnosis composition

Use the backend’s ordered diagnostic array without re-sorting or scoring in JavaScript. First diagnosis is the title; remaining diagnoses appear under “Juga terdeteksi” / “Also detected”. **Needs analysis review:** confirm evidence_score descending and category tie-break ordering in current code (CpuBound rank 0 … Unknown rank 8, Stable rank 9).

Stable alongside other diagnoses must not erase those findings. Use “Hasil stabilitas tersedia, tetapi ada temuan lain” / “Stability result available, but other findings remain” and still show other diagnoses.

Mappings are display copy only. The engine owns activation thresholds, minimum samples and Unknown. No active diagnosis means the engine’s Unknown result. Do not duplicate thresholds in UI. Wording/reasons/advice need analysis review before implementation.

| Category / rule | ID title | EN title | ID advice |
|---|---|---|---|
| CpuBound / cpu_bound.v1 | Ada indikasi hambatan prosesor | Possible CPU bottleneck | Coba kurangi pengaturan game yang membebani prosesor, lalu ukur ulang. |
| GpuBound / gpu_bound.v2 | Ada indikasi hambatan grafis | Possible graphics bottleneck | Coba kurangi kualitas grafis, lalu ukur ulang. |
| FramePacing / frame_pacing.v2 | Waktu antar-frame terindikasi kurang konsisten | Frame pacing may be inconsistent | Periksa detail pengukuran dan ulangi dengan kondisi yang serupa. |
| Thermal / thermal.v2 | Ada indikasi masalah terkait suhu | Possible thermal issue | Periksa pendinginan dan ventilasi PC. |
| MemoryPressure / memory_pressure.v1 | Ada indikasi tekanan pada memori | Possible memory pressure | Tutup aplikasi yang tidak diperlukan, lalu ukur ulang. |
| VramPressure / vram_pressure.v1 | Ada indikasi tekanan pada memori grafis | Possible graphics memory pressure | Coba turunkan resolusi atau kualitas tekstur, lalu ukur ulang. |
| BackgroundInterference / background_interference.v3 | Aktivitas latar mungkin memengaruhi hasil | Background activity may affect results | Tutup aplikasi latar yang tidak diperlukan dan ukur ulang. |
| Stable / stability.v1 | Belum ada masalah yang terdeteksi | No issues detected | Bandingkan dengan pengukuran lain dalam kondisi yang serupa. |
| Unknown | Belum bisa disimpulkan | Inconclusive | Coba ukur lagi hingga selesai. |
| StreamingOverhead (no implemented rule) | Belum bisa disimpulkan (fallback only) | Inconclusive (fallback only) | No category-specific advice; do not create a rule. |

Stable explanation: “Hasil memenuhi kriteria stabilitas yang ada; ini bukan bukti bahwa game pasti lancar.” / “The result meets the existing stability criteria; this does not prove the game is always smooth.”

Show only existing Expert detail values: average FPS, P99 frame time, 1% low, stutter-event count, evidence scores, and supporting/contradicting statements. Raw English technical statements are Expert details only. Missing values display “Tidak tersedia” / “Not available”; never fake zeroes.

### Failure, retries and technical details

Failure’s primary action is **Coba lagi / Try again**. Omit Compare on failure. Keep a utility **Salin detail teknis / Copy technical details** for unknown errors; raw text is not the main copy. Do not promise retry will succeed.

Re-check all nine remediation markers against code: wrong/missing PresentMon filename; Performance Log Users; permission; invalid telemetry interval; invalid duration; invalid target PID; session already running; worker start failure; unsupported PresentMon version. For an unknown failure, show “Terjadi masalah yang tidak terduga” / “An unexpected problem occurred” and technical-copy utility. Prefer typed errors/stable existing codes as an optional additive improvement; raw message remains available in Expert details.

### History and Before/After

- Render application label, unique session identifier derived from session_id, frame count, interrupted and verified state. SessionHistoryEntry has no timestamp field; never invent a date/time.
- Row label: “{application} · Sesi {short_session_id}” and “{frame_count} frame”. Ensure short ID uniqueness; lengthen it on collision. Full ID remains accessible from details/copy. Inspect whether session_id or an existing manifest field contains a usable timestamp before adding anything; no schema change.
- verified=true → “Data terverifikasi” / “Data verified”; false → “Data tidak utuh” / “Data incomplete”. Unverified entries are not comparison candidates. Backend verification remains mandatory.
- One selection mechanism: “Jadikan Sebelum” / “Set as Before” and “Jadikan Sesudah” / “Set as After”.
- Ask “Apakah kondisi kedua pengukuran cukup serupa?” / “Were the conditions sufficiently similar?” Yes maps to same_scenario=true; Not sure and No both map to false. Never send true for uncertainty; do not force the user to choose another pair.
- same_scenario=false → InconclusiveDifferentScenario and “Belum bisa disimpulkan” / “Inconclusive”.
- Map only existing statuses:
  - Comparable → “Hasil dapat dibandingkan” / “Results are comparable”.
  - LimitedSampleEquivalence → “Perbandingan terbatas” / “Limited comparison”; explain sample/frame counts differ and ask the user to interpret carefully.
  - InconclusiveDifferentScenario → “Belum bisa disimpulkan” / “Inconclusive”; suggest a repeat under more similar conditions.
- Disable Compare until two distinct, verified sessions of the same normalized application are selected. Explain why. Backend verifies both manifests and rejects invalid entries even if UI is bypassed.

## 6. ASCII wireframes

### Home

~~~text
+--------------------------------------------------+
| RWP                         Riwayat   Pengaturan |
+--------------------------------------------------+
| Ukur performa game Anda                          |
| Lihat hasil pengukuran performa game di PC Anda. |
|                                                  |
| PILIH GAME                                       |
| [ Aplikasi yang sedang berjalan             v ]  |
| [ Segarkan ]  Tidak ada game? Pilih dari daftar |
|                                                  |
| Waktu bersiap: (o) 5 dtk  ( ) 10 dtk  ( ) 15 dtk |
| [x] Beri tahu saya jika hasilnya stabil          |
|                                                  |
| Izin pengukuran: [Siap / Perlu diperbaiki]       |
| Komponen pengukuran: [Siap / Perlu disiapkan]    |
|                                                  |
|                   [ MULAI UKUR ]                 |
+--------------------------------------------------+
~~~

### Countdown

~~~text
+--------------------------------------------------+
| RWP                                              |
|                BERSIAP MENGUKUR                  |
|                Pindah ke game Anda               |
|                         5                        |
|        Pengukuran dimulai setelah hitung mundur. |
|                                                  |
|                    [ BATALKAN ]                  |
+--------------------------------------------------+
~~~

### Recording / Analyzing (exclusive states)

~~~text
+--------------------------------------------------+
| RWP               PENGUKURAN BERLANGSUNG         |
|                  Sedang merekam                 |
| Mainkan seperti biasa; hasil muncul setelah selesai.|
|                 00:42 / 01:00                   |
| [================================------] 70%     |
|              [ HENTIKAN PENGUKURAN ]            |
+--------------------------------------------------+

Analyzing replaces recording; it never overlays it:

+--------------------------------------------------+
| RWP                                              |
|                  MENGANALISIS...                 |
|        Menyiapkan hasil pengukuran Anda.         |
|                [ Hentikan ] (disabled)           |
+--------------------------------------------------+
~~~

### Result

~~~text
+--------------------------------------------------+
| RWP                         Riwayat   Pengaturan |
| HASIL PENGUKURAN                                 |
| Game: {application} · {frame_count} frame        |
|                                                  |
| [ VERDICT AWAM ]                                 |
| Alasan: satu alasan berdasarkan bukti            |
| Saran berikutnya: satu tindakan relevan          |
| [ Detail untuk ahli                         v ]  |
|                                                  |
| [ UKUR LAGI ]              [ BANDINGKAN ]        |
+--------------------------------------------------+
~~~

Failure uses one primary action Coba lagi, omits Compare, and may offer technical-copy utility when useful. Stopped-early results are labelled Dihentikan lebih awal and analyzed with available data.

### History & Before/After

~~~text
+--------------------------------------------------+
| RWP                         Beranda  Pengaturan  |
| RIWAYAT PENGUKURAN                                |
| [Jadikan Sebelum] [Jadikan Sesudah]              |
| Game A · Sesi abc123       Data terverifikasi    |
| 24.560 frame                                     |
| Game A · Sesi xyz789       Data tidak utuh       |
| 23.140 frame     Pilihan perbandingan nonaktif   |
| SEBELUM / SESUDAH                                |
| Sebelum: {selected session}                      |
| Sesudah: {selected session}                      |
| Kondisi cukup serupa?                            |
| ( ) Ya   ( ) Tidak yakin   ( ) Tidak              |
|                [ BANDINGKAN ]                    |
| [ STATUS PERBANDINGAN + PENJELASAN ]             |
+--------------------------------------------------+
~~~

## 7. Visual and accessibility direction

### Style

Use a restrained dark gaming aesthetic, not a neon dashboard: layered charcoal surfaces, readable near-white content, one warm red accent for primary action/current navigation, and distinct success/warning/error text and iconography. Avoid decorative noise, glowing borders, dense micro-labels and colour-only meaning.

Suggested starting CSS custom properties (not final until contrast is measured):

- --bg: #0B0F14
- --surface: #141A22
- --surface-raised: #1B2430
- --text: #F4F7FA
- --text-muted: #C0CBD8
- --accent: #C93842
- --focus: #80BFFF
- --success: #62D99A
- --warning: #FFD166
- --danger: #FF7A8A

Use system fonts (Segoe UI, system-ui, sans-serif). Do not download fonts or add icon packs, chart libraries, frameworks or build tooling. CSS-only icons or inline SVGs are acceptable only when labelled accessibly or marked decorative. Charts must remain understandable without colour.

### Accessibility acceptance

- WCAG 2.2 AA: at least 4.5:1 for normal text and 3:1 for large text. Measure actual final token combinations; do not infer compliance visually.
- Use semantic landmarks/headings and native controls. Every input has a label. All actions work through keyboard; visible focus is never removed.
- Prefer a 44×44 CSS-pixel target for commonly used buttons where layout permits; keep 900×600 practical. Do not rely on hover.
- Countdown uses concise aria-live="polite" announcements. Progress uses role="progressbar" with name and values. Do not announce each visual repaint.
- Dialogs use role="dialog" and aria-modal="true", accessible name, predictable initial focus, focus containment, safe Escape behavior, and focus restoration.
- Status is conveyed with text/icon as well as colour. No colour-only meaning.
- Support keyboard-only navigation, Windows High Contrast / forced-colors, system fonts, WebView zoom/text scaling, high DPI, prefers-reduced-motion and no clipping at 900×600. Check long strings in both languages.
- Charts/tables have text equivalents. Expert details start collapsed but remain accessible.

## 8. Bilingual copy deck (ID / EN)

Use stable keys in one vanilla-JS translation dictionary for webview strings. Persist language preference locally; default to Windows/user locale when available, otherwise Indonesian; always allow manual ID/EN override. Native notifications need a separate Rust-side static dictionary selected via the additive notification-language field.

### Global / Settings / first run

| Key | ID | EN |
|---|---|---|
| app.brand | RWP | RWP |
| nav.home | Beranda | Home |
| nav.history | Riwayat | History |
| nav.settings | Pengaturan | Settings |
| home.title | Ukur performa game Anda | Measure your game's performance |
| home.subtitle | Lihat hasil pengukuran performa game di PC Anda. | See your game's performance measurement results on your PC. |
| game.label | Pilih game | Choose a game |
| game.running_apps | Aplikasi yang sedang berjalan | Running applications |
| game.refresh | Segarkan | Refresh |
| game.manual | Pilih aplikasi dari daftar | Choose an application from the list |
| game.empty | Buka game Anda dulu, lalu Segarkan. | Open your game first, then select Refresh. |
| permission.ready | Izin pengukuran: Siap | Measurement permissions: Ready |
| permission.needs_action | Izin pengukuran perlu diperbaiki | Measurement permissions need attention |
| permission.body | Windows belum mengizinkan RWP memulai pengukuran. | Windows has not allowed RWP to start the measurement. |
| permission.fix | Lihat cara memperbaiki | View how to fix it |
| permission.elevation_warning | Menjalankan RWP sebagai administrator memberi aplikasi hak akses lebih luas. Lanjutkan hanya jika Anda mempercayai aplikasi ini. | Running RWP as an administrator gives the app broader privileges. Continue only if you trust this app. |
| permission.restart_admin | Jalankan ulang sebagai administrator | Restart as administrator |
| permission.instructions | Lihat petunjuk izin Windows | View Windows permission instructions |
| component.ready | Komponen pengukuran: Siap | Measurement component: Ready |
| component.needs_setup | Komponen pengukuran perlu disiapkan | Measurement component needs setup |
| component.setup_body | RWP memerlukan PresentMon yang didukung. RWP tidak mengunduh atau memasang komponen ini secara otomatis. | RWP requires a supported copy of PresentMon. RWP does not download or install this component automatically. |
| component.choose | Pilih PresentMon | Choose PresentMon |
| component.invalid | RWP tidak dapat menggunakan komponen pengukuran yang dipilih. | RWP cannot use the selected measurement component. |
| first_run.title | Mari siapkan pengukuran | Let's set up measurement |
| first_run.subtitle | RWP akan memeriksa apakah PC Anda siap mengukur performa game. | RWP will check whether your PC is ready to measure game performance. |
| first_run.permission_check | Memeriksa izin Windows… | Checking Windows permissions… |
| first_run.component_check | Memeriksa komponen pengukuran… | Checking the measurement component… |
| first_run.ready | Persiapan selesai. Anda siap mengukur. | Setup is complete. You're ready to measure. |
| first_run.retry | Periksa lagi | Check again |
| first_run.continue | Lanjutkan | Continue |
| first_run.open_advanced | Pengaturan lanjutan | Advanced settings |
| settings.title | Pengaturan | Settings |
| settings.language | Bahasa | Language |
| settings.language.auto | Ikuti bahasa Windows | Follow Windows language |
| settings.language.id | Bahasa Indonesia | Indonesian |
| settings.language.en | Bahasa Inggris | English |
| settings.preparation | Waktu bersiap sebelum mengukur | Preparation time before measuring |
| settings.prep_5 | 5 detik | 5 seconds |
| settings.prep_10 | 10 detik | 10 seconds |
| settings.prep_15 | 15 detik | 15 seconds |
| settings.notify_stable | Beri tahu saya jika hasilnya stabil | Notify me if the result is stable |
| settings.advanced | Pengaturan lanjutan | Advanced settings |
| settings.advanced.warning | Pengaturan ini dapat memengaruhi data yang dikumpulkan. Ubah hanya jika Anda memahami dampaknya. | These settings can affect the data collected. Change them only if you understand their impact. |
| settings.duration | Durasi pengukuran | Measurement duration |
| settings.interval | Interval telemetri (ms) | Telemetry interval (ms) |
| settings.exclude_dropped | Kecualikan frame yang terlewat | Exclude dropped presents |
| settings.no_process_probing | Nonaktifkan pemeriksaan thread/proses tambahan | Disable additional process/thread probing |
| settings.presentmon_path | Lokasi PresentMon | PresentMon location |
| settings.output_path | Lokasi file keluaran | Output file location |
| settings.save | Simpan pengaturan | Save settings |
| settings.cancel | Batal | Cancel |

### Countdown / recording / close confirmation

| Key | ID | EN |
|---|---|---|
| countdown.title | Bersiap mengukur | Getting ready to measure |
| countdown.body | Pindah ke game Anda. | Switch to your game. |
| countdown.announcement | Pindah ke game Anda. Pengukuran dimulai dalam {seconds} detik. | Switch to your game. Measurement starts in {seconds} seconds. |
| countdown.cancel | Batalkan | Cancel |
| countdown.game_missing | Game tidak ditemukan. Buka game, lalu coba lagi. | Game not found. Open the game and try again. |
| countdown.choose_again | Pilih game lagi | Choose game again |
| measuring.title | Pengukuran berlangsung | Measurement in progress |
| measuring.body | Sedang merekam. Mainkan seperti biasa; hasil muncul setelah selesai. | Recording. Play as usual; your results will appear when recording is complete. |
| measuring.elapsed | Waktu berlalu: {elapsed} dari {requested_duration} | Elapsed: {elapsed} of {requested_duration} |
| measuring.progress | Progres pengukuran | Measurement progress |
| measuring.stop | Hentikan pengukuran | Stop measurement |
| stop.confirm.title | Hentikan pengukuran? | Stop the measurement? |
| stop.confirm.body | Hasil mungkin belum cukup untuk menarik kesimpulan. Data yang tersedia tetap akan dianalisis sebagai hasil yang dihentikan lebih awal. | There may not be enough data to draw a conclusion. Available data will still be analyzed as an early-stopped result. |
| stop.confirm.continue | Lanjut mengukur | Continue measuring |
| stop.confirm.now | Hentikan sekarang | Stop now |
| analyzing.title | Menganalisis… | Analyzing… |
| analyzing.body | Menyiapkan hasil pengukuran Anda. | Preparing your measurement results. |
| window_close.title | Pengukuran masih berlangsung | Measurement is still in progress |
| window_close.body | Jika Anda menutup RWP sekarang, proses pengukuran akan berhenti dan hasil pengukuran ini tidak akan tersimpan. | If you close RWP now, the measurement process will stop and these results will not be saved. |
| window_close.stay | Tetap di RWP | Keep RWP open |
| window_close.stop_close | Tutup dan hentikan pengukuran | Close and stop measurement |

### Result / stability / diagnostics

| Key | ID | EN |
|---|---|---|
| result.title | Hasil pengukuran | Measurement results |
| result.application | Game: {application} | Game: {application} |
| result.frame_count | Jumlah frame: {count} | Frame count: {count} |
| result.completed | Selesai | Completed |
| result.interrupted | Dihentikan lebih awal | Stopped early |
| result.failed.title | Pengukuran gagal | Measurement failed |
| result.failed.body | Pengukuran atau analisis tidak berhasil diselesaikan. | The measurement or analysis could not be completed. |
| result.reason | Alasan | Reason |
| result.advice | Saran berikutnya | What to try next |
| result.also_detected | Juga terdeteksi | Also detected |
| result.expert | Detail untuk ahli | Expert details |
| result.expert.show | Tampilkan detail untuk ahli | Show expert details |
| result.expert.hide | Sembunyikan detail untuk ahli | Hide expert details |
| result.retry | Coba lagi | Try again |
| result.measure_again | Ukur lagi | Measure again |
| result.compare | Bandingkan | Compare |
| result.compare_disabled | Belum ada pengukuran terverifikasi dari game yang sama untuk dibandingkan. | There is no verified measurement from the same game available to compare yet. |
| verdict.unknown.title | Belum bisa disimpulkan | Inconclusive |
| verdict.unknown.reason | Data atau bukti yang tersedia belum mendukung kesimpulan yang dapat diandalkan. | The available data or evidence does not support a reliable conclusion. |
| verdict.unknown.advice | Coba ukur lagi hingga selesai. | Try measuring again and let it finish. |
| verdict.stable.title | Belum ada masalah yang terdeteksi | No issues detected |
| verdict.stable.reason | Hasil memenuhi kriteria stabilitas yang ada; ini bukan bukti bahwa game pasti lancar. | The result meets the existing stability criteria; this does not prove the game is always smooth. |
| verdict.stable.advice | Bandingkan dengan pengukuran lain dalam kondisi yang serupa. | Compare with another measurement taken under similar conditions. |
| verdict.stable_with_findings.title | Hasil stabilitas tersedia, tetapi ada temuan lain | Stability result available, but other findings remain |
| verdict.cpu.title | Ada indikasi hambatan prosesor | Possible CPU bottleneck |
| verdict.cpu.advice | Coba kurangi pengaturan game yang membebani prosesor, lalu ukur ulang. | Try lowering CPU-intensive game settings, then measure again. |
| verdict.gpu.title | Ada indikasi hambatan grafis | Possible graphics bottleneck |
| verdict.gpu.advice | Coba kurangi kualitas grafis, lalu ukur ulang. | Try lowering graphics quality, then measure again. |
| verdict.pacing.title | Waktu antar-frame terindikasi kurang konsisten | Frame pacing may be inconsistent |
| verdict.pacing.advice | Periksa detail pengukuran dan ulangi dengan kondisi yang serupa. | Review the measurement details and repeat under similar conditions. |
| verdict.thermal.title | Ada indikasi masalah terkait suhu | Possible thermal issue |
| verdict.thermal.advice | Periksa pendinginan dan ventilasi PC. | Check your PC's cooling and ventilation. |
| verdict.memory.title | Ada indikasi tekanan pada memori | Possible memory pressure |
| verdict.memory.advice | Tutup aplikasi yang tidak diperlukan, lalu ukur ulang. | Close unnecessary applications, then measure again. |
| verdict.vram.title | Ada indikasi tekanan pada memori grafis | Possible graphics memory pressure |
| verdict.vram.advice | Coba turunkan resolusi atau kualitas tekstur, lalu ukur ulang. | Try lowering resolution or texture quality, then measure again. |
| verdict.background.title | Aktivitas latar mungkin memengaruhi hasil | Background activity may affect results |
| verdict.background.advice | Tutup aplikasi latar yang tidak diperlukan dan ukur ulang. | Close unnecessary background applications and measure again. |
| stability.unknown | Stabilitas belum diketahui | Stability not yet known |
| stability.unknown.body | Belum ada bukti yang cukup untuk menetapkan status stabilitas. | There is not enough evidence to determine stability. |
| stability.stable | Stabil | Stable |
| stability.stable.body | Pengukuran memenuhi kriteria stabilitas yang ada. Ini bukan jaminan bahwa game selalu lancar. | The measurement meets the existing stability criteria. This does not guarantee that the game will always run smoothly. |
| stability.unstable | Tidak stabil | Unstable |
| stability.unstable.body | Pengukuran menunjukkan bahwa kriteria stabilitas tidak terpenuhi. | The measurement indicates that the stability criteria are not met. |
| stability.observations | Jumlah observasi: {count} | Observations: {count} |
| stability.stutters | Peristiwa stutter: {count} | Stutter events: {count} |
| expert.average_fps | FPS rata-rata | Average FPS |
| expert.p99 | Waktu frame P99 | P99 frame time |
| expert.one_percent_low | 1% low | 1% low |
| expert.stutter_count | Jumlah peristiwa stutter | Stutter event count |
| expert.evidence_score | Skor bukti | Evidence score |
| expert.evidence | Bukti analisis | Analysis evidence |
| expert.rule_id | ID aturan | Rule ID |
| expert.unavailable | Tidak tersedia | Not available |
| expert.raw_statement | Pernyataan teknis asli | Original technical statement |

### Known errors and generic fallback

| Key | ID | EN |
|---|---|---|
| error.permission.title | Izin Windows diperlukan | Windows permission required |
| error.permission.body | RWP tidak dapat memulai pengukuran karena izin Windows tidak mencukupi. | RWP could not start the measurement because Windows permissions are insufficient. |
| error.permission.fix | Ikuti petunjuk izin Windows untuk menjalankan pengukuran. | Follow the Windows permission instructions to run the measurement. |
| error.component.title | Komponen pengukuran tidak tersedia | Measurement component unavailable |
| error.component.body | RWP tidak dapat menemukan atau menggunakan komponen yang diperlukan untuk merekam performa. | RWP cannot find or use the component required to record performance. |
| error.presentmon_name.title | Komponen pengukuran tidak dapat digunakan | Measurement component cannot be used |
| error.presentmon_name.body | Periksa komponen PresentMon di Pengaturan lanjutan, lalu coba lagi. | Check the PresentMon component in Advanced settings, then try again. |
| error.interval.title | Pengaturan pengukuran tidak valid | Measurement setting is invalid |
| error.interval.body | Pilih interval telemetri yang didukung di Pengaturan lanjutan. | Choose a supported telemetry interval in Advanced settings. |
| error.duration.title | Durasi pengukuran tidak valid | Measurement duration is invalid |
| error.duration.body | Pilih durasi pengukuran yang didukung di Pengaturan lanjutan. | Choose a supported measurement duration in Advanced settings. |
| error.target.title | Game tidak ditemukan | Game not found |
| error.target.body | Buka game dan pilih aplikasi yang sedang berjalan, lalu coba lagi. | Open the game and select the running application, then try again. |
| error.session_active.title | Pengukuran masih berlangsung | Measurement is still in progress |
| error.session_active.body | Tunggu pengukuran saat ini selesai sebelum memulai yang baru. | Wait for the current measurement to finish before starting another. |
| error.worker.title | Pengukuran tidak dapat dimulai | Measurement could not start |
| error.worker.body | Coba mulai lagi. Jika masalah berulang, salin detail teknis untuk membantu menemukan penyebabnya. | Try starting again. If the problem continues, copy the technical details to help identify the cause. |
| error.version.title | Versi komponen tidak didukung | Component version not supported |
| error.version.body | Pilih versi PresentMon yang didukung. Lihat detail untuk ahli untuk informasi teknis. | Choose a supported PresentMon version. See Expert details for technical information. |
| error.unknown.title | Terjadi masalah yang tidak terduga | An unexpected problem occurred |
| error.unknown.body | Pengukuran tidak dapat diselesaikan. Detail teknis dapat membantu menemukan penyebabnya. | The measurement could not be completed. Technical details may help identify the cause. |
| error.copy_details | Salin detail teknis | Copy technical details |
| error.retry | Coba lagi | Try again |

### Notifications (Rust-generated, not web i18n)

| Key | ID | EN |
|---|---|---|
| notification.complete.title | Pengukuran selesai | Measurement complete |
| notification.complete.body | Hasil Anda siap dilihat. | Your results are ready to view. |
| notification.stable.title | Hasil pengukuran stabil | Measurement result is stable |
| notification.stable.body | {n} observasi; tidak ada peristiwa stutter; perubahan masih dalam batas. | {n} observations; no stutter events; drift within margin. |

Completion fires after a Completed result. Stable fires only after completion with the existing Stable verdict and enabled preference. Confirm handling of early-stopped completion against the actual worker result. Neither toast includes game/application/process identity.

### History and Compare

| Key | ID | EN |
|---|---|---|
| history.title | Riwayat pengukuran | Measurement history |
| history.subtitle | Pilih pengukuran untuk melihat hasil atau membandingkannya. | Select a measurement to view results or compare it. |
| history.session | Sesi {session_id_short} | Session {session_id_short} |
| history.frames | {count} frame | {count} frames |
| history.interrupted | Dihentikan lebih awal | Stopped early |
| history.verified | Data terverifikasi | Data verified |
| history.invalid | Data tidak utuh | Data incomplete |
| history.not_selectable | Sesi ini tidak dapat dibandingkan karena data tidak terverifikasi. | This session cannot be compared because its data is not verified. |
| history.set_before | Jadikan Sebelum | Set as Before |
| history.set_after | Jadikan Sesudah | Set as After |
| history.empty.title | Belum ada riwayat pengukuran | No measurement history yet |
| history.empty.body | Pengukuran yang tersimpan akan muncul di sini. | Saved measurements will appear here. |
| compare.title | Sebelum / Sesudah | Before / After |
| compare.before | Sebelum | Before |
| compare.after | Sesudah | After |
| compare.same_scenario | Apakah kondisi kedua pengukuran cukup serupa? | Were the conditions sufficiently similar? |
| compare.same_scenario.yes | Ya, kondisinya sebanding | Yes, the conditions were comparable |
| compare.same_scenario.unsure | Tidak yakin | Not sure |
| compare.same_scenario.no | Tidak, kondisinya berbeda | No, the conditions were different |
| compare.action | Bandingkan hasil | Compare results |
| compare.status.comparable.title | Hasil dapat dibandingkan | Results are comparable |
| compare.status.comparable.body | Kedua pengukuran memenuhi kriteria perbandingan yang tersedia. | Both measurements meet the available comparability criteria. |
| compare.status.limited.title | Perbandingan terbatas | Limited comparison |
| compare.status.limited.body | Jumlah data pada kedua pengukuran berbeda. Bandingkan hasil dengan hati-hati. | The two measurements contain different amounts of data. Compare the results with care. |
| compare.status.inconclusive.title | Belum bisa disimpulkan | Inconclusive |
| compare.status.inconclusive.body | Kondisi kedua pengukuran tidak dapat dipastikan sebanding. | The two measurements cannot be confirmed as comparable in their conditions. |
| compare.status.inconclusive.advice | Ulangi pengukuran dengan kondisi yang serupa agar hasil lebih mudah dibandingkan. | Repeat the measurements under similar conditions to make comparison easier. |
| compare.same_application | Pilih dua pengukuran dari aplikasi yang sama. | Select two measurements from the same application. |
| compare.verified_required | Pilihan ini tidak tersedia karena data sesi tidak terverifikasi. | This selection is unavailable because the session data is not verified. |
| compare.empty | Pilih sesi Sebelum dan Sesudah untuk membandingkan hasil. | Select Before and After sessions to compare results. |

## 9. Implementation plan for Muse

Implement small, independently auditable batches, preferably no more than about 8 files / 600 changed lines each. Add regression tests first whenever a logic contract changes. Keep code, comments and repo documentation in English; runtime strings are bilingual.

### Batch 1 — Verify facts and add integration adapters only when needed

1. Re-read current STATE, MASTER_PLAN, ADR-0028, commands, session worker, report/manifest types, diagnostics and comparison code.
2. Confirm how permission preflight can be checked without elevation; add session_preflight only if no existing API provides it.
3. Factor shared start validation/persistence so session_start_managed chooses session/output paths without changing session_start.
4. Use session_processes first. Add session_game_candidates only if it solves an observed picker problem with existing name/PID data; no guessed classifier.
5. Verify error taxonomy before adding typed UI error information. Do not invent codes or alter manifest/report schema.
6. Add an interrupted field to the UI-only Completed snapshot only if necessary and only additively; do not change persisted schema.
7. Add a generic Rust-side completion notification and bilingual Rust-side strings. Keep Stable semantics unchanged; exclude process identity.
8. Add Tauri close handling after verifying cancellation/persistence. Ensure child cleanup.
9. Add regression tests for adapters, notification behavior and state transitions before implementation.

### Batch 2 — Shell, Home, first run, countdown and recording

- Replace page structure, remove History from other views, hide technical fields, add Home/History/Settings navigation.
- Implement first-run preflight, guided user-supplied PresentMon selection, permission-repair instructions, process list/fallback and preparation preference.
- Add timestamp-based Countdown and progress from started_unix_ms plus requested duration.
- Add generic stop confirmation and UI-only Analyzing state with polling to Completed/Failed.
- Add accessible window-close confirmation.
- Do not add live FPS/frame count unless backend is explicitly enhanced.

### Batch 3 — Result, History and Before/After

- Render completed/interrupted/failed states and preserve local stop intent until result rendering.
- Localize categories based only on backend output. Preserve diagnosis order, scores and evidence.
- Add Stable-with-other-findings copy and Unknown fallback; no duplicate thresholds.
- Build one History list with unique labels and verification status. Disable unverified comparison choices.
- Use one selection mechanism and exact same_scenario mapping.
- Render only the three existing comparability statuses. Backend manifest checks remain mandatory.

### Batch 4 — Copy/i18n and accessibility

- Centralize translation keys and ID/EN strings in §8.
- Implement locale fallback/persistence; keep native notification localization in Rust via additive preference.
- Apply semantic HTML, accessible dialogs, aria-live, progressbar, keyboard/focus, forced-colors, reduced motion and focus restoration.
- Keep raw English technical evidence only in Expert details; preserve technical-copy utility for unknown errors.
- Measure actual contrast; do not assert compliance until verified.

### Batch 5 — Visual polish, docs, ADRs and release notes

- Implement CSS tokens, responsive layout, readable target spacing, focus and 900×600 minimum. Keep CSP/dependencies unchanged and add no external assets.
- Update GUI acceptance checklist and relevant UX/troubleshooting docs in English. Update CHANGELOG only for implemented user-visible behavior; this handoff alone does not warrant an entry.
- Do not revise ADR-0028 because PresentMon remains user-supplied. If bundling becomes necessary, stop and write a separate ADR for license/attribution, pinned version/SHA-256, supply-chain verification, installer size and threat-model changes. Never bundle silently.
- Revisit threat model for elevation, stored executable paths, error-detail copying and close lifecycle. No analysis rule, threshold or schema changes.

## 10. Test and acceptance checklist

### Behavior / IPC

- Countdown cancellation never calls session start. Expiry invokes it once.
- Countdown remains correct after minimized/background/restored/refocused at 5/10/15 seconds.
- Process exit during countdown yields game-not-found; component and permission errors each use the repair flow.
- Recording contains no live FPS/stability claim. Elapsed/progress comes from timestamps and requested duration.
- Stop confirmation sends one cancel request. UI enters Analyzing while backend may remain Running, then transitions to Completed/Failed without duplicate calls.
- Expiry does not call session_stop just to force a state transition.
- Completion toast is attempted after Completed. Stable toast is post-completion, verdict- and preference-controlled.
- Window close never silently discards a worker. Test Stay, confirm-close, Completed/Failed races, cancellation and child cleanup.
- Both manifests are verified before compare. Unverified choices are disabled; backend still rejects invalid sessions.
- same_scenario mapping is Yes→true and Not sure/No→false. False yields InconclusiveDifferentScenario.
- History labels remain unique without invented timestamps. interrupted comes from a confirmed field or local stop intent, never inferred from duration.
- Metrics/evidence remain unchanged. Missing remains missing. No analysis threshold is introduced.

### Accessibility / visual

- Keyboard-only walkthrough: Home → countdown cancel → recording → stop dialog → result → Expert details → History → Compare → Settings.
- Screen-reader check: Home status, countdown, progress, dialog, result, disabled Compare reason and error-copy action.
- Check focus visibility and 4.5:1 body / 3:1 large-text contrast. Status uses text/icon plus colour.
- Test at 900×600, 1120×720 and 1360×860, plus Windows 125%/150% scaling, WebView zoom/text scaling, forced-colors and reduced-motion.
- Test long application/session IDs and both languages for wrapping, clipping and accessible controls.
- Verify CSP still allows only local content; no new dependencies, network calls or telemetry.

### Required local gate

Run on the intended Windows development host using the pinned toolchain and exact commands documented in docs/local-gates.md. Attach real output to STATE; do not claim PASS until run:

1. cargo fmt --all -- --check
2. cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
3. cargo test --workspace --all-features --locked
4. cargo deny check
5. cargo audit
6. python tools/validate_schemas.py
7. python tools/check_docs.py
8. python -m unittest discover -s tools -p "test_*.py"
9. python tools/check_links.py

Then run a packaged Windows smoke test for countdown minimized, complete capture, early stop, failures, completion notification, Stable notification, history load/compare and close cleanup. This handoff claims no Rust build, test or runtime verification.

## 11. Research references

- W3C — WCAG 2.2: https://www.w3.org/TR/WCAG22/
- W3C — Contrast (Minimum): https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum
- Microsoft Learn — Designing inclusive software in Windows: https://learn.microsoft.com/en-us/windows/apps/design/accessibility/designing-inclusive-software
- Microsoft Learn — Accessible text requirements: https://learn.microsoft.com/en-us/windows/apps/design/accessibility/accessible-text-requirements
- Tauri v2 — Notifications: https://v2.tauri.app/plugin/notification/
- Tauri v2 — Event API / close-requested: https://v2.tauri.app/reference/javascript/api/namespaceevent/
- RWP ADR-0028: ../adr/0028-presentmon-distribution-and-verification.md

## 12. Ready-to-run prompt for Muse

You are implementing the approved RWP desktop UX redesign in docs/autopilot/UX_REDESIGN_HANDOFF.md. Re-read docs/autopilot/STATE.md, docs/autopilot/MASTER_PLAN.md and linked sources at current repository head before touching code. This handoff is design-only; verify all assumptions against current code.

Follow the repository direct-main/no-branches policy. Do not ask the owner questions; use documented defaults and record evidence for any blocker. Work in small batches, about 8 files / 600 changed lines maximum. Add regression tests before behavior changes. Do not weaken tests, add frameworks, Node build tooling, external assets or dependencies. Keep Cargo.lock unchanged unless an explicit project-plan-approved dependency decision justifies otherwise.

Preserve invariants: out-of-process PresentMon only; offline-first/no outbound telemetry/no analysis-engine networking; missing stays missing; evidence scores are not probabilities; deterministic output; no analysis rule/threshold/version changes; no manifest/report schema changes; current CLI compatibility remains. IPC changes must be additive and tested.

Build in vanilla HTML/CSS/JS with Tauri v2. Hide PID, paths, technical sampling settings, PresentMon internals and raw English evidence from ordinary screens. Use four-view IA plus Settings/Advanced. Implement exclusive Countdown → Recording → Analyzing states; poll until Completed/Failed after Stop or duration expiry. Use existing timestamps; never invent live metrics. Implement accessible keyboard/focus/dialog/progress patterns and all ID/EN copy.

Respect ADR-0028: do not bundle/download PresentMon. Use guided one-time selection and existing native picker; remember only a validated choice locally. Never elevate silently. Add explicit permission repair. Add generic native completion notification and bilingual Rust-side notification strings as an additive integration; Stable remains post-completion and preference-controlled. Notifications must not include process identity. Add close confirmation and prove cleanup/persistence behavior.

History uses only existing fields; do not invent dates. Use unique labels from session_id; expose manifest timestamp only if already present. Disable unverified comparison candidates. Use one Before/After selection method. Map answers exactly (Yes=true, Not sure/No=false) and render only Comparable, LimitedSampleEquivalence and InconclusiveDifferentScenario.

Execute five batches in §9. After each, run targeted tests and the full local gate when feasible, update CHANGELOG only for implemented user-visible behavior, update the complete STATE.md with real evidence and include precise local commands. Never claim tests/build/runtime acceptance without actual command output. If a Windows-only check cannot run in the current environment, record UNVERIFIED and list the exact smoke test. Do not publish a release for this UX task.
