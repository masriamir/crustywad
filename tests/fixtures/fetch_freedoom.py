"""Fetch FreeDoom WAD fixtures from a versioned GitHub release archive.

Usage:
    python tests/fixtures/fetch_freedoom.py [--version VERSION]

    VERSION defaults to the FREEDOOM_VERSION environment variable, then to
    the DEFAULT_VERSION constant below.  Pass a bare semver tag, e.g. "v0.13.0".
"""

import argparse
import os
import shutil
import tempfile
from pathlib import Path
from urllib.request import urlretrieve
from zipfile import ZipFile

DEFAULT_VERSION = "v0.13.0"
TARGET = Path("tests/fixtures/freedoom")
FILES = ["freedoom1.wad", "freedoom2.wad"]


def _normalise_version(version: str) -> str:
    """Return version with a leading 'v', e.g. '0.13.0' -> 'v0.13.0'."""
    return version if version.startswith("v") else f"v{version}"


def _archive_name(version: str) -> str:
    """Return the release archive filename for a version tag such as 'v0.13.0'."""
    bare = version.lstrip("v")
    return f"freedoom-{bare}.zip"


def fetch(version: str) -> None:
    version = _normalise_version(version)
    archive_file_name = _archive_name(version)
    archive_url = (
        f"https://github.com/freedoom/freedoom/releases/download/{version}/{archive_file_name}"
    )

    TARGET.mkdir(parents=True, exist_ok=True)
    missing = [name for name in FILES if not (TARGET / name).exists()]

    if not missing:
        for name in FILES:
            print(f"already present: {TARGET / name}")
        return

    with tempfile.NamedTemporaryFile(suffix=".zip", delete=False) as tmp:
        archive_path = Path(tmp.name)

    try:
        print(f"downloading {archive_url} -> {archive_path}")
        urlretrieve(archive_url, archive_path)

        with ZipFile(archive_path) as archive:
            members = {Path(member).name.lower(): member for member in archive.namelist()}
            for name in missing:
                destination = TARGET / name
                member = members.get(name.lower())
                if member is None:
                    raise FileNotFoundError(f"{name} not found in {archive_url}")

                print(f"extracting {member} -> {destination}")
                with archive.open(member) as source, destination.open("wb") as sink:
                    shutil.copyfileobj(source, sink)
    finally:
        archive_path.unlink(missing_ok=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--version",
        default=os.environ.get("FREEDOOM_VERSION", DEFAULT_VERSION),
        help=(
            "FreeDoom release version tag to download (default: %(default)s). "
            "Can also be set via the FREEDOOM_VERSION environment variable."
        ),
    )
    args = parser.parse_args()
    fetch(args.version)
