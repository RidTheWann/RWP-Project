#!/usr/bin/env python3
"""Unit tests for tools/check_docs.py version drift detection.

The checker pins docs/project-ledger.md, docs/cli-reference.md and README.md
to the Cargo.toml workspace version so stale release snapshots fail the gate
instead of drifting silently.
"""

import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__)))

import check_docs


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def make_tree(root, version="0.5.0-beta"):
    write(
        root / "Cargo.toml",
        '[workspace]\n[workspace.package]\nversion = "%s"\n' % version,
    )
    write(root / "docs" / "adr" / "README.md", "# ADR index\n")
    write(
        root / "README.md",
        "# RWP\n\n> Release scope (`%s` candidate): notes.\n" % version,
    )
    write(root / "CHANGELOG.md", "# Changelog\n\n## Unreleased\n")
    write(
        root / "docs" / "project-ledger.md",
        "# Ledger\n\n- Workspace version: %s (candidate)\n" % version,
    )
    write(root / "docs" / "cli-reference.md", "# CLI Reference (`rwp %s`)\n" % version)


class CheckDocsVersionTests(unittest.TestCase):
    def test_stale_ledger_version_is_detected(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            make_tree(root)
            write(
                root / "docs" / "project-ledger.md",
                "# Ledger\n\n- Workspace version: 0.4.0 (post-alpha)\n",
            )
            failures = check_docs.check_versions(root)
            self.assertTrue(
                any("project-ledger" in failure for failure in failures),
                f"stale ledger version not reported: {failures}",
            )

    def test_stale_cli_reference_header_is_detected(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            make_tree(root)
            write(
                root / "docs" / "cli-reference.md",
                "# CLI Reference (`rwp 0.4.0`)\n",
            )
            failures = check_docs.check_versions(root)
            self.assertTrue(
                any("cli-reference" in failure for failure in failures),
                f"stale cli-reference header not reported: {failures}",
            )

    def test_stale_readme_scope_is_detected(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            make_tree(root)
            write(
                root / "README.md",
                "# RWP\n\n> Release scope (`0.4.0` candidate): notes.\n",
            )
            failures = check_docs.check_versions(root)
            self.assertTrue(
                any("README" in failure for failure in failures),
                f"stale README scope not reported: {failures}",
            )

    def test_matching_versions_pass(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            make_tree(root)
            self.assertEqual(check_docs.check_versions(root), [])


if __name__ == "__main__":
    unittest.main()
