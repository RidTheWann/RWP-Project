#!/usr/bin/env python3
"""Unit tests for tools/check_coverage.py using synthetic llvm-cov JSON."""

import io
import json
import os
import sys
import tempfile
import unittest

sys.path.insert(0, os.path.join(os.path.dirname(__file__)))

import check_coverage


def make_report(entries):
    files = []
    for filename, covered, count in entries:
        files.append(
            {
                "filename": filename,
                "summary": {"lines": {"covered": covered, "count": count}},
            }
        )
    return {"data": [{"files": files}]}


class CheckCoverageTests(unittest.TestCase):
    def run_gate(self, report, minimum=85.0):
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False, encoding="utf-8"
        ) as handle:
            json.dump(report, handle)
            path = handle.name
        try:
            captured = io.StringIO()
            old_stdout = sys.stdout
            sys.stdout = captured
            try:
                failures = check_coverage.check(path, minimum)
            finally:
                sys.stdout = old_stdout
            return failures, captured.getvalue()
        finally:
            os.remove(path)

    def test_passing_report_has_no_failures(self):
        report = make_report(
            [
                ("/repo/crates/rwp-analysis/src/lib.rs", 90, 100),
                ("/repo/crates/rwp-diagnostics/src/lib.rs", 85, 100),
                ("/repo/crates/rwp-report/src/lib.rs", 100, 100),
                ("/repo/crates/rwp-capture/src/csv.rs", 86, 100),
                ("/repo/crates/rwp-capture/src/command.rs", 90, 100),
            ]
        )
        failures, output = self.run_gate(report)
        self.assertEqual(failures, [])
        self.assertIn("PASS rwp-analysis", output)

    def test_failing_crate_is_named(self):
        report = make_report(
            [
                ("/repo/crates/rwp-analysis/src/lib.rs", 90, 100),
                ("/repo/crates/rwp-diagnostics/src/lib.rs", 50, 100),
                ("/repo/crates/rwp-report/src/lib.rs", 100, 100),
                ("/repo/crates/rwp-capture/src/csv.rs", 90, 100),
                ("/repo/crates/rwp-capture/src/command.rs", 90, 100),
            ]
        )
        failures, output = self.run_gate(report)
        self.assertEqual(failures, ["rwp-diagnostics"])
        self.assertIn("FAIL rwp-diagnostics", output)

    def test_capture_scope_ignores_runner_externals(self):
        # Files outside src/csv.rs and src/command.rs do not count.
        report = make_report(
            [
                ("/repo/crates/rwp-analysis/src/lib.rs", 90, 100),
                ("/repo/crates/rwp-diagnostics/src/lib.rs", 90, 100),
                ("/repo/crates/rwp-report/src/lib.rs", 90, 100),
                ("/repo/crates/rwp-capture/src/csv.rs", 90, 100),
                ("/repo/crates/rwp-capture/src/command.rs", 90, 100),
                ("/repo/crates/rwp-capture/src/error.rs", 0, 100),
            ]
        )
        failures, _ = self.run_gate(report)
        self.assertEqual(failures, [])

    def test_windows_paths_are_normalized(self):
        report = make_report(
            [
                ("C:\\repo\\crates\\rwp-analysis\\src\\lib.rs", 90, 100),
                ("C:\\repo\\crates\\rwp-diagnostics\\src\\lib.rs", 90, 100),
                ("C:\\repo\\crates\\rwp-report\\src\\lib.rs", 90, 100),
                ("C:\\repo\\crates\\rwp-capture\\src\\csv.rs", 90, 100),
                ("C:\\repo\\crates\\rwp-capture\\src\\command.rs", 90, 100),
            ]
        )
        failures, _ = self.run_gate(report)
        self.assertEqual(failures, [])

    def test_missing_target_is_an_error(self):
        report = make_report(
            [("/repo/crates/rwp-analysis/src/lib.rs", 90, 100)]
        )
        # Direct call surfaces the missing-target error deterministically.
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False, encoding="utf-8"
        ) as handle:
            json.dump(report, handle)
            path = handle.name
        try:
            with open(path, encoding="utf-8") as handle:
                loaded = json.load(handle)
            with self.assertRaises(ValueError):
                check_coverage.crate_coverage(loaded, "rwp-diagnostics", None)
        finally:
            os.remove(path)


if __name__ == "__main__":
    unittest.main()
