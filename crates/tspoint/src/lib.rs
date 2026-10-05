//! Parse a `timestamp value` line into a `(timestamp, value)` point.
//!
//! Shared by the tools that read two-column series (`series`, `align`,
//! `timeline`). The timestamp half is `tstamp`'s job; this adds the value.

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
}
