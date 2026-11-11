//! Voice commands: when a transcript is an instruction ("segna fatto chiamare
//! Marta", "sposta il dentista a domani", "cosa ho oggi?") NOMA acts on it
//! instead of capturing a note. The local model is asked for the intent only
//! when the words start with a likely command cue; the target card is found
//! with full-text search. Anything unsure falls back to a normal capture.
//! See docs/specs/voice.md.

use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex};

use chrono::{DateTime, Duration, NaiveDate, NaiveDateTime, TimeZone, Timelike, Utc};
use chrono_tz::Tz;
use rusqlite::Connection;
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::{AppHandle, Manager};

use super::extras;
use crate::ai::service::AiService;
use crate::commands::emit_items_changed;
use crate::db::{
    self,
    models::{Item, ItemFilter, ItemPatch, ItemType, Status},
};
use crate::reminders::{BRIEF_ID, Reminders, Toast};
use crate::state::{AppState, local_tz, now};

/// Toast ids for command confirmations: `voice:done:<item>` / `voice:moved:<item>`.
pub const TOAST_PREFIX: &str = "voice:";

#[derive(Debug, Clone, PartialEq)]
pub enum Intent {
    Done { target: String },
    Move { target: String, date: String },
    Today,
}

/// What a handled command reports back to the voice session.
#[derive(Debug, Clone, Default)]
pub struct Handled {
    pub item_id: Option<String>,
}

const DONE_CUES: &[&str] = &[
    "segna",
    "completa",
    "ho fatto",
    "ho finito",
    "fatto",
    "finito",
    "spunta",
    "mark",
    "complete",
    "check off",
    "done",
    "finished",
];
const MOVE_CUES: &[&str] = &[
    "sposta",
    "rimanda",
    "posticipa",
    "anticipa",
    "move",
    "postpone",
    "reschedule",
    "push",
];
const TODAY_CUES: &[&str] = &[
    "cosa ho",
    "che cosa ho",
    "che ho",
    "cosa c'è",
    "cosa c'e",
    "cos'ho",
    "what's on",
    "whats on",
    "what is on",
    "what do i have",
    "what have i got",
    "what's today",
];

/// Lower case, curly quotes straightened, a leading "NOMA," and punctuation removed.
pub fn normalize(text: &str) -> String {
    let mut s = text.trim().to_lowercase().replace(['’', '‘'], "'");
    for wake in ["ehi noma", "hey noma", "ok noma", "noma"] {
        if let Some(rest) = s.strip_prefix(wake) {
            s = rest.to_string();
        }
    }
    s.trim_start_matches(|c: char| !c.is_alphanumeric())
        .trim_end_matches(['?', '!', '.', ' '])
        .to_string()
}

fn starts_with_word(s: &str, cue: &str) -> bool {
    s.strip_prefix(cue)
        .is_some_and(|rest| rest.is_empty() || !rest.starts_with(char::is_alphanumeric))
}

/// The transcript begins like a command; only then is the model asked.
pub fn has_cue(text: &str) -> bool {
    let s = normalize(text);
    DONE_CUES
        .iter()
        .chain(MOVE_CUES)
        .chain(TODAY_CUES)
        .any(|c| starts_with_word(&s, c))
}

/// "What's on today?" in its common forms: answered without the model.
pub fn is_today_query(text: &str) -> bool {
    let s = normalize(text);
    let short = s.split_whitespace().count() <= 7;
    short
        && TODAY_CUES.iter().any(|c| starts_with_word(&s, c))
        && (s.contains("oggi") || s.contains("today"))
}

pub const SYSTEM_PROMPT: &str = r#"You read one short sentence the user said to their personal organizer and decide whether it is a command about an existing card. Notes may be Italian or English. Reply with compact single-line JSON that matches the schema.

