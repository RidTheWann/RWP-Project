# MASTER PLAN — RidTheWann Performance (rwp) menuju v1.0

> Taruh file ini di repo sebagai `docs/autopilot/MASTER_PLAN.md`. Ini adalah sumber kebenaran rencana.
> Status pekerjaan ada di `docs/autopilot/STATE.md` (template di §6). Jangan campur keduanya.

## 0. Execution policy (local-first, no branches, CI as supplement)

There is one working tree and no other topology:

- All implementation, fixes, tests, documentation, audits, and release
  preparation happen directly in the tree, on `main`. Never create working
  branches of any kind. Commit, push, and tag only on explicit owner order.
- There is no promotion flow, no merge-back, no freeze, no trailers, and no
  skip markers. Since 2026-10-16 (owner order) `.github/workflows/` holds
  `ci.yml` (gates + debug build per push/PR) and `release-alpha.yml`
  (tag-only NSIS pre-release, published as GitHub Release) as a supplement;
  the per-batch local gate on the pinned 1.99.0 toolchain remains the
  verification authority (`docs/local-gates.md`).
- Every batch follows: work (tests first) → full local gate → owner audit →
  record in `docs/autopilot/STATE.md` (+ CHANGELOG only for user-facing
  changes). No verification claim without pasted command output.
- Git is used for publish and tree-cleanliness checks only: commit/push/tag
  on explicit owner order (`main` only), `git status` to guard a clean tree
  before gates. Never use `git log`/`git diff` output as proof of behavior;
  verify via direct file reads plus pasted gate output. Never remove the
  `.git` directory without an explicit owner order.

## 1. Definisi "siap release & matang" (Release Criteria)

v1.0.0 hanya dicatat (CHANGELOG + provenance) jika SEMUA kriteria ini terpenuhi dan tercatat di STATE.md dengan bukti (output gate / log / file).

| ID | Kriteria | Target awal (boleh disesuaikan lewat ADR) |
|---|---|---|
| RC-01 | Semua temuan AUD berprioritas P0 dan P1 tertutup, masing-masing dengan tes regresi | 100% |
| RC-02 | `Cargo.lock` present; semua perintah gate memakai `--locked`; build Windows + Linux (crate netral) hijau | hijau |
| RC-03 | Coverage baris crate inti; mutation score crate analisis | ≥85% line (analysis, diagnostics, report, parser capture); ≥70% mutation (analysis, diagnostics) |
| RC-04 | Tiap rule diagnosis punya tes: fire, no-fire, kontradiksi, data kurang; rule_id berversi; threshold terdokumentasi beserta dasar pemilihannya | semua rule |
| RC-05 | Overhead sampler terukur dan terdokumentasi | rata-rata ≤1% CPU total, p99 durasi satu sampel ≤2 ms pada interval default (mesin referensi dicatat) |
| RC-06 | Alur end-to-end capture → analyze → compare → report berjalan di CLI dan GUI dengan data nyata; hasil disimpan dengan manifest + hash | ya |
| RC-07 | Validasi pengukuran dengan data nyata | ≥6 capture nyata (≥2 vendor GPU bila tersedia) + hasil kalibrasi A/A |
| RC-08 | Input tidak tepercaya (CSV PresentMon, capture/report JSON, path dari webview) aman: batas ukuran, fuzz/proptest, threat model | ya |
| RC-09 | Installer NSIS dibangun lokal, diuji install/upgrade/uninstall, ber-SHA256 + SBOM + provenance; ditandatangani (atau batasan SmartScreen didokumentasikan secara eksplisit) | ya |
| RC-10 | Tidak ada koneksi jaringan keluar pada operasi normal (diverifikasi, bukan diasumsikan) | ya |
| RC-11 | Dokumentasi lengkap dan konsisten, diperiksa otomatis lokal | ya |
| RC-12 | Matriks akseptansi di hardware nyata lulus dan tercatat di `docs/acceptance/` | ya |

## 2. Baseline dan temuan audit (AUD)

Baseline: workspace 10 crate, versi 0.5.0-beta (kandidat; alpha 0.1.0-0.3.0 dan beta 0.4.0-beta frozen), edition 2024, MSRV 1.99, toolchain 1.99.0, target `x86_64-pc-windows-msvc`. Milestone M1–M8 terimplementasi, M10 baru fondasi packaging.

