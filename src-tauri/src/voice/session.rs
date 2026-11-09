//! One voice capture at a time: open the mic, stream level and partial
//! transcripts to the overlay, stop on silence (or ↵ / key release), run the
//! final transcription and hand the text to AI capture.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};

use super::audio::{Recorder, SAMPLE_RATE};
use super::stt::{Stt, Transcript};
use super::vad::{Endpointer, VadEvent};
use crate::ai::service::AiService;
use crate::commands::ai::{CaptureContext, create_capture};
use crate::db::settings::get_setting;
use crate::state::AppState;
use crate::windows;

pub const VOICE_STATE: &str = "voice:state";
pub const VOICE_LEVEL: &str = "voice:level";
pub const VOICE_PARTIAL: &str = "voice:partial";

const TICK: Duration = Duration::from_millis(60);
const PARTIAL_EVERY: Duration = Duration::from_millis(900);
const PARTIAL_WINDOW_S: usize = 15;
const UNLOAD_AFTER: Duration = Duration::from_secs(60);
/// Holding ⌥ Space longer than this makes it push-to-talk.
const HOLD_TO_TALK: Duration = Duration::from_millis(600);

#[derive(Serialize, Clone, Debug, Default)]
#[serde(rename_all = "camelCase")]
pub struct VoiceState {
    /// idle · listening · transcribing · editing · saved · empty · unavailable · error
    pub phase: String,
    pub text: Option<String>,
    pub message: Option<String>,
    pub capture_id: Option<String>,
    pub item_id: Option<String>,
}

