# Freedoom fixtures

This directory is reserved for locally downloaded Freedoom WAD files used by optional integration tests.

- Default source: https://github.com/freedoom/freedoom/releases/tag/v0.13.0
- Expected files: `freedoom1.wad`, `freedoom2.wad`
- Local path: `tests/fixtures/freedoom/`
- Environment variable: `CRUSTYWAD_FREEDOOM_DIR`

The fetch script downloads a versioned `freedoom-<version>.zip` release archive and extracts the expected WADs into this directory. The downloaded WAD files are intentionally gitignored so the repository stays small and tests can run offline.

## Changing the Freedoom version

The default version is controlled by the `DEFAULT_VERSION` constant in `fetch_freedoom.py`. You can override it without touching the source file:

```bash
# Via environment variable
FREEDOOM_VERSION=v0.14.0 just fetch-fixtures

# Via justfile argument
just fetch-fixtures version=v0.14.0

# Direct invocation
python3 tests/fixtures/fetch_freedoom.py --version v0.14.0
```

## Non-redistributable IWADs (Hexen, Doom 64)

Hexen and Doom 64 IWADs are **not freely redistributable**, so unlike Freedoom they are
never downloaded or committed. To run their optional tests, point the matching environment
variable at a directory containing the IWAD:

| Feature | Environment variable | Expected file |
|---|---|---|
| `hexen-tests` | `CRUSTYWAD_HEXEN_DIR` | `hexen.wad` |
| `doom64-tests` | `CRUSTYWAD_DOOM64_DIR` | `doom64.wad` |

Each test skips gracefully when its variable is unset or the file is missing, so CI (which
sets no fixture variables) stays green.
