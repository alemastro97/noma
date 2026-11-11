//! Voice capture: microphone → 16 kHz mono → whisper.cpp → AI capture.
//! See docs/specs/desktop-presence.md.

pub mod audio;
pub mod command;
pub mod commands;
pub mod extras;
pub mod session;
pub mod stt;
pub mod vad;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum VoiceError {
    #[error("no microphone found")]
    NoInput,
    #[error("microphone: {0}")]
    Audio(String),
    #[error("speech model: {0}")]
    Stt(String),
    #[error("{0}")]
    Unavailable(String),
}
