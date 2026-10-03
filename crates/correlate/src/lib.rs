//! Core logic for `correlate`: match events across two time series.
//!
//! Kept free of I/O so it can be unit-tested without spawning the binary.
//! The `--window` duration parser also lives here.

/// Count how many `events` have at least one entry in `others` within `window`
/// of each other (same integer unit as the slices; inclusive, either direction).
///
/// This counts *matched events*, not matching pairs: an event with three
/// neighbours in range still counts once. So the result answers "what fraction
/// of these events co-occurred with the other stream?" — run it both ways to get
/// both fractions (they differ when the streams have different sizes).
///
/// Both slices must be sorted ascending. O(n + m): `j` only moves forward
/// because `events` is sorted, so each lower bound `e - window` is non-decreasing.
pub fn count_matched(events: &[i64], others: &[i64], window: i64) -> usize {
    debug_assert!(events.is_sorted(), "events must be sorted ascending");
    debug_assert!(others.is_sorted(), "others must be sorted ascending");
    let mut matched = 0;
    let mut j = 0;
    for &e in events {
        // saturating so timestamps near the i64 range (or a huge window) don't overflow
        while j < others.len() && others[j] < e.saturating_sub(window) {
            j += 1;
        }
        // others[j] is now the smallest entry >= e - window; if it is also
        // <= e + window it lies in range, which is all we need to know.
        if j < others.len() && others[j] <= e.saturating_add(window) {
            matched += 1;
        }
    }
    matched
}

/// Parse a window duration (`5s`, `2m`, `1h`, or a bare number meaning seconds)
/// into a non-negative number of seconds.
pub fn parse_duration_secs(s: &str) -> Result<i64, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty duration".to_string());
    }
    let (digits, mult) = if let Some(rest) = s.strip_suffix('s') {
        (rest, 1)
    } else if let Some(rest) = s.strip_suffix('m') {
        (rest, 60)
    } else if let Some(rest) = s.strip_suffix('h') {
        (rest, 3600)
    } else {
        (s, 1)
    };
    let value: i64 = digits
        .trim()
        .parse()
        .map_err(|_| format!("invalid duration: {s:?}"))?;
    if value < 0 {
        return Err(format!("duration must be non-negative: {s:?}"));
    }
    value
        .checked_mul(mult)
        .ok_or_else(|| format!("duration too large: {s:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_each_event_once_not_pairs() {
        // one event with three neighbours in range counts as a single match
        let events = [100];
        let others = [98, 99, 101];
        assert_eq!(count_matched(&events, &others, 2), 1);
    }

    #[test]
    fn matched_is_directional() {
        let a = [0, 5, 20];
        let b = [3, 40];
        assert_eq!(count_matched(&a, &b, 5), 2); // 0←3, 5←3, 20 has none
        assert_eq!(count_matched(&b, &a, 5), 1); // 3←0/5, 40 has none
    }

    #[test]
    fn boundary_is_inclusive() {
        assert_eq!(count_matched(&[10], &[15], 5), 1); // exactly +window
        assert_eq!(count_matched(&[10], &[16], 5), 0);
        assert_eq!(count_matched(&[10], &[5], 5), 1); // exactly -window
    }

    #[test]
    fn empty_sides() {
        assert_eq!(count_matched(&[], &[1, 2], 5), 0);
        assert_eq!(count_matched(&[1, 2], &[], 5), 0);
    }

    #[test]
    fn parses_durations() {
        assert_eq!(parse_duration_secs("5s"), Ok(5));
        assert_eq!(parse_duration_secs("2m"), Ok(120));
        assert_eq!(parse_duration_secs("1h"), Ok(3600));
        assert_eq!(parse_duration_secs("10"), Ok(10));
        assert_eq!(parse_duration_secs("0s"), Ok(0));
    }

    #[test]
    fn rejects_bad_durations() {
        assert!(parse_duration_secs("").is_err());
        assert!(parse_duration_secs("soon").is_err());
        assert!(parse_duration_secs("-1s").is_err());
    }

    #[test]
    fn rejects_overlarge_duration() {
        assert!(parse_duration_secs("9000000000000000h").is_err());
    }

    #[test]
    fn saturating_bounds_do_not_panic() {
        // timestamps near the i64 range, and a huge window, must not overflow
        assert_eq!(count_matched(&[i64::MAX], &[i64::MAX], 1), 1);
        assert_eq!(count_matched(&[i64::MIN], &[i64::MIN], 1), 1);
        assert_eq!(count_matched(&[i64::MAX], &[i64::MAX], i64::MAX), 1);
    }
}
