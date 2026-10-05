//! Core logic for `align`: inner-join timestamped series. Kept free of I/O so it
//! can be unit-tested without spawning the binary.

use std::collections::BTreeMap;

/// Inner-join series on their key. For every key present in *all* series,
/// produce `(key, [v0, v1, ...])` with one value per series in input order.
/// Output is sorted by key (the first series is a `BTreeMap`, iterated in order).
pub fn inner_join<K: Ord + Copy>(series: &[BTreeMap<K, f64>]) -> Vec<(K, Vec<f64>)> {
    let Some((first, rest)) = series.split_first() else {
        return Vec::new();
    };
    let mut rows = Vec::new();
    for (&ts, &v0) in first {
        let mut values = Vec::with_capacity(series.len());
        values.push(v0);
        if rest.iter().all(|map| match map.get(&ts) {
            Some(&v) => {
                values.push(v);
                true
            }
            None => false,
        }) {
            rows.push((ts, values));
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(i64, f64)]) -> BTreeMap<i64, f64> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn keeps_only_timestamps_present_in_all() {
        let a = map(&[(10, 1.0), (20, 2.0), (30, 3.0)]);
        let b = map(&[(20, 20.0), (30, 30.0), (40, 40.0)]);
        let c = map(&[(20, 200.0), (30, 300.0)]);
        let rows = inner_join(&[a, b, c]);
        assert_eq!(
            rows,
            vec![(20, vec![2.0, 20.0, 200.0]), (30, vec![3.0, 30.0, 300.0]),]
        );
    }

    #[test]
    fn preserves_input_order_of_values() {
        let a = map(&[(1, 1.0)]);
        let b = map(&[(1, 2.0)]);
        assert_eq!(inner_join(&[a, b]), vec![(1, vec![1.0, 2.0])]);
    }

    #[test]
    fn single_series_passes_through() {
        let a = map(&[(1, 1.0), (2, 2.0)]);
        assert_eq!(inner_join(&[a]), vec![(1, vec![1.0]), (2, vec![2.0])]);
    }

    #[test]
    fn no_overlap_is_empty() {
        let a = map(&[(1, 1.0)]);
        let b = map(&[(2, 2.0)]);
        assert!(inner_join(&[a, b]).is_empty());
    }
}
