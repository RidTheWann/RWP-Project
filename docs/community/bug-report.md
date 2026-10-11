# Bug report template

Copy into a new issue and fill every section. Reports without reproduction
evidence are closed as incomplete.

## Summary

One sentence: what breaks, on which command or screen.

## Environment (exact)

- RWP version (`rwp --version`) and build profile:
- Windows build (`winver`) and machine class (CPU/GPU/RAM):
- PresentMon version + SHA-256 (if capture-related):

## Reproduction

1. Exact commands with flags, or exact GUI steps:
2. Input involved (fixture CSV? live capture? settings?):
3..redacted output attached (report JSON redacted, doctor bundle, or log
   excerpt). Never attach unredacted bundles or private paths.

## Observed vs expected

- Observed (with the exact error code, e.g. `RWP-E2200`):
- Expected (with reference to `docs/` behavior contracts):

## Checklist

- [ ] I searched existing issues for duplicates.
- [ ] Attached output is redacted (`docs/privacy.md`).
- [ ] This is not a hardware-acceptance request (those go to the acceptance matrix).