intent:
- "done": the user says a card is finished ("segna fatto chiamare Marta", "mark call Marta done", "ho finito il report").
- "move": the user asks to move or postpone a card to another day or time ("sposta il dentista a domani", "move the standup to Friday at 10").
- "today": the user asks what is on today ("cosa ho oggi?", "what's on today?").
- "none": anything else, such as a new note, task, reminder or idea ("segna di chiamare Marta domani" is a new task, so "none"). When in doubt, "none".
target: for done and move, the words naming the card, without command words, articles or dates ("chiamare Marta", "dentista", "call Marta").
date: for move only, the new local date "YYYY-MM-DD", or "YYYY-MM-DDTHH:MM" when a time is said. Use the Calendar line for weekday names; today/oggi = Now's date, tomorrow/domani = +1 day, dopodomani = +2 days.

Examples with Now 2026-11-09T10:00 (Monday):
Said: segna fatto chiamare Marta
{"intent":"done","target":"chiamare Marta"}
Said: mark call the plumber as done
{"intent":"done","target":"call the plumber"}
Said: sposta il dentista a domani
{"intent":"move","target":"dentista","date":"2026-11-10"}
Said: move the design review to Thursday at 3pm
{"intent":"move","target":"design review","date":"2026-11-12T15:00"}
Said: cosa ho oggi?
{"intent":"today"}
Said: segna che il codice del portone è 4821
{"intent":"none"}"#;

pub fn schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "intent": { "type": "string", "enum": ["done", "move", "today", "none"] },
            "target": { "type": "string", "maxLength": 80 },
            "date": { "type": "string", "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}(T[0-9]{2}:[0-9]{2})?$" }
        },
        "required": ["intent"]
    })
}

pub fn user_prompt(text: &str, now: DateTime<Utc>, tz: Tz) -> String {
    let local = now.with_timezone(&tz);
    let today = local.date_naive();
    let calendar = (1..=8)
        .map(|d| {
            let day = today + Duration::days(d);
            format!("{} {}", day.format("%a"), day.format("%Y-%m-%d"))
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Now: {} ({})\nCalendar: {}\nSaid: {}",
        local.format("%Y-%m-%dT%H:%M"),
        local.format("%A"),
        calendar,
        text.trim()
    )
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct RawIntent {
    intent: String,
    target: Option<String>,
    date: Option<String>,
}

/// The model's answer as an intent; None for "none" or anything incomplete.
pub fn parse(output: &str) -> Option<Intent> {
    let raw: RawIntent = serde_json::from_str(output.trim()).ok()?;
    let target = raw
        .target
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty());
    match raw.intent.as_str() {
        "done" => Some(Intent::Done { target: target? }),
        "move" => Some(Intent::Move {
            target: target?,
            date: raw.date.filter(|d| !d.is_empty())?,
        }),
        "today" => Some(Intent::Today),
        _ => None,
    }
}

/// The one open card `target` names, or None when nothing or several match.
pub fn resolve(conn: &Connection, target: &str) -> Option<Item> {
    let open = |items: Vec<Item>| -> Vec<Item> {
        items
            .into_iter()
            .filter(|i| i.status != Status::Done && i.kind != ItemType::ProjectRef)
            .collect()
    };
    let mut found = open(db::items::search_items(conn, target, 8).ok()?);
    if found.is_empty() {
        // Drop short words ("il", "the", "a") the model may have kept.
        let words: Vec<&str> = target
            .split_whitespace()
            .filter(|w| w.chars().count() > 3)
            .collect();
        if words.is_empty() {
            return None;
        }
        found = open(db::items::search_items(conn, &words.join(" "), 8).ok()?);
    }
    match found.len() {
        0 => None,
        1 => found.pop(),
        _ => {
            let want = target.trim().to_lowercase();
            let mut exact = found
                .into_iter()
                .filter(|i| i.title.trim().to_lowercase() == want);
            let first = exact.next()?;
            exact.next().is_none().then_some(first)
        }
    }
}

fn to_utc(naive: NaiveDateTime, tz: Tz) -> Option<DateTime<Utc>> {
    (0..3).find_map(|h| {
        tz.from_local_datetime(&(naive + Duration::hours(h)))
            .earliest()
            .map(|d| d.with_timezone(&Utc))
    })
}

/// The patch that moves `item` to `date` (local "YYYY-MM-DD" or
/// "YYYY-MM-DDTHH:MM"). A bare date keeps the card's time of day. The due
/// date moves (and its reminder and end move with it); a card with only a
/// reminder moves the reminder.
pub fn retime(item: &Item, date: &str, now: DateTime<Utc>, tz: Tz) -> Option<ItemPatch> {
    let date = date.trim();
    let primary = item.due_at.or(item.remind_at);
    let target = if let Ok(dt) = NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M") {
        to_utc(dt, tz)?
    } else {
        let day = NaiveDate::parse_from_str(date, "%Y-%m-%d").ok()?;
        let time = primary
            .map(|p| p.with_timezone(&tz).time())
            .unwrap_or_else(|| chrono::NaiveTime::from_hms_opt(9, 0, 0).expect("09:00"));
        to_utc(day.and_time(time.with_second(0).unwrap_or(time)), tz)?
    };
    // Never into the past (model confusion) or absurdly far ahead.
    let (start_of_today, _) = db::items::local_day_bounds(now, tz);
    if target < start_of_today || target > now + Duration::days(366 * 2) {
        return None;
    }
    let mut patch = ItemPatch::default();
    match (item.due_at, item.remind_at) {
        (Some(due), remind) => {
            let shift = target - due;
            patch.due_at = Some(Some(target));
            if let Some(r) = remind {
                patch.remind_at = Some(Some(r + shift));
            }
            if let Some(e) = item.ends_at {
                patch.ends_at = Some(Some(e + shift));
            }
        }
        (None, Some(_)) => patch.remind_at = Some(Some(target)),
        (None, None) if item.kind == ItemType::Reminder => patch.remind_at = Some(Some(target)),
        (None, None) => patch.due_at = Some(Some(target)),
    }
    if item.snoozed_until.is_some() {
        patch.snoozed_until = Some(None);
    }
    Some(patch)
}

