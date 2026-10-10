from pathlib import Path
import json
import sys

try:
    from jsonschema import Draft202012Validator
except ImportError as exc:
    print(f"jsonschema is required: {exc}", file=sys.stderr)
    sys.exit(2)

ROOT = Path(__file__).resolve().parents[1]
checks = [
    ("docs/schema/v2/rwp.report.v2.schema.json", "tests/schema/report.v2.golden.json"),
    ("docs/schema/v2/rwp.session.manifest.v2.schema.json", "tests/schema/session.manifest.v2.golden.json"),
]

for schema_path, golden_path in checks:
    schema = json.loads((ROOT / schema_path).read_text(encoding="utf-8"))
    golden = json.loads((ROOT / golden_path).read_text(encoding="utf-8"))
    Draft202012Validator.check_schema(schema)
    errors = sorted(Draft202012Validator(schema).iter_errors(golden), key=lambda error: list(error.path))
    if errors:
        for error in errors:
            print(f"{schema_path}: {error.message}", file=sys.stderr)
        sys.exit(1)

print("JSON Schema validation passed for report v2 and session manifest v2.")
