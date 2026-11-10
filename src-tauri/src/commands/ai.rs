use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use super::{emit_items_changed, lock_message, to_message};
use crate::ai::service::{AiConfigPatch, AiService, AiState};
use crate::db::{
    self,
    models::{Item, ItemType, NewItem, Status},
};
use crate::state::{AppState, now};
use crate::voice::session::VoiceService;

/// Where the capture happened, so the card lands in the right place.
#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct CaptureContext {
    pub project_id: Option<String>,
    pub board_id: Option<String>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub status: Option<Status>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub capture_id: String,
    pub item: Item,
}

const TITLE_MAX: usize = 120;

/// The placeholder card shown while the model reads the capture.
pub fn placeholder(text: &str, ctx: &CaptureContext) -> NewItem {
    let first_line = text.lines().next().unwrap_or("").trim();
    let title: String = first_line.chars().take(TITLE_MAX).collect();
    let body = (text.trim() != title).then(|| text.trim().to_string());
    NewItem {
        kind: ItemType::Note,
        title,
        body,
        status: ctx.status,
        project_id: ctx.project_id.clone(),
        board_id: ctx.board_id.clone(),
        x: ctx.x,
        y: ctx.y,
        ..Default::default()
    }
}

#[tauri::command]
pub async fn ai_state(ai: State<'_, Arc<AiService>>) -> Result<AiState, String> {
    Ok(ai.snapshot().await)
}

#[tauri::command]
pub async fn ai_refresh(ai: State<'_, Arc<AiService>>) -> Result<AiState, String> {
    ai.refresh_ollama().await;
    ai.nudge();
    Ok(ai.snapshot().await)
}

#[tauri::command]
pub async fn ai_configure(
    app: AppHandle,
    ai: State<'_, Arc<AiService>>,
    patch: AiConfigPatch,
) -> Result<AiState, String> {
    let speech_changed = patch.speech_model_id.is_some();
    ai.configure(patch).await.map_err(|e| e.to_string())?;
    if speech_changed && let Some(voice) = app.try_state::<Arc<VoiceService>>() {
        voice.preload();
    }
    Ok(ai.snapshot().await)
}

/// Starts a download in the background; progress arrives as `ai:state` events.
#[tauri::command]
pub async fn ai_download(
    app: AppHandle,
    ai: State<'_, Arc<AiService>>,
    model_id: String,
) -> Result<(), String> {
    let service = Arc::clone(&ai);
    tauri::async_runtime::spawn(async move {
        match service.download(model_id).await {
            Ok(()) => {
                // A freshly downloaded speech model is loaded right away.
                if let Some(voice) = app.try_state::<Arc<VoiceService>>() {
                    voice.preload();
                }
            }
            Err(e) => log::warn!("model download failed: {e}"),
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn ai_cancel_download(ai: State<'_, Arc<AiService>>) -> Result<(), String> {
    ai.cancel_download();
    Ok(())
}

#[tauri::command]
pub async fn ai_remove_model(
    ai: State<'_, Arc<AiService>>,
    model_id: String,
) -> Result<AiState, String> {
    ai.remove_model(&model_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(ai.snapshot().await)
}

/// Creates the placeholder card and the pending capture, then wakes the AI
/// worker. Shared by text capture (quick add, quick capture) and voice.
pub async fn create_capture(
    app: &AppHandle,
    source: &str,
    text: &str,
    ctx: &CaptureContext,
) -> Result<CaptureResult, String> {
    let text = text.trim().to_string();
    if text.is_empty() {
        return Err("nothing to capture".into());
    }
    let state = app.state::<AppState>();
    let (capture_id, item) = {
        let conn = state.db.lock().map_err(lock_message)?;
        let item =
            db::items::create_item(&conn, placeholder(&text, ctx), now()).map_err(to_message)?;
        let capture_id = db::captures::create_capture(&conn, source, &text, Some(&item.id), now())
            .map_err(to_message)?;
        let item = db::items::get_item(&conn, &item.id)
            .map_err(to_message)?
            .ok_or("capture card vanished")?;
        (capture_id, item)
    };
    emit_items_changed(app, vec![item.id.clone()]);
    if let Some(ai) = app.try_state::<Arc<AiService>>() {
        ai.nudge();
        ai.publish().await;
    }
    Ok(CaptureResult { capture_id, item })
}

/// Captures free text: a placeholder card appears at once and the AI worker
/// turns it into typed cards as soon as a model is available.
#[tauri::command]
pub async fn capture_text(
    app: AppHandle,
    text: String,
    context: Option<CaptureContext>,
) -> Result<CaptureResult, String> {
    create_capture(&app, "text", &text, &context.unwrap_or_default()).await
}

/// Re-runs the most recent capture behind a card (after a failure).
#[tauri::command]
pub async fn capture_retry(
    state: State<'_, AppState>,
    ai: State<'_, Arc<AiService>>,
    item_id: String,
) -> Result<(), String> {
    {
        let conn = state.db.lock().map_err(lock_message)?;
        if let Some(c) = db::captures::latest_for_item(&conn, &item_id).map_err(to_message)? {
            db::captures::retry(&conn, &c.id).map_err(to_message)?;
        }
    }
    ai.nudge();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn placeholder_uses_first_line_and_keeps_the_rest_as_body() {
        let p = placeholder(
            "Buy film\nKodak Portra, 3 rolls",
            &CaptureContext::default(),
        );
        assert_eq!(p.kind, ItemType::Note);
        assert_eq!(p.title, "Buy film");
        assert_eq!(p.body.as_deref(), Some("Buy film\nKodak Portra, 3 rolls"));
        let short = placeholder("call mum", &CaptureContext::default());
        assert_eq!(short.body, None);
    }

    #[test]
    fn placeholder_carries_the_capture_context() {
        let ctx = CaptureContext {
            board_id: Some("b1".into()),
            x: Some(10.0),
            y: Some(20.0),
            status: Some(Status::Doing),
            ..Default::default()
        };
        let p = placeholder("x", &ctx);
        assert_eq!(p.board_id.as_deref(), Some("b1"));
        assert_eq!((p.x, p.y), (Some(10.0), Some(20.0)));
        assert_eq!(p.status, Some(Status::Doing));
    }
}
