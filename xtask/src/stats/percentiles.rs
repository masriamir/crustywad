//! Deterministic percentile, histogram, and ratio primitives (DESIGN.md
//! §6.1). Every function here is pure and total over its inputs: no I/O,
//! no randomness, no wall-clock reads — the same corpus slice always
//! produces the same numbers, which is what makes `data/stats.json`
//! byte-identical across reruns (§9.3).

use std::collections::BTreeMap;

/// The 50th percentile, in tenths-of-a-percent (§6.1).
pub const P50: u32 = 500;
/// The 75th percentile, in tenths-of-a-percent (§6.1).
pub const P75: u32 = 750;
/// The 90th percentile, in tenths-of-a-percent (§6.1).
pub const P90: u32 = 900;
/// The 95th percentile, in tenths-of-a-percent (§6.1).
pub const P95: u32 = 950;
/// The 99th percentile, in tenths-of-a-percent (§6.1).
pub const P99: u32 = 990;
/// The 99.5th percentile, in tenths-of-a-percent (§6.1).
pub const P99_5: u32 = 995;
/// The 99.9th percentile, in tenths-of-a-percent (§6.1).
pub const P99_9: u32 = 999;

/// 1-indexed nearest-rank position for `n` sorted values at `p10`
/// (tenths-of-a-percent): `R = ceil(p10 * n / 1000)`, clamped to `[1, n]`.
/// Shared by [`nearest_rank`] and [`ratio_at`] so both use the exact same
/// rank arithmetic. Returns `None` for `n == 0`.
fn rank_index(n: usize, p10: u32) -> Option<usize> {
    if n == 0 {
        return None;
    }
    let n_u128 = u128::try_from(n).expect("usize fits u128 on every supported target");
    let rank = (u128::from(p10) * n_u128).div_ceil(1000);
    let rank = rank.clamp(1, n_u128);
    Some(usize::try_from(rank - 1).expect("rank fits usize: it is clamped to at most n"))
}

/// Nearest-rank percentile of `sorted` (ascending) at `p10` tenths-of-a-percent.
///
/// Method (§6.1, load-bearing — these numbers become production constants):
/// `R = ceil(p10 * n / 1000)`, 1-indexed, clamped to `[1, n]`; the result is
/// `sorted[R - 1]`. Returns `0` on an empty slice.
///
/// # Panics
/// Never in practice: `R` is clamped to `1..=n` and `n` originated as a
/// slice length, so `R - 1` always fits back into a `usize`.
#[must_use]
pub fn nearest_rank(sorted: &[u64], p10: u32) -> u64 {
    match rank_index(sorted.len(), p10) {
        Some(idx) => sorted[idx],
        None => 0,
    }
}

/// Vote-weighted nearest-rank percentile (§6.2) over `sorted` — `(value,
/// weight)` pairs ascending by value.
///
/// Walks cumulative weight and returns the first value whose running total
/// reaches `ceil(p10 * total_weight / 1000)`. Returns `0` when `sorted` is
/// empty or every weight is `0`.
#[must_use]
pub fn weighted_nearest_rank(sorted: &[(u64, u64)], p10: u32) -> u64 {
    let total: u128 = sorted.iter().map(|&(_, w)| u128::from(w)).sum();
    if total == 0 {
        return 0;
    }
    let target = (u128::from(p10) * total).div_ceil(1000);
    let mut cum: u128 = 0;
    for &(v, w) in sorted {
        cum += u128::from(w);
        if cum >= target {
            return v;
        }
    }
    sorted.last().map_or(0, |&(v, _)| v)
}

/// Mean and population standard deviation of `values`. Returns `(0.0, 0.0)`
/// on an empty slice.
///
/// Sums are accumulated exactly in `u128` before the single division to
/// `f64`, so the only floating-point operations are two divisions and one
/// correctly-rounded `sqrt` (IEEE 754) — the integer sums themselves never
/// lose precision.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "u128 sums of u64 corpus sizes into f64 mean/stddev: the precision loss is inherent \
              to reporting a statistical mean as a float and is documented here, not accidental"
)]
pub fn mean_stddev(values: &[u64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 0.0);
    }
    let n = values.len() as u128;
    let sum: u128 = values.iter().map(|&x| u128::from(x)).sum();
    let sum_sq: u128 = values.iter().map(|&x| u128::from(x) * u128::from(x)).sum();
    let mean = sum as f64 / n as f64;
    let var = (sum_sq as f64 / n as f64) - mean * mean;
    (mean, if var > 0.0 { var.sqrt() } else { 0.0 })
}

