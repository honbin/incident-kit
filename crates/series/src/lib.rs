//! Core logic for `series`: map a `timestamp value` level onto a sparkline. Kept
//! free of I/O so it can be unit-tested without the binary.

/// Map `value` to a sparkline level `0..=7` by linear interpolation over
/// `[min, max]`. A flat series (`min == max`) maps everything to `0`.
pub fn block_index(value: f64, min: f64, max: f64) -> usize {
    let range = max - min;
    if range <= 0.0 {
        return 0;
    }
    (((value - min) / range) * 7.0).round() as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn block_index_spans_the_ramp() {
        assert_eq!(block_index(12.0, 12.0, 103.0), 0); // min -> lowest
        assert_eq!(block_index(103.0, 12.0, 103.0), 7); // max -> highest
        assert_eq!(block_index(47.0, 12.0, 103.0), 3); // (35/91)*7 ≈ 2.69 -> 3
    }

    #[test]
    fn flat_series_maps_to_zero() {
        assert_eq!(block_index(5.0, 5.0, 5.0), 0);
    }
}
