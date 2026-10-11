# ADR-0028 — PresentMon distribution and verification

- Status: accepted
- Date: 2026-10-12
- Scope: Fase 5 / RC-08 / RC-09

## Context

RWP captures through an external PresentMon executable. The threat model
requires a canonical regular file named `PresentMon.exe`, a supported version
from the version matrix, and containment in a kill-on-close Job Object. The
executable's SHA-256 is recorded in session provenance. Open questions were
whether RWP should bundle PresentMon and whether unsigned binaries should be
refused.

## Decision

1. PresentMon stays user-supplied for v1.0. RWP never bundles or
   redistributes the executable; the operator provides it and `rwp doctor`
   validates it.
2. Verification layers, in order: basename plus regular-file validation,
   version-matrix support check, SHA-256 recorded into provenance, and an
   offline Authenticode check (`rwp_system::verify_presentmon_authenticode`,
   revocation checks disabled) reported as advisory doctor output.
3. An unsigned executable does not fail preflight. Signature enforcement is
   deferred to the signing phase: refusing unsigned binaries today would break
   self-built PresentMon with no compensating trust anchor, and hash
   allow-listing needs an owner-curated list that does not exist yet.

## Consequences

- Provenance records what ran (version plus SHA-256) without claiming the
  binary is authentic; authenticity stays an explicit residual risk.
- `presentmon.signature` in doctor output is `signed`, `unsigned`, or
  `verify-failed`; only `verify-failed` fails preflight (fail-closed on I/O).
- Bundling or enforcement changes require revisiting this record.
