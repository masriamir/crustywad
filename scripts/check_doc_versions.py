#!/usr/bin/env python3
"""Documentation version-pin drift detector.

The README and the mdBook guide show ``Cargo.toml`` snippets telling readers
which version of ``crustywad`` to depend on::

    crustywad = "0.3"
    crustywad = { version = "0.3", features = ["write"] }

Cargo treats a bare version as a caret requirement, and for a ``0.x`` crate a
caret is *minor*-pinned: ``"0.1"`` means ``>=0.1.0, <0.2.0``. So once the crate
released ``0.2.0``, every snippet still saying ``"0.1"`` stopped resolving --
readers copy-pasting them got a version that cannot be fetched. That is exactly
what happened (issue #235): the pins silently rotted through a minor bump and
went unnoticed until a reviewer spotted them in an unrelated PR.

This check makes that drift loud. It reads the real version from
``crates/crustywad/Cargo.toml``, derives the requirement a reader should write,
and fails if any documented pin disagrees.

**When a release bumps the minor version, this check fails until the docs are
updated.** That is deliberate: updating the pins is part of shipping a minor
release, and the release PR is where it should happen.

Run from the repository root::

    python3 scripts/check_doc_versions.py

Or via just::

    just docs-sync
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

CARGO_TOML = Path("crates/crustywad/Cargo.toml")

# Files that document a dependency version for library consumers.
CHECKED_FILES = [
    Path("README.md"),
    Path("docs/guide/src/getting-started.md"),
    Path("docs/guide/src/features.md"),
    Path("docs/guide/src/writing-wads.md"),
    Path("docs/guide/src/converting-maps.md"),
]

# Matches both snippet forms, capturing the version string:
#   crustywad = "0.3"
#   crustywad = { version = "0.3", features = ["write"] }
# The negative lookahead on the name keeps `crustywad-cli = ...` from matching.
PIN_RE = re.compile(
    r'crustywad(?!-)\s*=\s*(?:"(?P<bare>[^"]+)"|\{[^}]*?version\s*=\s*"(?P<table>[^"]+)")'
)


def crate_version() -> str:
    """Returns the `version` from the library crate's Cargo.toml."""
    for line in CARGO_TOML.read_text(encoding="utf-8").splitlines():
        match = re.match(r'^version\s*=\s*"([^"]+)"', line.strip())
        if match:
            return match.group(1)
    sys.exit(f"error: no version found in {CARGO_TOML}")


def expected_pin(version: str) -> str:
    """Returns the caret requirement a reader should write for `version`.

    Cargo's caret semantics differ either side of 1.0: for `0.x` the compatible
    range is minor-pinned, so a reader must write `0.3` (not `0`); from 1.0 the
    major alone is enough.
    """
    parts = version.split(".")
    major = parts[0]
    minor = parts[1] if len(parts) > 1 else "0"
    return major if major != "0" else f"0.{minor}"


def main() -> int:
    version = crate_version()
    want = expected_pin(version)

    problems: list[str] = []
    checked = 0

    for path in CHECKED_FILES:
        if not path.exists():
            problems.append(f"{path}: checked file is missing")
            continue
        for lineno, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for match in PIN_RE.finditer(line):
                found = match.group("bare") or match.group("table")
                checked += 1
                if found != want:
                    problems.append(
                        f"{path}:{lineno}: pins crustywad = {found!r}, "
                        f"but the crate is {version} (expected {want!r})"
                    )

    if problems:
        print("docs-versions: version-pin drift detected\n", file=sys.stderr)
        for problem in problems:
            print(f"  {problem}", file=sys.stderr)
        print(
            f"\nThe crate is at {version}, so every documented dependency snippet "
            f'should read crustywad = "{want}".\n'
            "For a 0.x crate a caret requirement is minor-pinned, so a stale pin "
            "does not resolve at all for readers.\n"
            "If a release just bumped the minor version, update the pins in the "
            "release PR.",
            file=sys.stderr,
        )
        return 1

    print(f"docs-versions: all {checked} documented pin(s) match crustywad {version}.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
