//! The AI service: configuration, model downloads, the llama-server lifecycle
//! and the capture worker. All state the UI needs is published as one
//! `ai:state` snapshot whenever something changes.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::{Mutex as AsyncMutex, Notify};

use super::download::{self, find_file, server_binary_name};
use super::extract::{self, Context};
use super::manifest::{self, ModelSpec, SpeechSpec};
use super::provider::{self, Provider};
use super::server::{self, LlamaServer};
use super::{AiError, Paths};
use crate::commands::emit_items_changed;
use crate::db::{
    self,
    captures::{self, CaptureStatus},
    settings::{get_setting, set_setting},
};
use crate::state::{AppState, local_tz, now};

pub const AI_STATE: &str = "ai:state";
pub const AI_CAPTURE: &str = "ai:capture";
const DEFAULT_OLLAMA_URL: &str = "http://127.0.0.1:11434";
const DEFAULT_KEEP_WARM_MIN: u64 = 10;

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Builtin,
    Ollama,
}

#[derive(Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    /// Nothing to do, or no model yet.
    Idle,
    /// Starting the server / loading the model.
    Loading,
    /// Reading a capture.
    Processing,
    Error,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub model_id: String,
    /// "server" or "model".
    pub stage: String,
    pub received: u64,
    pub total: u64,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    #[serde(flatten)]
    pub spec: ModelSpec,
    pub installed: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpeechInfo {
    #[serde(flatten)]
    pub spec: SpeechSpec,
    pub installed: bool,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct AiState {
    pub provider: ProviderKind,
    pub model_id: String,
    pub ollama_model: Option<String>,
    pub ollama_url: String,
    /// The configured provider can take captures right now (or after a warm-up).
    pub ready: bool,
    pub phase: Phase,
    pub message: Option<String>,
    pub server_installed: bool,
    pub server_running: bool,
    pub models: Vec<ModelInfo>,
    pub download: Option<DownloadProgress>,
    pub ollama_available: bool,
    pub ollama_models: Vec<String>,
    pub pending: u32,
    pub speech_model_id: String,
    pub speech_models: Vec<SpeechInfo>,
    /// A speech model is installed: voice capture works.
    pub speech_ready: bool,
}

#[derive(Deserialize, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct AiConfigPatch {
    pub provider: Option<ProviderKind>,
    pub model_id: Option<String>,
    pub ollama_model: Option<String>,
    pub speech_model_id: Option<String>,
}

#[derive(Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct CaptureEvent {
    pub capture_id: String,
    pub status: String,
    pub item_ids: Vec<String>,
    pub error: Option<String>,
    pub latency_ms: Option<i64>,
}

struct Config {
    provider: ProviderKind,
    model_id: String,
    speech_model_id: String,
    ollama_model: Option<String>,
    ollama_url: String,
    keep_warm: Duration,
}

struct Runtime {
    phase: Phase,
    message: Option<String>,
    download: Option<DownloadProgress>,
    ollama: Option<Vec<String>>,
}

pub struct AiService {
    app: AppHandle,
    paths: Paths,
    client: reqwest::Client,
    server: AsyncMutex<Option<LlamaServer>>,
    runtime: std::sync::Mutex<Runtime>,
    cancel: std::sync::Mutex<Option<Arc<AtomicBool>>>,
    last_used: std::sync::Mutex<Instant>,
    wake: Notify,
}

impl AiService {
    pub fn new(app: AppHandle, paths: Paths) -> Arc<Self> {
        server::reap_stale(&paths.server_log());
        Arc::new(Self {
            app,
            paths,
            client: reqwest::Client::new(),
            server: AsyncMutex::new(None),
            runtime: std::sync::Mutex::new(Runtime {
                phase: Phase::Idle,
                message: None,
                download: None,
                ollama: None,
            }),
            cancel: std::sync::Mutex::new(None),
            last_used: std::sync::Mutex::new(Instant::now()),
            wake: Notify::new(),
        })
    }

