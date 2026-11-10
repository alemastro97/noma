//! Turning a capture into cards: the prompt, the JSON schema the model is
//! constrained to, and the validation that makes its output safe to store.
//!
//! The model returns local wall-clock times ("2026-11-10T09:00") and a repeat
//! preset; Rust converts them to UTC instants and RRULEs, checks plausibility,
//! matches project names and records anything doubtful as a hint on the card.

use chrono::{DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::Deserialize;
use serde_json::{Value, json};

use super::AiError;
use crate::db::models::{CaptureHint, ItemType, Priority};
use crate::db::tags::normalize_tags;

pub const MAX_ITEMS: usize = 3;
const LOW_CONFIDENCE: f64 = 0.6;
const PROJECT_MATCH: f64 = 0.88;

/// What the model knows about the user's world when reading a capture.
pub struct Context<'a> {
    pub now: DateTime<Utc>,
    pub tz: Tz,
    /// (id, name) of existing projects.
    pub projects: &'a [(String, String)],
    /// (id, name) of personal boards.
    pub boards: &'a [(String, String)],
    /// Latest card titles per project / board id: what each place is about.
    pub recent: &'a [(String, Vec<String>)],
}

/// One validated card, ready to be written.
#[derive(Debug, Clone, PartialEq)]
pub struct Extracted {
    pub kind: ItemType,
    /// The capture asked to start a new project named `title`.
    pub creates_project: bool,
    pub title: String,
    pub body: Option<String>,
    pub due_at: Option<DateTime<Utc>>,
    pub ends_at: Option<DateTime<Utc>>,
    pub remind_at: Option<DateTime<Utc>>,
    pub recurrence: Option<String>,
    pub priority: Priority,
    pub project_id: Option<String>,
    /// Personal board the card belongs on when no project fits.
    pub board_id: Option<String>,
    pub tags: Vec<String>,
    pub hint: Option<CaptureHint>,
}

#[derive(Deserialize, Debug)]
struct RawOutput {
    items: Vec<RawItem>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(default)]
struct RawItem {
    #[serde(rename = "type")]
    kind: String,
    title: String,
    summary: Option<String>,
    due: Option<String>,
    end: Option<String>,
    remind: Option<String>,
    repeat: Option<String>,
    priority: Option<String>,
    project: Option<String>,
    board: Option<String>,
    tags: Option<Vec<String>>,
    date_phrase: Option<String>,
    confidence: Option<f64>,
    unsure: Option<bool>,
}

pub const SYSTEM_PROMPT: &str = r#"You turn one short note the user said or typed into cards for their personal organizer. Notes may be Italian or English. Reply with compact single-line JSON that matches the schema. Omit every optional field that does not apply.

Fields:
- type: "task" (something to do), "reminder" (a nudge at a moment: "remind me", "ricordami"), "event" (an appointment, meeting, lunch, flight or dinner at a time; a call the user has to make is a task), "note" (information to keep: codes, passwords, quotes, what someone said), "idea" (a thought or possibility: "idea", "what if", "maybe", "e se", "potrei"), "project" (only when the user explicitly starts a new project).
- title: a short headline of 2 to 6 words that says what the note is about, in the user's language; no date or time words, no "remind me to" / "ricordami di"; first letter capitalized, the rest as normal sentence case. Never copy a long note: condense it.
- summary: one short sentence restating the note in your own words. Only for notes longer than 15 words (always include it then); never for shorter ones.
- due: when a task is due or an event starts, local "YYYY-MM-DDTHH:MM", or "YYYY-MM-DD" for a task with only a day.
- end: the end of an event, only when said.
- remind: when to notify, local "YYYY-MM-DDTHH:MM". Reminders use remind, never due.
- repeat: only when the note repeats: "daily", "weekdays", "weekly", "monthly" or "yearly".
- priority: "urgent" for urgent/ASAP/subito/urgente, "high" for important/importante; otherwise omit.
- project: the exact name from Projects, when the note is about that project. Judge by topic, people and the recent cards listed in brackets, not only by the name.
- board: the exact name from Boards, only when no project fits and the note shares a specific topic with that board's name or its recent cards. A board is not a bucket for ideas or notes in general. Never use both project and board. When unsure, omit both: the note goes to the inbox.
- unsure: true only when you are unsure about the type or the date.

