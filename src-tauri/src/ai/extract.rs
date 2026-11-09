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
    body: Option<String>,
    due: Option<String>,
    end: Option<String>,
    remind: Option<String>,
    repeat: Option<String>,
    priority: Option<String>,
    project: Option<String>,
    tags: Option<Vec<String>>,
    date_phrase: Option<String>,
    confidence: Option<f64>,
}

pub const SYSTEM_PROMPT: &str = r#"You turn one short note that the user said or typed into structured cards for their personal organizer. Notes may be in Italian or English. Reply only with JSON that matches the schema.

Fields of each item:
- type: "task" (something to do), "reminder" (a nudge at a moment: "remind me", "ricordami"), "event" (an appointment or meeting at a time), "note" (information to keep), "idea" (a thought or possibility: "idea", "what if", "e se"), "project" (only when the user explicitly starts a new project).
- title: short, in the user's language, without the date and time words, first letter capitalized, at most 80 characters.
- body: extra details from the note that do not fit the title, otherwise null.
- due: local time "YYYY-MM-DDTHH:MM" when a task is due or an event starts; null if no date was said.
- end: local end time of an event; null if not said.
- remind: local time "YYYY-MM-DDTHH:MM" to notify the user; for a reminder put the time here and leave due null.
- repeat: "none", "daily", "weekdays", "weekly", "monthly" or "yearly".
- priority: "urgent" only for urgent/ASAP/subito/urgente, "high" for important/importante, else "normal".
- project: the exact name from Projects when the note clearly belongs to one, else null.
- tags: up to 3 lowercase topic words, only when obvious; otherwise [].
- date_phrase: the exact words that expressed the date or time, or null.
- confidence: 0 to 1, how sure you are about type and dates.

Dates: resolve relative words against Now. "today/oggi", "tomorrow/domani", "dopodomani" = in two days. A weekday name means its next occurrence after today. "next week/la prossima settimana" = Monday of next week. Default times when only a part of day is said: morning/mattina 09:00, noon/mezzogiorno 12:00, afternoon/pomeriggio 15:00, evening/stasera/sera 20:00, tonight 20:00. A date with no time: tasks 00:00, reminders 09:00, events 09:00.

Make several items only when the note clearly lists separate things (at most 3). Never invent dates, projects or details that are not in the note."#;

/// JSON schema the server constrains decoding to (GBNF under the hood).
pub fn schema() -> Value {
    let datetime = json!({ "type": ["string", "null"], "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}$" });
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
                        "body": { "type": ["string", "null"] },
                        "due": datetime,
                        "end": datetime,
                        "remind": datetime,
                        "repeat": { "type": "string", "enum": ["none", "daily", "weekdays", "weekly", "monthly", "yearly"] },
                        "priority": { "type": "string", "enum": ["low", "normal", "high", "urgent"] },
                        "project": { "type": ["string", "null"] },
                        "tags": { "type": "array", "items": { "type": "string" }, "maxItems": 3 },
                        "date_phrase": { "type": ["string", "null"] },
                        "confidence": { "type": "number", "minimum": 0, "maximum": 1 }
                    },
                    "required": ["type", "title", "body", "due", "end", "remind", "repeat", "priority", "project", "tags", "date_phrase", "confidence"]
                }
            }
        },
        "required": ["items"]
    })
}

/// The user message: the moment, the projects and the note itself.
pub fn user_prompt(text: &str, ctx: &Context<'_>) -> String {
    let local = ctx.now.with_timezone(&ctx.tz);
    let projects = if ctx.projects.is_empty() {
        "(none)".to_string()
    } else {
        ctx.projects
            .iter()
            .map(|(_, n)| n.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    };
    format!(
        "Now: {} ({}), time zone {}\nProjects: {}\nNote: {}",
        local.format("%Y-%m-%dT%H:%M"),
        local.format("%A"),
        ctx.tz.name(),
        projects,
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
        hint.date_phrase = Some(
            date_phrase
                .clone()
                .unwrap_or_else(|| "the date".to_string()),
        );
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

    if raw.confidence.unwrap_or(1.0) < LOW_CONFIDENCE {
        hint.low_confidence = true;
    }

    let tags = normalize_tags(&raw.tags.unwrap_or_default())
        .into_iter()
        .map(|t| t.to_lowercase())
        .take(3)
        .collect();

    let body = raw
        .body
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty() && b != &title);

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
        tags,
        hint: (!hint.is_empty()).then_some(hint),
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

    fn ctx(projects: &[(String, String)]) -> Context<'_> {
        // Monday 9 November 2026, 10:00 in Rome (09:00 UTC).
        Context {
            now: Utc.with_ymd_and_hms(2026, 11, 9, 9, 0, 0).unwrap(),
            tz: "Europe/Rome".parse().unwrap(),
            projects,
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
    fn schema_requires_every_field() {
        let s = schema();
        let req = s["properties"]["items"]["items"]["required"]
            .as_array()
            .unwrap();
        assert_eq!(req.len(), 12);
    }

    #[test]
    fn user_prompt_has_local_now_weekday_and_projects() {
        let projects = vec![("p1".to_string(), "Spring launch".to_string())];
        let p = user_prompt(" buy milk ", &ctx(&projects));
        assert!(p.contains("Now: 2026-11-09T10:00 (Monday), time zone Europe/Rome"));
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