PENTING: temuan di bawah berasal dari analisis statis (belum dikompilasi). **Verifikasi ulang tiap AUD terhadap kode terkini sebelum memperbaikinya.** Jika sudah tidak berlaku, tandai `STALE` di STATE.md beserta alasannya.

| ID | Prioritas | Temuan | Lokasi | WP |
|---|---|---|---|---|
| AUD-01 | P0 | `evidence()` selalu memberi `EvidencePolarity::Supporting`, juga untuk item `contradicting_evidence` (rule CPU-bound dan GPU-bound) | `rwp-diagnostics/src/lib.rs` | WP1 |
| AUD-02 | P0 | BackgroundInterference: evidence `background.overlap` (bobot 70 ≥ activation 60) diberikan hanya karena ada ≥1 sampel di jendela stutter, tanpa cek beban; event 1 frame punya jendela lebar nol (`start == end`) sehingga hampir tak pernah overlap pada sampling 50 ms | `evaluate_background_interference` | WP1 |
| AUD-03 | P0 | Cancel memakai `Child::kill()` lalu parse CSV; baris terakhir terpotong → `MalformedCsv` → sesi `Failed`, bertentangan dengan ledger M8. Hard kill juga berpotensi meninggalkan sesi ETW yatim | `rwp-session/src/windows.rs::collect_capture`, `rwp-capture/src/csv.rs` | WP1, WP2 |
| AUD-04 | P0 | Error telemetri non-esensial (`NonMonotonicCpu`, `ThreadEnumeration`, `ProcessSnapshot`) menghentikan PresentMon dan membuang seluruh capture; melanggar prinsip "absence, not fabrication" | `collect_capture`, `rwp-system` | WP1 |
| AUD-05 | P1 | Thread worker `session_start` tanpa guard panic: snapshot tertinggal `Running` selamanya, `session_reset` menolak | `rwp-ui-native/src-tauri/src/commands.rs` | WP1 |
| AUD-06 | P1 | `PairedInferenceConfig::default` memakai `minimum_pairs=5` dengan `alpha=0.05`; p minimum sign test dua sisi di n=5 adalah 0,0625, jadi Improved/Regressed tak mungkin tercapai pada 5 pasangan informatif (butuh n≥6) | `rwp-analysis/src/comparison.rs` | WP1 |
| AUD-07 | P2 | Kebijakan monoton tidak konsisten: parser CSV mengizinkan QPC sama, `detect_stutters` menolak timestamp sama | `csv.rs`, `stutter.rs` | WP1 |
| AUD-08 | P1 | `absolute_threshold_ms=30` membuat hitch 20–25 ms tak terdeteksi di 144–240 FPS | `StutterConfig::default` | WP3 |
| AUD-09 | P1 | Akurasi CPU per-thread: `GetThreadTimes` (VERIFY: granularitas berbasis tick ~15,6 ms) vs sampling 50 ms; `OpenThread(THREAD_QUERY_INFORMATION)` seharusnya `THREAD_QUERY_LIMITED_INFORMATION`; timestamp diambil di awal `sample()` padahal pembacaan terjadi belakangan | `rwp-system/src/threads.rs`, `windows.rs` | WP3 |
| AUD-10 | P1 | Overhead: tiap 50 ms snapshot seluruh thread + proses sistem + `OpenProcess` ke ratusan PID, sementara doc comment menyebut "low-overhead" | `rwp-system` | WP3 |
| AUD-11 | P1 | Rule Thermal aktif hanya dari suhu ≥85 °C (bobot 70); FramePacing aktif dari satu stutter event (bobot 60); keduanya tanpa bukti kuat/laju | `rwp-diagnostics` | WP3 |
| AUD-12 | P1 | GPU-bound memakai `gpu_time_ms` (VERIFY: di PresentMon v2 `GPUBusy` lebih mewakili kerja GPU); NVML hanya device 0 dan hanya NVIDIA; tidak ada telemetri GPU AMD/Intel | `rwp-diagnostics`, `rwp-system/src/windows.rs` | WP3 |
| AUD-13 | P1 | Parser CSV mengabaikan `SwapChainAddress` dan `FrameType` (overlay di PID sama, frame generation); VERIFY semantik `FrameTime` v2 vs interval tampil | `csv.rs` | WP2 |
| AUD-14 | P1 | stderr PresentMon `inherit` (hilang di GUI); error hanya exit code; tidak ada preflight izin (admin / Performance Log Users) | `rwp-capture/src/command.rs` | WP2 |
| AUD-15 | P2 | `--timed N` mulai setelah PresentMon siap, telemetri berhenti N detik sejak spawn: cakupan ekor frame vs telemetri tidak simetris | `rwp-session` | WP2 |
| AUD-16 | P1 | Alur produk terputus: tak ada penulis `capture.v1` ke disk, CLI tak menjalankan diagnosis, `PerformanceReport` tak memuat diagnostics, GUI tak menyimpan report, `paired_infer` tak dipanggil dari mana pun, `rwp-report` menjanjikan Markdown/HTML tetapi hanya JSON | CLI, report, session | WP4 |
| AUD-17 | P2 | `rwp-session` bergantung ke `rwp-ui` (orkestrator membangun `DashboardModel`), membalik arah dependensi | `rwp-session/Cargo.toml` | WP5 |
| AUD-18 | P1 | Hardening Tauri: `session_start` men-spawn path executable sembarang dari webview; PresentMon tidak dibersihkan saat app ditutup; `session_status` men-clone snapshot besar tiap 500 ms | `commands.rs` | WP5 |
| AUD-19 | P0 | `Cargo.lock` tidak di-commit; CI memakai `cargo generate-lockfile` lalu `--locked` (tautologi); tidak ada job build NSIS padahal gate M10 mensyaratkannya | `.github/workflows/ci.yml` | WP0 |
| AUD-20 | P2 | Drift dokumen: README (M1–M8) vs ledger (M10) vs CHANGELOG (M8); section M6/M7 ganda; ledger menyebut rule memory/VRAM "deferred" padahal ada; indeks ADR 14 tapi hanya 0009/0010/0014 ada; `docs/schema/v1/` dirujuk tetapi tidak ada | `docs/`, `README.md` | WP0, WP8 |
| AUD-21 | P1 | Cakupan tes timpang: diagnostics hanya 4 tes (cpu, ram, vram, unknown); ui-native 0 tes; belum ada fixture CSV PresentMon asli | seluruh repo | WP6 |

