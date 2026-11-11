//! Pinned downloads: the llama.cpp server build and the models, by URL and
//! SHA-256 (`src-tauri/models.json`). Upgrading a model is a manifest change
//! plus an eval run (`cargo run --example eval`).

use std::collections::HashMap;
use std::sync::OnceLock;

use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ServerSpec {
    pub name: String,
    pub version: String,
    pub release: String,
    pub assets: HashMap<String, Asset>,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct ModelSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub file: String,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub ram_gb: f32,
    pub context_size: u32,
    /// Hybrid thinking model: thinking must be turned off per request.
    pub thinking: bool,
    pub recommended: bool,
}

/// A speech-to-text (whisper.cpp ggml) model.
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub file: String,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub ram_gb: f32,
    pub recommended: bool,
}

/// A text-embedding GGUF served by `llama-server --embedding` (semantic search).
#[derive(Deserialize, Serialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct EmbeddingSpec {
    pub id: String,
    pub name: String,
    pub description: String,
    pub file: String,
    pub url: String,
    pub sha256: String,
    pub size_bytes: u64,
    pub ram_gb: f32,
    pub context_size: u32,
    /// Prepended to search queries (the model's retrieval prompt).
    #[serde(default)]
    pub query_prefix: String,
    /// Prepended to indexed card text.
    #[serde(default)]
    pub document_prefix: String,
    pub recommended: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Manifest {
    pub server: ServerSpec,
    pub models: Vec<ModelSpec>,
    pub speech: Vec<SpeechSpec>,
    #[serde(default)]
    pub embedding: Vec<EmbeddingSpec>,
}

/// The embedding model used for the semantic index.
pub fn embedding_model() -> Option<&'static EmbeddingSpec> {
    let m = manifest();
    m.embedding
        .iter()
        .find(|x| x.recommended)
        .or(m.embedding.first())
}

pub fn manifest() -> &'static Manifest {
    static M: OnceLock<Manifest> = OnceLock::new();
    M.get_or_init(|| {
        serde_json::from_str(include_str!("../../models.json")).expect("models.json is valid")
    })
}

/// The Rust target triple this binary was built for, as used in the manifest.
pub fn current_target() -> &'static str {
    if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
        "aarch64-apple-darwin"
    } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
        "x86_64-apple-darwin"
    } else if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
        "x86_64-pc-windows-msvc"
    } else {
        "x86_64-unknown-linux-gnu"
    }
}

pub fn server_asset() -> Option<&'static Asset> {
    manifest().server.assets.get(current_target())
}

pub fn model(id: &str) -> Option<&'static ModelSpec> {
    manifest().models.iter().find(|m| m.id == id)
}

pub fn speech_model(id: &str) -> Option<&'static SpeechSpec> {
    manifest().speech.iter().find(|m| m.id == id)
}

pub fn default_speech_model() -> &'static SpeechSpec {
    let m = manifest();
    m.speech
        .iter()
        .find(|x| x.recommended)
        .unwrap_or(&m.speech[0])
}

pub fn default_model() -> &'static ModelSpec {
    let m = manifest();
    m.models
        .iter()
        .find(|x| x.recommended)
        .unwrap_or(&m.models[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manifest_parses_and_has_a_recommended_model() {
        let m = manifest();
        assert!(!m.models.is_empty());
        assert!(default_model().recommended);
        for model in &m.models {
            assert_eq!(model.sha256.len(), 64, "{} sha256", model.id);
            assert!(model.url.ends_with(&model.file));
        }
        for speech in &m.speech {
            assert_eq!(speech.sha256.len(), 64, "{} sha256", speech.id);
        }
        assert!(default_speech_model().recommended);
        for asset in m.server.assets.values() {
            assert_eq!(asset.sha256.len(), 64);
        }
        assert!(server_asset().is_some(), "server asset for this target");
    }
}
