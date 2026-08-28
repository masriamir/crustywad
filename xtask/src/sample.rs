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
//! This module currently holds only the frame filter and the deterministic
//! draw; manifest I/O, the download loop, and the CLI subcommand land in
//! later tasks (Task A4 wires the CLI and removes the `dead_code` allow
//! below).

// Task A4 wires the CLI subcommand and calls `frame`/`draw`; until then
// nothing in this module is reachable from `main`, so clippy's dead-code
// lint fires on every item below.
#![allow(dead_code)]

use std::path::Path;

use anyhow::Context as _;
use serde::{Deserialize, Serialize};

use crate::schema::{FetchStatus, WadRecord};

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
pub(crate) fn read_manifest(path: &Path) -> Option<SampleManifest> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
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
}
