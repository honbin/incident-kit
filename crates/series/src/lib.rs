//! Core logic for `series`: parse `timestamp value` points and map values to a
//! sparkline level. Kept free of I/O so it can be unit-tested without the binary.

use chrono::{DateTime, FixedOffset};

/// Parse one `RFC3339 <ws> number` line into a `(timestamp, value)` point.
///
/// Any whitespace (tab or spaces) separates the two, so CloudWatch's
/// `--output text` drops straight in. Returns `None` for blank, malformed, or
/// non-finite lines.
pub fn parse_point(line: &str) -> Option<(DateTime<FixedOffset>, f64)> {
    let mut tokens = line.split_whitespace();
    let ts = tstamp::parse_line(tokens.next()?)?;
    let value: f64 = tokens
        .next()?
        .parse()
        .ok()
        .filter(|v: &f64| v.is_finite())?;
    Some((ts, value))
}

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
    fn parses_tab_and_space_separated() {
        assert_eq!(
            parse_point("2026-09-03T10:20:00+09:00\t12").map(|p| p.1),
            Some(12.0)
        );
        assert_eq!(
            parse_point("2026-09-03T10:20:00+09:00   47").map(|p| p.1),
            Some(47.0)
        );
        assert_eq!(
            parse_point("2026-09-03T10:20:00+09:00\t0.145").map(|p| p.1),
            Some(0.145)
        );
    }

    #[test]
    fn rejects_missing_or_bad_value() {
        assert!(parse_point("2026-09-03T10:20:00+09:00").is_none()); // no value
        assert!(parse_point("2026-09-03T10:20:00+09:00\tabc").is_none());
        assert!(parse_point("2026-09-03T10:20:00+09:00\tNaN").is_none());
        assert!(parse_point("not-a-timestamp\t5").is_none());
        assert!(parse_point("").is_none());
    }

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