impl VoiceState {
    fn phase(p: &str) -> Self {
        Self {
            phase: p.into(),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    Save,
    Cancel,
    /// Stop listening and return the transcript for editing instead of saving.
    Edit,
}

struct Active {
    id: u64,
    recorder: Recorder,
}

pub struct VoiceService {
    app: AppHandle,
    stt: Arc<Mutex<Option<Stt>>>,
    active: Mutex<Option<Active>>,
    generation: AtomicU64,
    pressed_at: Mutex<Option<Instant>>,
    partial_running: Arc<AtomicBool>,
    last: Mutex<VoiceState>,
}

impl VoiceService {
    pub fn new(app: AppHandle) -> Arc<Self> {
        Arc::new(Self {
            app,
            stt: Arc::new(Mutex::new(None)),
            active: Mutex::new(None),
            generation: AtomicU64::new(0),
            pressed_at: Mutex::new(None),
            partial_running: Arc::new(AtomicBool::new(false)),
            last: Mutex::new(VoiceState::phase("idle")),
        })
    }

    fn emit_state(&self, state: VoiceState) {
        *self.last.lock().expect("voice state") = state.clone();
        let _ = self.app.emit(VOICE_STATE, state);
    }

    pub fn current(&self) -> VoiceState {
        self.last.lock().expect("voice state").clone()
    }

    pub fn is_active(&self) -> bool {
        self.active.lock().map(|a| a.is_some()).unwrap_or(false)
    }

    fn auto_stop(&self) -> bool {
        self.app
            .try_state::<AppState>()
            .and_then(|s| {
                s.db.lock()
                    .ok()
                    .and_then(|c| get_setting(&c, "voice.autoStop").ok().flatten())
            })
            .is_none_or(|v| v != "0")
    }

    /// Loads the speech model on a blocking thread if it is not loaded yet.
    async fn ensure_model(&self) -> Result<(), String> {
        let ai = self.app.state::<Arc<AiService>>();
        let (model_id, path) = ai
            .speech_model()
            .ok_or("Download the speech model in Settings to use your voice.")?;
        let loaded = self
            .stt
            .lock()
            .map(|g| g.as_ref().is_some_and(|s| s.model_id == model_id))
            .unwrap_or(false);
        if loaded {
            return Ok(());
        }
        let stt = Arc::clone(&self.stt);
        tauri::async_runtime::spawn_blocking(move || -> Result<(), String> {
            let model = Stt::load(&path, &model_id).map_err(|e| e.to_string())?;
            *stt.lock().map_err(|_| "speech model lock")? = Some(model);
            Ok(())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    async fn transcribe(
        &self,
        audio: Vec<f32>,
        language: Option<String>,
    ) -> Result<Transcript, String> {
        let stt = Arc::clone(&self.stt);
        tauri::async_runtime::spawn_blocking(move || {
            let guard = stt.lock().map_err(|_| "speech model lock".to_string())?;
            let model = guard.as_ref().ok_or("speech model not loaded")?;
            model
                .transcribe(&audio, language.as_deref())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Opens the overlay and starts listening (no-op if already listening).
    pub async fn start(self: &Arc<Self>) {
        if self.is_active() {
            return;
        }
        windows::show(&self.app, windows::OVERLAY);
        if self.app.state::<Arc<AiService>>().speech_model().is_none() {
            self.emit_state(VoiceState {
                message: Some("Download the speech model in Settings to use your voice.".into()),
                ..VoiceState::phase("unavailable")
            });
            return;
        }
        let recorder = match Recorder::start() {
            Ok(r) => r,
            Err(e) => {
                self.emit_state(VoiceState {
                    message: Some(format!(
                        "{e}. Check that NOMA may use the microphone in System Settings › Privacy."
                    )),
                    ..VoiceState::phase("error")
                });
                return;
            }
        };
        let id = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
        *self.active.lock().expect("voice active") = Some(Active { id, recorder });
        self.emit_state(VoiceState::phase("listening"));

        // Warm the model while the user speaks.
        let me = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            if let Err(e) = me.ensure_model().await {
                log::warn!("speech model: {e}");
            }
        });
        let me = Arc::clone(self);
        tauri::async_runtime::spawn(async move { me.monitor(id).await });
    }

    async fn monitor(self: Arc<Self>, id: u64) {
        let mut vad = Endpointer::default();
        let mut last_partial = Instant::now();
        let mut partial_len = 0usize;
        let mut language: Option<String> = None;
        loop {
            tokio::time::sleep(TICK).await;
            let (level, len) = {
                let guard = self.active.lock().expect("voice active");
                match guard.as_ref() {
                    Some(a) if a.id == id => (a.recorder.level(), a.recorder.len()),
                    _ => return,
                }
            };
            let _ = self.app.emit(VOICE_LEVEL, level);
            if vad.push(level, TICK.as_millis() as u32) == VadEvent::Endpoint && self.auto_stop() {
                self.finish(Finish::Save).await;
                return;
            }
            let fresh = len.saturating_sub(partial_len);
            if last_partial.elapsed() >= PARTIAL_EVERY
                && fresh >= SAMPLE_RATE as usize / 2
                && vad.heard_speech()
                && !self.partial_running.swap(true, Ordering::SeqCst)
            {
                last_partial = Instant::now();
                partial_len = len;
                let from = len.saturating_sub(PARTIAL_WINDOW_S * SAMPLE_RATE as usize);
                let audio = {
                    let guard = self.active.lock().expect("voice active");
                    guard
                        .as_ref()
                        .map(|a| a.recorder.copy_from(from))
                        .unwrap_or_default()
                };
                let me = Arc::clone(&self);
                let lang = language.clone();
                let result = me.transcribe(audio, lang).await;
                self.partial_running.store(false, Ordering::SeqCst);
                if let Ok(t) = result {
                    if language.is_none() {
                        language = t.language.clone();
                    }
                    let still = self
                        .active
                        .lock()
                        .map(|g| g.as_ref().is_some_and(|a| a.id == id));
                    if still.unwrap_or(false) && !t.text.is_empty() {
                        let _ = self.app.emit(VOICE_PARTIAL, t.text);
                    }
                }
            }
        }
    }

    /// Stops listening. Save transcribes and captures; Edit returns the text;
    /// Cancel drops the audio.
    pub async fn finish(self: &Arc<Self>, mode: Finish) -> Option<String> {
        let active = self.active.lock().expect("voice active").take()?;
        let samples = active.recorder.stop();
        if mode == Finish::Cancel {
            self.emit_state(VoiceState::phase("idle"));
            self.schedule_unload();
            return None;
        }
        self.emit_state(VoiceState::phase("transcribing"));
        if let Err(e) = self.ensure_model().await {
            self.emit_state(VoiceState {
                message: Some(e),
                ..VoiceState::phase("error")
            });
            return None;
        }
        let transcript = match self.transcribe(samples, None).await {
            Ok(t) => t,
            Err(e) => {
                self.emit_state(VoiceState {
                    message: Some(e),
                    ..VoiceState::phase("error")
                });
                return None;
            }
        };
        self.schedule_unload();
        if transcript.text.is_empty() {
            self.emit_state(VoiceState::phase("empty"));
            return None;
        }
        if mode == Finish::Edit {
            self.emit_state(VoiceState {
                text: Some(transcript.text.clone()),
                ..VoiceState::phase("editing")
            });
            return Some(transcript.text);
        }
        match create_capture(
            &self.app,
            "voice",
            &transcript.text,
            &CaptureContext::default(),
        )
        .await
        {
            Ok(r) => self.emit_state(VoiceState {
                text: Some(transcript.text.clone()),
                capture_id: Some(r.capture_id),
                item_id: Some(r.item.id),
                ..VoiceState::phase("saved")
            }),
            Err(e) => self.emit_state(VoiceState {
                message: Some(e),
                ..VoiceState::phase("error")
            }),
        }
        Some(transcript.text)
    }

    /// Frees the speech model a minute after the last capture.
    fn schedule_unload(self: &Arc<Self>) {
        let generation = self.generation.load(Ordering::SeqCst);
        let me = Arc::clone(self);
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(UNLOAD_AFTER).await;
            if me.generation.load(Ordering::SeqCst) == generation
                && !me.is_active()
                && let Ok(mut g) = me.stt.lock()
            {
                *g = None;
            }
        });
    }

    /// ⌥ Space pressed: start, or stop if already listening (tap-to-toggle).
    pub async fn on_press(self: &Arc<Self>) {
        *self.pressed_at.lock().expect("pressed") = Some(Instant::now());
        if self.is_active() {
            self.finish(Finish::Save).await;
        } else {
            self.start().await;
        }
    }

    /// ⌥ Space released after a long hold: push-to-talk ends the capture.
    pub async fn on_release(self: &Arc<Self>) {
        let held = self
            .pressed_at
            .lock()
            .expect("pressed")
            .take()
            .map(|t| t.elapsed() >= HOLD_TO_TALK)
            .unwrap_or(false);
        if held && self.is_active() {
            self.finish(Finish::Save).await;
        }
    }
}
