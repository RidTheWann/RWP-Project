# Versioned JSON Schemas

WP4 defines two persisted JSON contracts:

- `rwp.report.v2`: deterministic analysis report.
- `rwp.session.manifest.v2`: deterministic persisted-session manifest and provenance.

The schemas are versioned artifacts and are validated against golden JSON fixtures by `tools/validate_schemas.py`.

V1 report readers remain supported by `rwp-report::from_json`; V2 is the current emitted report contract.
