# Archived: v1 Schema Placeholder

This directory is archived, not active. The `rwp.report.v1` contract named by
the original placeholder is reader-supported by `rwp-report::from_json`, but
the current emitted and validated contracts are `rwp.report.v2` and
`rwp.session.manifest.v2` under `docs/schema/v2/` (validated against goldens
by `tools/validate_schemas.py`).

No generated v1 schema artifacts were ever committed here; only this pointer
remains so old references resolve to an explicit superseded notice instead of
a missing path. `rwp migrate` normalizes legacy v1 documents to v2 without
recomputing their evidence.
