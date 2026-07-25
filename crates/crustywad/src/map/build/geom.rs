//! Pure geometry helpers shared by the node-building kernels (ADR-0026 §1).
//!
//! Everything here is a pure function of its arguments — no kernel state, no
//! I/O — extracted from the classic BSP kernel (`nodes.rs`) so the GL kernel
//! (`gl_nodes.rs`, #363) can share the exact same math. Moved code keeps its
//! original semantics verbatim; a behavior change here changes built lumps.
//! Bare `§` references in item docs (e.g. §B.2, §D) are ADR-0024 sections,
//! carried over verbatim from the classic kernel these helpers came from.

use crate::map::DoomWriteError;
use crate::map::build::NodeBuildError;

/// Rounds half away from zero to the nearest whole map unit (the write path's
/// rounding), returning `i32`. Inputs are bounded map coordinates, so the cast
/// cannot overflow.
#[allow(clippy::cast_possible_truncation)]
pub(super) fn round_half_away(value: f64) -> i32 {
    value.round() as i32
}

/// Euclidean distance between two integer points as `f64` (IEEE `sqrt` is
/// correctly rounded and deterministic — Global Constraint 8).
pub(super) fn distance(ax: i32, ay: i32, bx: i32, by: i32) -> f64 {
    f64::from(ax - bx).hypot(f64::from(ay - by))
}

/// The BAM angle of the vector `(dx, dy)` (§D): `atan2(dy, dx) / TAU * 65536`,
/// rounded and wrapped into `u16`. Axis-aligned and 45° directions are exact.
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(super) fn bam_angle(dx: i32, dy: i32) -> u16 {
    let radians = f64::from(dy).atan2(f64::from(dx));
    let scaled = radians / std::f64::consts::TAU * 65536.0;
    // `scaled` is within (-32768, 32768]; round then wrap into 0..65536.
    (scaled.round() as i64).rem_euclid(65536) as u16
}

/// The `[top, bottom, left, right]` union of two bboxes.
pub(super) fn bbox_union(a: [i32; 4], b: [i32; 4]) -> [i32; 4] {
    [
        a[0].max(b[0]),
        a[1].min(b[1]),
        a[2].min(b[2]),
        a[3].max(b[3]),
    ]
}

/// Narrows a whole-unit split-vertex coordinate to the extended stream's 16.16
/// fixed-point `i32` (`coord * 65536`), reversing the reader's `x / 65536.0`.
/// `build_nodes` creates split vertices as whole `i16`-range map units, so the
/// product always fits `i32`; a hand-constructed out-of-range value is a
/// defensive [`DoomWriteError::ValueOutOfRange`].
pub(super) fn fixed_16_16(
    value: f64,
    field: &'static str,
    index: usize,
) -> Result<i32, NodeBuildError> {
    let whole = i64::from(round_half_away(value));
    let fixed = whole * 65536;
    i32::try_from(fixed).map_err(|_| {
        NodeBuildError::Write(DoomWriteError::ValueOutOfRange {
            block: "vertex",
            field,
            index,
            value: fixed,
        })
    })
}

/// The exact `i64` cross product of the partition direction `(pdx, pdy)` with
/// the vector `(rx, ry)` from the partition start to a queried point: `> 0`
/// front, `< 0` back (§B.2, engine convention `R_PointOnSide`,
/// `src/doom/r_main.c:145`).
pub(super) fn cross_from_start(rx: i64, ry: i64, pdx: i64, pdy: i64) -> i64 {
    rx * pdy - ry * pdx
}

/// Whether cross product `cross` places its vertex **less than** 0.5 map units
/// from a line with squared length `len2` (`distance² < 1/4 ⇔ cross² < len²/4
/// ⇔ 4·cross² < len²`; exact in `i128`). The inequality is strict: a vertex
/// exactly 0.5 units off counts as front or back, not on the line.
pub(super) fn within_half_unit(cross: i64, len2: i128) -> bool {
    i128::from(cross) * i128::from(cross) * 4 < len2
}

