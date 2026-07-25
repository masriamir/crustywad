//! Pure geometry helpers shared by the node-building kernels (ADR-0026 §1).
//!
//! Everything here is a pure function of its arguments — no kernel state, no
//! I/O — extracted from the classic BSP kernel (`nodes.rs`) so the GL kernel
//! (`gl_nodes.rs`, #363) can share the exact same math. Moved code keeps its
//! original semantics verbatim; a behavior change here changes built lumps.

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
}