/// The patch that puts a card back the way it was before a command.
pub fn restore(before: &Item) -> ItemPatch {
    ItemPatch {
        status: Some(before.status),
        due_at: Some(before.due_at),
        ends_at: Some(before.ends_at),
        remind_at: Some(before.remind_at),
        snoozed_until: Some(before.snoozed_until),
        ..Default::default()
    }
}

/// What "what's on today?" says aloud.
pub fn today_answer(items: &[Item], lang: &str) -> String {
    const SPOKEN: usize = 5;
    let titles: Vec<&str> = items
        .iter()
        .take(SPOKEN)
        .map(|i| i.title.as_str())
        .collect();
    let more = items.len().saturating_sub(SPOKEN);
    let it = lang != "en";
    let list = |and: &str| -> String {
        match titles.as_slice() {
            [] => String::new(),
            [one] => one.to_string(),
            [rest @ .., last] if more == 0 => format!("{} {and} {last}", rest.join(", ")),
            all => all.join(", "),
        }
    };
    match (items.len(), it) {
        (0, true) => "Oggi non hai niente in programma.".into(),
        (0, false) => "Nothing on today.".into(),
        (1, true) => format!("Oggi hai una cosa: {}.", list("e")),
        (1, false) => format!("Today you have one thing: {}.", list("and")),
        (n, true) if more > 0 => format!("Oggi hai {n} cose: {}, e altre {more}.", list("e")),
        (n, true) => format!("Oggi hai {n} cose: {}.", list("e")),
        (n, false) if more > 0 => {
            format!(
                "Today you have {n} things: {}, and {more} more.",
                list("and")
            )
        }
        (n, false) => format!("Today you have {n} things: {}.", list("and")),
    }
}

/// Undo snapshots for the confirmation toasts still on screen.
static UNDO: LazyLock<Mutex<HashMap<String, Item>>> = LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn take_undo(toast_id: &str) -> Option<Item> {
    UNDO.lock().ok()?.remove(toast_id)
}

