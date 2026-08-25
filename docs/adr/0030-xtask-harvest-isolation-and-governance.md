# ADR-0030: xtask harvest — workspace isolation, data governance, and verified architecture

- **Status:** Accepted
- **Date:** 2026-08-12
- **Deciders:** @masriamir
- **Tracking issue:** https://github.com/masriamir/crustywad/issues/403

## Context and problem statement

The idgames corpus harvest (epic #401) is a network tool: it enumerates the
/idgames archive via the Doomworld API and reads zip central directories over
HTTP range requests. Its dependency profile (an HTTP client, an async runtime,
a zip parser) is alien to the library crate, whose dependency graph, MSRV
badge, and audit surface are deliberately small. The tool also touches two
things the library never does: personal data in third-party API responses,
and a shared, volunteer-run web service that deserves politeness guarantees.

The #402 spike verified the tool's factual assumptions against the live API
and mirrors before anything was built, and several were corrected — the spec
this ADR places in-repo (`xtask/DESIGN.md`) carries the corrections inline;
the spike issue carries the probe record. The corrections that force
decisions here:

- **`email` arrives in every `getcontents` listing record.** The draft
  posture — "PII is never fetched, so none ever touches disk" — is
  impossible: the API sends it whether wanted or not.
- The pre-spike mirror list was half dead (quaddicted and mancubus 404 real
  archive paths); the verified mirrors are `ftpmirror1.infania.net` and
  `www.gamers.org`, and both serve a root `ls-laR.gz` whose single ~418 KB
  response yields the full tree, every filename, and every zip size (462
  directories, 21,375 zips, 38.07 GiB on 2026-08-12).
- No idgames rsync module exists on either verified mirror — bulk transfer
  is HTTP or nothing.

Prior art in the repo: `fuzz/` already solves the alien-dependency problem —
its `Cargo.toml` carries an empty `[workspace]` table cutting it out of the
root workspace, its `Cargo.lock` is committed, and `.github/dependabot.yml`
carries a dedicated `/fuzz` cargo stanza precisely because no root manifest
reaches it. ADR-0016 already defines the hardening discipline for parsers fed
untrusted bytes.

## Decision

1. **`xtask/` is its own cargo workspace, on the `fuzz/` pattern.** An empty
   `[workspace]` table in `xtask/Cargo.toml`; `reqwest`/`tokio`/`zip` (major
   pinned to 8 — 8.6.0 is the latest stable as of the spike; 9.0 exists only
   as a pre-release) never enter the library's graph, MSRV resolution, or
   badges. The audit hole isolation opens is closed on the `fuzz/` pattern —
   a committed `xtask/Cargo.lock`, a `/xtask` cargo stanza in
   `.github/dependabot.yml` mirroring `/fuzz`'s, and a path-gated CI job
   (triggering on `xtask/**`) — and then one step further than fuzz
   currently goes: xtask's job also builds, tests, and runs
   `cargo deny check` for its workspace, because the root `security-deny`
   job audits only the root workspace's graph and a separate workspace
   escapes it (fuzz's path-gated job at the time ran fmt/clippy only; #428
   has since brought fuzz to the same posture).

2. **The harvest architecture is `ls-laR.gz`-first.** One request to a
   verified mirror bootstraps the complete tree, filenames, and zip sizes;
   the API is demoted to metadata-only enrichment (`rating`/`votes`/`date`/
   `description` via one `getcontents` call per directory, ≤462 calls). The
   verified mirror set is `ftpmirror1.infania.net` (primary; same-day
   `Last-Modified` on `ls-laR.gz` when probed) and `www.gamers.org`
   (fallback), both answering ranged GETs with `206`. The spike's API call
   rules bind the implementation: root listing via `action=getdirs`,
   trailing slash mandatory on every `getcontents` name, and a response
   whose `content.file` and `content.dir` are both `null` treated as a
   suspect path — never as an empty directory — because bad paths fail
   silently with those same nulls.

3. **Data governance is fetch-and-drop, not fetch-nothing.** `email` is
   dropped at deserialization — the record struct has no field for it — and
   the response cache scrubs `email` fields from bodies at write time (with
   `body_hash` computed over the scrubbed body), so `email` exists only in
   the transient HTTP response and reaches neither the outputs nor the
   on-disk cache. `description` is retained in local outputs for its metadata
   value — but as author-supplied free text it may itself embed personal
   data (addresses, emails), so it is treated as untrusted text, never
   asserted PII-free. All harvest output lives under gitignored
   `xtask/data/`; nothing generated is ever committed. Publishing remains
   deferred; if outputs are ever published, only the PII-free trio ships
   (`sweep-corpus.jsonl`, `stats.json`, `stats-report.md`), and no
   free-text field — `description` included — may appear in any of the
   three, per `xtask/DESIGN.md` §4.7.

4. **Politeness is a design invariant, not a tuning knob.** One request per
   second on a single connection against the API (a shared, volunteer-run PHP
   endpoint); a mainstream browser User-Agent (the API serves third-party
   clients and answered every spike probe under one, while the docs page
   blocks tool UAs); phase-2 ranged reads run against the mirror pool, not
   doomworld.com. The full-download fallback (for zips whose central
   directory cannot be read remotely) is budgeted at 2 GiB with a ~2%
   circuit-breaker abort, because a mirror/CDN change that turns "read the
   metadata" into "mirror the archive" must stop the run rather than
   silently escalate it. **This tool never mirrors the archive.** A future
   bulk corpus consumer is a separate tool with its own decision record —
   nothing here licenses it.

5. **Untrusted mirror bytes get ADR-0016 discipline.** Zip central
   directories fetched from mirrors are adversarial input: allocation
   bounded by a central-directory size cap, no panics on malformed data, and
   per-entry record-and-continue (a defective zip is recorded and skipped,
   never a run-aborting error and never a silent drop).

6. **The operational spec lives at `xtask/DESIGN.md`**, beside the tool it
   specifies, and is committed as of this ADR with the spike corrections
   folded in. `docs/adr/` records decisions; the DESIGN doc owns endpoint
   semantics, schemas, and phase mechanics. Corrections from future contact
   with reality land in the DESIGN doc; only decision-level changes reopen
   this ADR.

## Consequences

- The library's dependency graph, MSRV, and `deny` surface stay untouched by
  harvest work; the cost is a second workspace whose lock, dependabot
  stanza, and CI job must exist from the first xtask PR (they are part of
  #405's acceptance, not a follow-up).
- The PII guarantee rests on struct-shape discipline: deserialization must
  ignore unknown fields (the API may add more) while deliberately omitting
  `email`. A test asserting the serialized output schema carries no
  email-shaped field is cheap and belongs in phase 1.
- `ls-laR.gz`-first cuts phase 1 from a BFS crawl to ≤462 metadata calls
  (≈8 minutes at 1 req/s) and gives phase 2 a free per-file consistency
  check (API `size` vs listing size). Phase-2 ranged reads follow the DESIGN
  doc's §5.4 concurrency policy (4–8 connections against a mirror — the 1
  req/s invariant binds the API, not mirror payload reads): ~35k requests
  for `levels/doom*`, well under an hour, resumable by design.
- Mirror health is a monitored assumption, not a constant: the spike killed
  two of the draft's mirrors, so the tool records per-mirror failures and
  the DESIGN doc's §5.1 pool is expected to change over the years.
- Strife, Heretic, Hexen, and Doom 64 subtrees remain in scope for
  *enumeration* (sizes, statistics) but their maps carry different value
  universes than Doom's — downstream consumers segment by game subtree
  before interpreting anything (ADR-0028 provides identification for the
  Strife case).
