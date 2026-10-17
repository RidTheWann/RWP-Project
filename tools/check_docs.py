from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[1]


def workspace_version(root):
    """Return the [workspace.package] version from the root Cargo.toml."""
    text = (root / "Cargo.toml").read_text(encoding="utf-8")
    match = re.search(
        r"(?m)^\[workspace\.package\]\s*\r?\nversion\s*=\s*\"([^\"]+)\"",
        text,
    )
    if match:
        return match.group(1)
    return None


def check_adrs(root):
    adr_index = (root / "docs/adr/README.md").read_text(encoding="utf-8")
    missing = []
    for number in re.findall(r"ADR-(\d{4})", adr_index):
        if not list((root / "docs/adr").glob(f"{number}-*.md")):
            missing.append(number)
    if missing:
        return [
            "Missing ADR files: " + ", ".join(f"ADR-{n}" for n in sorted(set(missing)))
        ]
    return []


def check_readme(root):
    readme = (root / "README.md").read_text(encoding="utf-8")
    if re.search(r"^## Current milestone status$", readme, re.MULTILINE):
        return [
            "README must not duplicate milestone status; "
            "docs/project-ledger.md is the status source of truth."
        ]
    return []


def check_changelog(root):
    changelog = (root / "CHANGELOG.md").read_text(encoding="utf-8")
    if "## Unreleased" not in changelog:
        return ["CHANGELOG.md must contain an Unreleased section."]
    return []


def check_versions(root):
    """Pin ledger, cli-reference and README to the workspace version."""
    failures = []
    version = workspace_version(root)
    if version is None:
        return ["Cargo.toml has no parseable [workspace.package] version."]
    ledger = (root / "docs/project-ledger.md").read_text(encoding="utf-8")
    match = re.search(r"(?m)^- Workspace version:\s*([^\s(]+)", ledger)
    if match is None or match.group(1) != version:
        failures.append(
            "docs/project-ledger.md Workspace version "
            f"'{match.group(1) if match else 'missing'}' does not match "
            f"Cargo.toml workspace version '{version}'."
        )
    cli_reference = (root / "docs/cli-reference.md").read_text(encoding="utf-8")
    match = re.search(r"(?m)^# CLI Reference \(`rwp ([^`)]+)`\)", cli_reference)
    if match is None or match.group(1) != version:
        failures.append(
            "docs/cli-reference.md header version "
            f"'{match.group(1) if match else 'missing'}' does not match "
            f"Cargo.toml workspace version '{version}'."
        )
    readme = (root / "README.md").read_text(encoding="utf-8")
    match = re.search(r"Release scope \(`([^`]+)`", readme)
    if match is None or match.group(1) != version:
        failures.append(
            "README.md release-scope version "
            f"'{match.group(1) if match else 'missing'}' does not match "
            f"Cargo.toml workspace version '{version}'."
        )
    return failures


def main(root=ROOT):
    failures = []
    failures.extend(check_adrs(root))
    failures.extend(check_readme(root))
    failures.extend(check_changelog(root))
    failures.extend(check_versions(root))
    for failure in failures:
        print(failure)
    if failures:
        return 1
    print("Documentation consistency checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
