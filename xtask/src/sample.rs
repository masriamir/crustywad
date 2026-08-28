//! `harvest-sample` — a seeded, reproducible sample of the map-bearing
//! corpus, fully downloaded for offline sweeps (DESIGN.md §6.6).
//!
//! The consumer is crustygen's expressibility sweep, which must be able to
//! re-run against *the same maps* release after release. So the draw is
//! deterministic from a seed with a self-contained generator (not
//! `fastrand`, whose stream is not a cross-version stability contract),
//! and the manifest records everything needed to rebuild the sample on
//! another machine: seed, count, the frame's row count, and a hash of the
//! fetch list the frame was cut from.
//!
//! The module holds the frame filter and deterministic draw, manifest I/O,
//! the download loop, and the [`run`] orchestrator that ties them together
//! for the `harvest-sample` CLI subcommand.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use crate::schema::{FetchStatus, WadRecord};
use crate::zips::FALLBACK_PER_ENTRY_CAP;
use crate::zips::inspect::FetchFailure;
use crate::zips::range_reader::{MirrorRanges, TransferCounters};

/// The sampling frame: map-bearing entries only — a successful phase-2
/// read with at least one `.wad` member. Order is the fetch list's own
/// (sorted by `id`), which the draw depends on.
pub(crate) fn frame(records: Vec<WadRecord>) -> Vec<WadRecord> {
    records
        .into_iter()
        .filter(|r| r.fetch_status == FetchStatus::Ok && !r.wads.is_empty())
        .collect()
}

/// splitmix64 — Steele, Lea & Flood's public-domain mixer. Chosen for
/// being ~5 lines, dependency-free, and stable forever; statistical
/// quality beyond "well mixed" is irrelevant to a corpus sample.
fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Draws `count` entries from `frame` without replacement: a partial
/// Fisher–Yates over the index range, driven by [`splitmix64`] from
/// `seed`. `count >= frame.len()` returns the whole frame (shuffled). The
/// modulo in the index pick carries a negligible bias; determinism, not
/// uniformity to the last bit, is the contract.
pub(crate) fn draw(frame: &[WadRecord], seed: u64, count: usize) -> Vec<WadRecord> {
    let n = frame.len();
    let take = count.min(n);
    let mut idx: Vec<usize> = (0..n).collect();
    let mut state = seed;
    for i in 0..take {
        let remaining = u64::try_from(n - i).expect("usize fits u64");
        let j = i + usize::try_from(splitmix64(&mut state) % remaining).expect("fits usize");
        idx.swap(i, j);
    }
    idx[..take].iter().map(|&i| frame[i].clone()).collect()
}

/// One sampled entry's outcome, as written to `sample-manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SampleEntry {
    pub id: u64,
    pub dir: String,
    pub filename: String,
    pub zip_size: u64,
    /// `"ok"`, `"skipped_present"` (already on disk at the declared size),
    /// or `"failed:<detail>"`.
    pub status: String,
}

/// `sample-manifest.json` — everything needed to rebuild this sample.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct SampleManifest {
    pub seed: u64,
    pub count: usize,
    /// Rows in the sampling frame the draw ran over.
    pub frame_rows: usize,
    /// `blake3:<hex>` of the fetch list file (`idgames-wads.jsonl`).
    pub fetch_list_hash: String,
    pub entries: Vec<SampleEntry>,
}

