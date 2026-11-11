//! Speech-to-text with whisper.cpp (Metal) through whisper-rs. macOS only for
//! now; other platforms get a clear "not available" error.

use std::path::Path;

use super::VoiceError;
use super::audio::SAMPLE_RATE;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Transcript {
    pub text: String,
    /// Detected language code ("it", "en", …).
    pub language: Option<String>,
}

/// Phrases whisper produces on silence or noise; never a real capture.
const HALLUCINATIONS: &[&str] = &[
    "thank you",
    "thank you.",
    "thanks for watching!",
    "thanks for watching.",
    "grazie.",
    "grazie",
    "grazie a tutti.",
    "sottotitoli creati dalla comunità amara.org",
    "sottotitoli e revisione a cura di qtss",
    "you",
    ".",
];

/// Encoder frames (50 per second) for a clip, with margin, rounded to 64 and
/// clamped to Whisper's 30 s maximum of 1500.
pub fn audio_ctx_for(samples: usize) -> i32 {
    let frames = (samples as f32 / SAMPLE_RATE as f32 * 50.0) as i32 + 250;
    ((frames + 63) / 64 * 64).clamp(512, 1500)
}

/// Removes bracketed annotations ("[Music]", "(applause)") and known
/// silence hallucinations; collapses whitespace.
pub fn clean_transcript(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut depth = 0u32;
    for c in raw.chars() {
        match c {
            '[' | '(' => depth += 1,
            ']' | ')' if depth > 0 => depth -= 1,
            _ if depth > 0 => {}
            _ => out.push(c),
        }
    }
    let text = out.split_whitespace().collect::<Vec<_>>().join(" ");
    if HALLUCINATIONS.contains(&text.to_lowercase().as_str()) {
        String::new()
    } else {
        text
    }
}

#[cfg(target_os = "macos")]
pub struct Stt {
    #[allow(dead_code)]
    ctx: whisper_rs::WhisperContext,
    state: std::sync::Mutex<whisper_rs::WhisperState>,
    pub model_id: String,
}

#[cfg(target_os = "macos")]
impl Stt {
    /// Loads the model and runs one silent pass so Metal kernels are compiled
    /// and buffers allocated before the first real capture.
    pub fn load(path: &Path, model_id: &str) -> Result<Self, VoiceError> {
        let mut params = whisper_rs::WhisperContextParameters::default();
        params.flash_attn(true);
        let ctx = whisper_rs::WhisperContext::new_with_params(path, params)
            .map_err(|e| VoiceError::Stt(e.to_string()))?;
        let state = ctx
            .create_state()
            .map_err(|e| VoiceError::Stt(e.to_string()))?;
        let stt = Self {
            ctx,
            state: std::sync::Mutex::new(state),
            model_id: model_id.to_string(),
        };
        let _ = stt.run(&vec![0.0; SAMPLE_RATE as usize * 2], Some("en"), None);
        Ok(stt)
    }

    /// Transcribes 16 kHz mono audio. `language` pins the language (for
    /// partials after the first detection); None auto-detects. `prompt` is the
    /// personal vocabulary (project names, people) Whisper should expect.
    pub fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<Transcript, VoiceError> {
        if audio.len() < SAMPLE_RATE as usize / 4 {
            return Ok(Transcript::default());
        }
        self.run(audio, language, prompt)
    }

    fn run(
        &self,
        audio: &[f32],
        language: Option<&str>,
        prompt: Option<&str>,
    ) -> Result<Transcript, VoiceError> {
        use whisper_rs::{FullParams, SamplingStrategy};
        let mut state = self
            .state
            .lock()
            .map_err(|_| VoiceError::Stt("speech state lock".into()))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language.unwrap_or("auto")));
        params.set_n_threads(4);
        params.set_no_timestamps(true);
        // Whisper always encodes a 30 s window; shrink it to the real clip
        // (plus margin) so a 3 s utterance doesn't cost a 30 s encode.
        params.set_audio_ctx(audio_ctx_for(audio.len()));
        // No temperature fallback: it silently re-decodes and doubles latency.
        params.set_temperature_inc(0.0);
        params.set_no_context(true);
        params.set_suppress_blank(true);
        if let Some(p) = prompt.filter(|p| !p.is_empty()) {
            params.set_initial_prompt(p);
        }
        params.set_print_special(false);
        params.set_print_progress(false);
        params.set_print_realtime(false);
        params.set_print_timestamps(false);
        // whisper.cpp wants at least one second; pad short clips with silence.
        let min = SAMPLE_RATE as usize + SAMPLE_RATE as usize / 10;
        let padded;
        let data = if audio.len() < min {
            let mut v = audio.to_vec();
            v.resize(min, 0.0);
            padded = v;
            &padded[..]
        } else {
            audio
        };
        state
            .full(params, data)
            .map_err(|e| VoiceError::Stt(e.to_string()))?;
        let raw: String = state
            .as_iter()
            .filter_map(|seg| seg.to_str_lossy().ok().map(|c| c.into_owned()))
            .collect::<Vec<_>>()
            .join("");
        let language =
            whisper_rs::get_lang_str(state.full_lang_id_from_state()).map(str::to_string);
        Ok(Transcript {
            text: clean_transcript(&raw),
            language,
        })
    }
}

#[cfg(not(target_os = "macos"))]
pub struct Stt {
    pub model_id: String,
}

#[cfg(not(target_os = "macos"))]
impl Stt {
    pub fn load(_path: &Path, _model_id: &str) -> Result<Self, VoiceError> {
        Err(VoiceError::Unavailable(
            "voice capture is available on macOS only for now".into(),
        ))
    }

    pub fn transcribe(
        &self,
        _audio: &[f32],
        _language: Option<&str>,
        _prompt: Option<&str>,
    ) -> Result<Transcript, VoiceError> {
        Err(VoiceError::Unavailable("voice capture unavailable".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn audio_ctx_scales_with_clip_and_is_capped() {
        assert_eq!(audio_ctx_for(16_000), 512);
        assert_eq!(audio_ctx_for(16_000 * 10), 768);
        assert_eq!(audio_ctx_for(16_000 * 60), 1500);
    }

    #[test]
    fn cleans_annotations_and_hallucinations() {
        assert_eq!(
            clean_transcript(" [Music] call  Marta (laughs) tomorrow "),
            "call Marta tomorrow"
        );
        assert_eq!(clean_transcript("Grazie."), "");
        assert_eq!(clean_transcript(" Thank you. "), "");
        assert_eq!(
            clean_transcript("Ricordami di comprare il latte"),
            "Ricordami di comprare il latte"
        );
    }
}
