#!/usr/bin/env python3
"""Check or apply shared-file sync from canonical meta repositories.

Reads `.meta-manifest.toml` in the current directory (the consuming repository's
root). Each `[[file]]` entry names a canonical source file and a local destination:

    [[file]]
    source = "masriamir/.github"          # owner/repo, or "file:/abs/dir" (tests)
    ref    = "<40-char commit sha>"       # pinned commit; ignored for file: sources
    path   = "templates/rust/lefthook.yml"
    dest   = "lefthook.yml"
    mode   = "file"                       # or "block"
    marker = "gitignore-base"             # block mode only

Modes:

  file   The destination must equal the canonical file byte-for-byte.
  block  The destination must contain the canonical content between marker lines
         `>>> meta:<marker>` and `<<< meta:<marker>` (usually behind a comment
         leader). The marker lines belong to the destination, not the canonical
         file, and must already exist — seed them once when wiring a repo up. The
         canonical body is inserted with a trailing newline so the closing marker
         always keeps its own line (a source without a final newline still
         converges), and is re-indented to the opening marker's own leading
         whitespace, so one nesting-neutral canonical fragment serves a
         destination at any depth.

Comparison is byte-for-byte: content is fetched, read, compared, and written as
raw bytes, with no newline translation, so a CRLF/LF divergence between a
destination and its canonical source is caught rather than silently normalized
away. Diffs are decoded (UTF-8, replacing undecodable bytes) only for display.

Commands:

  check  Exit 1 if any entry is missing or out of sync, printing a unified diff.
  sync   Rewrite each destination from its pinned source.

Bumping a pin is deliberate and manual: edit the entry's `ref`, run `sync`, commit
both. GitHub sources are fetched anonymously from raw.githubusercontent.com, so
canonical repositories must be public.
"""

import difflib
import re
import sys
import urllib.request
from pathlib import Path

try:
    import tomllib
except ModuleNotFoundError:  # Python < 3.11
    print("meta_sync: Python 3.11+ required (tomllib)", file=sys.stderr)
    sys.exit(2)

MANIFEST = Path(".meta-manifest.toml")

# Canonical content by (source, ref, path). One logical policy is often split
# across several entries — the same file synced into two markers, or into two
# destinations — and without this each one would repeat the round-trip for bytes
# already in hand. A GitHub source is keyed on its 40-char commit pin, so a hit
# there is always that immutable commit's content. A `file:` source carries no
# such guarantee, so for it this is only an intra-run read cache: the file could
# change underneath a run. Nothing is cached across runs — each invocation is a
# fresh process.
_FETCHED: dict[tuple[str, str, str], bytes] = {}


def entry_id(entry: dict) -> str:
    return f"{entry['source']}:{entry['path']} -> {entry['dest']}"


def fetch(entry: dict) -> bytes:
    src = entry["source"]
    key = (src, entry.get("ref", ""), entry["path"])
    if key in _FETCHED:
        return _FETCHED[key]
    try:
        if src.startswith("file:"):
            return _FETCHED.setdefault(
                key, (Path(src[len("file:"):]) / entry["path"]).read_bytes()
            )
        ref = entry["ref"]
        if not re.fullmatch(r"[0-9a-f]{40}", ref):
            raise SystemExit(
                f"meta_sync: {entry_id(entry)}: ref {ref!r} is not a 40-char commit "
                "sha — pin an exact commit, not a branch or tag"
            )
        url = f"https://raw.githubusercontent.com/{src}/{ref}/{entry['path']}"
        with urllib.request.urlopen(url, timeout=30) as response:
            return _FETCHED.setdefault(key, response.read())
    except SystemExit:
        raise
    except OSError as error:  # URLError/HTTPError/FileNotFoundError are all OSError.
        raise SystemExit(f"meta_sync: cannot fetch {entry_id(entry)}: {error}") from error


def find_block(lines: list[bytes], marker: str) -> tuple[int, int] | None:
    start, end = f">>> meta:{marker}".encode(), f"<<< meta:{marker}".encode()
    first = None
    for i, line in enumerate(lines):
        if first is None:
            if start in line:
                first = i
        elif end in line:
            return first, i
    return None


def block_body(canonical: bytes) -> bytes:
    """Canonical block content as inserted between markers.

    A non-empty body is given a trailing newline so that, on insertion, the
    closing marker stays on its own line. Without this, a canonical source
    lacking a final newline would put the marker on the body's last line, where
    `find_block` then folds it into the closing-marker line and excludes the body
    from the next comparison — `sync` and `check` would never converge. Applied
    identically on both the check and sync paths so the two always agree.
    """
    if canonical and not canonical.endswith(b"\n"):
        return canonical + b"\n"
    return canonical


def marker_indent(marker_line: bytes) -> bytes:
    """Leading whitespace of an opening marker line, as the block's indentation."""
    return marker_line[: len(marker_line) - len(marker_line.lstrip())]


