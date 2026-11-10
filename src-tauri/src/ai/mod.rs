//! On-device AI: model downloads, the llama.cpp server, schema-constrained
//! extraction and the capture queue. See docs/specs/ai-capture.md.

pub mod apply;
pub mod download;
pub mod extract;
pub mod manifest;
pub mod provider;
pub mod related;
pub mod server;
pub mod service;

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::db::DbError;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("network error: {0}")]
    Http(#[from] reqwest::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("download checksum mismatch (expected {expected}, got {actual})")]
    Checksum { expected: String, actual: String },
    #[error("download cancelled")]
    Cancelled,
    #[error("model server: {0}")]
    Server(String),
    #[error("the model returned something unusable: {0}")]
    BadOutput(String),
    #[error("AI is not set up: {0}")]
    NotReady(String),
    #[error(transparent)]
    Db(#[from] DbError),
}

/// On-disk layout under the app data dir.
#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(app_data: &Path) -> Self {
        Self {
            root: app_data.join("ai"),
        }
    }
    pub fn models(&self) -> PathBuf {
        self.root.join("models")
    }
    pub fn model_file(&self, file: &str) -> PathBuf {
        self.models().join(file)
    }
    pub fn server_dir(&self, version: &str) -> PathBuf {
        self.root.join("bin").join(version)
    }
    pub fn downloads(&self) -> PathBuf {
        self.root.join("downloads")
    }
    pub fn server_log(&self) -> PathBuf {
        self.root.join("logs").join("llama-server.log")
    }
}