Dates: use the Calendar line to turn weekday names into dates; never compute weekdays yourself. A day-and-month date ("3 March", "il 5") means its next occurrence after today. Resolve relative words against Now. today/oggi = Now's date; tomorrow/domani = +1 day; dopodomani/day after tomorrow = +2; a weekday name = its next occurrence after today; next week/la prossima settimana = next Monday; by the end of the week/entro la settimana = this Friday. Times when only a part of the day is said: morning/mattina 09:00, noon/mezzogiorno 12:00, afternoon/pomeriggio 15:00, evening/sera/stasera/tonight 20:00. A reminder with only a day (including a weekday name): 09:00, and no repeat unless the note says every/ogni. An event with only a day: 09:00.

Several items only when the note clearly lists separate things to do (max 3). Never invent dates, projects or details.

Examples with Now 2026-11-09T10:00 (Monday), Projects: Spring launch (Draft launch email; Shortlist three venues), Studio move (Pack the printer; Book movers); Boards: Thinking space (A calmer notification sound), Trips 2027:
Note: ricordami di chiamare la mamma stasera
{"items":[{"type":"reminder","title":"Chiamare la mamma","remind":"2026-11-09T20:00"}]}
Note: buy milk and call the plumber tomorrow morning
{"items":[{"type":"task","title":"Buy milk","due":"2026-11-10"},{"type":"task","title":"Call the plumber","due":"2026-11-10T09:00"}]}
Note: every Monday at 9 remind me to do the weekly review
{"items":[{"type":"reminder","title":"Do the weekly review","remind":"2026-11-16T09:00","repeat":"weekly"}]}
Note: ricordami di mandare il preventivo a Luca venerdì
{"items":[{"type":"reminder","title":"Preventivo a Luca","remind":"2026-11-13T09:00"}]}
Note: call Marta at 11 about the contract
{"items":[{"type":"task","title":"Call Marta about the contract","due":"2026-11-09T11:00"}]}
Note: dentist Thursday at 3pm
{"items":[{"type":"event","title":"Dentist","due":"2026-11-12T15:00"}]}
Note: book the photographer for the spring launch shoot
{"items":[{"type":"task","title":"Book the photographer","project":"Spring launch"}]}
Note: ricordami di chiedere ai traslocatori se portano anche la stampante
{"items":[{"type":"reminder","title":"Stampante ai traslocatori","remind":"2026-11-10T09:00","project":"Studio move"}]}
Note: per il viaggio in Giappone devo prenotare l'hotel a Kyoto
{"items":[{"type":"task","title":"Hotel a Kyoto","board":"Trips 2027"}]}
Note: comprare le batterie per il telecomando
{"items":[{"type":"task","title":"Batterie per il telecomando"}]}
Note: e se l'orb si attenuasse durante le riunioni?
{"items":[{"type":"idea","title":"Orb che si attenua in riunione","board":"Thinking space"}]}
Note: I should really rethink how the onboarding explains the first capture, maybe a short video instead of three screens of text, and ask Luca what he thinks before we commit
{"items":[{"type":"idea","title":"Rethink onboarding explanation","summary":"Consider a short video instead of three text screens for onboarding, and get Luca's opinion first."}]}
Note: il codice del portone è 4821
{"items":[{"type":"note","title":"Codice del portone: 4821"}]}
Note: urgente: rinnovare la carta d'identità
{"items":[{"type":"task","title":"Rinnovare la carta d'identità","priority":"urgent"}]}
Note: pay the internet bill on the 5th of every month
{"items":[{"type":"task","title":"Pay the internet bill","due":"2026-12-05","repeat":"monthly"}]}
Note: ricordami il compleanno di Luca ogni anno il 2 febbraio
{"items":[{"type":"reminder","title":"Compleanno di Luca","remind":"2027-02-02T09:00","repeat":"yearly"}]}"#;

