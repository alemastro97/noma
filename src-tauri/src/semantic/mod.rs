//! Semantic search and Ask NOMA: a small local embedding model indexes every
//! card in the background; the palette and Ask NOMA rank cards by meaning.
//! See docs/specs/semantic.md.

pub mod ask;
pub mod server;
pub mod vector;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex as AsyncMutex, Notify};

use crate::ai::download::{self, find_file, server_binary_name};
use crate::ai::manifest::{self, EmbeddingSpec};
use crate::ai::service::AiService;
use crate::ai::{AiError, Paths};
use crate::db::{
    self,
    models::Item,
    settings::{get_setting, set_setting},
};
use crate::state::{AppState, local_tz, now};
use server::EmbedServer;

pub const SEMANTIC_STATE: &str = "semantic:state";
const ENABLED_KEY: &str = "semantic.enabled";
/// Cards per `/v1/embeddings` request.
const BATCH: usize = 16;
/// The embedding server stops after this long without use.
const KEEP_WARM: Duration = Duration::from_secs(5 * 60);
/// Item changes are batched for this long before re-indexing.
const DEBOUNCE: Duration = Duration::from_secs(2);
/// Minimum cosine similarity for a palette match / an Ask NOMA source.
pub const SEARCH_MIN_SCORE: f32 = 0.42;
pub const ASK_MIN_SCORE: f32 = 0.3;

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingInfo {
    #[serde(flatten)]
    pub spec: EmbeddingSpec,
    pub installed: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SemanticDownload {
    pub received: u64,
    pub total: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SemanticState {
    /// The user wants semantic search (setting, on by default).
    pub enabled: bool,
    /// Enabled, model and server build installed: the index is maintained.
    pub ready: bool,
    pub model: Option<EmbeddingInfo>,
    pub server_installed: bool,
    pub download: Option<SemanticDownload>,
    pub indexing: bool,
    pub indexed: u32,
    pub total: u32,
    pub running: bool,
    pub error: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SemanticHit {
    pub item: Item,
    pub score: f32,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AskAnswer {
    /// Empty when no card relates to the question (the model is not called).
    pub answer: String,
    /// The cards given to the model, in citation order (`[1]` is `cards[0]`).
    pub cards: Vec<Item>,
    /// 1-based numbers of the cards the answer cites.
    pub cited: Vec<u32>,
}

#[derive(Default)]
struct Runtime {
    download: Option<SemanticDownload>,
    indexing: bool,
    error: Option<String>,
}

pub struct SemanticService {
    app: AppHandle,
    paths: Paths,
    client: reqwest::Client,
    server: AsyncMutex<Option<EmbedServer>>,
    runtime: std::sync::Mutex<Runtime>,
    cancel: std::sync::Mutex<Option<Arc<AtomicBool>>>,
    /// In-memory copy of the index (model id, vectors); dropped on every write.
    cache: std::sync::Mutex<Option<Arc<Vec<(String, Vec<f32>)>>>>,
    last_used: std::sync::Mutex<Instant>,
    dirty: AtomicBool,
    wake: Notify,
}

impl SemanticService {
    pub fn new(app: AppHandle, paths: Paths) -> Arc<Self> {
        let me = Arc::new(Self {
            app,
            client: reqwest::Client::new(),
            server: AsyncMutex::new(None),
            runtime: std::sync::Mutex::new(Runtime::default()),
            cancel: std::sync::Mutex::new(None),
            cache: std::sync::Mutex::new(None),
            last_used: std::sync::Mutex::new(Instant::now()),
            dirty: AtomicBool::new(true),
            wake: Notify::new(),
            paths,
        });
        crate::ai::server::reap_stale(&me.log_path());
        me
    }

    fn log_path(&self) -> std::path::PathBuf {
        self.paths.root.join("logs").join("llama-embed.log")
    }

    fn with_db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> T) -> Option<T> {
        let state = self.app.try_state::<AppState>()?;
        let conn = state.db.lock().ok()?;
        Some(f(&conn))
    }

    fn enabled(&self) -> bool {
        self.with_db(|c| get_setting(c, ENABLED_KEY).ok().flatten())
            .flatten()
            .is_none_or(|v| v != "0")
    }

    fn server_dir(&self) -> Option<std::path::PathBuf> {
        let dir = self.paths.server_dir(&manifest::manifest().server.version);
        find_file(&dir, server_binary_name()).and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }

    fn model_installed(&self, spec: &EmbeddingSpec) -> bool {
        self.paths.model_file(&spec.file).exists()
    }

    /// Everything needed to embed is on disk and the feature is on.
    pub fn ready(&self) -> bool {
        self.enabled()
            && self.server_dir().is_some()
            && manifest::embedding_model().is_some_and(|m| self.model_installed(m))
    }

    pub async fn snapshot(&self) -> SemanticState {
        let spec = manifest::embedding_model();
        let running = match self.server.try_lock() {
            Ok(mut guard) => guard.as_mut().is_some_and(|s| s.is_running()),
            Err(_) => true,
        };
        let (indexed, total) = spec
            .and_then(|s| {
                self.with_db(|c| {
                    let indexed = db::embeddings::count(c, &s.id).unwrap_or(0);
                    let total: u32 = c
                        .query_row("SELECT COUNT(*) FROM items", [], |r| r.get(0))
                        .unwrap_or(0);
                    (indexed, total)
                })
            })
            .unwrap_or((0, 0));
        let rt = self.runtime.lock().expect("runtime lock");
        SemanticState {
            enabled: self.enabled(),
            ready: self.ready(),
            model: spec.map(|s| EmbeddingInfo {
                spec: s.clone(),
                installed: self.model_installed(s),
            }),
            server_installed: self.server_dir().is_some(),
            download: rt.download.clone(),
            indexing: rt.indexing,
            indexed,
            total,
            running,
            error: rt.error.clone(),
        }
    }

    pub async fn publish(&self) {
        let state = self.snapshot().await;
        let _ = self.app.emit(SEMANTIC_STATE, state);
    }

    /// Cards changed: re-index soon.
    pub fn nudge(&self) {
        self.dirty.store(true, Ordering::Relaxed);
        self.wake.notify_one();
    }

    pub async fn set_enabled(&self, enabled: bool) -> Result<(), AiError> {
        self.with_db(|c| set_setting(c, ENABLED_KEY, if enabled { "1" } else { "0" }))
            .ok_or_else(|| AiError::NotReady("database unavailable".into()))??;
        if !enabled {
            self.stop_server().await;
        }
        self.nudge();
        self.publish().await;
        Ok(())
    }

    /// Downloads the server build (when missing) and the embedding model.
    pub async fn download(self: &Arc<Self>) -> Result<(), AiError> {
        let spec = manifest::embedding_model()
            .ok_or_else(|| AiError::NotReady("no embedding model in the manifest".into()))?
            .clone();
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut slot = self.cancel.lock().expect("cancel lock");
            if slot.is_some() {
                return Err(AiError::NotReady("a download is already running".into()));
            }
            *slot = Some(cancel.clone());
        }
        let result = self.download_inner(&spec, cancel).await;
        *self.cancel.lock().expect("cancel lock") = None;
        {
            let mut rt = self.runtime.lock().expect("runtime lock");
            rt.download = None;
            rt.error = match &result {
                Ok(()) | Err(AiError::Cancelled) => None,
                Err(e) => Some(e.to_string()),
            };
        }
        if result.is_ok() {
            self.nudge();
        }
        self.publish().await;
        result
    }

    async fn download_inner(
        self: &Arc<Self>,
        spec: &EmbeddingSpec,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), AiError> {
        let me = Arc::clone(self);
        let progress = move |received: u64, total: u64| {
            me.runtime.lock().expect("runtime lock").download =
                Some(SemanticDownload { received, total });
            let me = Arc::clone(&me);
            tauri::async_runtime::spawn(async move { me.publish().await });
        };
        if self.server_dir().is_none() {
            let asset = manifest::server_asset()
                .ok_or_else(|| AiError::NotReady("no server build for this platform".into()))?;
            let version = &manifest::manifest().server.version;
            let archive = self
                .paths
                .downloads()
                .join(format!("llama-{version}-embed.tar.gz"));
            download::download(
                &asset.url,
                &archive,
                &asset.sha256,
                asset.size_bytes,
                cancel.clone(),
                &progress,
            )
            .await?;
            let into = self.paths.server_dir(version);
            let archive_for_blocking = archive.clone();
            tauri::async_runtime::spawn_blocking(move || {
                download::extract_server(&archive_for_blocking, &into)
            })
            .await
            .map_err(|e| AiError::Server(e.to_string()))??;
            let _ = tokio::fs::remove_file(&archive).await;
        }
        let dest = self.paths.model_file(&spec.file);
        if !dest.exists() {
            download::download(
                &spec.url,
                &dest,
                &spec.sha256,
                spec.size_bytes,
                cancel,
                &progress,
            )
            .await?;
        }
        Ok(())
    }

    pub fn cancel_download(&self) {
        if let Some(c) = self.cancel.lock().expect("cancel lock").as_ref() {
            c.store(true, Ordering::Relaxed);
        }
    }

    /// Deletes the model file and empties the index.
    pub async fn remove(&self) -> Result<(), AiError> {
        self.stop_server().await;
        if let Some(spec) = manifest::embedding_model() {
            let path = self.paths.model_file(&spec.file);
            if path.exists() {
                tokio::fs::remove_file(path).await?;
            }
        }
        let _ = self.with_db(db::embeddings::clear);
        *self.cache.lock().expect("cache lock") = None;
        self.publish().await;
        Ok(())
    }

    /// Drops the index and rebuilds it from scratch.
    pub async fn reindex(&self) {
        let _ = self.with_db(db::embeddings::clear);
        *self.cache.lock().expect("cache lock") = None;
        self.nudge();
        self.publish().await;
    }

    pub async fn stop_server(&self) {
        if let Some(s) = self.server.lock().await.take() {
            s.stop().await;
        }
    }

    pub fn kill_server_now(&self) {
        if let Ok(mut guard) = self.server.try_lock()
            && let Some(s) = guard.as_mut()
        {
            s.kill_now();
        }
    }

    /// Starts the embedding server if needed; returns its base URL.
    async fn ensure_server(&self, spec: &EmbeddingSpec) -> Result<String, AiError> {
        let bin = self
            .server_dir()
            .ok_or_else(|| AiError::NotReady("download the search model first".into()))?;
        let model = self.paths.model_file(&spec.file);
        if !model.exists() {
            return Err(AiError::NotReady("download the search model first".into()));
        }
        let mut guard = self.server.lock().await;
        let reusable = guard
            .as_mut()
            .is_some_and(|s| s.model_id == spec.id && s.is_running());
        if !reusable {
            if let Some(old) = guard.take() {
                old.stop().await;
            }
            let started = server::start(
                &self.client,
                &bin,
                &model,
                &spec.id,
                spec.context_size,
                &self.log_path(),
            )
            .await?;
            *guard = Some(started);
        }
        *self.last_used.lock().expect("last_used") = Instant::now();
        Ok(guard.as_ref().expect("server just ensured").base_url())
    }

    async fn embed(
        &self,
        spec: &EmbeddingSpec,
        texts: &[String],
    ) -> Result<Vec<Vec<f32>>, AiError> {
        let base = self.ensure_server(spec).await?;
        let vectors = server::embed(&self.client, &base, texts).await?;
        *self.last_used.lock().expect("last_used") = Instant::now();
        Ok(vectors.into_iter().map(vector::normalize).collect())
    }

    /// Background loop: keeps the index in step with the cards, stops the
    /// embedding server when idle.
    pub async fn run(self: Arc<Self>) {
        self.publish().await;
        loop {
            if self.dirty.swap(false, Ordering::Relaxed) && self.ready() {
                if let Err(e) = self.index_pass().await {
                    log::warn!("semantic index: {e}");
                    self.runtime.lock().expect("runtime lock").error = Some(e.to_string());
                    self.publish().await;
                }
            }
            tokio::select! {
                _ = self.wake.notified() => tokio::time::sleep(DEBOUNCE).await,
                _ = tokio::time::sleep(Duration::from_secs(60)) => self.stop_if_idle().await,
            }
        }
    }

    async fn stop_if_idle(&self) {
        if self.last_used.lock().expect("last_used").elapsed() < KEEP_WARM {
            return;
        }
        let stopped = match self.server.lock().await.take() {
            Some(s) => {
                s.stop().await;
                true
            }
            None => false,
        };
        if stopped {
            self.publish().await;
        }
    }

    /// Embeds every card whose text changed since it was indexed.
    async fn index_pass(&self) -> Result<(), AiError> {
        let Some(spec) = manifest::embedding_model() else {
            return Ok(());
        };
        let todo = self
            .with_db(|c| -> db::DbResult<Vec<(String, String, String)>> {
                db::embeddings::prune(c, &spec.id)?;
                let known = db::embeddings::hashes(c, &spec.id)?;
                Ok(db::embeddings::sources(c)?
                    .into_iter()
                    .filter_map(|s| {
                        let text = vector::doc_text(&s.title, s.body.as_deref());
                        let hash = vector::content_hash(&spec.id, &text);
                        (known.get(&s.id) != Some(&hash)).then_some((s.id, text, hash))
                    })
                    .collect())
            })
            .ok_or_else(|| AiError::NotReady("database unavailable".into()))??;
        *self.cache.lock().expect("cache lock") = None;
        if todo.is_empty() {
            self.publish().await;
            return Ok(());
        }
        {
            let mut rt = self.runtime.lock().expect("runtime lock");
            rt.indexing = true;
            rt.error = None;
        }
        self.publish().await;
        let result = async {
            for chunk in todo.chunks(BATCH) {
                let texts: Vec<String> = chunk
                    .iter()
                    .map(|(_, text, _)| format!("{}{text}", spec.document_prefix))
                    .collect();
                let vectors = self.embed(spec, &texts).await?;
                self.with_db(|c| -> db::DbResult<()> {
                    for ((id, _, hash), v) in chunk.iter().zip(&vectors) {
                        db::embeddings::upsert(c, id, &spec.id, hash, v, now())?;
                    }
                    Ok(())
                })
                .ok_or_else(|| AiError::NotReady("database unavailable".into()))??;
                self.publish().await;
                if !self.ready() {
                    break;
                }
            }
            Ok::<(), AiError>(())
        }
        .await;
        *self.cache.lock().expect("cache lock") = None;
        self.runtime.lock().expect("runtime lock").indexing = false;
        self.publish().await;
        result
    }

    fn index(&self, spec: &EmbeddingSpec) -> Arc<Vec<(String, Vec<f32>)>> {
        let mut cache = self.cache.lock().expect("cache lock");
        if let Some(index) = cache.as_ref() {
            return Arc::clone(index);
        }
        let index = Arc::new(
            self.with_db(|c| db::embeddings::vectors(c, &spec.id).unwrap_or_default())
                .unwrap_or_default(),
        );
        *cache = Some(Arc::clone(&index));
        index
    }

    /// Card ids ranked by meaning; empty when semantic search is not ready.
    pub async fn rank(
        &self,
        query: &str,
        limit: usize,
        min: f32,
    ) -> Result<Vec<(String, f32)>, AiError> {
        let query = query.trim();
        if query.is_empty() || !self.ready() {
            return Ok(vec![]);
        }
        let Some(spec) = manifest::embedding_model() else {
            return Ok(vec![]);
        };
        let index = self.index(spec);
        if index.is_empty() {
            return Ok(vec![]);
        }
        let text: String = query.chars().take(vector::MAX_DOC_CHARS).collect();
        let q = self
            .embed(spec, &[format!("{}{text}", spec.query_prefix)])
            .await?
            .pop()
            .unwrap_or_default();
        Ok(vector::top_k(&q, &index, limit, min))
    }

    fn items(&self, ids: &[String]) -> Vec<Item> {
        self.with_db(|c| {
            ids.iter()
                .filter_map(|id| db::items::get_item(c, id).ok().flatten())
                .collect()
        })
        .unwrap_or_default()
    }

    pub async fn search(&self, query: &str, limit: usize) -> Result<Vec<SemanticHit>, AiError> {
        let ranked = self.rank(query, limit, SEARCH_MIN_SCORE).await?;
        let ids: Vec<String> = ranked.iter().map(|(id, _)| id.clone()).collect();
        let items = self.items(&ids);
        Ok(ranked
            .into_iter()
            .filter_map(|(id, score)| {
                items.iter().find(|i| i.id == id).map(|item| SemanticHit {
                    item: item.clone(),
                    score,
                })
            })
            .collect())
    }

    /// Answers `question` from the most related cards (by meaning, plus a
    /// keyword fallback) with the local chat model.
    pub async fn ask(
        &self,
        ai: &AiService,
        question: &str,
        history: &[ask::Turn],
    ) -> Result<AskAnswer, AiError> {
        let question = question.trim();
        if question.is_empty() {
            return Err(AiError::NotReady("ask a question".into()));
        }
        let mut ids: Vec<String> = match self.rank(question, ask::MAX_CARDS, ASK_MIN_SCORE).await {
            Ok(r) => r.into_iter().map(|(id, _)| id).collect(),
            Err(e) => {
                log::warn!("ask: semantic retrieval failed, using keywords: {e}");
                vec![]
            }
        };
        let keyword = self
            .with_db(|c| db::embeddings::keyword_ids(c, question, ask::MAX_CARDS as u32))
            .unwrap_or(Ok(vec![]))?;
        for id in keyword {
            if ids.len() >= ask::MAX_CARDS {
                break;
            }
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        let cards = self.items(&ids);
        if cards.is_empty() {
            return Ok(AskAnswer {
                answer: String::new(),
                cards,
                cited: vec![],
            });
        }
        let today = now()
            .with_timezone(&local_tz())
            .format("%A %Y-%m-%d")
            .to_string();
        let user = ask::user_prompt(question, history, &cards, &today);
        let output = ai
            .complete_json(ask::SYSTEM_PROMPT, &user, &ask::schema(cards.len()))
            .await?;
        let (answer, cited) = ask::parse(&output, cards.len())?;
        Ok(AskAnswer {
            answer,
            cards,
            cited,
        })
    }
}