## 3. Work package (kerjakan berurutan; WP yang tidak terblokir boleh didahulukan)

Aturan umum semua WP:
- Bug → tulis tes yang gagal dulu, baru perbaiki.
- Perubahan perilaku rule/detektor → naikkan versi (`rule_id`, `analysis_config`) agar hasil lama tetap bisa direproduksi. Jangan mengubah makna field serialisasi tanpa bump schema.
- Setiap keputusan desain non-trivial → ADR singkat di `docs/adr/` (konteks, opsi, keputusan, konsekuensi).
- Tiap WP diakhiri: CHANGELOG diperbarui, STATE.md diperbarui, perintah verifikasi lokal dicantumkan.

### WP0 — Fondasi reproducibility dan gate lokal (AUD-19, AUD-20)
1. `Cargo.lock`: present in the source tree and validated with `--locked` on every gated command (historical note: the lockfile was committed during WP0; the tautological lockfile-generation CI step is long retired with CI itself).
2. Gate lokal berurutan: `fmt --all -- --check`, `clippy --workspace --all-targets --all-features --locked -- -D warnings`, `test --workspace --all-features --locked`, `cargo deny check`, `cargo audit`, `validate_schemas.py`, `check_docs.py` + unit checker, di toolchain pin 1.99.0. Perintah persisnya di `docs/local-gates.md`.
3. Job NSIS: build bundle Tauri lokal (versi `tauri-cli` tercatat), installer + `SHA256SUMS` sebagai artefak lokal yang dihash ke provenance (biner tak dikomit).
4. CI stays as a supplement (owner order 2026-10-16): `ci.yml` gates + debug build per push/PR and `release-alpha.yml` tag-only NSIS pre-release; Dependabot/action pins (full-SHA `uses`) remain enforced in `.github/workflows/`. The local authority is unchanged: pin toolchain + `--locked` + kebijakan lisensi `deny.toml`/`about.toml` yang ditegakkan lokal.
5. Lint: aktifkan `clippy::undocumented_unsafe_blocks` (deny) dan tulis `// SAFETY:` di setiap blok `unsafe` di `rwp-system`.
6. Sinkronkan dokumen: satu sumber status (ledger), README hanya menaut. Rekonstruksi ADR-0001..0014 yang hilang berdasarkan ledger dan kode (tandai "reconstructed"). Hapus section ganda. Buat `docs/schema/v1/` (lihat WP4).
7. Skrip cek dokumen lokal: setiap ADR di indeks ada file-nya; milestone README = CHANGELOG terbaru; tautan relatif tidak patah (`tools/check_docs.py`, `tools/check_links.py`).

