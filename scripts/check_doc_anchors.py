#!/usr/bin/env python3
"""Living-docs anchor drift detector (ADR-0007).

Reads anchor strings from ``anchors.txt`` (one per line) and verifies that
each anchor appears verbatim in every checked documentation file:

- ``.github/copilot-instructions.md``
- ``.claude/CLAUDE.md``
- ``docs/design.md``

Exits with status 0 when every anchor is found in every file.
Exits with status 1 and prints a diagnostic when any anchor is missing,
naming the anchor and the files where it is absent.

Run from the repository root::

    python scripts/check_doc_anchors.py

Or via just::

    just docs-sync
"""

from __future__ import annotations

import sys
from pathlib import Path


ANCHORS_FILE = Path("anchors.txt")
CHECKED_FILES: list[Path] = [
    Path(".github/copilot-instructions.md"),
    Path(".claude/CLAUDE.md"),
    Path("docs/design.md"),
]


def load_anchors(path: Path) -> list[str]:
    """Return non-empty, non-comment lines from *path*."""
    lines = []
    for raw in path.read_text(encoding="utf-8").splitlines():
        stripped = raw.strip()
        if stripped and not stripped.startswith("#"):
            lines.append(stripped)
    return lines


def load_text(path: Path) -> str:
    """Return the full text of *path*."""
    return path.read_text(encoding="utf-8")


def main() -> int:
    # Verify required files exist before doing any checks.
    missing_inputs: list[str] = []
    if not ANCHORS_FILE.is_file():
        missing_inputs.append(str(ANCHORS_FILE))
    for p in CHECKED_FILES:
        if not p.is_file():
            missing_inputs.append(str(p))
    if missing_inputs:
        for name in missing_inputs:
            print(f"error: required file not found: {name}", file=sys.stderr)
        return 1

    anchors = load_anchors(ANCHORS_FILE)
    if not anchors:
        print("error: anchors.txt is empty or contains only comments", file=sys.stderr)
        return 1

    texts: dict[Path, str] = {p: load_text(p) for p in CHECKED_FILES}

    failures: list[tuple[str, list[Path]]] = []
    for anchor in anchors:
        absent = [p for p, text in texts.items() if anchor not in text]
        if absent:
            failures.append((anchor, absent))

    if not failures:
        print(f"docs-sync: all {len(anchors)} anchor(s) found in all checked files.")
        return 0

    print("docs-sync: anchor drift detected — the following anchors are missing:\n")
    for anchor, absent_files in failures:
        print(f"  anchor: {anchor!r}")
        for p in absent_files:
            print(f"    missing from: {p}")
    print(
        f"\n{len(failures)} anchor(s) out of {len(anchors)} failed. "
        "Update the missing files to include the anchor text, "
        "or update anchors.txt if the convention wording changed."
    )
    return 1


if __name__ == "__main__":
    sys.exit(main())
