//! Core logic for `window`: test a timestamp against a range.
//!
//! Timestamp parsing lives in the shared `tstamp` crate. Kept free of I/O so it
//! can be unit-tested without spawning the binary.

use chrono::{DateTime, FixedOffset};

/// Inclusive range test. A `None` bound is unbounded on that side.
///
/// Comparison is on the instant (`DateTime` orders by UTC instant), so `from`
/// and `to` may each use any offset and still compare correctly against `ts`.
pub fn in_range(
    ts: DateTime<FixedOffset>,
    from: Option<DateTime<FixedOffset>>,
    to: Option<DateTime<FixedOffset>>,
) -> bool {
    if let Some(from) = from
        && ts < from
    {
        return false;
    }
    if let Some(to) = to
        && ts > to
    {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ts(s: &str) -> DateTime<FixedOffset> {
        tstamp::parse_line(s).unwrap()
    }

    #[test]
    fn bounded_both_sides_is_inclusive() {
        let from = ts("2026-09-03T10:20:00+09:00");
        let to = ts("2026-09-03T10:30:00+09:00");
        assert!(!in_range(
            ts("2026-09-03T10:19:59+09:00"),
            Some(from),
            Some(to)
        ));
        assert!(in_range(
            ts("2026-09-03T10:20:00+09:00"),
            Some(from),
            Some(to)
        )); // lower edge
        assert!(in_range(
            ts("2026-09-03T10:25:00+09:00"),
            Some(from),
            Some(to)
        ));
        assert!(in_range(
            ts("2026-09-03T10:30:00+09:00"),
            Some(from),
            Some(to)
        )); // upper edge
        assert!(!in_range(
            ts("2026-09-03T10:30:01+09:00"),
            Some(from),
            Some(to)
        ));
    }

    #[test]
    fn open_ended_bounds() {
        let from = ts("2026-09-03T10:20:00+09:00");
        assert!(in_range(ts("2026-09-03T23:00:00+09:00"), Some(from), None));
        assert!(!in_range(ts("2026-09-03T09:00:00+09:00"), Some(from), None));
        let to = ts("2026-09-03T10:20:00+09:00");
        assert!(in_range(ts("2026-09-03T00:00:00+09:00"), None, Some(to)));
        assert!(!in_range(ts("2026-09-03T11:00:00+09:00"), None, Some(to)));
    }

    #[test]
    fn compares_across_offsets() {
        // 10:25+09:00 == 01:25Z, which is inside [10:20+09:00, 10:30+09:00].
        let from = ts("2026-09-03T10:20:00+09:00");
        let to = ts("2026-09-03T10:30:00+09:00");
        assert!(in_range(ts("2026-09-03T01:25:00Z"), Some(from), Some(to)));
    }
}