DoD: gate lokal hijau penuh; `--locked` bermakna; installer artifact terbentuk; cek dokumen lulus.

### WP1 — Perbaikan correctness (AUD-01..07), test-first
- AUD-01: ganti helper jadi `supporting()` / `contradicting()`; tes: semua item di `contradicting_evidence` berpolaritas `Contradicting`; snapshot serialisasi.
- AUD-02: tes (a) event multi-frame dengan background 0% → tidak ada BackgroundInterference; (b) event 1 frame + burst ≥ threshold dalam ±1 interval sampling → terdeteksi; (c) hasil memuat `aligned_events / total_events`. Bangun jendela dari interval frame + margin, ambil sampel terdekat, jadikan burst ≥ threshold bukti utama. Rule menjadi `background_interference.v2`.
- AUD-03: hentikan PresentMon secara graceful (VERIFY mekanisme: `CREATE_NEW_PROCESS_GROUP` + `GenerateConsoleCtrlEvent`, fallback kill setelah timeout); parser mendapat `ParseMode::{Strict, TolerateTruncatedTail}`; hasil cancel = capture `interrupted` (flag eksplisit di report), bukan `Failed`; tes dengan CSV terpotong.
- AUD-04: perkenalkan trait `SystemSampler` agar bisa difake; error telemetri non-esensial → `None` + `TelemetryWarning` di `SessionResult`; hanya kegagalan inisialisasi QPC yang fatal. Tes dengan sampler palsu yang gagal intermiten.
- AUD-05: guard/`catch_unwind` yang menyetel `Failed` bila worker panic; tes.
- AUD-06: validasi kombinasi `minimum_pairs` terhadap `alpha` (fungsi `minimum_pairs_for_alpha`), default 6; property test p-value sign test vs enumerasi brute-force untuk n ≤ 20; hapus special-case dan `mul_add` yang redundan.
- AUD-07: satu kebijakan monoton (dokumentasikan); tes.

DoD: setiap AUD punya tes yang gagal di kode lama dan lulus di kode baru.

### WP2 — Ketahanan capture (AUD-13, 14, 15)
1. Kelompokkan baris per `SwapChainAddress`, pilih swapchain dominan (jumlah frame), laporkan yang dibuang; filter/label `FrameType` (frame generation). VERIFY semantik `FrameTime` v2 di dokumentasi PresentMon; bila interval CPU-start, tambahkan metrik sisi layar (mis. DisplayedTime) sebagai metrik terpisah, jangan mengganti diam-diam.
2. Tangkap stderr PresentMon (pipe) dan sertakan di error; preflight izin (admin / grup Performance Log Users) dengan pesan perbaikan; opsi eksplisit `--stop_existing_session` dengan persetujuan pengguna.
3. Deteksi versi PresentMon (VERIFY mekanismenya) dan tolak versi di luar matriks yang diuji.
4. Cakupan simetris: analisis memakai irisan rentang waktu frame dan telemetri; laporkan `coverage` (detik frame, detik telemetri, irisan).
5. Fixture nyata: BLOCKED_ON_USER (≥6 CSV PresentMon v2 `--qpc_time`, dianonimkan: DX11, DX12, Vulkan, windowed/fullscreen, vsync on/off, overlay aktif, frame gen bila ada) di `crates/rwp-capture/tests/fixtures/`.

DoD: parser lulus pada fixture nyata; proptest "tidak panic untuk input arbitrer"; memori O(n).

