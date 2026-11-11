//! Tauri commands for the organize features: thin adapters over the modules
//! next door. Model calls go through `AiService::complete_json`; the database
//! lock is never held across one.

use std::sync::Arc;

use serde::Serialize;
use tauri::{AppHandle, State};

use super::{breakdown, briefs, duplicates, resort, review};
use crate::ai::extract::{self, Context};
use crate::ai::service::AiService;
use crate::commands::{emit_items_changed, lock_message, to_message};
use crate::db::{self, models::Item};
use crate::state::{AppState, local_tz, now};

fn ai_message(e: crate::ai::AiError) -> String {
    e.to_string()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResortResult {
    /// Cards looked at.
    pub checked: u32,
    /// Cards moved out of the Inbox.
    pub moved: u32,
}

/// Re-runs filing for Inbox cards that came from a capture.
#[tauri::command]
pub async fn organize_resort_inbox(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
) -> Result<ResortResult, String> {
    let (candidates, fc) = {
        let conn = state.db.lock().map_err(lock_message)?;
        (
            resort::candidates(&conn).map_err(to_message)?,
            resort::filing_context(&conn).map_err(to_message)?,
        )
    };
    let mut result = ResortResult {
        checked: 0,
        moved: 0,
    };
    if fc.projects.is_empty() && fc.boards.is_empty() {
        return Ok(result);
    }
    let mut moved_ids = Vec::new();
    for c in candidates {
        let (now, tz) = (now(), local_tz());
        let examples = db::filing::pick(&c.text, &fc.examples, 4);
        let ctx = Context {
            now,
            tz,
            projects: &fc.projects,
            boards: &fc.boards,
            recent: &fc.recent,
            examples: &examples,
        };
        let reply = ai
            .complete_json(
                extract::SYSTEM_PROMPT,
                &extract::user_prompt(&c.text, &ctx),
                &extract::schema_for(&c.text, &ctx),
            )
            .await;
        result.checked += 1;
        let reply = match reply {
            Ok(r) => r,
            // No model: nothing else will work either.
            Err(e @ crate::ai::AiError::NotReady(_)) => return Err(ai_message(e)),
            Err(e) => {
                log::warn!("re-sort of {} failed: {e}", c.item_id);
                continue;
            }
        };
        let Some(place) = extract::parse_output(&reply, &c.text, &ctx)
            .ok()
            .and_then(|x| resort::place_for(&x))
        else {
            continue;
        };
        let conn = state.db.lock().map_err(lock_message)?;
        // The user may have moved it meanwhile.
        if db::filing::place_of(&conn, &c.item_id)
            .map(|(p, _)| p == db::filing::Place::Inbox)
            .unwrap_or(false)
        {
            db::filing::move_item(&conn, &c.item_id, &place, now, tz).map_err(to_message)?;
            result.moved += 1;
            moved_ids.push(c.item_id.clone());
        }
    }
    if !moved_ids.is_empty() {
        emit_items_changed(&app, moved_ids);
    }
    Ok(result)
}

/// Folds `duplicate` into `into` ("Merge" on a "Looks like …" card).
#[tauri::command]
pub fn organize_merge(
    app: AppHandle,
    state: State<'_, AppState>,
    duplicate: String,
    into: String,
) -> Result<Item, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    let item =
        duplicates::merge(&conn, &duplicate, &into, now(), local_tz()).map_err(to_message)?;
    drop(conn);
    emit_items_changed(&app, vec![duplicate, into]);
    Ok(item)
}

/// Asks the model for next steps and creates them as linked task cards.
#[tauri::command]
pub async fn organize_break_down(
    app: AppHandle,
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    item_id: String,
) -> Result<Vec<String>, String> {
    let (parent, project) = {
        let conn = state.db.lock().map_err(lock_message)?;
        let parent = db::items::get_item(&conn, &item_id)
            .map_err(to_message)?
            .ok_or("card not found")?;
        let project = match &parent.project_id {
            Some(p) => db::projects::get_project(&conn, p)
                .map_err(to_message)?
                .map(|p| p.name),
            None => None,
        };
        (parent, project)
    };
    let reply = ai
        .complete_json(
            breakdown::SYSTEM_PROMPT,
            &breakdown::user_prompt(&parent, project.as_deref()),
            &breakdown::schema(),
        )
        .await
        .map_err(ai_message)?;
    let steps = breakdown::parse(&reply, &parent.title).map_err(ai_message)?;
    let ids = {
        let conn = state.db.lock().map_err(lock_message)?;
        breakdown::create_steps(&conn, &parent, &steps, now()).map_err(to_message)?
    };
    let mut changed = ids.clone();
    changed.push(item_id);
    emit_items_changed(&app, changed);
    Ok(ids)
}