/// Vote-weighted mean and population standard deviation over `(value,
/// weight)` pairs (§6.2). Returns `(0.0, 0.0)` when `pairs` is empty or the
/// total weight is `0`.
///
/// Same shape as [`mean_stddev`]: `Σw`, `Σwx`, and `Σwx²` are accumulated
/// exactly in `u128` before the division to `f64`.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "u128 weighted sums into f64 mean/stddev: the precision loss is inherent to \
              reporting a statistical mean as a float and is documented here, not accidental"
)]
pub fn weighted_mean_stddev(pairs: &[(u64, u64)]) -> (f64, f64) {
    let total_w: u128 = pairs.iter().map(|&(_, w)| u128::from(w)).sum();
    if total_w == 0 {
        return (0.0, 0.0);
    }
    let sum_wx: u128 = pairs
        .iter()
        .map(|&(x, w)| u128::from(x) * u128::from(w))
        .sum();
    let sum_wx2: u128 = pairs
        .iter()
        .map(|&(x, w)| u128::from(x) * u128::from(x) * u128::from(w))
        .sum();
    let mean = sum_wx as f64 / total_w as f64;
    let var = (sum_wx2 as f64 / total_w as f64) - mean * mean;
    (mean, if var > 0.0 { var.sqrt() } else { 0.0 })
}

/// Log2 histogram of `sorted` (§6.1). Buckets are labelled `"0"` for the
/// zero value, then `"2^k-2^(k+1)"` for `k = floor(log2(x))` on `x >= 1`, in
/// ascending `k`; empty buckets are omitted from the result.
#[must_use]
pub fn log2_histogram(sorted: &[u64]) -> Vec<(String, u64)> {
    let mut buckets: BTreeMap<Option<u32>, u64> = BTreeMap::new();
    for &x in sorted {
        let key = if x == 0 {
            None
        } else {
            // floor(log2(x)) for x >= 1: the index of the highest set bit.
            Some(x.ilog2())
        };
        *buckets.entry(key).or_insert(0) += 1;
    }
    buckets
        .into_iter()
        .map(|(key, count)| {
            let label = key.map_or_else(|| "0".to_string(), |k| format!("2^{k}-2^{}", k + 1));
            (label, count)
        })
        .collect()
}

/// Sorts `(uncompressed, compressed)` pairs ascending by the exact ratio
/// `uncompressed / compressed`, comparing via a cross-multiplied `u128`
/// product rather than a lossy float division. Stable: equal ratios keep
/// their input order.
pub fn sort_ratio_pairs(pairs: &mut [(u64, u64)]) {
    pairs.sort_by(|&(u1, c1), &(u2, c2)| {
        (u128::from(u1) * u128::from(c2)).cmp(&(u128::from(u2) * u128::from(c1)))
    });
}

