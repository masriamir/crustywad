import shutil
import tempfile
from pathlib import Path
from urllib.request import urlretrieve
from zipfile import ZipFile

ARCHIVE_URL = "https://github.com/freedoom/freedoom/releases/download/v0.13.0/freedoom-0.13.0.zip"
TARGET = Path("tests/fixtures/freedoom")
FILES = ["freedoom1.wad", "freedoom2.wad"]

TARGET.mkdir(parents=True, exist_ok=True)
missing = [name for name in FILES if not (TARGET / name).exists()]

if not missing:
    for name in FILES:
        print(f"already present: {TARGET / name}")
else:
    with tempfile.NamedTemporaryFile(suffix=".zip", delete=False) as archive_file:
        archive_path = Path(archive_file.name)

    try:
        print(f"downloading {ARCHIVE_URL} -> {archive_path}")
        urlretrieve(ARCHIVE_URL, archive_path)

        with ZipFile(archive_path) as archive:
            members = {Path(member).name.lower(): member for member in archive.namelist()}
            for name in missing:
                destination = TARGET / name
                member = members.get(name.lower())
                if member is None:
                    raise FileNotFoundError(f"{name} not found in {ARCHIVE_URL}")

                print(f"extracting {member} -> {destination}")
                with archive.open(member) as source, destination.open("wb") as sink:
                    shutil.copyfileobj(source, sink)
    finally:
        archive_path.unlink(missing_ok=True)