/// JSON schema the server constrains decoding to (GBNF under the hood).
/// Only `type` and `title` are required: every omitted field is output the
/// model does not have to generate, which is most of the latency on a laptop.
pub fn schema() -> Value {
    let datetime =
        json!({ "type": "string", "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}$" });
    json!({
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "minItems": 1,
                "maxItems": MAX_ITEMS,
                "items": {
                    "type": "object",
                    "properties": {
                        "type": { "type": "string", "enum": ["task", "reminder", "event", "note", "idea", "project"] },
                        "title": { "type": "string", "minLength": 1, "maxLength": 120 },
                        "due": { "type": "string", "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}(T[0-9]{2}:[0-9]{2})?$" },
                        "end": datetime,
                        "remind": datetime,
                        "repeat": { "type": "string", "enum": ["daily", "weekdays", "weekly", "monthly", "yearly"] },
                        "priority": { "type": "string", "enum": ["high", "urgent"] },
                        "summary": { "type": "string", "maxLength": 240 },
                        "project": { "type": "string" },
                        "board": { "type": "string" },
                        "unsure": { "type": "boolean" }
                    },
                    "required": ["type", "title"]
                }
            }
        },
        "required": ["items"]
    })
}

/// Like [`schema`], but a note longer than 15 words must come with a summary
/// (the constrained decoder enforces it; a prompt rule alone is ignored by a
/// small model). Short notes keep the lean schema.
pub fn schema_for(text: &str) -> Value {
    let mut s = schema();
    if text.split_whitespace().count() > 15 {
        s["properties"]["items"]["items"]["required"] = json!(["type", "title", "summary"]);
    }
    s
}

