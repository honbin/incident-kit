//! Shared timestamp parsing for the toolbox.
//!
//! Holds exactly one concept: reading an RFC3339 timestamp out of a log line.

use chrono::{DateTime, FixedOffset};

/// Parse a timestamp from the first whitespace-delimited token of the line.
///
/// Forgiving: the token may be wrapped in double quotes, so both `jq` without
/// `-r` (`"…Z"`) and a quoted leading field followed by text (`"…Z" message`)
/// parse. Returns `None` for blank or unparseable lines; the caller decides
/// whether to count them as skipped.
pub fn parse_line(line: &str) -> Option<DateTime<FixedOffset>> {
    let token = line.split_whitespace().next()?.trim_matches('"');
    DateTime::parse_from_rfc3339(token).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_rfc3339_with_offset_and_fraction() {
        assert!(parse_line("2026-09-03T10:21:31.124+09:00").is_some());
        assert!(parse_line("2026-09-03T01:21:31Z").is_some());
    }

    #[test]
    fn strips_quotes_and_takes_leading_token() {
        assert!(parse_line("\"2026-09-03T10:21:31+09:00\"").is_some());
        assert!(parse_line("2026-09-03T10:21:31+09:00 SIGTERM received").is_some());
        // quoted leading field followed by text: the closing quote must be stripped
        assert!(parse_line("\"2026-09-03T10:21:31+09:00\" message").is_some());
    }

    #[test]
    fn rejects_blank_and_garbage() {
        assert!(parse_line("").is_none());
        assert!(parse_line("   ").is_none());
        assert!(parse_line("not a timestamp").is_none());
        // offset is required: a bare local time is rejected
        assert!(parse_line("2026-09-03T10:21:31").is_none());
    }
}
