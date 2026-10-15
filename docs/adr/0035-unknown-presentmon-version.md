# ADR-0035 — Unknown PresentMon version (amends ADR-0016)

- **Status:** accepted
- **Date:** 2026-10-16
- **Scope:** WP2 / WP4 / AUD-14
- **Amends:** ADR-0016 (version matrix), complements ADR-0028 (verification layers)

## Context

ADR-0016 gates capture on the Windows file-version resource: major version
2.x is accepted, any other detected major is rejected before spawn. The
genuine signed PresentMon 2.6.0 binary observed on the owner machine
(980,320 bytes, SHA-256
`B2A706BC6AD475749E3B7E3409263AA1E6906D45BDCF993F6DBC0F660188F1AF`)
carries **no version resource at all**: `GetFileVersionInfoSizeW` fails with
`ERROR_RESOURCE_TYPE_NOT_FOUND` (1813), so `major_version` is null and
`start()` refuses every capture. The binary is Authenticode-signed, which
proves signature status and version-resource presence are independent
signals. PresentMon 2.6.0 exposes no `--version` flag (it prints
`error: unrecognized option '--version'`); the only in-band version signal
is the undocumented `PresentMon 2.6.0` stdout banner on `--help`/usage
output. Empirically, non-executable input fails the same call with 1812
(`ERROR_RESOURCE_DATA_NOT_FOUND`), not 1813.

## Decision

1. `ERROR_RESOURCE_TYPE_NOT_FOUND` (1813) from the version-resource APIs
   maps to a new `PresentMonVersionError::UnknownVersion` outcome:
   version-unknown, explicitly distinct from detected-but-unsupported.
   Every other failure code (including 1812 on non-executable input) keeps
   failing closed as a hard `Api` error.
2. Version-unknown executables are **allowed with warnings**, never silent:
   `start()` proceeds and emits a field-free `presentmon_version_unknown`
   tracing event (GUI allowlist); the session records a `TelemetryWarning`
   and the manifest keeps `presentmon_version: null` with the executable
   SHA-256; `rwp doctor` reports the `presentmon.version` check as advisory
   (`ok: null`) instead of failed.
3. The compensating control is unchanged: the v2 CSV parser still rejects
   non-v2 output (`MissingColumn` on `ProcessID`, `CPUStartQPC`,
   `FrameTime`, `CPUBusy`, `CPUWait`, `GPUTime`, `GPUBusy`, `GPUWait`),
   so a non-2.x binary that slips past the unknown path fails closed at
   parse time. Detected non-2.x versions are still refused before spawn,
   and the GUI remediation table now names that refusal explicitly.
4. Rejected alternatives: a SHA-256 allowlist (no owner-curated list
   exists yet per ADR-0028, and every PresentMon release would churn it)
   and stdout-banner parsing (undocumented format, requires spawning the
   executable, and ADR-0016 already refuses flag-based detection).

## Consequences

- `rwp doctor --presentmon` passes the version check as advisory for
  versionless binaries; live capture proceeds with provenance intact.
- Unknown and unsupported stay textually distinct (`version is unknown`
  vs `unsupported PresentMon version`) in doctor output, session
  warnings, and GUI remediation.
- ADR-0016, ADR-0028, and the threat-model verification row keep their
  wording as the accepted record; this file is the amendment authority
  for the unknown-version path.

## References

- ADR-0016 (version matrix), ADR-0028 (distribution and verification)
- Microsoft Win32 version-information API: <https://learn.microsoft.com/en-us/windows/win32/api/winver/>
