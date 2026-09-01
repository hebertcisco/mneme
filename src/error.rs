use std::io;
use std::path::PathBuf;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum MnemeError {
    #[error("vault not found: {0}")]
    VaultMissing(PathBuf),
    #[error("invalid note {path}: {message}")]
    InvalidNote { path: PathBuf, message: String },
    #[error("vault is locked by another mneme process")]
    Locked,
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("yaml: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

impl MnemeError {
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}
