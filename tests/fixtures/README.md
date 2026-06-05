# FreeDoom fixtures

This directory is reserved for locally downloaded FreeDoom WAD files used by optional integration tests.

- Source: https://github.com/freedoom/freedoom/releases/tag/v0.13.0
- Expected files: `freedoom1.wad`, `freedoom2.wad`
- Local path: `tests/fixtures/freedoom/`
- Environment variable: `CRUSTYWAD_FREEDOOM_DIR`

The fetch script downloads the `freedoom-0.13.0.zip` release archive and extracts the expected WADs into this directory. The downloaded WAD files are intentionally gitignored so the repository stays small and tests can run offline.
