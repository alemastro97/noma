//! RRULE support (RFC 5545 subset): FREQ DAILY|WEEKLY|MONTHLY|YEARLY, INTERVAL,
//! BYDAY, BYMONTHDAY, UNTIL, COUNT.

use chrono::{DateTime, Utc};
use rrule::{Frequency, RRule, RRuleSet, Tz, Unvalidated};

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
        .validate(Utc::now().with_timezone(&Tz::UTC))
        .map(|_| ())
        .map_err(|e| DbError::InvalidRecurrence(e.to_string()))
}

/// The first occurrence of `rule` (anchored at `dtstart`) strictly after `after`,
/// or `None` when the rule is exhausted (COUNT/UNTIL).
pub fn next_after(
    rule: &str,
    dtstart: DateTime<Utc>,
    after: DateTime<Utc>,
) -> DbResult<Option<DateTime<Utc>>> {
    let start = dtstart.with_timezone(&Tz::UTC);
    let rrule = parse_unvalidated(rule)?
        .validate(start)
        .map_err(|e| DbError::InvalidRecurrence(e.to_string()))?;
    let set = RRuleSet::new(start)
        .rrule(rrule)
        .after(after.with_timezone(&Tz::UTC));
    let result = set.all(8);
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
    }

    #[test]
    fn next_after_weekly_monday() {
        // 2026-11-09 is a Monday.
        let start = utc(2026, 11, 9, 9);
        let next = next_after("FREQ=WEEKLY;BYDAY=MO", start, start).unwrap();
        assert_eq!(next, Some(utc(2026, 11, 16, 9)));
    }

    #[test]
    fn next_after_exhausted_count_is_none() {
        let start = utc(2026, 11, 9, 9);
        let next = next_after("FREQ=DAILY;COUNT=1", start, start).unwrap();
        assert_eq!(next, None);
    }
}
