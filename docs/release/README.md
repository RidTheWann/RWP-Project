# Local Release Artifacts (Fase 6)

Unsigned alpha artifacts. Binaries are never committed; the text artifacts
below are generated locally and hashed into `provenance.json`.

## Artifact set (generated 2026-10-12, pinned toolchain 1.99.0)

- SBOM: 10 per-crate CycloneDX JSON files (`cargo-cyclonedx 0.5.9 --locked`),
  517 components total:
  analysis 10, capture 33, cli 66, core 9, diagnostics 11, report 16,
  session 50, system 32, ui 17, ui-native 273.
- Licenses: `licenses.json` (`cargo-about 0.9.2 --locked --features cli`
  with `about.toml`): 104 crates — MIT 58, Apache-2.0 37, MPL-2.0 3,
  BSD-3-Clause 2, Unicode-3.0 2, ISC 1, Zlib 1. No new license families
  beyond the `deny.toml` policy (plus the SPDX LLVM exception used by
  `target-lexicon`, reporting-only).
- Provenance: `provenance.json` (`rwp.release.provenance.v1`) pins the
  toolchain, `Cargo.lock` SHA-256
  (`75295E6021D9D2AAF96EA9D61032CF492A563914FD6E6C0E410CCEA4ABC01085`),
  source file count (166), SBOM/license hashes, and the unsigned status.
- Installer: NSIS bundle built locally via the Tauri CLI; the `.exe` is not
  committed. Install/uninstall test results are recorded in STATE; the
  SmartScreen expectation (unsigned warning) is documented in
  `docs/release-packaging.md`.

## Regeneration

```powershell
cargo install cargo-cyclonedx --locked          # 0.5.9
cargo install cargo-about --locked --features cli  # 0.9.2
cargo cyclonedx --format json
cargo about generate --format json -o licenses.json
```

SBOM output embeds absolute local paths, so generated files stay out of the
tree; only this index and `about.toml` are committed.