### WP3 — Validitas pengukuran (AUD-08..12)
1. Detektor stutter berbasis relatif/refresh-aware (mis. ambang = max(k·baseline, baseline + Δms)), config versi baru; versi lama tetap bisa dipakai lewat provenance. Validasi dengan trace sintetis ber-seed pada 60/144/240 Hz (golden).
2. Harness akurasi CPU per-thread: proses uji yang spin pada duty-cycle diketahui (25/50/75/100%); bandingkan `GetThreadTimes`, `QueryThreadCycleTime`, dan (VERIFY, butuh admin) ETW CSwitch; keputusan di ADR-0004 berdasarkan data. Ganti hak akses ke `THREAD_QUERY_LIMITED_INFORMATION`; ambil timestamp tepat di sekitar pembacaan. BLOCKED_ON_USER: menjalankan harness di hardware dan menempel hasilnya.
3. Ukur overhead sampler (`SamplerStats`: rata-rata dan p99 durasi sampel); tentukan interval default dari data; evaluasi satu panggilan `NtQuerySystemInformation(SystemProcessInformation)` menggantikan dua snapshot ToolHelp (catat risiko API semi-terdokumentasi di ADR).
4. Rule: Thermal berbasis throttle reasons/penurunan clock (VERIFY nama API di `nvml-wrapper` 0.13), FramePacing berbasis laju (event/menit) plus ekor; GPU-bound berbasis `GPUBusy` (VERIFY); semua threshold punya dasar tertulis di `docs/diagnostics.md`.
5. GPU: pilih adapter yang dipakai proses (bukan index 0); jalur vendor-agnostic via PDH `GPU Engine` per-PID (VERIFY path counter); telemetri tetap `None` bila tak tersedia.
6. `rwp calibrate`: beberapa run A/A pada skenario sama → estimasi variansi P99 antar-run → rekomendasi `equivalence_margin_percent` dan `minimum_pairs`. Perbarui ADR-0009.
7. Flag `--no-process-probing` untuk mematikan probing thread/proses (mitigasi risiko dengan anti-cheat); dokumentasikan dan putuskan default lewat ADR.

DoD: tiap rule punya tes fire / no-fire / kontradiksi / data kurang; keputusan teknis punya data dan ADR.

### WP4 — Alur produk end-to-end (AUD-16)
1. Skema: putuskan `rwp.report.v2` (menambah diagnostics, metadata capture, lingkungan, coverage, warnings) dengan pembaca v1 tetap didukung; tulis `capture.v1` ke disk; JSON Schema dihasilkan dari tipe (mis. `schemars`) ke `docs/schema/`, divalidasi di CI dengan golden.
2. Penyimpanan sesi: `sessions/<tanggal>-<app>/{manifest.json, capture.json, report.json, telemetry.jsonl, presentmon.csv}`; manifest v2 memuat versi rwp + git sha, build target/profile, OS, CPU/GPU/driver, versi + SHA-256 PresentMon, konfigurasi, frekuensi QPC, dan SHA-256 tiap file ("Prove").
3. CLI: `rwp doctor` (preflight), `rwp capture`, `rwp import-presentmon <csv>`, `rwp analyze` (termasuk diagnosis), `rwp compare` (single dan `--pairs <dir>` memakai `paired_infer`), `rwp calibrate`, `rwp report --format json|md|html --redact`. Exit code terdokumentasi; opsi `--json` untuk error mesin-terbaca.
4. Renderer Markdown dan HTML (HTML self-contained tanpa aset eksternal). Redaksi default (username, path penuh, inventori proses) dengan tes yang memindai output terhadap path profil/nama mesin.
5. GUI: pemilih proses (proses dengan aktivitas present), dialog file (plugin dialog Tauri), progres live (elapsed, jumlah frame, grafik frame time via event backend), riwayat sesi, ekspor, tab Compare dengan dua sesi nyata, pesan error + langkah perbaikan.

DoD: tes end-to-end otomatis fixture CSV → import → analyze → compare → report dengan golden; checklist manual GUI di `docs/acceptance/gui.md`.