def indent_body(body: bytes, prefix: bytes) -> bytes:
    """Canonical block content re-indented to its destination's nesting depth.

    Canonical fragments are stored nesting-neutral (no leading indentation), and
    the destination supplies the depth via its own opening marker line. Baking
    the depth into the canonical file instead would silently break any adopter
    whose file nests differently — for YAML, into an indentation error that
    neither `check` (byte-oriented, never parses) nor the consumer's own tooling
    would attribute to the sync.

    Blank lines are left alone rather than padded into trailing whitespace.
    Applied identically on the check and sync paths so the two always agree.
    """
    if not prefix:
        return body
    return b"".join(
        prefix + line if line.strip() else line
        for line in body.splitlines(keepends=True)
    )


def load_manifest() -> list[dict]:
    try:
        data = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    except (tomllib.TOMLDecodeError, OSError, UnicodeError) as error:
        # UnicodeError: a non-UTF-8 manifest should give this diagnostic, not a
        # traceback (UnicodeDecodeError is a ValueError, not an OSError).
        raise SystemExit(f"meta_sync: cannot parse {MANIFEST}: {error}") from error
    entries = data.get("file", [])
    if not isinstance(entries, list) or not all(isinstance(e, dict) for e in entries):
        raise SystemExit(f"meta_sync: {MANIFEST}: [[file]] must be an array of tables")
    for i, entry in enumerate(entries, 1):
        mode = entry.get("mode", "file")
        if mode not in ("file", "block"):
            raise SystemExit(
                f"meta_sync: {MANIFEST}: entry {i}: mode must be 'file' or 'block', not {mode!r}"
            )
        required = {"source", "path", "dest"}
        if mode == "block":
            required.add("marker")
        if not str(entry.get("source", "")).startswith("file:"):
            required.add("ref")
        bad = sorted(k for k in required if not isinstance(entry.get(k), str))
        if bad:
            raise SystemExit(
                f"meta_sync: {MANIFEST}: entry {i}: missing or non-string key(s): {', '.join(bad)}"
            )
    return entries


def check_entry(entry: dict, canonical: bytes) -> str | None:
    dest = Path(entry["dest"])
    if not dest.exists():
        return f"{entry_id(entry)}: destination missing"
    try:
        data = dest.read_bytes()
    except OSError as error:
        return f"{entry_id(entry)}: cannot read {dest}: {error}"
    if entry.get("mode", "file") == "file":
        actual, expected, label = data, canonical, str(dest)
    else:
        marker = entry["marker"]
        lines = data.splitlines(keepends=True)
        bounds = find_block(lines, marker)
        if bounds is None:
            return (
                f"{entry_id(entry)}: marker lines '>>> meta:{marker}' / "
                f"'<<< meta:{marker}' not found"
            )
        actual = b"".join(lines[bounds[0] + 1 : bounds[1]])
        expected = indent_body(block_body(canonical), marker_indent(lines[bounds[0]]))
        label = f"{dest} [block {marker}]"
    if actual == expected:
        return None
    # Old file = the local destination, new file = canonical: the `+` lines are
    # what `sync` would bring in, answering "what changes locally to match?".
    # Bytes are decoded only here, for a human-readable diff.
    diff = difflib.unified_diff(
        actual.decode("utf-8", "replace").splitlines(keepends=True),
        expected.decode("utf-8", "replace").splitlines(keepends=True),
        label,
        "canonical",
    )
    return f"{entry_id(entry)}: drift\n" + "".join(diff)


def sync_entry(entry: dict, canonical: bytes) -> None:
    dest = Path(entry["dest"])
    if entry.get("mode", "file") == "file":
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_bytes(canonical)
        return
    if not dest.exists():
        raise SystemExit(
            f"meta_sync: cannot sync {entry_id(entry)} — destination missing; "
            "block mode needs the file with its marker lines seeded first"
        )
    try:
        lines = dest.read_bytes().splitlines(keepends=True)
    except OSError as error:
        raise SystemExit(f"meta_sync: cannot read {entry_id(entry)}: {error}") from error
    bounds = find_block(lines, entry["marker"])
    if bounds is None:
        raise SystemExit(
            f"meta_sync: cannot sync {entry_id(entry)} — marker lines missing "
            "(seed them first)"
        )
    dest.write_bytes(
        b"".join(lines[: bounds[0] + 1])
        + indent_body(block_body(canonical), marker_indent(lines[bounds[0]]))
        + b"".join(lines[bounds[1] :])
    )


def main(argv: list[str]) -> int:
    if len(argv) != 2 or argv[1] not in ("check", "sync"):
        print(__doc__, file=sys.stderr)
        return 2
    if not MANIFEST.exists():
        print(f"meta_sync: no {MANIFEST} in {Path.cwd()}", file=sys.stderr)
        return 2
    entries = load_manifest()
    failures = []
    for entry in entries:
        canonical = fetch(entry)
        if argv[1] == "check":
            problem = check_entry(entry, canonical)
            if problem:
                failures.append(problem)
        else:
            sync_entry(entry, canonical)
            print(f"synced {entry_id(entry)}")
    if argv[1] == "check":
        if failures:
            print("\n\n".join(failures), file=sys.stderr)
            noun = "entry" if len(failures) == 1 else "entries"
            print(
                f"\nmeta_sync: {len(failures)} {noun} out of sync — "
                "run `python3 scripts/meta_sync.py sync`",
                file=sys.stderr,
            )
            return 1
        print(f"meta_sync: all {len(entries)} entries in sync")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv))