fn confirm(app: &AppHandle, kind: &str, before: Item, after: Item) {
    let id = format!("{TOAST_PREFIX}{kind}:{}", after.id);
    if let Ok(mut u) = UNDO.lock() {
        // Keep the map small: only the latest few commands can be undone.
        if u.len() > 20 {
            u.clear();
        }
        u.insert(id.clone(), before);
    }
    if let Some(r) = app.try_state::<Arc<Reminders>>() {
        r.push(Toast {
            id,
            items: vec![after],
            missed: false,
            filed: None,
            brief: false,
        });
    }
}

async fn ask_model(app: &AppHandle, text: &str) -> Option<Intent> {
    let ai = app.try_state::<Arc<AiService>>()?;
    let ai = Arc::clone(&ai);
    let user = user_prompt(text, now(), local_tz());
    match ai.complete_once(SYSTEM_PROMPT, &user, &schema()).await {
        Ok(out) => parse(&out),
        Err(e) => {
            log::info!("voice command skipped: {e}");
            None
        }
    }
}

/// Acts on `text` if it is a command; None means "capture it as usual".
/// `lang` is the language answers are spoken in ("it" or "en").
pub async fn try_handle(app: &AppHandle, text: &str, lang: &str) -> Option<Handled> {
    if !has_cue(text) {
        return None;
    }
    let intent = if is_today_query(text) {
        Intent::Today
    } else {
        ask_model(app, text).await?
    };
    let state = app.try_state::<AppState>()?;
    let (now, tz) = (now(), local_tz());
    match intent {
        Intent::Today => {
            let items = {
                let conn = state.db.lock().ok()?;
                db::items::list_items(&conn, &ItemFilter::Today { now, tz }, false)
                    .unwrap_or_default()
            };
            if extras::speak_enabled(app) {
                extras::speak(&today_answer(&items, lang), lang);
            }
            if let Some(r) = app.try_state::<Arc<Reminders>>() {
                r.push(Toast {
                    id: BRIEF_ID.into(),
                    items,
                    missed: false,
                    filed: None,
                    brief: true,
                });
            }
            Some(Handled::default())
        }
        Intent::Done { target } => {
            let (before, after) = {
                let conn = state.db.lock().ok()?;
                let before = resolve(&conn, &target)?;
                let after = db::items::complete_item(&conn, &before.id, now, tz).ok()?;
                (before, after)
            };
            emit_items_changed(app, vec![after.id.clone()]);
            let item_id = Some(after.id.clone());
            confirm(app, "done", before, after);
            Some(Handled { item_id })
        }
        Intent::Move { target, date } => {
            let (before, after) = {
                let conn = state.db.lock().ok()?;
                let before = resolve(&conn, &target)?;
                let patch = retime(&before, &date, now, tz)?;
                let after = db::items::update_item(&conn, &before.id, patch, now, tz).ok()?;
                (before, after)
            };
            emit_items_changed(app, vec![after.id.clone()]);
            let item_id = Some(after.id.clone());
            confirm(app, "moved", before, after);
            Some(Handled { item_id })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::models::NewItem;

    fn rome(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        let tz: Tz = "Europe/Rome".parse().unwrap();
        tz.with_ymd_and_hms(y, m, d, h, min, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    #[test]
    fn cues_gate_the_model() {
        assert!(has_cue("Segna fatto chiamare Marta"));
        assert!(has_cue("NOMA, sposta il dentista a domani"));
        assert!(has_cue("What’s on today?"));
        assert!(!has_cue("Comprare il latte domani"));
        assert!(!has_cue("Marking scheme for the exam")); // "mark" as a whole word only
        assert!(is_today_query("Cosa ho oggi?"));
        assert!(is_today_query("what's on today"));
        assert!(!is_today_query("cosa ho domani?"));
    }

    #[test]
    fn parses_intents_and_rejects_incomplete_ones() {
        assert_eq!(
            parse(r#"{"intent":"done","target":"chiamare Marta"}"#),
            Some(Intent::Done {
                target: "chiamare Marta".into()
            })
        );
        assert_eq!(
            parse(r#"{"intent":"move","target":"dentista","date":"2026-11-10"}"#),
            Some(Intent::Move {
                target: "dentista".into(),
                date: "2026-11-10".into()
            })
        );
        assert_eq!(parse(r#"{"intent":"move","target":"dentista"}"#), None);
        assert_eq!(parse(r#"{"intent":"done"}"#), None);
        assert_eq!(parse(r#"{"intent":"none"}"#), None);
        assert_eq!(parse("garbage"), None);
    }

    #[test]
    fn resolves_one_open_card_or_nothing() {
        let conn = db::open_in_memory().unwrap();
        let now = Utc::now();
        let mk = |t: &str| {
            db::items::create_item(
                &conn,
                NewItem {
                    title: t.into(),
                    ..Default::default()
                },
                now,
            )
            .unwrap()
        };
        let marta = mk("Chiamare Marta");
        mk("Call the plumber");
        mk("Call the electrician");
        assert_eq!(resolve(&conn, "chiamare Marta").unwrap().id, marta.id);
        assert!(resolve(&conn, "call the").is_none(), "ambiguous");
        assert!(resolve(&conn, "dentista").is_none());
        let plumber = resolve(&conn, "the plumber").unwrap();
        assert_eq!(plumber.title, "Call the plumber");
    }

    #[test]
    fn retime_keeps_time_of_day_and_moves_reminder_along() {
        let tz: Tz = "Europe/Rome".parse().unwrap();
        let now = rome(2026, 11, 9, 10, 0);
        let item = Item {
            id: "x".into(),
            kind: ItemType::Event,
            title: "Dentista".into(),
            body: None,
            status: Status::Next,
            priority: crate::db::models::Priority::Normal,
            due_at: Some(rome(2026, 11, 9, 15, 0)),
            ends_at: Some(rome(2026, 11, 9, 16, 0)),
            remind_at: Some(rome(2026, 11, 9, 14, 0)),
            snoozed_until: None,
            recurrence: None,
            project_id: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            tags: vec![],
            link_count: 0,
            hint: None,
            capturing: false,
        };
        let p = retime(&item, "2026-11-10", now, tz).unwrap();
        assert_eq!(p.due_at, Some(Some(rome(2026, 11, 10, 15, 0))));
        assert_eq!(p.ends_at, Some(Some(rome(2026, 11, 10, 16, 0))));
        assert_eq!(p.remind_at, Some(Some(rome(2026, 11, 10, 14, 0))));
        let p = retime(&item, "2026-11-12T10:30", now, tz).unwrap();
        assert_eq!(p.due_at, Some(Some(rome(2026, 11, 12, 10, 30))));
        assert!(retime(&item, "2019-01-01", now, tz).is_none());

        let bare = Item {
            due_at: None,
            ends_at: None,
            remind_at: None,
            kind: ItemType::Reminder,
            ..item
        };
        let p = retime(&bare, "2026-11-10", now, tz).unwrap();
        assert_eq!(p.remind_at, Some(Some(rome(2026, 11, 10, 9, 0))));
        assert_eq!(p.due_at, None);
    }

    #[test]
    fn spoken_answer_lists_titles() {
        let now = Utc::now();
        let item = |t: &str| Item {
            id: t.into(),
            kind: ItemType::Task,
            title: t.into(),
            body: None,
            status: Status::Next,
            priority: crate::db::models::Priority::Normal,
            due_at: None,
            ends_at: None,
            remind_at: None,
            snoozed_until: None,
            recurrence: None,
            project_id: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            tags: vec![],
            link_count: 0,
            hint: None,
            capturing: false,
        };
        assert_eq!(today_answer(&[], "en"), "Nothing on today.");
        assert_eq!(
            today_answer(&[item("Dentista"), item("Latte")], "it"),
            "Oggi hai 2 cose: Dentista e Latte."
        );
        assert_eq!(
            today_answer(&[item("Call Marta")], "en"),
            "Today you have one thing: Call Marta."
        );
    }
}
