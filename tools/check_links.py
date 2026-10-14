#!/usr/bin/env python3
"""Check Markdown links without network access (local-first replacement for CI link checks).

Rules:
- Relative file links (`docs/foo.md`, `../bar.md`) must resolve to an existing
  file or directory, relative to the linking document.
- Same-document `#anchor` links must match a heading in that document
  (GitHub-style slugs).
- `http(s)` links are listed, never fetched: offline runs cannot verify them.

Usage:
    python tools/check_links.py

Exits 0 when every local link resolves, 1 otherwise.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LINK = re.compile(r"(?<!!)\[([^\]]*)\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
FENCE = re.compile(r"^\s*```")


def slug(text):
    """GitHub-style heading slug."""
    text = text.strip().lower()
    text = re.sub(r"[^\w\s-]", "", text, flags=re.UNICODE)
    return re.sub(r"\s+", "-", text)


def headings(document):
    """Collect heading slugs, ignoring fenced code blocks."""
    slugs = set()
    fenced = False
    for line in document.read_text(encoding="utf-8").splitlines():
        if FENCE.match(line):
            fenced = not fenced
        if fenced:
            continue
        match = re.match(r"#{1,6}\s+(.*)", line)
        if match:
            slugs.add(slug(match.group(1)))
    return slugs


def check(document):
    """Return (failures, external_count) for one Markdown file."""
    failures = []
    external = 0
    anchors = headings(document)
    fenced = False
    for number, line in enumerate(document.read_text(encoding="utf-8").splitlines(), 1):
        if FENCE.match(line):
            fenced = not fenced
        if fenced:
            continue
        for _, target in LINK.findall(line):
            if target.startswith(("http://", "https://", "mailto:")):
                external += 1
                continue
            path, _, anchor = target.partition("#")
            if path in ("",):
                if anchor and anchor not in anchors:
                    failures.append(f"{document}:{number}: missing anchor #{anchor}")
                continue
            resolved = (document.parent / path).resolve()
            try:
                resolved.relative_to(ROOT)
            except ValueError:
                failures.append(f"{document}:{number}: link escapes repo: {target}")
                continue
            if not resolved.exists():
                failures.append(f"{document}:{number}: missing target: {target}")
                continue
            if anchor and resolved.suffix == ".md":
                if anchor not in headings(resolved):
                    failures.append(f"{document}:{number}: missing anchor #{anchor} in {target}")
    return failures, external


#: Directory names never scanned for documents. Besides VCS and build
#: output, this covers git-ignored local output trees (owner smoke-test
#: results), dependency directories, and bytecode caches, so the document
#: count stays deterministic and no git access is needed.
SKIP_DIRS = frozenset({"target", ".git", "smoke-out", "node_modules", "__pycache__"})


def collect_documents(root):
    """All Markdown files under root, minus skipped trees, sorted."""
    return sorted(
        path for path in root.rglob("*.md") if SKIP_DIRS.isdisjoint(path.parts)
    )


def main():
    documents = collect_documents(ROOT)
    failures = []
    external = 0
    for document in documents:
        document_failures, document_external = check(document)
        failures.extend(document_failures)
        external += document_external
    for failure in failures:
        print(failure)
    print(f"checked {len(documents)} documents, {external} external links listed (not fetched)")
    if failures:
        return 1
    print("Markdown link checks passed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
