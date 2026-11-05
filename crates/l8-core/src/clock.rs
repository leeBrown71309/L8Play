//! Timestamp helpers. The database stores UTC instants as RFC 3339 text.

use std::time::SystemTime;

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

/// Current UTC time as RFC 3339 (`2026-11-04T21:12:00Z`).
pub fn now_rfc3339() -> String {
    to_rfc3339(SystemTime::now())
}

pub fn to_rfc3339(instant: SystemTime) -> String {
    OffsetDateTime::from(instant).format(&Rfc3339).unwrap_or_default()
}

/// RFC 3339 instant `seconds` from now (negative values are in the past).
pub fn now_plus_seconds_rfc3339(seconds: i64) -> String {
    let instant = OffsetDateTime::now_utc() + time::Duration::seconds(seconds);
    instant.format(&Rfc3339).unwrap_or_default()
}

/// Unix seconds (as found in Steam's config files) to RFC 3339; `None` for 0 or out-of-range values.
pub fn unix_to_rfc3339(seconds: i64) -> Option<String> {
    if seconds <= 0 {
        return None;
    }
    OffsetDateTime::from_unix_timestamp(seconds).ok().and_then(|instant| instant.format(&Rfc3339).ok())
}

/// Seconds between two RFC 3339 timestamps (0 if either is unreadable or the order is reversed).
pub fn seconds_between(start: &str, end: &str) -> i64 {
    match (OffsetDateTime::parse(start, &Rfc3339), OffsetDateTime::parse(end, &Rfc3339)) {
        (Ok(start), Ok(end)) => (end - start).whole_seconds().max(0),
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_unix_seconds() {
        assert_eq!(unix_to_rfc3339(1_793_826_720).as_deref(), Some("2026-11-04T21:12:00Z"));
        assert_eq!(unix_to_rfc3339(0), None);
    }

    #[test]
    fn measures_durations() {
        assert_eq!(seconds_between("2026-11-04T20:00:00Z", "2026-11-04T21:12:30Z"), 4350);
        assert_eq!(seconds_between("2026-11-04T21:00:00Z", "2026-11-04T20:00:00Z"), 0);
        assert_eq!(seconds_between("garbage", "2026-11-04T20:00:00Z"), 0);
    }
}
