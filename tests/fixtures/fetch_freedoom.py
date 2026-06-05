from pathlib import Path
from urllib.request import urlretrieve

BASE = "https://github.com/freedoom/freedoom/releases/download/v0.13.0"
TARGET = Path("tests/fixtures/freedoom")
FILES = ["freedoom1.wad", "freedoom2.wad"]

TARGET.mkdir(parents=True, exist_ok=True)
for name in FILES:
    destination = TARGET / name
    if destination.exists():
        print(f"already present: {destination}")
        continue
    url = f"{BASE}/{name}"
    print(f"downloading {url} -> {destination}")
    urlretrieve(url, destination)
