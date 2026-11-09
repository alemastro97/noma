//! Debug-only sample content mirroring `design/screens/Workspace.html`, so the
//! UI can be compared against the mockups. Only runs on an empty database.

use chrono::{DateTime, Datelike, Duration, TimeZone, Utc, Weekday};
use chrono_tz::Tz;
use rusqlite::Connection;

use super::error::DbResult;
use super::items::create_item;
use super::links::create_link;
use super::models::{ItemType, NewItem, Priority};
use super::projects::create_project;

fn at(tz: Tz, base: DateTime<Utc>, days: i64, hour: u32, minute: u32) -> DateTime<Utc> {
    let local = (base.with_timezone(&tz) + Duration::days(days)).date_naive();
    tz.from_local_datetime(&local.and_hms_opt(hour, minute, 0).expect("valid time"))
        .earliest()
        .map(|d| d.with_timezone(&Utc))
        .unwrap_or(base)
}

fn days_until(now: DateTime<Utc>, tz: Tz, weekday: Weekday) -> i64 {
    let today = now.with_timezone(&tz).weekday().num_days_from_monday() as i64;
    let target = weekday.num_days_from_monday() as i64;
    (target - today).rem_euclid(7)
}

/// Returns `false` (and does nothing) when the database already has items.
pub fn seed(conn: &Connection, now: DateTime<Utc>, tz: Tz) -> DbResult<bool> {
    let existing: u32 = conn.query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))?;
    if existing > 0 {
        return Ok(false);
    }

    let spring = create_project(conn, "Spring launch", "#79B8F5", now)?;
    create_project(conn, "Studio move", "#78DCC0", now)?;
    create_project(conn, "Reading list", "#F2BD73", now)?;
    super::boards::create_board(conn, "Thinking space", now)?;
    super::boards::create_board(conn, "Trips 2027", now)?;

    let on = |kind: ItemType, title: &str, x: f64, y: f64| NewItem {
        kind,
        title: title.into(),
        project_id: Some(spring.id.clone()),
        x: Some(x),
        y: Some(y),
        ..Default::default()
    };

    let project = create_item(
        conn,
        NewItem {
            body: Some("Site, deck and press kit for the April release.".into()),
            ..on(ItemType::ProjectRef, "Spring launch", 80.0, 110.0)
        },
        now,
    )?;
    let deck = create_item(
        conn,
        NewItem {
            priority: Some(Priority::High),
            due_at: Some(at(tz, now, days_until(now, tz, Weekday::Thu), 18, 0)),
            ..on(ItemType::Task, "Send revised deck to Marta", 520.0, 60.0)
        },
        now,
    )?;
    let printer = create_item(
        conn,
        NewItem {
            remind_at: Some(at(tz, now, 0, 16, 0)),
            ..on(
                ItemType::Reminder,
                "Call the printer about proofs",
                520.0,
                290.0,
            )
        },
        now,
    )?;
    let notes = create_item(
        conn,
        NewItem {
            body: Some(
                "She keeps reminders in three apps and trusts none of them. \
                 \u{201c}I just want to say it once and forget about it.\u{201d}"
                    .into(),
            ),
            tags: Some(vec!["research".into(), "voice".into()]),
            ..on(ItemType::Note, "Interview notes — Giulia", 110.0, 470.0)
        },
        now,
    )?;
    let idea = create_item(
        conn,
        NewItem {
            tags: Some(vec!["product".into()]),
            ..on(
                ItemType::Idea,
                "What if the orb dims while you\u{2019}re in a meeting?",
                860.0,
                110.0,
            )
        },
        now,
    )?;
    let review_day = days_until(now, tz, Weekday::Wed);
    let review = create_item(
        conn,
        NewItem {
            due_at: Some(at(tz, now, review_day, 14, 30)),
            ends_at: Some(at(tz, now, review_day, 15, 30)),
            ..on(ItemType::Event, "Design review", 860.0, 300.0)
        },
        now,
    )?;
    let photographer = create_item(
        conn,
        NewItem {
            due_at: Some(at(tz, now, days_until(now, tz, Weekday::Mon).max(1), 10, 0)),
            remind_at: Some(at(tz, now, days_until(now, tz, Weekday::Fri), 9, 0)),
            ..on(
                ItemType::Task,
                "Book photographer for launch shoot",
                600.0,
                470.0,
            )
        },
        now,
    )?;
    for t in [
        "Review the press kit copy",
        "Shortlist three venues",
        "Draft launch email",
    ] {
        create_item(
            conn,
            NewItem {
                kind: ItemType::Task,
                title: t.into(),
                project_id: Some(spring.id.clone()),
                ..Default::default()
            },
            now,
        )?;
    }

    create_link(conn, &project.id, &deck.id, None, now)?;
    create_link(conn, &project.id, &printer.id, None, now)?;
    create_link(conn, &project.id, &notes.id, None, now)?;
    create_link(conn, &review.id, &photographer.id, None, now)?;
    create_link(conn, &idea.id, &notes.id, None, now)?;

    // A few Inbox items and a weekly recurring reminder.
    create_item(
        conn,
        NewItem {
            kind: ItemType::Task,
            title: "Pick up the studio keys".into(),
            due_at: Some(at(tz, now, 0, 18, 0)),
            ..Default::default()
        },
        now,
    )?;
    create_item(
        conn,
        NewItem {
            kind: ItemType::Reminder,
            title: "Weekly review".into(),
            remind_at: Some(at(tz, now, days_until(now, tz, Weekday::Mon), 9, 0)),
            recurrence: Some("FREQ=WEEKLY;BYDAY=MO".into()),
            ..Default::default()
        },
        now,
    )?;
    create_item(
        conn,
        NewItem {
            kind: ItemType::Idea,
            title: "A calmer notification sound".into(),
            ..Default::default()
        },
        now,
    )?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    #[test]
    fn seeds_once() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        assert!(seed(&conn, now, chrono_tz::UTC).unwrap());
        assert!(!seed(&conn, now, chrono_tz::UTC).unwrap());
        let n: u32 = conn
            .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
            .unwrap();
        assert!(n >= 10);
    }
}