#[tauri::command]
pub fn organize_brief_get(
    state: State<'_, AppState>,
    project_id: String,
) -> Result<briefs::ProjectBrief, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    briefs::get(&conn, &project_id).map_err(to_message)
}

fn project_name(state: &AppState, project_id: &str) -> Result<String, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    db::projects::get_project(&conn, project_id)
        .map_err(to_message)?
        .map(|p| p.name)
        .ok_or_else(|| "project not found".into())
}

/// Writes (or rewrites) the project's status paragraph.
#[tauri::command]
pub async fn organize_summarize_project(
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    project_id: String,
) -> Result<briefs::ProjectBrief, String> {
    let name = project_name(&state, &project_id)?;
    let (now, tz) = (now(), local_tz());
    let (done, open) = {
        let conn = state.db.lock().map_err(lock_message)?;
        briefs::cards(&conn, &project_id, now).map_err(to_message)?
    };
    if done.is_empty() && open.is_empty() {
        return Err("this project has no cards yet".into());
    }
    let reply = ai
        .complete_json(
            briefs::SUMMARY_PROMPT,
            &briefs::summary_prompt(&name, &done, &open, now, tz),
            &briefs::summary_schema(),
        )
        .await
        .map_err(ai_message)?;
    let summary = briefs::parse_summary(&reply).map_err(ai_message)?;
    let conn = state.db.lock().map_err(lock_message)?;
    briefs::set_summary(&conn, &project_id, &summary, now).map_err(to_message)?;
    briefs::get(&conn, &project_id).map_err(to_message)
}

/// Picks the project's single next action from its open cards.
#[tauri::command]
pub async fn organize_next_action(
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    project_id: String,
) -> Result<briefs::ProjectBrief, String> {
    let name = project_name(&state, &project_id)?;
    let (now, tz) = (now(), local_tz());
    let open = {
        let conn = state.db.lock().map_err(lock_message)?;
        briefs::actionable(
            &briefs::cards(&conn, &project_id, now)
                .map_err(to_message)?
                .1,
        )
    };
    let pick = match open.len() {
        0 => None,
        // One open card: it is the next action, no model needed.
        1 => Some((0, open[0].title.clone())),
        n => {
            let reply = ai
                .complete_json(
                    briefs::NEXT_PROMPT,
                    &briefs::next_prompt(&name, &open, now, tz),
                    &briefs::next_schema(n),
                )
                .await
                .map_err(ai_message)?;
            Some(briefs::parse_next(&reply, n).map_err(ai_message)?)
        }
    };
    let conn = state.db.lock().map_err(lock_message)?;
    match pick {
        Some((i, action)) => {
            briefs::set_next(&conn, &project_id, Some(&action), Some(&open[i].id), now)
        }
        None => briefs::set_next(&conn, &project_id, None, None, now),
    }
    .map_err(to_message)?;
    briefs::get(&conn, &project_id).map_err(to_message)
}

#[tauri::command]
pub fn organize_review(state: State<'_, AppState>) -> Result<review::Review, String> {
    let conn = state.db.lock().map_err(lock_message)?;
    review::build(&conn, now()).map_err(to_message)
}

/// The model's three-sentence take on the week.
#[tauri::command]
pub async fn organize_review_summary(
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
) -> Result<String, String> {
    let (now, tz) = (now(), local_tz());
    let r = {
        let conn = state.db.lock().map_err(lock_message)?;
        review::build(&conn, now).map_err(to_message)?
    };
    let reply = ai
        .complete_json(
            review::SUMMARY_PROMPT,
            &review::user_prompt(&r, now, tz),
            &review::schema(),
        )
        .await
        .map_err(ai_message)?;
    review::parse(&reply).map_err(ai_message)
}