    fn with_db<T>(&self, f: impl FnOnce(&rusqlite::Connection) -> T) -> Option<T> {
        let state = self.app.try_state::<AppState>()?;
        let conn = state.db.lock().ok()?;
        Some(f(&conn))
    }

    fn config(&self) -> Config {
        self.with_db(|conn| {
            let get = |k: &str| get_setting(conn, k).ok().flatten();
            Config {
                provider: match get("ai.provider").as_deref() {
                    Some("ollama") => ProviderKind::Ollama,
                    _ => ProviderKind::Builtin,
                },
                model_id: get("ai.model")
                    .filter(|id| manifest::model(id).is_some())
                    .unwrap_or_else(|| manifest::default_model().id.clone()),
                speech_model_id: get("ai.speechModel")
                    .filter(|id| manifest::speech_model(id).is_some())
                    .unwrap_or_else(|| manifest::default_speech_model().id.clone()),
                ollama_model: get("ai.ollamaModel").filter(|m| !m.is_empty()),
                ollama_url: get("ai.ollamaUrl").unwrap_or_else(|| DEFAULT_OLLAMA_URL.into()),
                keep_warm: Duration::from_secs(
                    60 * get("ai.keepWarmMinutes")
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(DEFAULT_KEEP_WARM_MIN),
                ),
            }
        })
        .unwrap_or(Config {
            provider: ProviderKind::Builtin,
            model_id: manifest::default_model().id.clone(),
            speech_model_id: manifest::default_speech_model().id.clone(),
            ollama_model: None,
            ollama_url: DEFAULT_OLLAMA_URL.into(),
            keep_warm: Duration::from_secs(60 * DEFAULT_KEEP_WARM_MIN),
        })
    }

    fn server_dir(&self) -> Option<std::path::PathBuf> {
        let dir = self.paths.server_dir(&manifest::manifest().server.version);
        find_file(&dir, server_binary_name()).and_then(|p| p.parent().map(|d| d.to_path_buf()))
    }

    fn model_installed(&self, spec: &ModelSpec) -> bool {
        self.paths.model_file(&spec.file).exists()
    }

    /// The configured speech model, when installed.
    pub fn speech_model(&self) -> Option<(String, std::path::PathBuf)> {
        let cfg = self.config();
        let spec = manifest::speech_model(&cfg.speech_model_id)?;
        let path = self.paths.model_file(&spec.file);
        path.exists().then(|| (spec.id.clone(), path))
    }

    fn ready(&self, cfg: &Config, ollama: &Option<Vec<String>>) -> bool {
        match cfg.provider {
            ProviderKind::Builtin => {
                self.server_dir().is_some()
                    && manifest::model(&cfg.model_id).is_some_and(|m| self.model_installed(m))
            }
            ProviderKind::Ollama => match (&cfg.ollama_model, ollama) {
                (Some(model), Some(list)) => list.iter().any(|m| m == model),
                _ => false,
            },
        }
    }

    pub async fn snapshot(&self) -> AiState {
        let cfg = self.config();
        // Never wait on the server lock here: `provider()` holds it while the
        // model loads and publishes progress, which calls this. Locked = starting.
        let server_running = match self.server.try_lock() {
            Ok(mut guard) => guard.as_mut().is_some_and(|s| s.is_running()),
            Err(_) => true,
        };
        let pending = self
            .with_db(|c| captures::count_pending(c).unwrap_or(0))
            .unwrap_or(0);
        let rt = self.runtime.lock().expect("runtime lock");
        AiState {
            provider: cfg.provider,
            ready: self.ready(&cfg, &rt.ollama),
            model_id: cfg.model_id.clone(),
            ollama_model: cfg.ollama_model.clone(),
            ollama_url: cfg.ollama_url.clone(),
            phase: rt.phase,
            message: rt.message.clone(),
            server_installed: self.server_dir().is_some(),
            server_running,
            models: manifest::manifest()
                .models
                .iter()
                .map(|m| ModelInfo {
                    spec: m.clone(),
                    installed: self.model_installed(m),
                })
                .collect(),
            download: rt.download.clone(),
            ollama_available: rt.ollama.is_some(),
            ollama_models: rt.ollama.clone().unwrap_or_default(),
            pending,
            speech_ready: manifest::speech_model(&cfg.speech_model_id)
                .is_some_and(|m| self.paths.model_file(&m.file).exists()),
            speech_model_id: cfg.speech_model_id.clone(),
            speech_models: manifest::manifest()
                .speech
                .iter()
                .map(|m| SpeechInfo {
                    spec: m.clone(),
                    installed: self.paths.model_file(&m.file).exists(),
                })
                .collect(),
        }
    }

