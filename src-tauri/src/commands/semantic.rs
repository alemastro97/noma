//! Semantic search and Ask NOMA commands: thin adapters over `SemanticService`.

use std::sync::Arc;

use tauri::State;

use crate::ai::service::AiService;
use crate::semantic::{AskAnswer, SemanticHit, SemanticService, SemanticState, ask::Turn};

#[tauri::command]
pub async fn semantic_state(sem: State<'_, Arc<SemanticService>>) -> Result<SemanticState, String> {
    Ok(sem.snapshot().await)
}

#[tauri::command]
pub async fn semantic_configure(
    sem: State<'_, Arc<SemanticService>>,
    enabled: bool,
) -> Result<SemanticState, String> {
    sem.set_enabled(enabled).await.map_err(|e| e.to_string())?;
    Ok(sem.snapshot().await)
}

/// Starts the download in the background; progress arrives as `semantic:state`.
#[tauri::command]
pub async fn semantic_download(sem: State<'_, Arc<SemanticService>>) -> Result<(), String> {
    let service = Arc::clone(&sem);
    tauri::async_runtime::spawn(async move {
        if let Err(e) = service.download().await {
            log::warn!("embedding model download failed: {e}");
        }
    });
    Ok(())
}

#[tauri::command]
pub async fn semantic_cancel_download(sem: State<'_, Arc<SemanticService>>) -> Result<(), String> {
    sem.cancel_download();
    Ok(())
}

#[tauri::command]
pub async fn semantic_remove(
    sem: State<'_, Arc<SemanticService>>,
) -> Result<SemanticState, String> {
    sem.remove().await.map_err(|e| e.to_string())?;
    Ok(sem.snapshot().await)
}

#[tauri::command]
pub async fn semantic_reindex(
    sem: State<'_, Arc<SemanticService>>,
) -> Result<SemanticState, String> {
    sem.reindex().await;
    Ok(sem.snapshot().await)
}

/// Cards related by meaning; empty when semantic search is off or not set up.
#[tauri::command]
pub async fn semantic_search(
    sem: State<'_, Arc<SemanticService>>,
    query: String,
    limit: Option<u32>,
) -> Result<Vec<SemanticHit>, String> {
    sem.search(&query, limit.unwrap_or(8).min(50) as usize)
        .await
        .map_err(|e| e.to_string())
}

/// One Ask NOMA turn: retrieve related cards, answer with citations.
#[tauri::command]
pub async fn ask_noma(
    sem: State<'_, Arc<SemanticService>>,
    ai: State<'_, Arc<AiService>>,
    question: String,
    history: Option<Vec<Turn>>,
) -> Result<AskAnswer, String> {
    sem.ask(&ai, &question, &history.unwrap_or_default())
        .await
        .map_err(|e| e.to_string())
}
