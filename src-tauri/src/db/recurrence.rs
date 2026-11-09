//! RRULE support (RFC 5545 subset): FREQ DAILY|WEEKLY|MONTHLY|YEARLY, INTERVAL,
//! BYDAY, BYMONTHDAY, UNTIL, COUNT.
//!
//! Rules are expanded in the user's local time zone so BYDAY/BYMONTHDAY and the
//! wall-clock hour mean what the user said ("every Monday at 9") across DST.

use chrono::{DateTime, Utc};
use rrule::{Frequency, RRule, RRuleSet, Tz as RTz, Unvalidated};

use super::error::{DbError, DbResult};

const ALLOWED_PARTS: &[&str] = &[
    "FREQ",
    "INTERVAL",
    "BYDAY",
    "BYMONTHDAY",
    "UNTIL",
    "COUNT",
    "WKST",
];

fn parse_unvalidated(rule: &str) -> DbResult<RRule<Unvalidated>> {
    let body = rule.trim().strip_prefix("RRULE:").unwrap_or(rule.trim());
    if body.is_empty() {
        return Err(DbError::InvalidRecurrence("empty rule".into()));
    }
    for part in body.split(';').filter(|p| !p.is_empty()) {
        let key = part.split('=').next().unwrap_or("").to_ascii_uppercase();
        if !ALLOWED_PARTS.contains(&key.as_str()) {
            return Err(DbError::InvalidRecurrence(format!(
                "unsupported part {key}"
            )));
        }
    }
    let parsed = body
        .parse::<RRule<Unvalidated>>()
        .map_err(|e| DbError::InvalidRecurrence(e.to_string()))?;
    match parsed.get_freq() {
        Frequency::Daily | Frequency::Weekly | Frequency::Monthly | Frequency::Yearly => Ok(parsed),
        other => Err(DbError::InvalidRecurrence(format!(
            "unsupported FREQ={other}"
        ))),
    }
}

/// Checks that `rule` is within the supported subset.
pub fn validate_rrule(rule: &str) -> DbResult<()> {
    let parsed = parse_unvalidated(rule)?;
    parsed
        .validate(Utc::now().with_timezone(&RTz::UTC))
        .map(|_| ())
        .map_err(|e| DbError::InvalidRecurrence(e.to_string()))
}

/// The first occurrence of the series `rule` (started at `dtstart`, expanded in
/// `tz`) strictly after `after`, or `None` when the series is exhausted
/// (COUNT/UNTIL). `dtstart` must be the fixed series start, not the current
/// occurrence, or COUNT would restart on every completion.
pub fn next_after(
    rule: &str,
    dtstart: DateTime<Utc>,
    after: DateTime<Utc>,
    tz: chrono_tz::Tz,
) -> DbResult<Option<DateTime<Utc>>> {
    let rtz = RTz::Tz(tz);
    let start = dtstart.with_timezone(&rtz);
    let rrule = parse_unvalidated(rule)?
        .validate(start)
        .map_err(|e| DbError::InvalidRecurrence(e.to_string()))?;
    let set = RRuleSet::new(start)
        .rrule(rrule)
        .after(after.with_timezone(&rtz));
    let result = set.all(4);
    Ok(result
        .dates
        .into_iter()
        .map(|d| d.with_timezone(&Utc))
        .find(|d| *d > after))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn utc(y: i32, m: u32, d: u32, h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, m, d, h, 0, 0).unwrap()
    }

    #[test]
    fn validate_accepts_weekly_monday() {
        validate_rrule("FREQ=WEEKLY;BYDAY=MO").unwrap();
        validate_rrule("RRULE:FREQ=DAILY;INTERVAL=2;COUNT=5").unwrap();
    }

    #[test]
    fn validate_rejects_unsupported_freq() {
        assert!(matches!(
            validate_rrule("FREQ=HOURLY"),
            Err(DbError::InvalidRecurrence(_))
        ));
        assert!(matches!(
            validate_rrule("FREQ=WEEKLY;BYHOUR=9"),
            Err(DbError::InvalidRecurrence(_))
        ));
        assert!(validate_rrule("not a rule").is_err());
        assert!(validate_rrule("").is_err());
    }

    #[test]
    fn next_after_weekly_monday() {
        // 2026-11-09 is a Monday.
        let start = utc(2026, 11, 9, 9);
        let next = next_after("FREQ=WEEKLY;BYDAY=MO", start, start, chrono_tz::UTC).unwrap();
        assert_eq!(next, Some(utc(2026, 11, 16, 9)));
    }

    #[test]
    fn next_after_exhausted_count_is_none() {
        let start = utc(2026, 11, 9, 9);
        let next = next_after("FREQ=DAILY;COUNT=1", start, start, chrono_tz::UTC).unwrap();
        assert_eq!(next, None);
    }

    #[test]
    fn count_is_measured_from_series_start() {
        let start = utc(2026, 11, 9, 9);
        let rule = "FREQ=DAILY;COUNT=3";
        assert_eq!(
            next_after(rule, start, utc(2026, 11, 10, 9), chrono_tz::UTC).unwrap(),
            Some(utc(2026, 11, 11, 9))
        );
        assert_eq!(
            next_after(rule, start, utc(2026, 11, 11, 9), chrono_tz::UTC).unwrap(),
            None
        );
    }

    #[test]
    fn weekday_is_local_not_utc() {
        // Monday 20:00 in Los Angeles = Tuesday 04:00 UTC.
        let la: chrono_tz::Tz = "America/Los_Angeles".parse().unwrap();
        let start = la
            .with_ymd_and_hms(2026, 11, 9, 20, 0, 0)
            .unwrap()
            .with_timezone(&Utc);
        let next = next_after("FREQ=WEEKLY;BYDAY=MO", start, start, la)
            .unwrap()
            .unwrap();
        assert_eq!(
            next.with_timezone(&la).format("%a %H:%M").to_string(),
            "Mon 20:00"
        );
    }

    #[test]
    fn wall_clock_hour_survives_dst() {
        // Rome leaves DST on 2026-10-25: 09:00 stays 09:00 local.
        let rome: chrono_tz::Tz = "Europe/Rome".parse().unwrap();
        let start = rome
            .with_ymd_and_hms(2026, 10, 19, 9, 0, 0)
            .unwrap()
            .with_timezone(&Utc);
        let next = next_after("FREQ=WEEKLY", start, start, rome)
            .unwrap()
            .unwrap();
        assert_eq!(
            next.with_timezone(&rome).format("%H:%M").to_string(),
            "09:00"
        );
    }
}