/// `blake3:<hex>` over the fetch list's bytes — the same convention
/// `cache.rs` uses for body hashes.
pub(crate) fn fetch_list_hash(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

/// On-disk name for a sampled zip: `<id>-<filename>`, so two archive
/// directories carrying the same filename cannot collide.
pub(crate) fn entry_filename(rec: &WadRecord) -> String {
    format!("{}-{}", rec.id, rec.filename)
}

/// Writes the manifest atomically (pretty JSON).
///
/// # Errors
/// Serialization or filesystem failure.
pub(crate) fn write_manifest(path: &Path, manifest: &SampleManifest) -> anyhow::Result<()> {
    let text = serde_json::to_string_pretty(manifest).context("serializing sample manifest")?;
    crate::cache::atomic_write(path, text.as_bytes())
        .with_context(|| format!("writing {}", path.display()))
}

/// Reads a prior manifest; `None` when missing or unparseable.
///
/// `write_manifest`'s read-side counterpart, kept only to prove the
/// manifest round-trips in `manifest_round_trips` below — the manifest's
/// production reader is crustygen (a separate repo), not xtask itself, so
/// there is no in-crate caller and no `--resume` path is planned.
#[cfg(test)]
pub(crate) fn read_manifest(path: &Path) -> Option<SampleManifest> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

/// The one network operation this module needs, abstracted so tests can
/// serve bytes without a mirror (the same shape `zips::EntrySource` takes
/// for the phase-2 fakes).
#[allow(async_fn_in_trait)]
pub(crate) trait SampleSource {
    /// Fetch the whole archive entry.
    ///
    /// # Errors
    /// [`FetchFailure`] after the source's own retries are exhausted.
    async fn download_full(&mut self, expected_size: u64) -> Result<Vec<u8>, FetchFailure>;
}

impl SampleSource for MirrorRanges {
    async fn download_full(&mut self, expected_size: u64) -> Result<Vec<u8>, FetchFailure> {
        MirrorRanges::download_full(self, expected_size).await
    }
}

/// Downloads every entry of `sample` into `out_dir` sequentially (one
/// outstanding request at a time — the sample is small and politeness
/// outranks throughput here), skipping an entry already present at its
/// declared size. Never aborts on one entry's failure: the outcome is
/// recorded and the loop continues, matching the harvest's "record, don't
/// skip" discipline. An entry (not already present at its declared size)
/// whose `zip_size` exceeds phase 2's [`FALLBACK_PER_ENTRY_CAP`] is
/// refused as a `failed:` status without contacting a mirror:
/// `download_full` buffers the whole zip in memory, and that cap is
/// phase 2's existing bound on the same buffering.
///
/// # Errors
/// Creating `out_dir` or writing a downloaded file — filesystem failures
/// only; network failures become `failed:` statuses.
pub(crate) async fn download_all<S, F>(
    sample: &[WadRecord],
    out_dir: &Path,
    make_source: F,
) -> anyhow::Result<Vec<SampleEntry>>
where
    S: SampleSource,
    F: Fn(&WadRecord) -> anyhow::Result<S>,
{
    std::fs::create_dir_all(out_dir).with_context(|| format!("creating {}", out_dir.display()))?;
    let mut entries = Vec::with_capacity(sample.len());
    for rec in sample {
        let target = out_dir.join(entry_filename(rec));
        let status = match std::fs::metadata(&target) {
            Ok(meta) if meta.len() == rec.zip_size => "skipped_present".to_owned(),
            _ if rec.zip_size > FALLBACK_PER_ENTRY_CAP => format!(
                "failed:zip_size {} exceeds the per-entry cap {}",
                rec.zip_size, FALLBACK_PER_ENTRY_CAP
            ),
            _ => match make_source(rec) {
                Err(e) => format!("failed:{e:#}"),
                Ok(mut source) => match source.download_full(rec.zip_size).await {
                    Ok(bytes) => {
                        crate::cache::atomic_write(&target, &bytes)
                            .with_context(|| format!("writing {}", target.display()))?;
                        "ok".to_owned()
                    }
                    Err(e) => format!("failed:{e}"),
                },
            },
        };
        tracing::info!(id = rec.id, file = %rec.filename, %status, "sample entry");
        entries.push(SampleEntry {
            id: rec.id,
            dir: rec.dir.clone(),
            filename: rec.filename.clone(),
            zip_size: rec.zip_size,
            status,
        });
    }
    Ok(entries)
}

/// `xtask/data/samples/<seed>-<count>/` — under the gitignored data root,
/// so nothing generated is ever committed (DESIGN.md §4.7).
pub(crate) fn default_out_dir(seed: u64, count: usize) -> PathBuf {
    crate::phase1::data_root()
        .join("samples")
        .join(format!("{seed}-{count}"))
}

/// Orchestrates one sample run: read the fetch list, cut the frame, draw,
/// download, write the manifest. Reports the outcome counts and fails
/// (exit 1) when any entry failed, after the manifest is on disk.
///
/// # Errors
/// A missing fetch list, an empty frame, a filesystem failure, or — after
/// the manifest is written — at least one failed download.
pub(crate) fn run(seed: u64, count: usize, out: Option<PathBuf>) -> anyhow::Result<()> {
    let fetch_list = crate::phase1::output_dir(false).join("idgames-wads.jsonl");
    let text = std::fs::read_to_string(&fetch_list).with_context(|| {
        format!(
            "no fetch list at {} — run `just harvest-zips` first",
            fetch_list.display()
        )
    })?;
    // Hash and parse the same read (not two separate `fs::read`s): if the
    // file changed between them, the recorded hash would not describe the
    // frame actually sampled.
    let hash = fetch_list_hash(text.as_bytes());
    let records = crate::schema::parse_wads_jsonl(&text, &fetch_list);
    drop(text);
    let frame = frame(records);
    anyhow::ensure!(!frame.is_empty(), "the sampling frame is empty");
    let frame_rows = frame.len();
    let sample = draw(&frame, seed, count);
    let out_dir = out.unwrap_or_else(|| default_out_dir(seed, count));
    tracing::info!(seed, count, frame_rows, out = %out_dir.display(), "drawing sample");

    let client = crate::mirror::build_zips_http()?;
    let counters = Arc::new(TransferCounters::new());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("building tokio runtime")?;
    let entries = runtime.block_on(download_all(&sample, &out_dir, |rec| {
        MirrorRanges::new(
            client.clone(),
            &rec.dir,
            &rec.filename,
            rec.zip_size,
            Arc::clone(&counters),
        )
    }))?;

    let manifest = SampleManifest {
        seed,
        count,
        frame_rows,
        fetch_list_hash: hash,
        entries,
    };
    write_manifest(&out_dir.join("sample-manifest.json"), &manifest)?;
    let failed = manifest
        .entries
        .iter()
        .filter(|e| e.status.starts_with("failed:"))
        .count();
    tracing::info!(
        downloaded = manifest.entries.len() - failed,
        failed,
        "sample complete"
    );
    anyhow::ensure!(failed == 0, "{failed} entries failed — see the manifest");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(id: u64, status: FetchStatus, wads: usize) -> WadRecord {
        WadRecord {
            id,
            dir: "levels/doom/a/".into(),
            filename: format!("f{id}.zip"),
            zip_size: 100 + id,
            date: "1994-01-01".into(),
            rating: None,
            votes: 0,
            is_zip: true,
            zip64: false,
            member_count: 1,
            wads: (0..wads)
                .map(|k| crate::schema::WadMember {
                    name: format!("W{k}.WAD"),
                    compressed: 1,
                    uncompressed: 2,
                    method: "stored".into(),
                    encrypted: false,
                })
                .collect(),
            other_members: vec![],
            mirror: "infania".into(),
            fetch_status: status,
        }
    }

    #[test]
    fn frame_keeps_only_ok_entries_with_wads() {
        let records = vec![
            rec(1, FetchStatus::Ok, 1),
            rec(2, FetchStatus::Ok, 0),
            rec(3, FetchStatus::FetchError, 1),
            rec(4, FetchStatus::FullDownload, 1),
            rec(5, FetchStatus::Ok, 2),
        ];
        let ids: Vec<u64> = frame(records).iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![1, 5]);
    }

    #[test]
    fn draw_is_deterministic_for_a_seed_and_differs_across_seeds() {
        let frame: Vec<WadRecord> = (1..=50).map(|i| rec(i, FetchStatus::Ok, 1)).collect();
        let a: Vec<u64> = draw(&frame, 7, 10).iter().map(|r| r.id).collect();
        let b: Vec<u64> = draw(&frame, 7, 10).iter().map(|r| r.id).collect();
        let c: Vec<u64> = draw(&frame, 8, 10).iter().map(|r| r.id).collect();
        assert_eq!(a, b, "same seed, same sample");
        assert_ne!(a, c, "different seed, different sample");
        assert_eq!(a.len(), 10);
        let mut dedup = a.clone();
        dedup.sort_unstable();
        dedup.dedup();
        assert_eq!(dedup.len(), 10, "without replacement");
    }

    #[test]
    fn draw_pins_the_generator_stream() {
        // Pins splitmix64 + the partial Fisher–Yates: if either changes,
        // every recorded sample seed silently stops reproducing.
        let frame: Vec<WadRecord> = (1..=10).map(|i| rec(i, FetchStatus::Ok, 1)).collect();
        let ids: Vec<u64> = draw(&frame, 42, 3).iter().map(|r| r.id).collect();
        assert_eq!(ids, vec![4, 3, 5]);
    }

    #[test]
    fn draw_caps_at_the_frame_size() {
        let frame: Vec<WadRecord> = (1..=3).map(|i| rec(i, FetchStatus::Ok, 1)).collect();
        assert_eq!(draw(&frame, 1, 10).len(), 3);
        assert!(draw(&[], 1, 10).is_empty());
    }

    #[test]
    fn fetch_list_hash_is_blake3_prefixed_and_stable() {
        let h = fetch_list_hash(b"{}\n");
        assert!(h.starts_with("blake3:"));
        assert_eq!(h.len(), "blake3:".len() + 64);
        assert_eq!(h, fetch_list_hash(b"{}\n"));
        assert_ne!(h, fetch_list_hash(b"{ }\n"));
    }

    #[test]
    fn entry_filename_prefixes_the_id() {
        assert_eq!(entry_filename(&rec(11, FetchStatus::Ok, 1)), "11-f11.zip");
    }

    #[test]
    fn manifest_round_trips() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sample-manifest.json");
        let manifest = SampleManifest {
            seed: 42,
            count: 1,
            frame_rows: 9,
            fetch_list_hash: fetch_list_hash(b"x"),
            entries: vec![SampleEntry {
                id: 11,
                dir: "levels/doom/a/".into(),
                filename: "f11.zip".into(),
                zip_size: 111,
                status: "ok".into(),
            }],
        };
        write_manifest(&path, &manifest).unwrap();
        assert_eq!(read_manifest(&path), Some(manifest));
        assert_eq!(read_manifest(&dir.path().join("missing.json")), None);
    }

    struct FakeSource {
        outcome: Result<Vec<u8>, String>,
    }

    impl SampleSource for FakeSource {
        fn download_full(
            &mut self,
            _expected_size: u64,
        ) -> impl std::future::Future<Output = Result<Vec<u8>, FetchFailure>> {
            std::future::ready(self.outcome.clone().map_err(FetchFailure::Http))
        }
    }

    #[tokio::test]
    async fn download_all_writes_ok_entries_records_failures_and_skips_present() {
        let dir = tempfile::tempdir().unwrap();
        let mut present = rec(1, FetchStatus::Ok, 1);
        present.zip_size = 3;
        std::fs::write(dir.path().join("1-f1.zip"), b"abc").unwrap();
        let mut good = rec(2, FetchStatus::Ok, 1);
        good.zip_size = 2;
        let bad = rec(3, FetchStatus::Ok, 1);
        let sample = vec![present, good, bad];

        let entries = download_all(&sample, dir.path(), |r| {
            Ok(FakeSource {
                outcome: if r.id == 3 {
                    Err("boom".into())
                } else {
                    Ok(b"zz".to_vec())
                },
            })
        })
        .await
        .unwrap();

        let statuses: Vec<&str> = entries.iter().map(|e| e.status.as_str()).collect();
        assert_eq!(statuses, vec!["skipped_present", "ok", "failed:boom"]);
        assert_eq!(std::fs::read(dir.path().join("2-f2.zip")).unwrap(), b"zz");
        assert!(
            !dir.path().join("3-f3.zip").exists(),
            "a failed entry writes nothing"
        );
    }

    #[tokio::test]
    async fn download_all_records_a_source_construction_failure() {
        let dir = tempfile::tempdir().unwrap();
        let sample = vec![rec(9, FetchStatus::Ok, 1)];
        let entries = download_all(&sample, dir.path(), |_| -> anyhow::Result<FakeSource> {
            anyhow::bail!("bad url")
        })
        .await
        .unwrap();
        assert_eq!(entries[0].status, "failed:bad url");
    }

    #[tokio::test]
    async fn download_all_refuses_an_entry_over_the_per_entry_cap_without_a_network_call() {
        let dir = tempfile::tempdir().unwrap();
        let mut oversized = rec(4, FetchStatus::Ok, 1);
        oversized.zip_size = FALLBACK_PER_ENTRY_CAP + 1;
        let sample = vec![oversized];

        let entries = download_all(&sample, dir.path(), |_| -> anyhow::Result<FakeSource> {
            panic!("must not be called");
        })
        .await
        .unwrap();

        assert!(
            entries[0].status.starts_with("failed:zip_size"),
            "{}",
            entries[0].status
        );
        assert!(!dir.path().join("4-f4.zip").exists());
    }

    #[test]
    fn default_out_dir_lives_under_the_data_root() {
        let p = default_out_dir(42, 400);
        assert!(p.ends_with("data/samples/42-400"), "{}", p.display());
    }
}
