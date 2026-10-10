#!/usr/bin/env python3
"""Enforce per-crate line-coverage thresholds from a cargo-llvm-cov JSON report.

Usage:
    python tools/check_coverage.py coverage.json [--min-lines 85]

Scope (RC-03):
    rwp-analysis, rwp-diagnostics, rwp-report: whole crate.
    rwp-capture: parser scope only (src/csv.rs and src/command.rs); the
        OS process-runner paths that need a live PresentMon executable are
        covered by dedicated runner tests where possible, while Windows-only
        blocks are compiled out on the Linux coverage job.

Exits 0 when every target meets the threshold, 1 otherwise, printing the
actual percentage per target so failures name the crate and its number.
"""

import json
import sys

DEFAULT_MIN_LINES = 85.0

# Crate -> filename suffixes within the crate that count toward the gate.
# None means the whole crate.
TARGETS = {
    "rwp-analysis": None,
    "rwp-diagnostics": None,
    "rwp-report": None,
    "rwp-capture": ("src/csv.rs", "src/command.rs"),
}


def crate_coverage(report, crate, suffixes):
    """Return (covered, total) line counts for one gate target."""
    covered = 0
    total = 0
    matched = 0
    for entry in report["data"][0]["files"]:
        filename = entry["filename"].replace("\\", "/")
        if "/crates/" + crate + "/" not in filename:
            continue
        if suffixes is not None and not filename.endswith(suffixes):
            continue
        matched += 1
        lines = entry["summary"]["lines"]
        covered += lines["covered"]
        total += lines["count"]
    if matched == 0:
        raise ValueError(f"no files matched gate target {crate}")
    if total == 0:
        raise ValueError(f"gate target {crate} has no coverable lines")
    return covered, total


def check(path, minimum):
    with open(path, encoding="utf-8") as handle:
        report = json.load(handle)
    failures = []
    for crate, suffixes in TARGETS.items():
        covered, total = crate_coverage(report, crate, suffixes)
        percent = 100.0 * covered / total
        status = "PASS" if percent + 1e-9 >= minimum else "FAIL"
        print(f"{status} {crate}: {covered}/{total} lines = {percent:.1f}% (min {minimum:.1f}%)")
        if status == "FAIL":
            failures.append(crate)
    return failures


def main(argv):
    path = None
    minimum = DEFAULT_MIN_LINES
    args = list(argv)
    while args:
        arg = args.pop(0)
        if arg == "--min-lines" and args:
            minimum = float(args.pop(0))
        elif path is None:
            path = arg
        else:
            print(f"unexpected argument: {arg}", file=sys.stderr)
            return 2
    if path is None:
        print("usage: python tools/check_coverage.py coverage.json [--min-lines 85]", file=sys.stderr)
        return 2
    try:
        failures = check(path, minimum)
    except (OSError, ValueError, KeyError) as error:
        print(f"coverage gate error: {error}", file=sys.stderr)
        return 2
    if failures:
        print(f"coverage below threshold for: {', '.join(failures)}", file=sys.stderr)
        return 1
    print("Per-crate line coverage thresholds passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