### WP5 — Arsitektur dan keamanan (AUD-17, AUD-18)
1. Putuskan ketergantungan `rwp-session → rwp-ui`; `build_dashboard_model` pindah ke lapisan aplikasi.
2. Tauri: validasi path executable (ada, nama file, path kanonik; opsi verifikasi tanda tangan digital via `WinVerifyTrust` atau pin SHA-256), capability minimal, CSP ketat; PresentMon dimasukkan ke Job Object `KILL_ON_JOB_CLOSE` atau dibersihkan di `RunEvent::ExitRequested`; `session_status` tanpa deep clone (`Arc` atau event berversi).
3. Logging: `tracing-subscriber` + file bergulir di `%LOCALAPPDATA%`, tanpa PII, level terkonfigurasi; `rwp doctor --bundle` menghasilkan paket diagnostik terredaksi.
4. Taksonomi error dengan kode (`RWP-Exxxx`) + teks perbaikan.
5. `docs/threat-model.md`: batas kepercayaan webview↔backend, executable PresentMon, CSV/JSON masukan, penanganan path. Batas masukan (ukuran file, jumlah frame, kedalaman JSON).

DoD: tes validasi path; target fuzz/proptest untuk parser; threat model ditinjau pemilik.

### WP6 — Kualitas dan tes (AUD-21)
1. Coverage (`cargo-llvm-cov`) dengan gate per crate sesuai RC-03; mutation testing (`cargo-mutants`) pada analysis, diagnostics, report; bunuh mutan yang selamat atau catat alasannya.
2. Property test: persentil vs implementasi referensi naif; invarian stutter (event terurut, tak tumpang tindih, merge gap dihormati); serde round-trip; determinisme (dua run → byte identik).
3. Benchmark Criterion analyzer untuk 1 juta frame dengan anggaran regresi; benchmark overhead sampler.
4. Fuzz (`cargo-fuzz`, pin 0.13.2, nightly) untuk parser CSV dan pembaca JSON: korpus seed ter-commit; eksekusi libFuzzer berjalan di Linux (deferred dari Windows lokal — lihat STATE Fase 2); smoke-test mutasi deterministik berjalan di gate stabil.
5. Aplikasi uji sintetis `tools/rwp-testapp` yang mem-present dengan kadensi dan hitch terkontrol untuk harness akseptansi (VERIFY kelayakan di mesin owner; fallback: manual).
6. Golden dari capture nyata (BLOCKED_ON_USER, bagian dari WP2).

DoD: RC-03 dan RC-04 terpenuhi.

### WP7 — Rekayasa rilis (RC-09, RC-10)
1. Kebijakan SemVer; CHANGELOG format Keep a Changelog; proses rilis lokal: build, test, NSIS, `SHA256SUMS`, SBOM (`cargo-cyclonedx`), `THIRD_PARTY_LICENSES` (`cargo-about`), attestation provenance build, draft catatan rilis dari CHANGELOG. Rilis dipublikasikan lewat tag git + GitHub Release (pre-release) atas perintah owner.
2. Penandatanganan Authenticode untuk exe + installer: BLOCKED_ON_USER (sertifikat atau layanan signing untuk OSS). Sebelum tersedia, dokumentasikan peringatan SmartScreen.
3. ADR `webviewInstallMode` (bootstrapper vs offline installer WebView2) mengingat posisi offline-first.
4. Tes installer lokal: silent install/uninstall, upgrade dari versi sebelumnya, lokasi data pengguna dipertahankan/dihapus sesuai pilihan.
5. ADR distribusi PresentMon: (a) disuplai pengguna (sekarang), (b) unduhan terpandu dengan SHA-256 pin, (c) dibundel (verifikasi lisensi dan atribusi dulu). Keputusan: pemilik.
6. ADR auto-update: default OFF; bila ada harus opt-in dan bertanda tangan.
7. Migrasi skema: `rwp migrate` untuk laporan/sesi lama.
8. Bukti RC-10: pemindaian dependensi engine (nol HTTP-client di graf target) dan run terpantau yang tercatat, bukan asumsi (dicatat di STATE dan `docs/acceptance/`).

DoD: artefak rilis lokal lengkap; installer lulus tes.

### WP8 — Dokumentasi dan UX akhir (RC-11)
README ditulis ulang dengan contoh hasil ukur nyata (berlabel jelas), instalasi, quickstart GUI dan CLI, ringkasan metodologi, keterbatasan, privasi. Dokumen: `docs/methodology.md` (definisi statistik, detektor, rule, threshold, hasil validasi), `docs/limitations.md` (frame generation, multi-GPU, anti-cheat, apa yang tidak bisa disimpulkan), `docs/troubleshooting.md`, `docs/privacy.md`. Tambah CODE_OF_CONDUCT, template issue/PR, perbarui SECURITY (kanal pelaporan). Referensi CLI dihasilkan dari `clap`. Aksesibilitas GUI dasar (keyboard, kontras, label). Cek tautan via `tools/check_links.py` lokal.