    pub async fn publish(&self) {
        let state = self.snapshot().await;
        let _ = self.app.emit(AI_STATE, state);
    }

    fn set_phase(&self, phase: Phase, message: Option<String>) {
        let mut rt = self.runtime.lock().expect("runtime lock");
        rt.phase = phase;
        rt.message = message;
    }

    pub async fn refresh_ollama(&self) {
        let url = self.config().ollama_url;
        let list = provider::ollama_models(&self.client, &url).await;
        self.runtime.lock().expect("runtime lock").ollama = list;
    }

    pub async fn configure(&self, patch: AiConfigPatch) -> Result<(), AiError> {
        let changed_model = patch.model_id.is_some();
        self.with_db(|conn| -> Result<(), AiError> {
            if let Some(p) = patch.provider {
                let v = if p == ProviderKind::Ollama {
                    "ollama"
                } else {
                    "builtin"
                };
                set_setting(conn, "ai.provider", v)?;
            }
            if let Some(m) = &patch.model_id {
                if manifest::model(m).is_none() {
                    return Err(AiError::NotReady(format!("unknown model {m}")));
                }
                set_setting(conn, "ai.model", m)?;
            }
            if let Some(m) = &patch.ollama_model {
                set_setting(conn, "ai.ollamaModel", m)?;
            }
            if let Some(m) = &patch.speech_model_id {
                if manifest::speech_model(m).is_none() {
                    return Err(AiError::NotReady(format!("unknown speech model {m}")));
                }
                set_setting(conn, "ai.speechModel", m)?;
            }
            Ok(())
        })
        .ok_or_else(|| AiError::NotReady("database unavailable".into()))??;
        if changed_model {
            self.stop_server().await;
        }
        self.refresh_ollama().await;
        self.set_phase(Phase::Idle, None);
        self.publish().await;
        self.wake.notify_one();
        Ok(())
    }

    /// Downloads the server build (if missing) and `model_id`, publishing progress.
    pub async fn download(self: &Arc<Self>, model_id: String) -> Result<(), AiError> {
        if let Some(speech) = manifest::speech_model(&model_id) {
            return self.download_speech(speech.clone()).await;
        }
        let spec = manifest::model(&model_id)
            .ok_or_else(|| AiError::NotReady(format!("unknown model {model_id}")))?
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
        self.runtime.lock().expect("runtime lock").download = None;
        match &result {
            Ok(()) => {
                // Make the freshly downloaded model the active one.
                let _ = self
                    .configure(AiConfigPatch {
                        provider: Some(ProviderKind::Builtin),
                        model_id: Some(spec.id.clone()),
                        ..Default::default()
                    })
                    .await;
            }
            Err(AiError::Cancelled) => self.set_phase(Phase::Idle, None),
            Err(e) => self.set_phase(Phase::Error, Some(e.to_string())),
        }
        self.publish().await;
        result
    }

