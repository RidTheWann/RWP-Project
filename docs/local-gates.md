# Local Gates — Otoritas Verifikasi Pengganti CI (Fase 0)

Status: MENGIKAT sejak Fase 0. Sejak 2026-10-16 repo memiliki workflow
`.github/workflows/` (`ci.yml` gates + debug build per push/PR ke `main`,
`release-alpha.yml` NSIS + pre-release hanya pada tag `v*-alpha*`) atas
perintah owner sebagai suplemen — bukan pengganti. Remote GitHub dipakai
untuk publish (branch `main` + tag alpha, atas perintah owner). Gate lokal di
bawah tetap satu-satunya otoritas verifikasi per batch; CI yang merah tidak
pernah menghalalkan klaim "lolos" tanpa output gate lokal di STATE, dan CI
yang hijau tidak menggantikan kewajiban gate lokal.

> Catatan 2026-10-12 (revisi kebijakan owner): git dipakai untuk publish —
> commit/push/tag atas perintah eksplisit owner, di branch `main` saja tanpa
> branches kerja. Yang tetap berlaku dari catatan 2026-10-10: tanpa
> mengandalkan git log/diff untuk klaim perilaku; verifikasi berjalan via
> baca file langsung + output gate yang ditempel di STATE. Frasa "tree bersih
> (`git status ...`)" di §3 dibaca sebagai `git status` untuk kebersihan tree
> dan pra-commit. Otoritas gate dan urutan perintah tak berubah.

## 1. Pin toolchain

Toolchain Rust wajib `1.99.0` persis (`rust-toolchain.toml`, `cargo --version`
+ `rustc --version`). Toolchain lain = churn layout, dilarang.

Alat gate yang dipakai baseline Fase 0 (2026-10-10, Windows):

- `cargo-deny 0.20.2` (`cargo install cargo-deny --locked`)
- `cargo-audit 0.22.2` (`cargo install cargo-audit --locked`)
- `python 3.14.7` + `jsonschema` (untuk `validate_schemas.py`)
- `cargo-llvm-cov`, `cargo-mutants`, `cargo-fuzz` TIDAK termasuk gate Fase 0;
  dipakai mulai Fase 1/2 dengan pin masing-masing.

## 2. Perintah gate (berurutan, berhenti pada gagal pertama)

Jalankan dari root repo, dalam urutan ini:

1. `cargo fmt --all -- --check`
2. `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings`
3. `cargo test --workspace --all-features --locked`
4. `cargo deny check`
5. `cargo audit`
6. `python tools/validate_schemas.py`
7. `python tools/check_docs.py` + unit checker `python -m unittest discover -s tools -v`

Semua perintah memakai `--locked` di mana didukung (`Cargo.lock` ter-commit).
`fuzz/` tetap `exclude` dari workspace stabil sehingga gate di atas tidak
pernah mengompilasi dependensi nightly/libFuzzer.

Akuntansi hitungan tes: total logis 305 termasuk 7 uji platform-gated
(`unix`/`not(windows)`) yang tak terkompilasi di Windows, sehingga hitungan
Windows yang diharapkan adalah 298.

## 3. Aturan bukti (mengikat)

- Tanpa klaim "lolos" tanpa output perintah. Setiap batch menempel ringkasan
  hasil per perintah (pass/fail + angka: jumlah tes, crate, warning) ke
  `docs/autopilot/STATE.md`.
- Klaim lama bertanda "CI-verified" / run-ID / PR GitHub dinyatakan
  tak-terverifikasi-ulang; penggantinya adalah baris "locally-verified" dengan
  output gate di STATE.
- Tree harus bersih sebelum gate dijalankan (`git status`): tidak ada file
  tak-terstage di luar yang disengaja, dan tidak ada binary/artefak build
  (`target/`, `*.exe`, `SHA256SUMS`) yang ikut ter-stage. Klaim perilaku
  tetap diverifikasi via baca file langsung + output gate yang ditempel di
  STATE, bukan via `git log`/`git diff`.

## 4. Daftar pensiun eksplisit (non-berlaku sejak Fase 0)

Dokumen lama di bawah TIDAK dihapus/diubah penuh di Fase 0 (tulis ulang penuh
di Fase 7), tetapi seksi-seksi berikut dinyatakan NON-BERLAKU sebagai otoritas:

- `CONTRIBUTING.md`: kebijakan dua-branch (`development`/`main`), "Local builds
  are optional / CI owns gates", "Hard CI trigger gate" (`Deep-Audit: PASS`
  trailer, `[skip ci]`), dan topologi promosi/PR — non-berlaku.
- `docs/autopilot/MASTER_PLAN.md`: §0 topologi dua-branch + audit-first CI,
  §1 bukti RC berupa link CI, WP0 job CI/pin Actions/Dependabot, dan semua
  rujukan "pinned CI / CI hijau / trigger run" — non-berlaku sebagai syarat.
- Ritual yang pensiun: trailer `Deep-Audit`/`Local-Verification`, marker
  `[skip ci]`, promotion `development`→`main`, push freeze, merge-back,
  workflow dispatch, `check_workflow_security` (kini selalu-skip karena
  `.github/` tidak ada).
- Disiplin pengganti: tanpa exchange/branch/trailer — tiap batch:
  kerja → gate lokal → audit owner → STATE (+ CHANGELOG hanya bila menyentuh
  perilaku user-facing).

## 5. Keputusan versi

Versi berjalan `0.4.0` pasca-alpha (SemVer formal + Keep-a-Changelog; rekam alpha `0.1.0`-`0.3.0` tetap beku di catatan rilisnya).

## 6. Gate mutants (diadopsi owner 2026-10-10)

Baseline mutants per-package (`cargo-mutants 27.1.0 --locked`) wajib kill-rate ≥70% dari mutants viable (unviable dikecualikan; ekuivalen wajib didokumentasikan via call-site); baseline Fase 2: 492/504 = 97.6%.
