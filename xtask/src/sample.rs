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
}
