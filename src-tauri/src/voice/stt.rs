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
    ctx: whisper_rs::WhisperContext,
    pub model_id: String,
}

#[cfg(target_os = "macos")]
impl Stt {
    pub fn load(path: &Path, model_id: &str) -> Result<Self, VoiceError> {
        let ctx = whisper_rs::WhisperContext::new_with_params(
            path,
            whisper_rs::WhisperContextParameters::default(),
        )
        .map_err(|e| VoiceError::Stt(e.to_string()))?;
        Ok(Self {
            ctx,
            model_id: model_id.to_string(),
        })
    }

    /// Transcribes 16 kHz mono audio. `language` pins the language (for
    /// partials after the first detection); None auto-detects.
    pub fn transcribe(
        &self,
        audio: &[f32],
        language: Option<&str>,
    ) -> Result<Transcript, VoiceError> {
        use whisper_rs::{FullParams, SamplingStrategy};
        if audio.len() < SAMPLE_RATE as usize / 4 {
            return Ok(Transcript::default());
        }
        let mut state = self
            .ctx
            .create_state()
            .map_err(|e| VoiceError::Stt(e.to_string()))?;
        let mut params = FullParams::new(SamplingStrategy::Greedy { best_of: 1 });
        params.set_language(Some(language.unwrap_or("auto")));
        params.set_n_threads(4);
        params.set_no_timestamps(true);
        params.set_no_context(true);
        params.set_suppress_blank(true);
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
    ) -> Result<Transcript, VoiceError> {
        Err(VoiceError::Unavailable("voice capture unavailable".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
