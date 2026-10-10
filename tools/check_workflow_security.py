#!/usr/bin/env python3
"""Check that CI shell scripts do not interpolate untrusted branch refs directly."""

from __future__ import annotations

import re
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
WORKFLOW_PATH = ROOT / ".github" / "workflows" / "ci.yml"
UNSAFE_GITHUB_REF = re.compile(
    r"\$\{\{\s*github\.(?:head_ref|base_ref|ref_name)\s*\}\}"
)


def shell_scripts(workflow: str) -> list[str]:
    """Extract inline and block-form run scripts from the workflow text."""
    lines = workflow.splitlines()
    scripts: list[str] = []
    index = 0
    while index < len(lines):
        match = re.match(r"^([ \t]*)run:\s*(.*)$", lines[index])
        if match is None:
            index += 1
            continue

        indentation = len(match.group(1))
        value = match.group(2)
        if value.lstrip().startswith("|"):
            block: list[str] = []
            following = index + 1
            while following < len(lines):
                line = lines[following]
                if line.strip():
                    line_indentation = len(line) - len(line.lstrip(" \t"))
                    if line_indentation <= indentation:
                        break
                block.append(line)
                following += 1
            scripts.append("\n".join(block))
            index = following
        else:
            scripts.append(value)
            index += 1
    return scripts


def check_workflow(workflow: str) -> list[str]:
    """Return deterministic security-policy violations for the CI workflow."""
    violations: list[str] = []
    for index, script in enumerate(shell_scripts(workflow), start=1):
        if UNSAFE_GITHUB_REF.search(script):
            violations.append(
                f"run script {index} interpolates github.head_ref/base_ref/ref_name directly"
            )

    for variable, context in (
        ("HEAD_REF", "head_ref"),
        ("BASE_REF", "base_ref"),
        ("REF_NAME", "ref_name"),
    ):
        mapping = re.compile(
            r"(?m)^\s*" + re.escape(variable) + r":\s*\$\{\{\s*github\." + re.escape(context) + r"\s*\}\}\s*$"
        )
        if mapping.search(workflow) is None:
            violations.append(f"workflow env must map {variable} from github.{context}")

    permissions = re.search(
        r"(?m)^permissions:\s*\n((?:^[ \t]+[^\n]*(?:\n|$))*)",
        workflow,
    )
    if permissions is None:
        violations.append("workflow must declare explicit GITHUB_TOKEN permissions")
    else:
        permission_block = permissions.group(1)
        if re.search(r"(?m)^\s*contents:\s*read\s*$", permission_block) is None:
            violations.append("workflow must retain contents: read for checkout")
        if re.search(r"(?m)^\s*actions:\s*write\s*$", permission_block):
            violations.append("workflow must not grant actions: write without an API use")
        if re.search(r"(?m)^\s*pull-requests:\s*read\s*$", permission_block):
            violations.append("workflow must not grant pull-requests: read without an API use")

    if re.search(r"(?m)^\s*fetch-depth:\s*0\s*$", workflow):
        violations.append("workflow must not fetch full history when the gate fetches the required PR SHA")

    return violations


def main() -> int:
    if not WORKFLOW_PATH.is_file():
        print("CI workflow security check skipped: .github/workflows/ci.yml is absent (local-only mode).")
        return 0
    workflow = WORKFLOW_PATH.read_text(encoding="utf-8")
    violations = check_workflow(workflow)
    if violations:
        print("CI workflow security check failed:", file=sys.stderr)
        for violation in violations:
            print(f"- {violation}", file=sys.stderr)
        return 1
    print("CI workflow security checks passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
