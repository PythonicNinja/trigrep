use std::path::PathBuf;

#[derive(thiserror::Error, Debug)]
pub enum IndexError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Index not found at {path}")]
    NotFound { path: PathBuf },

    #[error("Corrupt index: {details}")]
    Corrupt { details: String },

    #[error("Meta version {found} not supported (expected {expected})")]
    VersionMismatch { found: u32, expected: u32 },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}
