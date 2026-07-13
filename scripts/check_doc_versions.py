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
    """Returns `[package].version` from the library crate's Cargo.toml.

    Reads the key from its table rather than grabbing the first `version = ...`
    line in the file: `[dependencies]` entries and any future table could also
    carry a `version` key, and matching the wrong one would make this check
    silently wrong in either direction.
    """
    text = CARGO_TOML.read_text(encoding="utf-8")

    try:
        import tomllib  # Python 3.11+
    except ImportError:
        pass
    else:
        version = tomllib.loads(text).get("package", {}).get("version")
        if isinstance(version, str):
            return version
        sys.exit(f"error: no [package].version found in {CARGO_TOML}")

    # Fallback for Python < 3.11: a table-aware scan, so a `version` key in any
    # other table cannot be mistaken for the package's own.
    table = None
    for line in text.splitlines():
        stripped = line.strip()
        if stripped.startswith("[") and stripped.endswith("]"):
            table = stripped[1:-1].strip()
            continue
        if table == "package":
            match = re.match(r'^version\s*=\s*"([^"]+)"', stripped)
            if match:
                return match.group(1)
    sys.exit(f"error: no [package].version found in {CARGO_TOML}")


def expected_pin(version: str) -> str:
    """Returns the caret requirement a reader should write for `version`.

    Cargo's caret semantics narrow as the version approaches zero, and each tier
    needs a different pin for the requirement to actually resolve:

    - ``1.2.3`` -> ``1``      (``^1`` is ``>=1.2.3, <2.0.0``; the major suffices)
    - ``0.3.0`` -> ``0.3``    (``^0.3`` is ``>=0.3.0, <0.4.0``; minor-pinned)
    - ``0.0.5`` -> ``0.0.5``  (``^0.0.5`` is ``>=0.0.5, <0.0.6``; patch-pinned --
      the full version is required, since ``0.0`` denotes a different range)
    """
    parts = version.split(".")
    major = parts[0]
    minor = parts[1] if len(parts) > 1 else "0"
    patch = parts[2] if len(parts) > 2 else "0"

    if major != "0":
        return major
    if minor != "0":
        return f"0.{minor}"
    return f"0.0.{patch}"


def main() -> int:
    version = crate_version()
    want = expected_pin(version)

    problems: list[str] = []
    checked = 0

    for path in CHECKED_FILES:
        if not path.exists():
            problems.append(f"{path}: checked file is missing")
            continue
        # Scan the whole file, not line by line: a snippet split across lines
        # (`crustywad = {\n  version = "0.3",\n  ...\n}`) is valid TOML and just
        # as copy-pastable, and a per-line scan would miss it entirely — letting
        # drift slip through simply by reformatting.
        text = path.read_text(encoding="utf-8")
        for match in PIN_RE.finditer(text):
            found = match.group("bare") or match.group("table")
            lineno = text.count("\n", 0, match.start()) + 1
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
