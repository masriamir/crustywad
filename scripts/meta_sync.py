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
         file, and must already exist — seed them once when wiring a repo up.

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


def entry_id(entry: dict) -> str:
    return f"{entry['source']}:{entry['path']} -> {entry['dest']}"


def fetch(entry: dict) -> str:
    src = entry["source"]
    try:
        if src.startswith("file:"):
            return (Path(src[len("file:"):]) / entry["path"]).read_text(encoding="utf-8")
        ref = entry["ref"]
        if not re.fullmatch(r"[0-9a-f]{40}", ref):
            raise SystemExit(
                f"meta_sync: {entry_id(entry)}: ref {ref!r} is not a 40-char commit "
                "sha — pin an exact commit, not a branch or tag"
            )
        url = f"https://raw.githubusercontent.com/{src}/{ref}/{entry['path']}"
        with urllib.request.urlopen(url, timeout=30) as response:
            return response.read().decode("utf-8")
    except SystemExit:
        raise
    except (OSError, UnicodeError) as error:  # URLError/HTTPError/FileNotFoundError
        # are all OSError; UnicodeError covers non-UTF8 canonical content.
        raise SystemExit(f"meta_sync: cannot fetch {entry_id(entry)}: {error}") from error


def find_block(lines: list[str], marker: str) -> tuple[int, int] | None:
    start, end = f">>> meta:{marker}", f"<<< meta:{marker}"
    first = None
    for i, line in enumerate(lines):
        if first is None:
            if start in line:
                first = i
        elif end in line:
            return first, i
    return None


def load_manifest() -> list[dict]:
    try:
        data = tomllib.loads(MANIFEST.read_text(encoding="utf-8"))
    except (tomllib.TOMLDecodeError, OSError) as error:
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


def check_entry(entry: dict, canonical: str) -> str | None:
    dest = Path(entry["dest"])
    if not dest.exists():
        return f"{entry_id(entry)}: destination missing"
    try:
        text = dest.read_text(encoding="utf-8")
    except (OSError, UnicodeError) as error:
        return f"{entry_id(entry)}: cannot read {dest}: {error}"
    if entry.get("mode", "file") == "file":
        actual, label = text, str(dest)
    else:
        marker = entry["marker"]
        lines = text.splitlines(keepends=True)
        bounds = find_block(lines, marker)
        if bounds is None:
            return (
                f"{entry_id(entry)}: marker lines '>>> meta:{marker}' / "
                f"'<<< meta:{marker}' not found"
            )
        actual, label = "".join(lines[bounds[0] + 1 : bounds[1]]), f"{dest} [block {marker}]"
    if actual == canonical:
        return None
    # Old file = the local destination, new file = canonical: the `+` lines are
    # what `sync` would bring in, answering "what changes locally to match?".
    diff = difflib.unified_diff(
        actual.splitlines(keepends=True),
        canonical.splitlines(keepends=True),
        label,
        "canonical",
    )
    return f"{entry_id(entry)}: drift\n" + "".join(diff)


def sync_entry(entry: dict, canonical: str) -> None:
    dest = Path(entry["dest"])
    if entry.get("mode", "file") == "file":
        dest.parent.mkdir(parents=True, exist_ok=True)
        dest.write_text(canonical, encoding="utf-8")
        return
    if not dest.exists():
        raise SystemExit(
            f"meta_sync: cannot sync {entry_id(entry)} — destination missing; "
            "block mode needs the file with its marker lines seeded first"
        )
    try:
        lines = dest.read_text(encoding="utf-8").splitlines(keepends=True)
    except (OSError, UnicodeError) as error:
        raise SystemExit(f"meta_sync: cannot read {entry_id(entry)}: {error}") from error
    bounds = find_block(lines, entry["marker"])
    if bounds is None:
        raise SystemExit(
            f"meta_sync: cannot sync {entry_id(entry)} — marker lines missing "
            "(seed them first)"
        )
    dest.write_text(
        "".join(lines[: bounds[0] + 1]) + canonical + "".join(lines[bounds[1] :]),
        encoding="utf-8",
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
