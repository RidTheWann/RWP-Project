#!/usr/bin/env python3
"""Unit tests for tools/check_links.py using synthetic document trees."""

import os
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, os.path.join(os.path.dirname(__file__)))

import check_links


def write(path, text):
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


class CheckLinksTests(unittest.TestCase):
    def test_collect_documents_finds_tracked_docs(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            write(root / "docs" / "a.md", "# A\n\nSee [b](b.md).\n")
            write(root / "docs" / "b.md", "# B\n")
            found = check_links.collect_documents(root)
            self.assertEqual(
                sorted(path.relative_to(root).as_posix() for path in found),
                ["docs/a.md", "docs/b.md"],
            )

    def test_ignored_output_dirs_are_skipped(self):
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw)
            write(root / "docs" / "a.md", "# A\n")
            write(root / "smoke-out" / "summary.md", "# Summary\n")
            write(root / "node_modules" / "note.md", "# Note\n")
            write(root / "pkg" / "__pycache__" / "note.md", "# Note\n")
            found = check_links.collect_documents(root)
            self.assertEqual(
                sorted(path.relative_to(root).as_posix() for path in found),
                ["docs/a.md"],
            )

    def test_broken_link_and_bad_anchor_detected(self):
        # resolve() keeps temp paths free of Windows 8.3 short names so the
        # checker's repo-containment comparison sees one consistent spelling.
        with tempfile.TemporaryDirectory() as raw:
            root = Path(raw).resolve()
            previous = check_links.ROOT
            check_links.ROOT = root
            self.addCleanup(setattr, check_links, "ROOT", previous)
            write(root / "docs" / "ok.md", "# Ok\n\nSee [other](other.md#ok).\n")
            write(root / "docs" / "other.md", "# Ok\n")
            failures, _ = check_links.check(root / "docs" / "ok.md")
            self.assertEqual(failures, [])
            write(root / "docs" / "bad.md", "# Bad\n\nSee [gone](gone.md).\n")
            failures, _ = check_links.check(root / "docs" / "bad.md")
            self.assertTrue(
                any("missing target" in failure for failure in failures),
                f"broken link not reported: {failures}",
            )
            write(
                root / "docs" / "anchor.md",
                "# Title\n\nSee [nope](#no-such-heading).\n",
            )
            failures, _ = check_links.check(root / "docs" / "anchor.md")
            self.assertTrue(
                any("missing anchor" in failure for failure in failures),
                f"bad anchor not reported: {failures}",
            )


if __name__ == "__main__":
    unittest.main()