/// The partition-candidate score (ADR-0024 §B.3, lower wins):
/// `split_cost · split + |front − back|`, plus the diagonal penalty
/// `(front + back + split) / aa_preference` when `diagonal` — a larger
/// `aa_preference` is a weaker penalty; `0` disables it (guarded divide).
#[allow(clippy::cast_possible_truncation)]
pub(super) fn partition_score(
    front: usize,
    back: usize,
    split: usize,
    split_cost: u32,
    aa_preference: u32,
    diagonal: bool,
) -> u64 {
    let mut score = u64::from(split_cost) * split as u64 + front.abs_diff(back) as u64;
    if diagonal && aa_preference > 0 {
        score += (front + back + split) as u64 / u64::from(aa_preference);
    }
    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_half_away_matches_the_write_path() {
        assert_eq!(round_half_away(0.5), 1);
        assert_eq!(round_half_away(-0.5), -1);
        assert_eq!(round_half_away(2.4), 2);
        assert_eq!(round_half_away(-2.6), -3);
        assert_eq!(round_half_away(0.0), 0);
    }

    #[test]
    fn distance_is_euclidean() {
        assert!((distance(0, 0, 3, 4) - 5.0).abs() < 1e-9);
        assert!(distance(10, 10, 10, 10).abs() < 1e-9);
        // 64 units straight east: exact.
        assert!((distance(0, 0, 64, 0) - 64.0).abs() < 1e-9);
    }

    #[test]
    fn bam_angle_is_exact_for_axis_aligned_and_45() {
        // The controller square-room angles.
        assert_eq!(bam_angle(1, 0), 0x0000); // east
        assert_eq!(bam_angle(0, 1), 0x4000); // north
        assert_eq!(bam_angle(-1, 0), 0x8000); // west
        assert_eq!(bam_angle(0, -1), 0xC000); // south
        // 45° diagonals are exact too (Global Constraint 8).
        assert_eq!(bam_angle(1, 1), 0x2000);
        assert_eq!(bam_angle(-1, 1), 0x6000);
        assert_eq!(bam_angle(-1, -1), 0xA000);
        assert_eq!(bam_angle(1, -1), 0xE000);
    }

    /// The `[top,bottom,left,right]` union takes the outermost edge on every
    /// side (new test — `bbox_union` previously had no direct unit test).
    #[test]
    fn bbox_union_takes_the_outermost_edges() {
        let a = [10, -5, -20, 30]; // top, bottom, left, right
        let b = [8, -9, -15, 45];
        assert_eq!(bbox_union(a, b), [10, -9, -20, 45]);
        // Union with itself is identity.
        assert_eq!(bbox_union(a, a), a);
    }

    #[test]
    fn fixed_16_16_encodes_whole_units() {
        assert_eq!(fixed_16_16(32.0, "x", 0).unwrap(), 32 * 65536);
        assert_eq!(fixed_16_16(-1.0, "y", 0).unwrap(), -65536);
        // i16::MAX * 65536 is the largest that fits i32.
        assert_eq!(
            fixed_16_16(f64::from(i16::MAX), "x", 0).unwrap(),
            32767 * 65536
        );
    }

    /// `cross_from_start` is the exact engine side test: positive = front/right of
    /// the direction vector, negative = back/left, zero = on the infinite line.
    #[test]
    fn cross_from_start_signs_match_the_engine_convention() {
        // Partition pointing east (pdx=10, pdy=0): a point below the line
        // (ry = -3) is front (cross = rx*0 - (-3)*10 = 30 > 0).
        assert_eq!(cross_from_start(5, -3, 10, 0), 30);
        // A point above (ry = 3) is back.
        assert_eq!(cross_from_start(5, 3, 10, 0), -30);
        // On the line: zero.
        assert_eq!(cross_from_start(7, 0, 10, 0), 0);
    }

    /// ADR-0024 §B.3 score: `split_cost·splits` + |front−back| + diagonal penalty.
    #[test]
    fn partition_score_matches_the_adr_0024_formula() {
        // 8*2 + |5-3| = 18, axis-aligned (no penalty).
        assert_eq!(partition_score(5, 3, 2, 8, 16, false), 18);
        // Diagonal penalty: + (5+3+2)/16 = 0 (integer division).
        assert_eq!(partition_score(5, 3, 2, 8, 16, true), 18);
        // Larger set makes the penalty visible: (20+12+0)/16 = 2.
        assert_eq!(partition_score(20, 12, 0, 8, 16, true), 8 + 2);
        // aa_preference = 0 means "no diagonal penalty" (guarded divide).
        assert_eq!(partition_score(20, 12, 0, 8, 0, true), 8);
        // split_cost = 0 degrades to balance-only.
        assert_eq!(partition_score(7, 2, 3, 0, 16, false), 5);
    }

    /// The 0.5-unit epsilon is strict: distance² < 1/4 ⇔ 4·cross² < len².
    #[test]
    fn within_half_unit_boundary_is_strict() {
        // Line of length 10 (len2 = 100). cross = 5 ⇒ distance exactly 0.5:
        // 4·25 = 100, NOT < 100 → false (counts as a side, not on-line).
        assert!(!within_half_unit(5, 100));
        // cross = 4 ⇒ distance 0.4 → true.
        assert!(within_half_unit(4, 100));
        // Zero cross is always on-line for any non-degenerate len2.
        assert!(within_half_unit(0, 1));
    }
}