/// The user message: the moment, a short calendar (small models are bad at
/// weekday arithmetic), the projects and the note itself.
pub fn user_prompt(text: &str, ctx: &Context<'_>) -> String {
    let local = ctx.now.with_timezone(&ctx.tz);
    let places = |list: &[(String, String)]| {
        if list.is_empty() {
            return "(none)".to_string();
        }
        list.iter()
            .map(|(id, name)| {
                let recent: Vec<String> = ctx
                    .recent
                    .iter()
                    .find(|(rid, _)| rid == id)
                    .map(|(_, titles)| {
                        titles
                            .iter()
                            .take(3)
                            .map(|t| t.chars().take(40).collect::<String>())
                            .collect()
                    })
                    .unwrap_or_default();
                if recent.is_empty() {
                    name.clone()
                } else {
                    format!("{name} ({})", recent.join("; "))
                }
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let projects = places(ctx.projects);
    let boards = places(ctx.boards);
    let today = local.date_naive();
    let calendar = (1..=8)
        .map(|d| {
            let day = today + Duration::days(d);
            format!("{} {}", day.format("%a"), day.format("%Y-%m-%d"))
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "Now: {} ({}), time zone {}\nCalendar: {}\nProjects: {}\nBoards: {}\nNote: {}",
        local.format("%Y-%m-%dT%H:%M"),
        local.format("%A"),
        ctx.tz.name(),
        calendar,
        projects,
        boards,
        text.trim()
    )
}

/// Parses a model reply and validates every item. `raw_text` is the original
/// capture, used as a fallback title.
pub fn parse_output(
    json_text: &str,
    raw_text: &str,
    ctx: &Context<'_>,
) -> Result<Vec<Extracted>, AiError> {
    let cleaned = strip_fences(json_text);
    let raw: RawOutput = serde_json::from_str(cleaned)
        .map_err(|e| AiError::BadOutput(format!("{e}: {}", truncate(cleaned, 200))))?;
    if raw.items.is_empty() {
        return Err(AiError::BadOutput("no items".into()));
    }
    Ok(raw
        .items
        .into_iter()
        .take(MAX_ITEMS)
        .map(|item| validate(item, raw_text, ctx))
        .collect())
}

fn strip_fences(s: &str) -> &str {
    let t = s.trim();
    let t = t
        .strip_prefix("```json")
        .or_else(|| t.strip_prefix("```"))
        .unwrap_or(t);
    t.strip_suffix("```").unwrap_or(t).trim()
}

fn truncate(s: &str, n: usize) -> String {
    s.chars().take(n).collect()
}

fn validate(raw: RawItem, raw_text: &str, ctx: &Context<'_>) -> Extracted {
    let mut hint = CaptureHint::default();

    let (kind, creates_project) = match raw.kind.as_str() {
        "task" => (ItemType::Task, false),
        "reminder" => (ItemType::Reminder, false),
        "event" => (ItemType::Event, false),
        "idea" => (ItemType::Idea, false),
        "project" => (ItemType::ProjectRef, true),
        _ => (ItemType::Note, false),
    };

    let mut title = clean_title(&raw.title);
    if title.is_empty() {
        title = clean_title(raw_text);
    }
    if title.is_empty() {
        title = "Untitled".into();
    }

    let date_phrase = raw.date_phrase.filter(|p| !p.trim().is_empty());
    let flag_date = |hint: &mut CaptureHint| {
        hint.date_phrase = Some(date_phrase.clone().unwrap_or_default());
    };

    let parse = |s: Option<String>, default_hour: u32, hint: &mut CaptureHint| match s {
        None => None,
        Some(s) if s.trim().is_empty() => None,
        Some(s) => match local_to_utc(&s, ctx.tz, default_hour) {
            Some(d) if plausible(d, ctx.now) => Some(d),
            _ => {
                flag_date(hint);
                None
            }
        },
    };

    let mut due_at = parse(raw.due, 0, &mut hint);
    let mut ends_at = parse(raw.end, 9, &mut hint);
    let mut remind_at = parse(raw.remind, 9, &mut hint);

    match kind {
        // A reminder lives on its remind time.
        ItemType::Reminder if remind_at.is_none() => remind_at = due_at.take(),
        // An event starts at `due`; a reminder-only event moves its time there.
        ItemType::Event => {
            if due_at.is_none() {
                due_at = remind_at.take();
            }
            if let (Some(start), None) = (due_at, ends_at) {
                ends_at = Some(start + Duration::hours(1));
            }
        }
        _ => {}
    }
    if let (Some(start), Some(end)) = (due_at, ends_at)
        && end <= start
    {
        ends_at = Some(start + Duration::hours(1));
    }
    if kind != ItemType::Event {
        ends_at = None;
    }

    let anchor = due_at.or(remind_at);
    let recurrence = raw
        .repeat
        .as_deref()
        .and_then(|preset| anchor.and_then(|a| rule_for(preset, a, ctx.tz)));
    // "Every day at 8", said at 10:00: the first occurrence is tomorrow, not this morning.
    if let (Some(rule), Some(a)) = (&recurrence, anchor)
        && a < ctx.now
        && let Ok(Some(next)) = crate::db::recurrence::next_after(rule, a, ctx.now, ctx.tz)
    {
        let shift = next - a;
        due_at = due_at.map(|d| d + shift);
        remind_at = remind_at.map(|d| d + shift);
        ends_at = ends_at.map(|d| d + shift);
    }

    let priority = match raw.priority.as_deref() {
        Some("urgent") => Priority::Urgent,
        Some("high") => Priority::High,
        Some("low") => Priority::Low,
        _ => Priority::Normal,
    };

    let project_id = match raw.project.as_deref().map(str::trim) {
        Some(name) if !name.is_empty() && !creates_project => {
            match match_project(name, ctx.projects) {
                Some(id) => Some(id),
                None => {
                    hint.suggested_project = Some(name.to_string());
                    None
                }
            }
        }
        _ => None,
    };

    let board_id = if project_id.is_none() && !creates_project {
        raw.board
            .as_deref()
            .map(str::trim)
            .filter(|n| !n.is_empty())
            .and_then(|name| match_project(name, ctx.boards))
    } else {
        None
    };

    if raw.unsure == Some(true) || raw.confidence.unwrap_or(1.0) < LOW_CONFIDENCE {
        hint.low_confidence = true;
    }

    let tags = normalize_tags(&raw.tags.unwrap_or_default())
        .into_iter()
        .map(|t| t.to_lowercase())
        .take(3)
        .collect();

    let body = compose_body(raw_text, raw.summary.as_deref(), &title);

    Extracted {
        kind,
        creates_project,
        title,
        body,
        due_at,
        ends_at,
        remind_at,
        recurrence,
        priority,
        project_id,
        board_id,
        tags,
        hint: (!hint.is_empty()).then_some(hint),
    }
}

fn plain(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// The card description: an AI summary (only for long notes) above what the
/// user actually said. A note that is just its own title needs no description.
pub fn compose_body(raw_text: &str, summary: Option<&str>, title: &str) -> Option<String> {
    let original = raw_text.trim();
    let summary = summary
        .map(str::trim)
        .filter(|s| !s.is_empty() && plain(s) != plain(title) && plain(s) != plain(original));
    let original = (!original.is_empty() && plain(original) != plain(title)).then_some(original);
    match (summary, original) {
        (Some(s), Some(o)) => Some(format!("{s}\n\nYou said:\n{o}")),
        (Some(s), None) => Some(s.to_string()),
        (None, Some(o)) => Some(o.to_string()),
        (None, None) => None,
    }
}

fn clean_title(s: &str) -> String {
    let line = s.lines().next().unwrap_or("").trim();
    let t: String = line
        .trim_matches(|c: char| c == '"' || c == '.' || c.is_whitespace())
        .chars()
        .take(120)
        .collect();
    let mut chars = t.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Parses a local wall-clock time ("2026-11-10T09:00", with or without seconds,
/// or a bare date) in `tz` and returns it as UTC. RFC 3339 strings are taken as is.
pub fn local_to_utc(s: &str, tz: Tz, default_hour: u32) -> Option<DateTime<Utc>> {
    let s = s.trim();
    if let Ok(d) = DateTime::parse_from_rfc3339(s) {
        return Some(d.with_timezone(&Utc));
    }
    let naive = ["%Y-%m-%dT%H:%M", "%Y-%m-%dT%H:%M:%S", "%Y-%m-%d %H:%M"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(s, f).ok())
        .or_else(|| {
            NaiveDate::parse_from_str(s, "%Y-%m-%d")
                .ok()
                .and_then(|d| d.and_hms_opt(default_hour, 0, 0))
        })?;
    // DST gaps: step forward until the wall time exists.
    (0..3).find_map(|h| {
        tz.from_local_datetime(&(naive + Duration::hours(h)))
            .earliest()
            .map(|d| d.with_timezone(&Utc))
    })
}

/// Rejects dates far in the past (model confusion) or absurdly far ahead.
fn plausible(d: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    d >= now - Duration::days(1) && d <= now + Duration::days(366 * 2)
}

/// RRULE for a repeat preset, anchored on the card's local weekday / day of month.
pub fn rule_for(preset: &str, anchor: DateTime<Utc>, tz: Tz) -> Option<String> {
    let local = anchor.with_timezone(&tz);
    const DAYS: [&str; 7] = ["MO", "TU", "WE", "TH", "FR", "SA", "SU"];
    match preset {
        "daily" => Some("FREQ=DAILY".into()),
        "weekdays" => Some("FREQ=WEEKLY;BYDAY=MO,TU,WE,TH,FR".into()),
        "weekly" => Some(format!(
            "FREQ=WEEKLY;BYDAY={}",
            DAYS[local.weekday().num_days_from_monday() as usize]
        )),
        "monthly" => Some(format!("FREQ=MONTHLY;BYMONTHDAY={}", local.day())),
        "yearly" => Some("FREQ=YEARLY".into()),
        _ => None,
    }
}

fn norm(s: &str) -> String {
    s.trim().to_lowercase()
}

/// Exact, containment or close (Jaro-Winkler) match against existing project names.
pub fn match_project(name: &str, projects: &[(String, String)]) -> Option<String> {
    let n = norm(name);
    if let Some((id, _)) = projects.iter().find(|(_, p)| norm(p) == n) {
        return Some(id.clone());
    }
    let best = projects
        .iter()
        .map(|(id, p)| {
            let p = norm(p);
            let contained = n.len() >= 4 && p.len() >= 4 && (p.contains(&n) || n.contains(&p));
            let score = if contained {
                0.95
            } else {
                strsim::jaro_winkler(&n, &p)
            };
            (id, score)
        })
        .max_by(|a, b| a.1.total_cmp(&b.1))?;
    (best.1 >= PROJECT_MATCH).then(|| best.0.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_keeps_what_was_said_and_adds_a_summary_for_long_notes() {
        // Short capture: the description is the original words.
        assert_eq!(
            compose_body("ricordami di chiamare Marta domani", None, "Chiamare Marta").as_deref(),
            Some("ricordami di chiamare Marta domani")
        );
        // A capture that is just its title adds nothing.
        assert_eq!(compose_body("Buy milk.", None, "Buy milk"), None);
        // Long capture: summary on top, original below.
        assert_eq!(
            compose_body("a very long ramble", Some("A ramble."), "Ramble").as_deref(),
            Some("A ramble.\n\nYou said:\na very long ramble")
        );
    }

    #[test]
    fn prompt_lists_boards_and_what_each_place_is_about() {
        let projects = vec![("p1".to_string(), "Spring launch".to_string())];
        let boards = vec![("b1".to_string(), "Trips 2027".to_string())];
        let recent = vec![(
            "p1".to_string(),
            vec![
                "Book the photographer".to_string(),
                "Draft launch email".to_string(),
            ],
        )];
        let c = Context {
            boards: &boards,
            recent: &recent,
            ..ctx(&projects)
        };
        let p = user_prompt("x", &c);
        assert!(p.contains("Projects: Spring launch (Book the photographer; Draft launch email)"));
        assert!(p.contains("Boards: Trips 2027"));
    }

    #[test]
    fn board_is_matched_only_when_no_project_applies() {
        let boards = vec![("b1".to_string(), "Trips 2027".to_string())];
        let c = Context {
            boards: &boards,
            ..ctx(&[])
        };
        let json = item(r#""type":"note","title":"Ciliegi","board":"trips 2027""#);
        let e = parse_output(&json, "raw", &c).unwrap().remove(0);
        assert_eq!(e.board_id.as_deref(), Some("b1"));
        let json = item(r#""type":"note","title":"Ciliegi","board":"Unknown board""#);
        let e = parse_output(&json, "raw", &c).unwrap().remove(0);
        assert_eq!(e.board_id, None);
    }

    fn ctx(projects: &[(String, String)]) -> Context<'_> {
        // Monday 9 November 2026, 10:00 in Rome (09:00 UTC).
        Context {
            now: Utc.with_ymd_and_hms(2026, 11, 9, 9, 0, 0).unwrap(),
            tz: "Europe/Rome".parse().unwrap(),
            projects,
            boards: &[],
            recent: &[],
        }
    }

    fn rome(y: i32, m: u32, d: u32, h: u32, min: u32) -> DateTime<Utc> {
        let tz: Tz = "Europe/Rome".parse().unwrap();
        tz.with_ymd_and_hms(y, m, d, h, min, 0)
            .unwrap()
            .with_timezone(&Utc)
    }

    fn one(json: &str, projects: &[(String, String)]) -> Extracted {
        parse_output(json, "raw", &ctx(projects)).unwrap().remove(0)
    }

    const BASE: &str = r#""body":null,"due":null,"end":null,"remind":null,"repeat":"none","priority":"normal","project":null,"tags":[],"date_phrase":null,"confidence":0.9"#;

    /// One-item reply: BASE defaults overridden by `fields` (merged, no duplicate keys).
    fn item(fields: &str) -> String {
        let mut base: serde_json::Map<String, Value> =
            serde_json::from_str(&format!("{{{BASE}}}")).unwrap();
        let over: serde_json::Map<String, Value> =
            serde_json::from_str(&format!("{{{fields}}}")).unwrap();
        base.extend(over);
        json!({ "items": [Value::Object(base)] }).to_string()
    }

    #[test]
    fn long_notes_require_a_summary() {
        let short = schema_for("buy milk");
        let long = schema_for(&"word ".repeat(20));
        let req = |s: &Value| s["properties"]["items"]["items"]["required"].clone();
        assert_eq!(req(&short), json!(["type", "title"]));
        assert_eq!(req(&long), json!(["type", "title", "summary"]));
    }

    #[test]
    fn schema_requires_every_field() {
        let s = schema();
        let req = s["properties"]["items"]["items"]["required"]
            .as_array()
            .unwrap();
        assert_eq!(req.len(), 2);
        assert!(SYSTEM_PROMPT.contains("compact single-line JSON"));
    }

    #[test]
    fn user_prompt_has_local_now_weekday_and_projects() {
        let projects = vec![("p1".to_string(), "Spring launch".to_string())];
        let p = user_prompt(" buy milk ", &ctx(&projects));
        assert!(p.contains("Now: 2026-11-09T10:00 (Monday), time zone Europe/Rome"));
        assert!(p.contains("Calendar: Tue 2026-11-10, Wed 2026-11-11"));
        assert!(p.contains("Tue 2026-11-17"));
        assert!(p.contains("Projects: Spring launch"));
        assert!(p.ends_with("Note: buy milk"));
    }

    #[test]
    fn task_with_local_due_converts_to_utc() {
        let e = one(
            &item(r#""type":"task","title":"comprare il latte","due":"2026-11-10T09:00""#),
            &[],
        );
        assert_eq!(e.kind, ItemType::Task);
        assert_eq!(e.title, "Comprare il latte");
        assert_eq!(e.due_at, Some(rome(2026, 11, 10, 9, 0)));
        assert!(e.hint.is_none());
    }

    #[test]
    fn reminder_uses_remind_time_even_when_model_put_it_in_due() {
        let e = one(
            &item(r#""type":"reminder","title":"Call mum","due":"2026-11-09T18:00""#),
            &[],
        );
        assert_eq!(e.remind_at, Some(rome(2026, 11, 9, 18, 0)));
        assert_eq!(e.due_at, None);
    }

    #[test]
    fn event_gets_a_default_one_hour_end() {
        let e = one(
            &item(r#""type":"event","title":"Design review","due":"2026-11-11T14:30""#),
            &[],
        );
        assert_eq!(e.ends_at, Some(rome(2026, 11, 11, 15, 30)));
    }

    #[test]
    fn implausible_date_is_dropped_with_a_check_date_hint() {
        let e = one(
            &item(
                r#""type":"task","title":"Pay rent","due":"2019-01-01T09:00","date_phrase":"the first""#,
            ),
            &[],
        );
        assert_eq!(e.due_at, None);
        assert_eq!(e.hint.unwrap().date_phrase.as_deref(), Some("the first"));
    }

    #[test]
    fn repeat_weekly_anchors_on_local_weekday() {
        let e = one(
            &item(
                r#""type":"reminder","title":"Weekly review","remind":"2026-11-16T09:00","repeat":"weekly""#,
            ),
            &[],
        );
        assert_eq!(e.recurrence.as_deref(), Some("FREQ=WEEKLY;BYDAY=MO"));
    }

    #[test]
    fn recurring_time_already_passed_today_starts_at_the_next_occurrence() {
        // Now is 10:00; "every day at 8" must start tomorrow at 08:00.
        let e = one(
            &item(
                r#""type":"reminder","title":"Vitamin","remind":"2026-11-09T08:00","repeat":"daily""#,
            ),
            &[],
        );
        assert_eq!(e.remind_at, Some(rome(2026, 11, 10, 8, 0)));
    }

    #[test]
    fn repeat_without_a_date_is_ignored() {
        let e = one(
            &item(r#""type":"task","title":"Stretch","repeat":"daily""#),
            &[],
        );
        assert_eq!(e.recurrence, None);
    }

    #[test]
    fn project_is_matched_fuzzily_or_suggested() {
        let projects = vec![
            ("p1".to_string(), "Spring launch".to_string()),
            ("p2".to_string(), "Studio move".to_string()),
        ];
        let e = one(
            &item(r#""type":"task","title":"Book venue","project":"spring launch""#),
            &projects,
        );
        assert_eq!(e.project_id.as_deref(), Some("p1"));
        let e = one(
            &item(r#""type":"task","title":"Book venue","project":"Sprng launch""#),
            &projects,
        );
        assert_eq!(e.project_id.as_deref(), Some("p1"));
        let e = one(
            &item(r#""type":"task","title":"Buy paint","project":"Kitchen""#),
            &projects,
        );
        assert_eq!(e.project_id, None);
        assert_eq!(
            e.hint.unwrap().suggested_project.as_deref(),
            Some("Kitchen")
        );
    }

    #[test]
    fn low_confidence_is_flagged() {
        let e = one(
            &item(r#""type":"note","title":"Hmm","confidence":0.3"#),
            &[],
        );
        assert!(e.hint.unwrap().low_confidence);
    }

    #[test]
    fn project_type_creates_a_project() {
        let e = one(
            &item(r#""type":"project","title":"Kitchen renovation""#),
            &[],
        );
        assert!(e.creates_project);
        assert_eq!(e.kind, ItemType::ProjectRef);
    }

    #[test]
    fn several_items_capped_at_three_and_fences_tolerated() {
        let one: Value = serde_json::from_str(&item(r#""type":"task","title":"x""#)).unwrap();
        let entry = one["items"][0].clone();
        let body = format!(
            "```json\n{}\n```",
            json!({ "items": [entry.clone(), entry.clone(), entry.clone(), entry] })
        );
        let out = parse_output(&body, "raw", &ctx(&[])).unwrap();
        assert_eq!(out.len(), 3);
    }

    #[test]
    fn garbage_is_an_error_and_blank_title_falls_back_to_raw_text() {
        assert!(parse_output("not json", "raw", &ctx(&[])).is_err());
        let out = parse_output(
            &item(r#""type":"note","title":"  ""#),
            "buy milk tomorrow",
            &ctx(&[]),
        )
        .unwrap();
        assert_eq!(out[0].title, "Buy milk tomorrow");
    }

    #[test]
    fn local_to_utc_accepts_dates_and_rfc3339() {
        let tz: Tz = "Europe/Rome".parse().unwrap();
        assert_eq!(
            local_to_utc("2026-11-10", tz, 9),
            Some(rome(2026, 11, 10, 9, 0))
        );
        assert_eq!(
            local_to_utc("2026-11-10T08:00:00Z", tz, 9),
            Some(Utc.with_ymd_and_hms(2026, 11, 10, 8, 0, 0).unwrap())
        );
        assert_eq!(local_to_utc("tomorrow", tz, 9), None);
    }
}