/// Nearest-rank percentile of the compression ratio `uncompressed /
/// compressed` over `sorted` pairs (already ordered by [`sort_ratio_pairs`])
/// at `p10` tenths-of-a-percent. Returns `0.0` on an empty slice.
///
/// Uses the same rank arithmetic as [`nearest_rank`] (§6.1) and then
/// divides the selected pair as `f64`.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    reason = "reporting a single (uncompressed, compressed) pair's ratio as f64; both operands \
              are corpus byte counts well within f64's exact integer range in practice"
)]
pub fn ratio_at(sorted: &[(u64, u64)], p10: u32) -> f64 {
    match rank_index(sorted.len(), p10) {
        Some(idx) => {
            let (u, c) = sorted[idx];
            u as f64 / c as f64
        }
        None => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_rank_documented_method() {
        // R = ceil(p10 * n / 1000), 1-indexed. n=10, p50 → R=5 → v[4].
        let v: Vec<u64> = (1..=10).collect();
        assert_eq!(nearest_rank(&v, P50), 5);
        assert_eq!(nearest_rank(&v, P99_9), 10);
        assert_eq!(nearest_rank(&v, P75), 8); // ceil(7.5)
        assert_eq!(nearest_rank(&[42], P50), 42); // n = 1
        assert_eq!(nearest_rank(&[], P50), 0); // empty convention
        assert_eq!(nearest_rank(&[7, 7, 7], P99), 7); // all-equal
    }

    #[test]
    fn weighted_rank_walks_cumulative_votes() {
        // values 10 (w1), 20 (w1), 30 (w98): unweighted p50 = 20, weighted p50 = 30.
        let pairs = [(10, 1), (20, 1), (30, 98)];
        assert_eq!(weighted_nearest_rank(&pairs, P50), 30);
        assert_eq!(nearest_rank(&[10, 20, 30], P50), 20);
    }

    #[test]
    fn weighted_rank_empty_and_zero_weight() {
        assert_eq!(weighted_nearest_rank(&[], P50), 0);
        assert_eq!(weighted_nearest_rank(&[(10, 0), (20, 0)], P50), 0);
    }

    #[test]
    fn mean_stddev_exact_sums() {
        let (m, s) = mean_stddev(&[2, 4, 4, 4, 5, 5, 7, 9]);
        assert!((m - 5.0).abs() < 1e-12);
        assert!((s - 2.0).abs() < 1e-12); // classic population-σ example
    }

    #[test]
    fn mean_stddev_empty() {
        assert_eq!(mean_stddev(&[]), (0.0, 0.0));
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "a single-element population has zero variance exactly (0.0 is not computed \
                  through any lossy operation), so bit-exact equality is the correct assertion"
    )]
    fn mean_stddev_single_value_has_zero_stddev() {
        let (m, s) = mean_stddev(&[42]);
        assert!((m - 42.0).abs() < 1e-12);
        assert_eq!(s, 0.0);
    }

    #[test]
    fn weighted_mean_stddev_matches_expansion_by_repetition() {
        // Weighted (10, w2), (20, w1) must equal the unweighted mean/stddev
        // of the expanded multiset [10, 10, 20].
        let (wm, ws) = weighted_mean_stddev(&[(10, 2), (20, 1)]);
        let (m, s) = mean_stddev(&[10, 10, 20]);
        assert!((wm - m).abs() < 1e-9);
        assert!((ws - s).abs() < 1e-9);
    }

    #[test]
    fn weighted_mean_stddev_empty_and_zero_weight() {
        assert_eq!(weighted_mean_stddev(&[]), (0.0, 0.0));
        assert_eq!(weighted_mean_stddev(&[(10, 0), (20, 0)]), (0.0, 0.0));
    }

    #[test]
    fn histogram_buckets_and_zero() {
        let v = [0, 1, 1, 2, 3, 4, 1024];
        let h = log2_histogram(&v);
        assert_eq!(h[0], ("0".into(), 1));
        assert_eq!(h[1], ("2^0-2^1".into(), 2)); // 1, 1
        assert_eq!(h[2], ("2^1-2^2".into(), 2)); // 2, 3
        assert_eq!(h[3], ("2^2-2^3".into(), 1)); // 4
        assert_eq!(h[4], ("2^10-2^11".into(), 1)); // 1024; empty buckets omitted
        assert_eq!(h.len(), 5);
    }

    #[test]
    fn histogram_empty_input_yields_no_buckets() {
        assert_eq!(log2_histogram(&[]), Vec::<(String, u64)>::new());
    }

    #[test]
    fn ratio_sort_is_exact() {
        // 3/2 vs 149/100 vs 151/100: floats agree here, but the comparator must
        // be cross-multiplied u128 — include a tie: 2/1 == 4/2 (stable order).
        let mut p = [(151, 100), (3, 2), (149, 100), (4, 2), (2, 1)];
        sort_ratio_pairs(&mut p);
        assert_eq!(p[0], (149, 100));
        assert_eq!(p[1], (3, 2));
        assert_eq!(p[2], (151, 100));
        assert_eq!(&p[3..], &[(4, 2), (2, 1)]); // ties keep input order (stable sort)
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "ratio_at divides two small exact integers whose IEEE-754 quotient rounds to \
                  the same bit pattern as the literal (both go through the same correctly-rounded \
                  division), so bit-exact equality is the correct assertion here"
    )]
    fn ratio_at_selects_nearest_rank_pair() {
        let mut p = [(3, 2), (149, 100), (151, 100)]; // ratios 1.5, 1.49, 1.51
        sort_ratio_pairs(&mut p);
        assert_eq!(ratio_at(&p, P50), 1.5);
        assert_eq!(ratio_at(&p, P99_9), 1.51);
    }

    #[test]
    #[allow(
        clippy::float_cmp,
        reason = "ratio_at's empty-slice convention returns the exact literal 0.0, not a computed \
                  value, so bit-exact equality is the correct assertion here"
    )]
    fn ratio_at_empty() {
        assert_eq!(ratio_at(&[], P50), 0.0);
    }
}
