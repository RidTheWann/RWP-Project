from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]
adr_index = (ROOT / "docs/adr/README.md").read_text(encoding="utf-8")
missing = []
for number in re.findall(r"ADR-(\d{4})", adr_index):
    if not list((ROOT / "docs/adr").glob(f"{number}-*.md")):
        missing.append(number)

if missing:
    print("Missing ADR files:", ", ".join(f"ADR-{n}" for n in sorted(set(missing))))
    sys.exit(1)

readme = (ROOT / "README.md").read_text(encoding="utf-8")
if re.search(r"^## Current milestone status$", readme, re.MULTILINE):
    print("README must not duplicate milestone status; docs/project-ledger.md is the status source of truth.")
    sys.exit(1)

changelog = (ROOT / "CHANGELOG.md").read_text(encoding="utf-8")
if "## Unreleased" not in changelog:
    print("CHANGELOG.md must contain an Unreleased section.")
    sys.exit(1)

print("Documentation consistency checks passed.")
