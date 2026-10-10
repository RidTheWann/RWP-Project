# Local Gates — Otoritas Verifikasi Pengganti CI (Fase 0)

Status: MENGIKAT sejak Fase 0. CI GitHub mati bersama remote (`.github/` dihapus);
gate lokal di bawah adalah satu-satunya otoritas verifikasi sampai repo baru
(Fase 8) memiliki baseline CI hijau sendiri.

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

## 3. Aturan bukti (mengikat)

- Tanpa klaim "lolos" tanpa output perintah. Setiap batch menempel ringkasan
  hasil per perintah (pass/fail + angka: jumlah tes, crate, warning) ke
  `docs/autopilot/STATE.md`.
- Klaim lama bertanda "CI-verified" / run-ID / PR GitHub dinyatakan
  tak-terverifikasi-ulang; penggantinya adalah baris "locally-verified" dengan
  output gate di STATE.
- Tree harus bersih (`git status --short --branch` kosong selain `## main`)
  sebelum gate dijalankan.

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
