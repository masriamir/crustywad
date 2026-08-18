# idgames Corpus Harvest — Tool Spec

**Status:** spike-verified 2026-08-12 (issue #402) — API and mirror assumptions
probed against reality and corrections folded in; ready to plan against.
Committed home decided by ADR-0030. All three phases implemented (#405, #406,
#407); the first full harvest is recorded in §8.5 (2026-08-18, #408).
**Target repo:** `masriamir/crustywad`
**Audience:** implementer (human or Claude Code CLI)

---

## 0. How to use this document

This spec is verified against the live API and mirrors (§2, spike issue #402,
run 2026-08-12). Work in this order:

1. Read §1 (goals) and §2 (the verified checklist and its corrections).
2. Produce an implementation plan.
3. Implement phase by phase, with the §9 acceptance criteria as the gate.

Provenance: the endpoint semantics were originally reconstructed from a
third-party Go client (`zmnpl/goidgames`, `types.go`) because the official API
docs page blocks automated fetching. The §2 spike then verified every
load-bearing assumption against the live API — several were corrected (root
listing call, trailing-slash requirement, PII field arrival, empty-collection
shape, mirror list) — so the body text now reflects observed behavior, not
the second-hand reconstruction.

---

## 1. Goals and non-goals

### Goals

1. Produce a complete, machine-readable manifest of every map-bearing entry in the
   /idgames archive, with **true uncompressed WAD sizes**, without downloading
   archive payloads (beyond §5.2's budgeted, circuit-broken full-download
   fallback for the rare file whose ranges every mirror refuses).
2. Derive the size and format statistics needed to set defensible upload limits
   for the crustywad web UI.
3. Emit a corpus manifest usable as a fetch list for `sweep-tests`
   (`CRUSTYWAD_SWEEP_DIR`) and as `cargo-fuzz` seed material.

### Non-goals

- Downloading or mirroring the archive. Phase 2 reads zip metadata only.
- Any change to the `crustywad` or `crustywad-cli` public API.
- Implementing the web UI. §8 specifies the limits the UI must enforce; building
  it is separate work.
- pk3/pk7 support. Tracked as an open item in §10.

---

## 2. Verification checklist

**Spike run 2026-08-12 (issue #402 carries the full probe record). Every item
below is resolved; corrections are folded into the body sections they affect.**

Verified before implementing Phase 1:

- [x] `api.php` reachable by a non-browser client — every probe answered 200
      to plain curl with a mainstream browser UA (§4.6).
- [x] `?action=about&out=json` — API version **3** (`meta.version`).
- [x] Top-level listing — via **`action=getdirs`** with no `name` (there is no
      root `getcontents` call); real root recorded and §4.2's set justified
      against it. No top-level `ports/`; `misc/`, `historic/`, `roguestuff/`
      remain to triage (§4.2).
- [x] Listing field set — all §6 load-bearing fields (`size`, `date`,
      `rating`, `votes`) present; full set in §4.3's table, including the
      unexpected `email` and `description` (PII consequence in §4.7).
- [x] `ls-laR.gz` — present on both verified mirrors; bootstraps the full
      tree + names + zip sizes in one 418 KB request (§5.0).
- [x] `getcontents` envelope — `meta` + `content`, `content.file`/`content.dir`
      collection keys, path via `name` **with a mandatory trailing slash**
      (§4.1; bare or bad paths return null/null silently).
- [x] `dir` field trailing slash — **confirmed present** (`"levels/doom/0-9/"`);
      §5 URL construction holds.
- [x] Single-element quirk — **not reproduced** (cardinalities 2/8/9/60 all
      arrays); empty collections arrive as explicit `null` instead (§4.4);
      defensive `Option<OneOrMany<T>>` retained.
- [x] Error envelope — `{"error":{"type":"…","message":"…"},"meta":…}`
      (from a missing-argument request; §4.1).
- [x] `latestfiles&limit=1` — works; returned the current max id (22083)
      (record shape is abbreviated — see the §4.5 correction).

Verified before implementing Phase 2:

- [x] Mirrors — the pre-spike pair was half dead; **verified pool is
      `ftpmirror1.infania.net` + `gamers.org`** (§5.1), both with
      `Accept-Ranges: bytes`.
- [x] Ranged GET returns **206** with exactly the requested bytes on both
      verified mirrors (and infania's `Content-Length` matches the API `size`
      field exactly on the cross-checked file).
- [x] `zip` crate — pin major **8** (8.6.0 stable as of the spike; 9.0 is
      pre-release); accessor-name verification against 8.x docs happens at
      #406 implementation time (§5.2).

---

## 3. Placement and workspace isolation

Create `xtask/` at the repo root with **its own `[workspace]` table**, making it a
separate workspace that the root workspace does not see:

```toml
# xtask/Cargo.toml
[workspace]           # empty table — excludes this crate from the root workspace

[package]
name = "xtask"
version = "0.0.0"
edition = "2024"
publish = false

[dependencies]
anyhow = "1"
blake3 = "1"
chrono = { version = "0.4", features = ["serde"] }
clap = { version = "4", features = ["derive"] }
fastrand = "2"        # §3 addendum (#405): backoff jitter RNG — the original list omitted one
flate2 = "1"          # §3 addendum (#405): ls-laR.gz is a gzipped payload file; a decoder is required
indicatif = "0.18"    # §3 correction (#405): 0.17's number_prefix is unmaintained (RUSTSEC-2025-0119)
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["rt-multi-thread", "macros", "fs", "sync", "time"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter", "json"] }
zip = "8"             # spike-pinned major (§2, §5.2); verify accessor names at #406
```

> Correction (#405): the original dependency list omitted a gzip decoder (the §5.0 bootstrap fetches a .gz payload file, which HTTP transport decoding never touches) and a jitter RNG for §4.6 backoff. flate2 and fastrand added.

> Correction (#406): the module sketch above undersold `range_reader.rs` and omitted `store.rs` entirely. `range_reader.rs` carries far more than the `Read + Seek` reader: it also owns the §5.1 mirror pool source (`MirrorRanges`, with per-mirror retry/failover and `Content-Range` validation), the run-wide `TransferCounters`, and the §5.2 full-download fallback budget (`FallbackBudget`). `store.rs` is a new module for the §5.4 per-id results log and its `body_hash` invalidation — an append-only JSONL log that makes the phase resumable.

**Rationale for isolation:** the harvester needs `reqwest`, `tokio`, and `zip`.
Pulling those into the root workspace would surface in the `deps.rs` badge, widen
the CodeQL scan surface, slow `cargo test --workspace`, and impose an MSRV floor
unrelated to the library. The library's dependency graph is a feature of the
project; don't contaminate it with tooling.

Pin `default-features = false` + `rustls-tls` on reqwest to avoid an OpenSSL
system dependency.

### Justfile recipes

```make
harvest-api:
    cargo run --manifest-path xtask/Cargo.toml --release --locked -- harvest-api

harvest-zips:
    cargo run --manifest-path xtask/Cargo.toml --release --locked -- harvest-zips

harvest-outliers:
    cargo run --manifest-path xtask/Cargo.toml --release --locked -- harvest-outliers

harvest-stats:
    cargo run --manifest-path xtask/Cargo.toml --release --locked -- stats

harvest: harvest-api harvest-zips harvest-outliers harvest-stats
```

### CI and supply chain

Do not add `xtask` to the default lint/test jobs. Instead:

- A separate workflow gated on `paths: ['xtask/**']` runs fmt, clippy, the §9
  unit tests, **and `cargo deny check`** — never the network-touching commands.
- **Isolation opens an audit hole; close it explicitly.** An out-of-workspace
  crate escapes the root `security-deny` job, CodeQL, and Dependabot. The repo
  already solved this for `fuzz/`: add a matching `/xtask` stanza to
  `.github/dependabot.yml`, with the same explanatory comment.
- **Commit `xtask/Cargo.lock`.** It is a binary tool; reproducible builds and
  dependency audits both want the lockfile.
- reqwest/tokio/zip is a far juicier dependency tree than the library's — treat
  its audit story as first-class, not tooling-exempt.

### Module layout

```
xtask/src/
  main.rs           # clap dispatch
  phase1.rs         # phase-1 orchestrator (#405)
  scope.rs          # §4.2 include/skip/triage table (#405)
  lslar.rs          # §5.0 ls-laR.gz tree parser (#405)
  mirror.rs         # §5.1 mirror pool + conditional bootstrap fetch (#405)
  cache.rs          # §4.5 disk cache with tiered TTL, email-scrubbed bodies
  schema.rs         # output record types (§4.7, §5.6, §6.5) + deterministic writers
  api/
    mod.rs
    client.rs       # rate-limited API client, backoff
    model.rs        # FileRecord, OneOrMany, response envelopes
    traverse.rs     # §4.2 enrichment walk over the §5.0 tree (BFS fallback)
  zips/             # #406
    mod.rs
    range_reader.rs # §5.2 Read + Seek over HTTP ranges + mirror range source, transfer counters, fallback budget
    inspect.rs      # CD extraction, member filtering
    store.rs        # §5.4 per-id results log + body_hash invalidation
    url_source.rs   # §6.4 single-URL RangeSource (#407)
  outliers.rs       # §6.4 harvest-outliers orchestrator (#407)
  stats/            # #407
    mod.rs
    percentiles.rs  # (#407)
    report.rs       # (#407)
```

---

## 4. Phase 1 — API enumeration

**Command:** `xtask harvest-api`

### 4.1 Endpoint

`https://www.doomworld.com/idgames/api/api.php`

All calls take `action=<name>` and `out=json`. Responses wrap in an object
containing `meta` (with a `version` integer) and `content`. Errors wrap as
`{"error":{"type":"…","message":"…"},"meta":…}` instead (spike-verified).

**Spike-verified call rules (2026-08-12, API version 3):**

- The top level is listed with **`action=getdirs` and no `name`** — there is
  no root `getcontents` call.
- `getcontents` takes a path via `name`, and the **trailing slash is
  mandatory**: `name=levels/doom/` works; `name=levels/doom` returns a
  success envelope whose `content.file` and `content.dir` are both `null` —
  indistinguishable from a nonexistent path, which answers with the same
  nulls. Always append the slash, and treat a both-fields-`null` response
  as a suspect path, never as an empty directory. (Check the two fields;
  don't byte-compare bodies — the envelope also carries `meta`.)

### 4.2 Traversal roots

**Enumeration is `ls-laR.gz`-first (§5.0):** the mirror bootstrap yields the
full tree in one request, and Phase 1's `getcontents` calls are **metadata
enrichment** — one call per in-scope directory from that tree. BFS discovery
via `getcontents` is the explicit fallback only if the bootstrap is
unavailable on every §5.1 mirror. **Scope decision: everything map-bearing.**
The real top level (spike-verified via `getdirs`, 2026-08-12): `levels/`,
`utils/`, `prefabs/`, `combos/`, `themes/`, `skins/`, `idstuff/`, `music/`,
`graphics/`, `deathmatch/`, `docs/`, `sounds/`, `source/`, `lmps/`, `misc/`,
`roguestuff/`, `historic/`. Inclusions justified against it:

- `levels/` — **all** game subtrees, including `levels/strife`: ADR-0028
  established Strife map records are byte-identical to Doom's, and crustywad
  parses them today (#247 adds game identification on top). Note the real
  sub-tree also carries `levels/doom64/` and `levels/reviews/` (text-only —
  skip). Per-game `Ports/` directories live *inside* `levels/*`, so this
  root covers them.
- `deathmatch/` — map-bearing (spike-confirmed at top level)
- `combos/` — gameplay mods frequently carry WADs
- `prefabs/` — WAD fragments
- `themes/`

**Skip:** `music/`, `sounds/`, `utils/`, `lmps/`, `docs/`, `graphics/`,
`source/`, `idstuff/`, `skins/` (player skins, not maps). There is **no
top-level `ports/`** — the pre-spike draft listed one; it does not exist.
**Triage resolution (2026-08-16, #407):** `misc/`, `historic/`, `roguestuff/`
are **Skip**. Inspection via the cached `ls-laR` listing: `historic/` = 42
zips of DOOM engine alphas/betas/shareware distributions, id utilities
(`dmutils`, `bsp11x`, `deth23`), and press coverage — pre-release IWADs, not
community maps, and a direct skew on the §8 size statistics; `misc/` = 26
zips of fonts, themes, posters, and walkthroughs whose only WAD-bearing
items are official id content (`betraysewers.zip`, `sigil_bfg.zip` — RETAIL
measurement material, not community corpus); `roguestuff/` = 2 Strife demo
distributions. **Triage resolution (2026-08-18, #408):** `incoming/` and
`newstuff/` are **Skip**. Both surfaced as untriaged roots during the #408
warm re-runs. Inspection via the cached `ls-laR` listing (2026-08-18):
`newstuff/` = 29 zips, every one *also* present at its final path elsewhere
in the tree — recently approved uploads in transit, so including the root
double-counts 100% of its entries; `incoming/` = 25 zips awaiting review
(5 already duplicated at final paths) — unreviewed, unstable staging
content, not part of the curated archive. Any *new* top-level root still
lands in `Triage` (skipped loudly) until a decision is recorded here.

`action=search` is capped and will not enumerate exhaustively. On the API
side, `getcontents` is the only reliable traversal — but the primary
exhaustive enumeration is the §5.0 `ls-laR.gz` bootstrap, not an API walk.

### 4.3 File record fields

**Decision: Phase 1 uses `getcontents` listings only — `action=get` full records
are never fetched.** Consequences, all intentional:

- ~~`email`, `textfile`, and `reviews` never arrive. PII is eliminated at the
  source rather than fetched-and-dropped.~~ **CORRECTED by the §2 spike
  (2026-08-12): `email` and `description` DO arrive in every listing record**
  (`textfile`/`reviews` genuinely don't). PII posture is therefore
  **fetch-and-drop**: `email` is discarded at deserialization (never written to
  any output), and the #403 data-governance ADR must record this.
- The UPLTEMPL port-targeting fields (`Map Format`, `Advanced engine needed`)
  are forgone — format truth comes from parsing the WADs in the sweep corpus,
  which is more reliable than free-text metadata anyway (§10).
- Request count stays one per *directory*, not one per *file*.

Fields expected in a listing record (the §2 spike records the actual set —
`size`, `date`, `rating`, and `votes` are load-bearing for §6):

| Field | Type | Notes |
| --- | --- | --- |
| `id` | int | Archive file ID |
| `title` | string \| null | **Nullable** (spike-observed on 1994-era records) |
| `dir` | string | Full directory path — **carries a trailing slash** (spike-verified: `"levels/doom/0-9/"`) |
| `filename` | string | Filename only, no path |
| `size` | int | **Size of the zip, in bytes. Not the WAD size.** Spike cross-check: matches the mirror's `Content-Length` exactly |
| `age` | int64 | Unix epoch seconds of archive addition |
| `date` | string | `YYYY-MM-DD` |
| `author` | string | |
| `email` | string | **Arrives whether wanted or not — dropped at deserialization (§4.7)** |
| `description` | string | Full upload description text — arrives in listings (spike-verified) |
| `rating` | float \| null | Mean user rating; null when unrated |
| `votes` | int | |
| `url` | string | doomworld.com frontend URL |
| `idgamesurl` | string | `idgames://` protocol URL |

### 4.4 Deserialization gotchas

**Single-element collections.** The API is PHP-backed and — per the
third-party client this spec was reconstructed from — **may** serialize a
one-element list as a bare object rather than a one-element array,
potentially affecting both `content.file` and `content.dir`:

```rust
#[derive(Deserialize)]
#[serde(untagged)]
enum OneOrMany<T> {
    Many(Vec<T>),   // try Vec first — an object won't match it
    One(T),
}

impl<T> OneOrMany<T> {
    fn into_vec(self) -> Vec<T> {
        match self {
            OneOrMany::Many(v) => v,
            OneOrMany::One(x) => vec![x],
        }
    }
}
```

Order the variants `Many` before `One`. ~~Empty directories omit the key
entirely~~ **Spike correction (2026-08-12): an empty collection arrives as an
explicit `null`** (`"file":null` / `"dir":null` observed on every leaf/branch
probe), so wrap in `Option<OneOrMany<T>>` — it covers `null`, a bare object,
and an array alike — and default to empty **only after** the §4.1
suspect-path check: both fields `None` means "suspect path" and must be
detected on the raw `Option`s, before defaulting erases the distinction
between "empty collection" and "nothing came back". The one-element bare-object quirk
itself did **not** reproduce in the spike (probed cardinalities 2, 8, 9, and
60 all serialize as arrays); keep the defensive `OneOrMany` shape anyway —
it costs nothing and the PHP backend's behavior is not contractual.

**Numeric fields arriving as strings.** PHP's JSON encoder is inconsistent about
this. Use `#[serde(deserialize_with = ...)]` helpers that accept both for `size`,
`age`, `votes`, and `rating`, rather than discovering it as a runtime failure
2,000 directories in.

**Missing / null strings.** Default to empty rather than failing the record.

### 4.5 Caching and freshness

**Cache key is the request, not the response.** A response hash cannot be a cache
key — computing it requires making the request the cache exists to avoid. Key on
`(action, path)`, hashed to a safe filename.

Cache entry envelope:

```json
{
  "action": "getcontents",
  "path": "levels/doom2/Ports/",
  "fetched_at": "2026-08-03T14:22:01Z",
  "api_version": 3,
  "body_hash": "blake3:...",
  "body": { }
}
```

**Cached bodies are email-scrubbed at write time** (§4.7): the raw response
is never persisted verbatim — `email` fields are removed from the body before
the cache entry is written, and `body_hash` is computed over the *scrubbed*
body. A change only in an email field therefore goes undetected, which is
immaterial: the hash exists to invalidate Phase 2 when a directory's file
list or sizes move, and email cannot affect either.

`body_hash` is **change detection, not keying**. On a TTL-triggered refetch,
compare the new hash against the stored one: unchanged means you can skip
invalidating the dependent Phase 2 results for that directory; changed means the
directory's file list moved and Phase 2 must rerun for it.

**Tiered TTL:**

| Data | TTL | Rationale |
| --- | --- | --- |
| `getcontents` directory listings | 7 days | Structure changes slowly |
| `latestfiles` probe | none — always live | It *is* the freshness check |

**Cheap addition detection.** Before any TTL sweep, call
`action=latestfiles&limit=1` (spike-verified working; returned id 22083 on
2026-08-12). Compare the returned ID against the max `id` in the
existing manifest. If it hasn't moved, nothing has been added and you only need
the TTL sweep to catch mutations — which for the purposes of size analysis is
almost never material. If it has moved, walk `latestfiles` backwards from the new
max to your known max to pick up additions without re-walking the tree.

> **Correction (#405, observed live 2026-08-15):** `latestfiles` listing
> records are **abbreviated** — `{id, title, author, description, rating}`
> only; `dir`, `filename`, `size`, `age`, `date`, and the URL fields do not
> arrive (and at `limit=1` the `file` collection arrives as a bare object, so
> the §4.4 `OneOrMany` handling is load-bearing here). The probe therefore
> yields the max `id` and nothing else, and "walk `latestfiles` backwards"
> cannot name the directories to refresh. Replacement: additions/deletions/
> replacements are detected **mirror-side** — on a fresh (non-304)
> `ls-laR.gz`, the per-directory `.zip` `(name, size)` sets are diffed against
> the previous run's `idgames-files.jsonl`, and exactly the drifted in-scope
> directories have their `getcontents` cache entries invalidated. This costs
> zero extra API requests (the warm-rerun budget of §9.3 is unchanged) and
> converges via the 7-day TTL when a mirror lags the API.

This makes the common rerun cost one HTTP request.

### 4.6 Politeness and error handling

- **One request per second**, single connection, enforced by an interval-gated
  semaphore. This is one shared PHP endpoint maintained by volunteers. The
  harvest is not time-critical; do not parallelize it.
- **User-Agent:** a mainstream browser UA string (project-standard practice;
  spike-verified — every probe answered 200 to curl with a browser UA).
  The API exists to serve third-party clients, and an identifying tool UA risks
  tripping incidental anti-bot layers — the docs page already does. Politeness
  is enforced by the request rate, not the UA.
- **Backoff:** exponential with jitter on `429` and `5xx`, capped at ~5 minutes,
  max 6 attempts. Then record the failure and continue — do not abort the run.
- **Resumability:** every response is cached on arrival (email-scrubbed at
  write, §4.5/§4.7), so an interrupted run resumes for free. The BFS frontier should also be checkpointed so a resume
  doesn't re-derive it.
- **Failure ledger:** write unresolvable failures to `data/harvest-errors.jsonl`
  with path, status, and attempt count. A silent partial harvest is worse than a
  loud one.
- **Dev mode:** `--root <path>` and `--limit <n>` flags scope a run to a single
  small directory so development and the §9.2 integration tests never hammer
  the API. Scoped runs write their outputs under `xtask/data/dev/` (same
  filenames) so a dev run never clobbers a full harvest; the response cache is
  shared — it is request-keyed, so mixing modes is always safe.

### 4.7 Output, PII, and data governance

`data/idgames-files.jsonl` — one record per archive entry, one JSON object per
line. All `data/` paths in this spec live at `xtask/data/`, which is
**gitignored** — no harvest output is ever committed.

**PII policy — fetch-and-drop (corrected by the §2 spike):** `email` arrives
in every `getcontents` listing record whether wanted or not, so elimination
at the source is impossible. Instead, `email` is dropped on **both** disk-touching paths: the record
struct has no field for it (so no output can carry it), and the §4.5
response cache scrubs `email` fields from bodies at write time (so the
cache cannot carry it either — raw responses are never persisted verbatim).
`textfile` and `reviews` genuinely never arrive in listings. If a future phase needs textfile-derived fields, it gets
its own opt-in fetch pass with its own PII handling.

**Publishing is deferred.** This is an internal tool. If outputs are ever
published (e.g. a release artifact for populating `CRUSTYWAD_SWEEP_DIR`), only
the PII-free trio ships: `sweep-corpus.jsonl`, `stats.json`, `stats-report.md`.

Also write `data/harvest-manifest.json`: run timestamp, API version, tool version,
root paths traversed, counts, and duration. Statistics without provenance are not
reproducible. **The manifest is the only output carrying wall-clock timestamps**
— downstream outputs reference its ID so reruns stay byte-identical (§9.3).

---

## 5. Phase 2 — True WAD sizes via HTTP range requests

**Command:** `xtask harvest-zips`

**Goal:** the uncompressed size of every `.wad` member of every archive entry,
without transferring payload bytes (except through §5.2's budgeted fallback).

### 5.0 `ls-laR.gz` bootstrap (spike-verified, architecture-changing)

Both verified mirrors serve a root `ls-laR.gz` — a recursive `ls -laR` of the
whole archive. **One request (418 KB on 2026-08-12, same-day Last-Modified on
infania) yields the full directory tree, every filename, and every zip's
byte size**: 462 directories, 21,375 zips, 38.07 GiB archive-wide
(`levels/doom2` 27.48 GiB, `levels/doom` 1.08 GiB). Consequences:

- Phase 1's BFS discovery role collapses to this single request; the API is
  demoted to **metadata-only enrichment** (per-directory `getcontents` for
  `rating`/`votes`/`date`/`description`), ≤462 calls ≈ 8 minutes at 1 req/s.
- The zip `size` field arrives from both sources — cross-checking API `size`
  against the `ls-laR` size per file is a free consistency guard.
- **No idgames rsync module exists** on either verified mirror (spike-probed);
  bulk transfer stays HTTP.

### 5.1 Mirrors

- `https://ftpmirror1.infania.net/pub/idgames` — **verified 2026-08-12**: ranged GET 206,
  `Accept-Ranges: bytes`, `ls-laR.gz` present with same-day Last-Modified
- `https://www.gamers.org/pub/idgames` — **verified 2026-08-12**: ranged GET 206,
  `ls-laR.gz` present
- ~~`https://www.quaddicted.com/files/idgames`~~ — **dead for idgames** (404 on real
  archive paths, 2026-08-12); ftp.mancubus.net likewise 404; youfailit.net 301s
  (redirect target untriaged)

URL construction: `<mirror>/<dir><filename>`, assuming `dir` carries a trailing
slash (verify per §2).

**Never pull binaries from doomworld.com.** It is a web frontend, not a file host.

### 5.2 Recommended implementation

Do not hand-roll the zip parser. Pin `zip` major **8** (spike-checked
2026-08-12: 8.6.0 is the latest stable; 9.0 exists only as a pre-release) and
verify the accessor names below against that version's docs at implementation
time. Implement `Read + Seek` over HTTP range GETs
with a small cache, and hand it to `zip::ZipArchive::new`. The crate handles
central directory parsing including ZIP64 correctly, and a caching reader
collapses its access pattern into 2–3 HTTP requests per file:

1. Cache the last 66 KiB of the file — ≥ the worst-case 65,557-byte EOCD
   backward-scan window (22-byte record + 65,535-byte max comment, §5.3);
   a bare 64 KiB tail can miss the signature by up to 21 bytes.
2. Cache the central directory extent once located.

Then iterate entries and read declared sizes. **On this range path, no entry
data is ever read**, so no payload transfers (the budgeted full-download
fallback below is the sole exception). Verify these accessors against your pinned `zip` version:
uncompressed size, compressed size, compression method, encryption flag, and the
member name.

> Correction (#406, zip 8.6.0): the "2–3 requests per file" estimate above
> undercounts by construction. `by_index_raw` — the only call an actual
> `.wad` match makes — never decompresses payload, but it does eagerly parse
> that member's local file header (a fixed ~30-byte block,
> `ZipLocalEntryBlock` in the vendored `src/types.rs`) to compute a data
> offset the tool never uses. Real per-file request cost is: 1 tail fetch
> (66 KiB) + at most 1 central-directory fetch (capped 64 MiB, §5.4) + one
> ≤256-byte "nibble" fetch per `.wad` member whose local header lies outside
> every already-cached extent — budgeted at 24 member fetches, after which
> the entry fails closed as `zip_parse_error` rather than silently
> under-reporting members. Still zero *payload* bytes move on the range
> path — only header-sized metadata.

**No per-file `HEAD` preflight** — the zip's size is already known from
`ls-laR.gz` (§5.0) and cross-checkable against the API `size`, so issue the
first ranged GET directly; a `200` response (payload instead of a range) IS
the no-range-support signal. On `200`, fall back to the second mirror; if
both refuse ranges, download in full and flag the record. This saves one
request per entry (~20k+ across the archive). **The full-download fallback is budgeted, not open-ended:** a global
byte budget (default 2 GiB) plus an abort threshold — if more than ~2% of
entries hit the fallback, stop the phase and report, because a CDN change has
turned "read the metadata" into "mirror the archive," which this tool must never
silently do.

> Correction (#406): two more safety invariants sit alongside the 2 GiB
> global fallback budget and the ~2% breaker above. A **per-entry
> full-download cap** of 512 MiB refuses any single entry that big outright
> — recorded as `no_range_support` with a ledger note, never charged against
> the shared budget, so one giant file can't silently consume the whole
> run's fallback allowance. A **range-path runaway ceiling** of 4 GiB total
> transferred bytes aborts the phase exactly like the 2% breaker if
> exceeded: a pathological EOCD/central-directory shape can push up to
> roughly 3×64 MiB down the *range* path per entry — bytes the fallback
> budget never sees at all, since they never go through the full-download
> path.

> Correction (#406): every `206` response's `Content-Range` header is
> validated against the requested `[offset, end]` before its bytes are
> trusted — an exact start/end match is required; a missing or mismatched
> header is treated as an ordinary attempt failure (retried, then failed
> over to the next mirror/attempt), never as a pass. RFC 7233 §4.2 requires
> the header on every `206`, so its absence is itself suspicious. This
> closes a path where a lying proxy/CDN could otherwise splice foreign bytes
> into the sparse buffer at a requested offset.

> Correction (#441): the "declared size" the Content-Range guard above
> validates against is the §5.0 ls-laR listing's own size for the entry —
> not the Phase-1 API's `size` field — falling back to the API size only
> for an entry the listing doesn't have (a join miss, e.g. one added
> between the listing snapshot and the API walk). The listing is the
> mirror's own account of what it serves, while the API `size` is a
> separately-maintained field; the first full run (2026-08-17) proved the
> API size stale for 1,099/15,732 entries (~7%), all of which failed
> closed as `fetch_error` ("content-range total X != declared size Y")
> before this fix, because the guard was right and its input was wrong.

For files under ~64 KiB, fetch the whole thing — cheaper than three round trips.

### 5.3 Hand-rolled fallback

Only if the crate's seek pattern proves too chatty. All values little-endian.

**EOCD** — signature `0x06054b50`, 22 bytes minimum. Scan backwards over the last
`min(len, 65557)` bytes (22 + max comment length 65535):

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 4 | signature |
| 10 | 2 | total CD records |
| 12 | 4 | CD size in bytes |
| 16 | 4 | CD offset from start of archive |
| 20 | 2 | comment length |

**Central directory file header** — signature `0x02014b50`, 46 bytes fixed:

| Offset | Size | Field |
| --- | --- | --- |
| 0 | 4 | signature |
| 8 | 2 | general purpose bit flag |
| 10 | 2 | compression method |
| 20 | 4 | compressed size |
| 24 | 4 | **uncompressed size** |
| 28 | 2 | filename length |
| 30 | 2 | extra field length |
| 32 | 2 | comment length |
| 46 | n | filename |

Filename is followed by the extra field, then the comment.

**ZIP64.** If the CD offset, CD size, or a member's uncompressed size reads
`0xFFFFFFFF`, the real value lives elsewhere:

- **ZIP64 EOCD locator** — signature `0x07064b50`, 20 bytes, immediately preceding
  the EOCD. ZIP64 EOCD record offset at byte 8 (8 bytes).
- **ZIP64 EOCD record** — signature `0x06064b50`, 56 bytes. CD size at offset 40
  (8 bytes), CD offset at offset 48 (8 bytes).
- **Per-member overflow** lives in the ZIP64 extended information extra field,
  header ID `0x0001`, packing uncompressed size / compressed size / local header
  offset / disk start in that order — but **only for the fields that actually
  read `0xFFFFFFFF`**. Parsing it positionally without checking which fields
  overflowed is the classic bug.

ZIP64 is rare in this archive, which mostly predates it. But the files that use it
are precisely the large modern megawads that define your upper bound, and
silently truncating them to 4 GiB−1 would poison the tail of the distribution you
are building this tool to measure.

### 5.4 Concurrency and resilience

- 4–8 concurrent connections against a mirror, keep-alive enabled. At roughly
  20k entries × ~3 requests that's ~60k requests; expect well under an hour.
- Cache per idgames `id` so the phase is resumable and reruns are incremental.
- Invalidate a cached result only when Phase 1's `body_hash` for the containing
  directory changed, or the entry's `id` is new.
- Same failure-ledger discipline as §4.6.
- **ADR-0016 discipline applies to the harvester too** — mirror responses are
  untrusted bytes. Cap the central directory size read into memory (64 MiB is
  generous; a lying EOCD can declare anything), never panic on a malformed CD,
  and record-and-continue.

> Correction (#406): the per-id cache stores only **conclusive** outcomes —
> `ok`, `full_download`, `mirror_404_all`, `zip_parse_error` — facts about
> the archive entry itself. `no_range_support` and `fetch_error` are
> run-scoped (a budget/breaker state, or a transient mirror/transport blip —
> not a fact about the entry) and are deliberately never cached; they retry
> live on the next run against a fresh budget and fresh mirror state. Since
> the cache invalidates only on a `body_hash` change, caching a transient
> failure would otherwise make it permanent.

### 5.5 Edge cases — record, don't skip

Every one of these is a data point that feeds a §8 limit:

- Entries with **zero** `.wad` members (pk3/pk7-only, deh-only, source releases)
- Entries with **more than one** `.wad`
- Entries that are not zips at all
- Members matched **case-insensitively** — `.WAD`, `.Wad`, `.wad` all appear
- Members with compression methods other than stored/deflate
- Encrypted members
- Nested archives
- ZIP64 entries
- Mirror 404s (archive entry exists in the DB but not on that mirror)

### 5.6 Output

`data/idgames-wads.jsonl`:

```json
{
  "id": 15156,
  "dir": "levels/doom2/Ports/megawads/",
  "filename": "example.zip",
  "zip_size": 3145728,
  "date": "2019-04-02",
  "rating": 4.61,
  "votes": 38,
  "is_zip": true,
  "zip64": false,
  "member_count": 3,
  "wads": [
    {
      "name": "EXAMPLE.WAD",
      "compressed": 3102841,
      "uncompressed": 14680064,
      "method": "deflate",
      "encrypted": false
    }
  ],
  "other_members": ["EXAMPLE.TXT", "README.MD"],
  "mirror": "infania",
  "fetch_status": "ok"
}
```

`fetch_status` is a **closed enum** mirroring §5.5 — every edge case gets a
named value, or "record, don't skip" is unenforceable: `ok`, `not_zip`,
`mirror_404_all`, `no_range_support`, `full_download`, `zip_parse_error`,
`fetch_error`. Member-level cases (odd compression method, encryption) are
captured on the `wads[]` entries themselves.

**Realized outputs (#406):** the ledger lands at `data/wads-errors.jsonl`
(the same Phase-1 `LedgerEntry` shape, action `harvest-zips`); run
provenance and the §9.3 witnesses land at `data/wads-manifest.json`
(`bytes_transferred`, `zip64_entries`, `status_counts`, `unaccounted_entries`,
fallback accounting, `aborted`); the per-id resumability log lives at
`data/cache/zips-log.jsonl` (§5.4). `member_count` counts distinct
central-directory entries — `zip::ZipArchive` keys its parsed entries in an
`IndexMap<name, ..>`, so duplicate member names collapse and the last one
wins; an archive with duplicate names under-counts here by design.

---

## 6. Phase 3 — Statistics

**Command:** `xtask stats`

**Unit of analysis is one `.wad`, not one archive entry** — a user uploads one WAD
at a time.

### 6.1 Core distribution

Over `wads[].uncompressed`: `n, min, p50, p75, p90, p95, p99, p99.5, p99.9, max,
mean, stddev`, plus a log2 histogram.

Use a deterministic percentile method (nearest-rank on the sorted vector) and
document which. Percentile definitions differ and these numbers become production
constants.

### 6.2 Segmentations

- By top-level bucket: `levels/doom`, `levels/doom2`, `levels/heretic`,
  `levels/hexen`, `themes/`, and the per-game `levels/*/Ports/` subtrees
  (there is no top-level `ports/` — §4.2)
- By year, from `date` — UDMF and compressed nodes are post-2005 phenomena and
  the distribution shifts hard over time
- **Vote-weighted** — the size profile of *popular* files, a far better predictor
  of what users actually upload than a uniform sample. Weight by `votes`, and
  report the unweighted version alongside so the skew is visible.

### 6.3 Decision-driving counts

Each of these becomes a literal constant or a UX decision in §8:

- Fraction of entries with zero `.wad` members → the share of the archive the UI
  must reject even with zip support, and the size of the case for pk3
- Fraction with more than one `.wad`, plus the member-count distribution → sizes
  the member-picker UX
- **Max members per entry** and **max total declared uncompressed bytes per
  entry** → the zip envelope limits in §8.3
- Distribution of `uncompressed / zip_size` ratio, per member and per entry → the
  per-member compression-ratio ceiling, plus a sanity check on how badly the
  API's `size` field would have misled this decision
- Compression methods observed across all members → confirms the §8.3 allowlist
- ZIP64 entry count → confirms whether the §5.3 handling was load-bearing

### 6.4 Modern-outliers supplement

idgames enforces its own upload limits, and the largest modern projects live
elsewhere or arrive split — so the corpus's upper tail is truncated by the
archive's own cap, and constants derived from it alone would reject exactly the
modern megawads users most want to inspect. Decision: supplement with a small
hand-curated list of modern non-idgames releases (Cacoward-tier megawads),
analyzed with the same Phase 2 machinery where hosting permits, and reported
**separately** so the bias stays visible. The §8 constants must consider both
populations.

### 6.5 Outputs

- `data/stats-report.md` — human-readable, with the histograms and the recommended
  constants called out explicitly
- `data/stats.json` — machine-readable, for regression on future harvests
- `data/sweep-corpus.jsonl` — id, mirror URL, expected WAD names and uncompressed
  sizes. A ready-made fetch list for `CRUSTYWAD_SWEEP_DIR` and `cargo-fuzz` seeds.

Arguably the corpus manifest is worth more to the project than the statistics.

Stats outputs embed the input harvest-manifest ID and **no wall-clock
timestamps**, so re-running against unchanged inputs is byte-identical (§9.3).

**Realized outputs (#407):** `stats`/`harvest-outliers` land per this section
with: outliers analyzed via `xtask/outliers.toml` → `data/outliers-wads.jsonl`
+ `data/outliers-manifest.json` + `data/outliers-errors.jsonl` (no
full-download fallback — range-refusing hosts are ledgered, §6.4); wire-cap
statistics computed over ls-laR listing sizes with the API-size delta
reported (§6.3's sanity check — the 2026-08-17 harvest ledgered ~7% of
entries as `size_mismatch`); envelope byte totals cover `.wad` members only
(phase 2 retains no non-wad member sizes); `stats` reads only local files
and reruns byte-identical on unchanged inputs (§9.3).

---

## 7. Reproducibility

Every output file carries the harvest manifest ID from §4.7. A statistics report
that can't be traced to a specific archive snapshot is not defensible as the basis
for a production constant.

Reruns should be diffable: `stats.json` from two harvests should be comparable to
show what moved.

---

## 8. Web UI upload limits

**Scope note:** this section is *informative* — it records the limits design the
harvest exists to parameterize. It is consumed by the future web-UI epic and its
ADR, not implemented by this one.

The UI accepts **`.wad` and `.zip`**. Zip is the primary path in practice —
essentially every idgames download is a zip, and requiring users to unzip first
puts friction on the main acquisition route.

Both paths converge on the same endpoint: a validated WAD in a temp file on disk,
parsed via `Wad::from_path_mapped`.

### 8.1 Two caps, not one

- **Wire cap** — bytes accepted over the network. From p99.5 of `zip_size` for zip
  uploads, p99.5 of uncompressed WAD size for direct `.wad` uploads.
- **Decoded cap** — uncompressed bytes of the selected WAD. Identical regardless
  of path, because it bounds the same downstream work.

Expect the decoded cap to land meaningfully higher than intuition suggests, since
you're measuring uncompressed bytes rather than the API's `size` field.

Both caps need a documented override path for the rare legitimate outlier.

### 8.2 Type detection

Magic bytes, never extension or `Content-Type`:

| Bytes | Meaning |
| --- | --- |
| `IWAD` / `PWAD` | Direct WAD path |
| `PK\x03\x04` | Zip path |
| `PK\x05\x06` | Empty archive — reject |
| `PK\x07\x08` | Spanned archive — reject |
| anything else | Reject |

### 8.3 Zip handling

**Central directory first.** Parse the CD and apply every envelope limit against
declared sizes *before* extracting a byte. Same discipline as the harvester — a
bomb gets rejected on metadata alone.

**Then enforce again during extraction.** The CD is attacker-controlled and can
lie. Wrap the member reader in `reader.take(decoded_cap + 1)` and fail on
hitting the ceiling. Declared size is a fast rejection, not a guarantee.

**Envelope limits** — derive constants from §6.3:

| Limit | Source |
| --- | --- |
| Max archive size on the wire | p99.5 of `zip_size` |
| Max member count | observed max, with headroom |
| Max total declared uncompressed bytes | observed max, with headroom |
| Max single WAD uncompressed | decoded cap |
| Max per-member compression ratio | observed ratio distribution |

**Compression method allowlist:** stored (0) and deflate (8) only. Reject bzip2,
LZMA, zstd, ppmd, and everything else. The archive predates all of them; each one
allowed is another decompressor's attack surface for zero benefit.

**Encrypted members:** general purpose bit flag bit 0 set → reject the upload.
Do not prompt for a password.

**Nested archives:** refuse. Depth 1 only.

**Path traversal — ignore member paths entirely.** Never join a member name to a
filesystem path. Take the basename for display only; extract to a temp file with a
name *you* generate. This eliminates zip-slip, absolute paths, `..` segments,
backslash separators, and embedded nulls in one move, and is simpler and stricter
than sanitizing. (`enclosed_name()` exists for the sanitizing approach; you
shouldn't need it.)

**Member selection:**

- Filter to `.wad`, case-insensitively.
- **Zero matches** → reject with a message naming what *was* found. "This archive
  contains a `.pk3`, which isn't supported yet" beats "no WAD found."
- **Exactly one** → proceed.
- **More than one** → show a picker. Do not guess; do not silently take the
  largest. Richness of the picker follows the §6.3 multi-WAD share.

**Free bonus:** idgames zips carry the UPLTEMPL `.txt`. Surfacing it beside the
analysis costs almost nothing and is exactly the context a user wants next to a
lump listing.

**Don't duplicate the limit constants.** The harvester's CD reader and the
server's CD reader are the same logic against the same crate. Whichever crate the
web backend lives in should own the constants.

### 8.4 Defenses for both paths

1. **Never buffer in the handler.** Stream to disk with a byte counter that aborts
   at the wire cap, then `Wad::from_path_mapped`. The `mmap` feature exists for
   exactly this workload.
2. **`Limits` is the actual safety boundary.** A small WAD can explode during
   decode: a zlib `ZNODES` stream, a pathological TEXTUREx composition, a Doom 64
   PNG lump. `max_decoded_pixels` and `max_decoded_node_bytes` are what stand
   between you and an OOM, and neither correlates with file size. This is a
   *second* amplification stage after zip inflation — bound them independently
   rather than assuming one cap covers both.
3. **Nesting depth.** Doom 64 stores maps as nested WADs inside `MAPxx` lumps.
   "It's a valid WAD" is insufficient — bound the recursion explicitly.
4. **Per-request CPU/wall timeout**, derived from Criterion benches. A
   parse-time-vs-size curve over the harvested corpus gives a defensible number
   instead of a guess.
5. **Temp file lifecycle.** Extracted WADs need cleanup that survives a panic
   mid-parse, plus a disk quota independent of per-request caps. Concurrent
   uploads × decoded cap is what fills the volume.

**On bandwidth:** almost certainly not the constraint. Egress at hobbyist traffic
is cheap even with a generous cap. The numbers that bite are *concurrent parses ×
peak RSS* and *concurrent extractions × decoded cap on disk*. Measure both before
tuning upload size.

### 8.5 First-harvest record (2026-08-18)

The first full harvest ran to completion and produced a recommended value for
every §8.1/§8.3 constant — the §9.3 phase-3 acceptance. This subsection is the
durable record of that run: the numbers the web-UI limits ADR (#447) starts
from. The §8 scope note applies — these are evidence, not adopted policy. The
full report (`xtask/data/stats-report.md`) and its inputs are local and
gitignored (§4.7 governance; internal-only per §10); `just harvest-stats`
against the manifests below reproduces it byte-identically.

**Provenance**

| field | value |
| --- | --- |
| phase-1 manifest | `harvest-20260817T023144Z` — 15,732 files, 182 dirs; ls-laR bootstrap: infania, Last-Modified Sun, 16 Aug 2026 05:05:18 GMT |
| phase-2 manifest | `harvest-zips-20260818T023856Z` — 14,633 cached + 1,099 live retries after #441's listing-size correction |
| outliers manifest | `harvest-outliers-20260817T065223Z` — 2 analyzed (freedoom, sigil-ii), 4 size-tail anchors refused by their hosts |
| stats git_rev | `26f2ffc` |
| coverage | 15,716 of 15,732 phase-1 entries `ok` (99.90%); the residual 16 (1 `fetch_error`, 5 `mirror_404_all`, 10 `zip_parse_error`) are accepted in #408's close-out |
| determinism | the §9.3 byte-identity criterion was witnessed by double runs on both the 2026-08-17 cold corpus and the 2026-08-18 corrected corpus |
| populations | 15,716 entries / 16,876 `.wad` members; zip wire-cap population 15,267; sweep corpus 15,267 entries |

**Recommendations** (verbatim from the report's §8 table):

| key | recommended | formula | source |
| --- | --- | --- | --- |
| wire_cap_zip | 67108864 (64 MiB) | pow2_ceil(max(idgames p99.5 listing zip size, outliers max zip size)) | idgames p99.5 zip_size_listing = 45414309; outliers max zip_size = 24143781 |
| wire_cap_wad | 134217728 (128 MiB) | pow2_ceil(max(idgames p99.5 wad uncompressed, outliers max wad uncompressed)) | idgames p99.5 wad_uncompressed = 89601066; outliers max wad_uncompressed = 28795076 |
| decoded_cap | 134217728 (128 MiB) | same as wire_cap_wad (bounds the same downstream work, §8.1) | mirrors wire_cap_wad: idgames p99.5 wad_uncompressed = 89601066; outliers max wad_uncompressed = 28795076 |
| max_member_count | 4096 | pow2_ceil(2 × max(idgames max member_count, outliers max member_count)) | idgames max member_count = 1200; outliers max member_count = 11 |
| max_entry_uncompressed_bytes | 1073741824 (1 GiB) | pow2_ceil(2 × max(idgames max entry_wad_total_uncompressed, outliers max_entry_total_uncompressed)) | idgames max entry_wad_total_uncompressed = 343146439; outliers max_entry_total_uncompressed = 57582824 (wad-member totals only — phase 2 retains no non-wad member sizes) |
| max_member_compression_ratio | 150 | 10 × ceil(2 × observed max deflate ratio / 10) | idgames max member_deflate ratio = 71.96 (n = 16628) (idgames population only) |
| compression_method_allowlist | stored, deflate | assertion: report every observed method with its count; recommend stored + deflate; flag any method beyond the allowlist | deflate = 16628, stored = 28, unsupported(1) = 4, unsupported(6) = 215, unsupported(9) = 1 — OBSERVED UNEXPECTED METHOD (.wad members only — non-wad archive members carry no recorded method); adopting stored+deflate would reject 220 members (1.3%): Shrink(1)=4, Implode(6)=215, Deflate64(9)=1 (idgames population only) |
| zip64_statement | zip64 handled | count of records with zip64: true | 0 zip64 entries observed (records); manifest agrees (idgames population only) |

**Caveats carried with the record** (full method notes live in the report):

- `member_count` counts *distinct* central-directory entry names — a duplicate
  member name under-counts by design.
- The envelope-byte and compression-method figures are a `.wad`-members-only
  census: phase 2 retains no size or method data for non-wad members, while
  §8.3's allowlist governs *every* member of an upload — so the method census
  under-represents the true method population.
- The upper size tail remains truncated by the idgames upload cap — every §6.4
  size-tail anchor host refused analysis this run, so the recommendations rest
  on the idgames population plus the two analyzed outliers.
- Zip-size statistics use the ls-laR listing size (mirror truth), not the API
  `size` field (§6.3's locked decision). The phase-1 ledger recorded 1,099
  `size_mismatch` findings; among entries where listing and API disagreed, the
  API would have misled by up to 31,438,807 bytes (max relative error ≈1,453×).

---

## 9. Testing and acceptance criteria

### 9.1 Unit tests (no network)

- `OneOrMany` round-trips: zero, one, and many elements, for both `dir` and
  `file`; plus the key-absent case.
- Lenient numeric deserialization: `size` as int and as string.
- Zip CD extraction against synthetic fixtures built in-test: standard zip,
  ZIP64 zip, zip with a 65535-byte comment (worst-case EOCD scan), zip with an
  encrypted member, zip with a non-deflate method, zip with zero `.wad` members,
  zip with multiple `.wad` members, and a member named `../../etc/passwd`.
- Range reader against a file-backed fake implementing the same interface —
  assert the *number* of reads, not just correctness. Regression on request count
  is the point of the caching.
- Percentile function against a known vector with a documented method.

### 9.2 Integration tests (network, opt-in)

Gate behind a feature or env var so CI never hits the network. Assert Phase 1
against one small known directory and Phase 2 against one known archive entry with
a hand-verified uncompressed size.

### 9.3 Acceptance criteria

**Phase 1 complete when:**
- Full traversal finishes with zero unresolved errors, or all errors are recorded
  in the failure ledger with reasons.
- A second run with a warm cache and no archive changes makes exactly one
  Doomworld API request (the `latestfiles` probe) and at most one mirror
  request — a conditional `ls-laR.gz` re-fetch, skipped entirely when the
  mirror answers 304 to `If-Modified-Since` (infania serves `Last-Modified`).
- No `email` data appears anywhere in the output or cache (`email` arrives
  in listings but is dropped at deserialization and scrubbed from cached
  bodies at write — §4.5/§4.7); `textfile`/`reviews` never arrive in
  listings at all. A schema test asserts no email-shaped field in any
  output; a cache test asserts a stored body carries none.
- `harvest-manifest.json` is present and complete.

**Phase 2 complete when:**
- Every entry from Phase 1 has a Phase 2 record or a ledger entry.
- Total bytes transferred is a small fraction of the archive's nominal size —
  if it approaches the real thing, the range reader is broken.
- At least one ZIP64 entry is correctly resolved, or the report explicitly states
  none were found.

**Phase 3 complete when:**
- `stats-report.md` states a recommended value for every constant in §8.1 and
  §8.3, each traceable to a specific statistic.
- Re-running against unchanged inputs produces byte-identical output.

---

## 10. Open items

- **pk3/pk7 support**, driven by the zero-`.wad` count from §6.3. Materially
  bigger than zip: pk3 is a resource archive with its own directory semantics,
  not a container around a WAD. "We already handle zip" does not carry over.
  Quantified and filed as #445 (2026-08-18): zero-`.wad` share 449 entries
  (2.86%).
- **Member picker richness** — whether it needs map names and lump counts or a
  plain filename list suffices. Driven by the multi-WAD share. Quantified and
  filed as #446 (2026-08-18): multi-WAD share 863 entries (5.49%).
- **Whether the web UI limits warrant their own ADR** once numbers are in. This
  spec covers the harvest; the limits are a design decision with trade-offs worth
  recording alongside ADR-0024 through ADR-0026. Numbers are in (§8.5); filed
  as #447 (2026-08-18).
- ~~**Strife**~~ — resolved 2026-08-03: `levels/strife` is in scope (§4.2,
  ADR-0028).
- ~~**Publishing the corpus manifest** as a release artifact~~ — resolved
  2026-08-18 (#408): stays internal-only; no external consumer needs
  `CRUSTYWAD_SWEEP_DIR` today. Revisit if one appears — if flipped on, only the
  PII-free trio ships (§4.7).
- **UPLTEMPL port-targeting fields** — deliberately forgone along with
  full-record fetching (§4.3). Revisit only if a concrete need for
  metadata-derived format segmentation appears; the sweep corpus answers format
  questions better.
