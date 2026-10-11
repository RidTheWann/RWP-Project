# Local Gates — Otoritas Verifikasi Pengganti CI (Fase 0)

Status: MENGIKAT sejak Fase 0. CI GitHub mati bersama remote (`.github/` dihapus);
gate lokal di bawah adalah satu-satunya otoritas verifikasi sampai repo baru
(Fase 8) memiliki baseline CI hijau sendiri.

> Catatan 2026-10-10 (kebijakan owner, berlaku segera): git tidak dipakai
> untuk apa pun lagi — tanpa log/status/diff/commit/tag/branch. Frasa
> "tree bersih (`git status ...`)" di §3 dan `repo_ref` di header STATE
> dinyatakan USANG; verifikasi berjalan via baca file langsung + output gate
> yang ditempel di STATE. Revisi penuh dokumen ini menyusul tanpa mengganggu
> kerja alpha; otoritas gate dan urutan perintah tak berubah.

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
- Tree harus bersih sebelum gate dijalankan — USANG sejak 2026-10-10: klausa
  `git status` dihapus (git tak dipakai); penggantinya: batch melaporkan
  daftar file yang diubah/dibuat, diverifikasi via baca file langsung.

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

Tetap `0.1.0` selama alpha (SemVer formal + Keep-a-Changelog di Fase 6).

## 6. Gate mutants (diadopsi owner 2026-10-10)

Baseline mutants per-package (`cargo-mutants 27.1.0 --locked`) wajib kill-rate ≥70% dari mutants viable (unviable dikecualikan; ekuivalen wajib didokumentasikan via call-site); baseline Fase 2: 492/504 = 97.6%.