    async fn download_speech(self: &Arc<Self>, spec: SpeechSpec) -> Result<(), AiError> {
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut slot = self.cancel.lock().expect("cancel lock");
            if slot.is_some() {
                return Err(AiError::NotReady("a download is already running".into()));
            }
            *slot = Some(cancel.clone());
        }
        let me = Arc::clone(self);
        let model_id = spec.id.clone();
        let progress = move |received: u64, total: u64| {
            me.runtime.lock().expect("runtime lock").download = Some(DownloadProgress {
                model_id: model_id.clone(),
                stage: "speech".into(),
                received,
                total,
            });
            let me = Arc::clone(&me);
            tauri::async_runtime::spawn(async move { me.publish().await });
        };
        let dest = self.paths.model_file(&spec.file);
        let result = if dest.exists() {
            Ok(())
        } else {
            download::download(
                &spec.url,
                &dest,
                &spec.sha256,
                spec.size_bytes,
                cancel,
                &progress,
            )
            .await
        };
        *self.cancel.lock().expect("cancel lock") = None;
        self.runtime.lock().expect("runtime lock").download = None;
        match &result {
            Ok(()) => {
                let _ = self
                    .configure(AiConfigPatch {
                        speech_model_id: Some(spec.id.clone()),
                        ..Default::default()
                    })
                    .await;
            }
            Err(AiError::Cancelled) => self.set_phase(Phase::Idle, None),
            Err(e) => self.set_phase(Phase::Error, Some(e.to_string())),
        }
        self.publish().await;
        result
    }

    async fn download_inner(
        self: &Arc<Self>,
        spec: &ModelSpec,
        cancel: Arc<AtomicBool>,
    ) -> Result<(), AiError> {
        let progress_for = |stage: &'static str| {
            let me = Arc::clone(self);
            let model_id = spec.id.clone();
            move |received: u64, total: u64| {
                me.runtime.lock().expect("runtime lock").download = Some(DownloadProgress {
                    model_id: model_id.clone(),
                    stage: stage.into(),
                    received,
                    total,
                });
                let me = Arc::clone(&me);
                tauri::async_runtime::spawn(async move { me.publish().await });
            }
        };

        if self.server_dir().is_none() {
            let asset = manifest::server_asset()
                .ok_or_else(|| AiError::NotReady("no server build for this platform".into()))?;
            let archive = self.paths.downloads().join(format!(
                "llama-{}.tar.gz",
                manifest::manifest().server.version
            ));
            download::download(
                &asset.url,
                &archive,
                &asset.sha256,
                asset.size_bytes,
                cancel.clone(),
                &progress_for("server"),
            )
            .await?;
            let into = self.paths.server_dir(&manifest::manifest().server.version);
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
                &progress_for("model"),
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

    pub async fn remove_model(&self, model_id: &str) -> Result<(), AiError> {
        if let Some(speech) = manifest::speech_model(model_id) {
            let path = self.paths.model_file(&speech.file);
            if path.exists() {
                tokio::fs::remove_file(path).await?;
            }
            self.publish().await;
            return Ok(());
        }
        let spec = manifest::model(model_id)
            .ok_or_else(|| AiError::NotReady(format!("unknown model {model_id}")))?;
        {
            let mut guard = self.server.lock().await;
            if guard.as_ref().is_some_and(|s| s.model_id == model_id)
                && let Some(s) = guard.take()
            {
                s.stop().await;
            }
        }
        let path = self.paths.model_file(&spec.file);
        if path.exists() {
            tokio::fs::remove_file(path).await?;
        }
        self.publish().await;
        Ok(())
    }

    pub async fn stop_server(&self) {
        if let Some(s) = self.server.lock().await.take() {
            s.stop().await;
        }
    }

    /// Synchronous shutdown for the app exit path.
    pub fn kill_server_now(&self) {
        if let Ok(mut guard) = self.server.try_lock()
            && let Some(s) = guard.as_mut()
        {
            s.kill_now();
        }
    }

    /// Ensures a provider is up and returns it.
    async fn provider(&self) -> Result<Provider, AiError> {
        let cfg = self.config();
        match cfg.provider {
            ProviderKind::Ollama => {
                let model = cfg
                    .ollama_model
                    .ok_or_else(|| AiError::NotReady("choose an Ollama model".into()))?;
                Ok(Provider::Ollama {
                    base_url: cfg.ollama_url,
                    model,
                })
            }
            ProviderKind::Builtin => {
                let spec = manifest::model(&cfg.model_id)
                    .ok_or_else(|| AiError::NotReady("unknown model".into()))?;
                let bin = self
                    .server_dir()
                    .ok_or_else(|| AiError::NotReady("download the model first".into()))?;
                let model_path = self.paths.model_file(&spec.file);
                if !model_path.exists() {
                    return Err(AiError::NotReady("download the model first".into()));
                }
                let mut guard = self.server.lock().await;
                let reusable = guard
                    .as_mut()
                    .is_some_and(|s| s.model_id == spec.id && s.is_running());
                if !reusable {
                    if let Some(old) = guard.take() {
                        old.stop().await;
                    }
                    self.set_phase(Phase::Loading, Some(format!("Loading {}", spec.name)));
                    self.publish().await;
                    let started = server::start(
                        &self.client,
                        &bin,
                        &model_path,
                        &spec.id,
                        spec.context_size,
                        &self.paths.server_log(),
                    )
                    .await?;
                    *guard = Some(started);
                }
                let s = guard.as_ref().expect("server just ensured");
                Ok(Provider::Llama {
                    base_url: s.base_url(),
                    thinking: spec.thinking,
                })
            }
        }
    }

    /// One schema-constrained completion outside the capture queue (organize
    /// features: re-sort, break down, summaries; Ask NOMA). Starts the model if needed.
    pub async fn complete_json(
        &self,
        system: &str,
        user: &str,
        schema: &serde_json::Value,
    ) -> Result<String, AiError> {
        let provider = self.provider().await?;
        if self.runtime.lock().expect("runtime lock").phase == Phase::Loading {
            self.set_phase(Phase::Idle, None);
            self.publish().await;
        }
        let out = provider
            .complete_json(&self.client, system, user, schema)
            .await;
        *self.last_used.lock().expect("last_used") = Instant::now();
        out
    }

    /// Wakes the worker (new capture, model became available…).
    pub fn nudge(&self) {
        self.wake.notify_one();
    }

    /// Long-running loop: processes pending captures one at a time.
    pub async fn run_worker(self: Arc<Self>) {
        let _ = self.with_db(captures::requeue_interrupted);
        self.refresh_ollama().await;
        self.publish().await;
        loop {
            self.drain().await;
            tokio::select! {
                _ = self.wake.notified() => {}
                _ = tokio::time::sleep(Duration::from_secs(30)) => {
                    self.stop_if_idle().await;
                }
            }
        }
    }

    async fn stop_if_idle(&self) {
        let keep_warm = self.config().keep_warm;
        let idle = self.last_used.lock().expect("last_used").elapsed() > keep_warm;
        if idle {
            let mut guard = self.server.lock().await;
            if let Some(s) = guard.take() {
                s.stop().await;
                drop(guard);
                self.publish().await;
            }
        }
    }

    async fn drain(&self) {
        loop {
            let cfg = self.config();
            if cfg.provider == ProviderKind::Ollama {
                self.refresh_ollama().await;
            }
            let ready = {
                let rt = self.runtime.lock().expect("runtime lock");
                self.ready(&cfg, &rt.ollama)
            };
            if !ready {
                return;
            }
            let Some(Some(capture)) = self.with_db(|c| captures::next_pending(c).ok().flatten())
            else {
                return;
            };
            self.process(capture).await;
        }
    }

    async fn process(&self, capture: captures::Capture) {
        let _ = self.with_db(|c| {
            captures::set_status(c, &capture.id, CaptureStatus::Processing, None, None)
        });
        if let Some(item) = &capture.item_id {
            emit_items_changed(&self.app, vec![item.clone()]);
        }
        let started = Instant::now();
        let outcome = self.extract_and_apply(&capture).await;
        let latency = started.elapsed().as_millis() as i64;
        *self.last_used.lock().expect("last_used") = Instant::now();

        let event = match outcome {
            Ok((ids, output)) => {
                let _ = self.with_db(|c| {
                    captures::set_status(
                        c,
                        &capture.id,
                        CaptureStatus::Done,
                        Some(&output),
                        Some(latency),
                    )
                });
                self.set_phase(Phase::Idle, None);
                crate::filed::announce(&self.app, &capture.id, &ids);
                CaptureEvent {
                    capture_id: capture.id.clone(),
                    status: "done".into(),
                    item_ids: ids,
                    error: None,
                    latency_ms: Some(latency),
                }
            }
            Err(AiError::NotReady(msg)) => {
                // Leave it pending; it runs once the model is available.
                let _ = self.with_db(|c| {
                    captures::set_status(c, &capture.id, CaptureStatus::Pending, None, None)
                });
                self.set_phase(Phase::Idle, Some(msg));
                self.publish().await;
                return;
            }
            Err(e) => {
                log::warn!("capture {} failed: {e}", capture.id);
                let _ = self.with_db(|c| {
                    captures::set_status(c, &capture.id, CaptureStatus::Failed, None, Some(latency))
                });
                self.set_phase(Phase::Error, Some(e.to_string()));
                CaptureEvent {
                    capture_id: capture.id.clone(),
                    status: "failed".into(),
                    item_ids: capture.item_id.clone().into_iter().collect(),
                    error: Some(e.to_string()),
                    latency_ms: Some(latency),
                }
            }
        };
        emit_items_changed(&self.app, event.item_ids.clone());
        let _ = self.app.emit(AI_CAPTURE, event);
        self.publish().await;
    }

    async fn extract_and_apply(
        &self,
        capture: &captures::Capture,
    ) -> Result<(Vec<String>, String), AiError> {
        let provider = self.provider().await?;
        self.set_phase(Phase::Processing, None);
        self.publish().await;

        let (projects, boards, recent, examples) = self
            .with_db(|c| -> db::DbResult<_> {
                let projects: Vec<(String, String)> = db::projects::list_projects(c)?
                    .into_iter()
                    .map(|p| (p.id, p.name))
                    .collect();
                let boards: Vec<(String, String)> = db::boards::list_boards(c)?
                    .into_iter()
                    .filter(|b| b.kind == db::models::BoardKind::Personal)
                    .map(|b| (b.id, b.name))
                    .collect();
                // What each place is about: its latest cards.
                let mut recent = Vec::new();
                for (id, _) in &projects {
                    recent.push((id.clone(), db::items::recent_titles(c, id, false, 3)?));
                }
                for (id, _) in &boards {
                    recent.push((id.clone(), db::items::recent_titles(c, id, true, 3)?));
                }
                let examples =
                    db::filing::pick(&capture.raw_text, &db::filing::examples(c, 60)?, 4);
                Ok((projects, boards, recent, examples))
            })
            .ok_or_else(|| AiError::NotReady("database unavailable".into()))??;
        let (now, tz) = (now(), local_tz());
        let ctx = Context {
            now,
            tz,
            projects: &projects,
            boards: &boards,
            recent: &recent,
            examples: &examples,
        };
        // The user's own tags: the model may reuse them, never invent new ones.
        let tags = self
            .with_db(|c| crate::organize::tags::known(c).unwrap_or_default())
            .unwrap_or_default();
        let user = crate::organize::tags::with_prompt(
            extract::user_prompt(&capture.raw_text, &ctx),
            &tags,
        );
        let output = provider
            .complete_json(
                &self.client,
                extract::SYSTEM_PROMPT,
                &user,
                &crate::organize::tags::with_schema(
                    extract::schema_for(&capture.raw_text, &ctx),
                    &tags,
                ),
            )
            .await?;
        let mut extracted = extract::parse_output(&output, &capture.raw_text, &ctx)?;
        for e in &mut extracted {
            e.tags = crate::organize::tags::snap(&e.tags, &tags);
        }
        let ids = self
            .with_db(|c| super::apply::apply(c, capture.item_id.as_deref(), &extracted, now, tz))
            .ok_or_else(|| AiError::NotReady("database unavailable".into()))??;
        Ok((ids, output))
    }
}