### WP9 — Release candidate dan v1.0 (RC-12)
Freeze fitur; RC1; matriks akseptansi di hardware nyata (BLOCKED_ON_USER), dicatat di `docs/acceptance/<tanggal>.md`:
- ≥2 mesin (idealnya NVIDIA + non-NVIDIA), ≥3 judul/API (DX11, DX12, Vulkan).
- Skenario: baseline; beban CPU latar sengaja (harus memunculkan BackgroundInterference/CPU-bound dengan benar, dan tidak memunculkannya saat tanpa beban); stres GPU (GPU-bound); cancel di tengah run; PresentMon tanpa izin; game ditutup di tengah run; run 30 menit (memori stabil); jaringan diputus (tidak ada koneksi keluar); kalibrasi A/A.
Daftar known issues; checklist go/no-go; catat `v1.0.0` di CHANGELOG + provenance hanya bila RC-01..RC-12 terpenuhi; tag + GitHub Release atas perintah owner.

## 4. Gate umum (otoritas: `docs/local-gates.md`, berlaku di setiap batch)
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
- `cargo test --workspace --all-features --locked`
- `cargo deny check` dan `cargo audit`
- `python tools/validate_schemas.py`, `python tools/check_docs.py` + unit checker, `python tools/check_links.py`
- Tes baru untuk setiap perubahan perilaku; tidak ada tes yang dilemahkan agar lulus.
- Tanpa klaim lolos tanpa output perintah yang ditempel di STATE.

## 5. Perkiraan daftar BLOCKED_ON_USER
`Cargo.lock` present · fixture CSV PresentMon nyata · hasil harness akurasi CPU per-thread · hasil benchmark overhead di mesin referensi · sertifikat code signing · keputusan ADR (distribusi PresentMon, WebView2, auto-update, default `--no-process-probing`) · akseptansi hardware (WP9).
Kumpulkan semuanya dalam satu daftar, beri rekomendasi default, dan teruskan WP lain yang tidak terblokir.

## 6. Template `docs/autopilot/STATE.md`

```
---
plan_version: 1
last_updated: <ISO-8601>
repo_ref: main @ <sha> (publish ref; trail lives in Log + reports)
active_wp: WP0
---
## Status WP
| WP | status (TODO/IN_PROGRESS/DONE/BLOCKED_ON_USER) | catatan |
|---|---|---|
| WP0 | TODO | |

## Status AUD
| AUD | status (OPEN/FIXED/STALE/WONTFIX) | bukti (tes/laporan/STATE) |
|---|---|---|

## Status RC
| RC | status | bukti |
|---|---|---|

## Log (terbaru di atas)
- <tanggal> <WP>/<AUD>: <apa yang dikerjakan>; output gate ditempel

## Keputusan terbuka
- <pertanyaan> — rekomendasi default: <...>

## BLOCKED_ON_USER
- <item> — dibutuhkan untuk: <WP>

## NEXT
<satu kalimat: langkah berikutnya yang konkret>
```
## Autopilot execution rule — audit-first, local gates

- All project work happens directly in the single working tree, on `main`. Never create working branches. Commit, push, and tag only on explicit owner order, and never use git log/diff output as proof of behavior (no `git log`/`git diff` reads for verification claims).
- Every batch runs: work (tests first) → full local gate in order → owner audit → record in STATE (+ CHANGELOG only for user-facing changes).
- Local Rust verification is the gate, not an option: pinned 1.99.0 toolchain, formatting, lint, tests, schema, docs, dependency, and installer checks per `docs/local-gates.md`.
- After a gate failure, inspect the exact failing output, fix the root cause, and re-run the full gate; never weaken tests or lint gates. Record the failure and the fix in STATE.
- No claim without pasted command output. Git is used only for publish (on owner order) and tree-cleanliness checks (`git status` before gates).
- Threat-model sign-off and release decisions belong to the owner; never sign unilaterally.
