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

def checked_files() -> list[Path]:
    """Returns the *reader-facing* docs whose pins must stay installable.

    Scope is the README, CONTRIBUTING, and the mdBook guide (recursively, so a
    future subdirectory cannot become a blind spot). Within that scope the files
    are *discovered*, never listed: a hardcoded list is itself a drift source --
    a new guide page carrying a pin would silently escape, which is the exact
    failure this script exists to prevent.

    **ADRs are deliberately out of scope.** `docs/adr/` records decisions as they
    stood when taken -- ADR-0011, for instance, quotes `version = "0.1.0"` --
    and rewriting those snippets to track the current release would falsify the
    historical record. They instruct nobody to install anything; these files do.
    """
    paths = [Path("README.md"), Path("CONTRIBUTING.md")]
    paths.extend(sorted(Path("docs/guide/src").rglob("*.md")))
    return [p for p in paths if p.exists()]

# Matches both snippet forms, capturing the version string:
#   crustywad = "0.3"
#   crustywad = { version = "0.3", features = ["write"] }
# The negative lookahead on the name keeps `crustywad-cli = ...` from matching.
PIN_RE = re.compile(
    r'crustywad(?!-)\s*=\s*(?:"(?P<bare>[^"]+)"|\{[^}]*?version\s*=\s*"(?P<table>[^"]+)")'
)

# A template placeholder, and only that: `X.Y.Z` / `x.y.z` (CONTRIBUTING.md uses
# one when showing the shape of the CLI's dependency entry). Deliberately narrow
# — `0.x` and `0.3-beta.1` are malformed pins, not placeholders, and must be
# reported rather than waved through.
PLACEHOLDER_RE = re.compile(r"[XYZ](?:\.[XYZ])*", re.IGNORECASE)


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
    """Returns the pin the docs should show for `version` -- the full `X.Y.Z`.

    This is what ``cargo add crustywad`` writes into a reader's ``Cargo.toml``
    (verified: it emits ``crustywad = "0.3.0"``, not ``"0.3"``), so it is the
    form a reader will actually end up with, and it states the minimum patch
    they need -- which matters as soon as a documented example relies on
    something added in a patch release.

    Shorter forms still *work* (``"0.3"`` resolves to 0.3.0), and
    :func:`pin_resolves` accepts any requirement that genuinely fetches the
    current version. This function only supplies the canonical spelling to
    suggest when a pin is wrong.
    """
    return version


def pin_resolves(pin: str, version: str) -> bool:
    """Whether the caret requirement `pin` would actually fetch `version`.

    A pin is judged by what Cargo does with it, not by string equality against
    one canonical spelling: `"0.3"` and `"0.3.0"` are both valid requirements
    that fetch 0.3.0 (the docs legitimately use both -- the short form in install
    snippets, the full form when illustrating the caret rules themselves).

    `^REQ` matches a version when the leading non-zero component agrees and the
    version is not below the requirement.
    """
    try:
        want = [int(part) for part in pin.split(".")]
        have = [int(part) for part in version.split(".")]
    except ValueError:
        return False

    want += [0] * (3 - len(want))
    have += [0] * (3 - len(have))

    # The caret's "compatible" component is the first non-zero one; every
    # component up to and including it must match exactly.
    lead = next((i for i, part in enumerate(want) if part != 0), 2)
    if want[: lead + 1] != have[: lead + 1]:
        return False
    # And the version must not predate the requirement (^0.3.1 does not fetch 0.3.0).
    return have >= want


def main() -> int:
    version = crate_version()
    want = expected_pin(version)

    problems: list[str] = []
    checked = 0

    for path in checked_files():
        # Scan the whole file, not line by line: a snippet split across lines
        # (`crustywad = {\n  version = "0.3",\n  ...\n}`) is valid TOML and just
        # as copy-pastable, and a per-line scan would miss it entirely — letting
        # drift slip through simply by reformatting.
        text = path.read_text(encoding="utf-8")
        for match in PIN_RE.finditer(text):
            found = match.group("bare") or match.group("table")
            lineno = text.count("\n", 0, match.start()) + 1

            # Skip deliberate placeholders (`version = "X.Y.Z"` in a template
            # snippet): they instruct nobody to install anything, so pinning them
            # to a real version would be wrong. The match must be a placeholder
            # *and nothing else* — a pin like `0.x` or `0.3-beta.1` is malformed,
            # not a template, and is still reported. Skipping on "contains a
            # letter" would let those through silently.
            if PLACEHOLDER_RE.fullmatch(found):
                continue

            checked += 1
            if not pin_resolves(found, version):
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
            f'should read crustywad = "{want}" (the full X.Y.Z, matching what '
            "`cargo add` writes).\n"
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
