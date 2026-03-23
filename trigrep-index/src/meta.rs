use std::path::Path;
use serde::{Deserialize, Serialize};
use crate::error::IndexError;

pub const INDEX_VERSION: u32 = 1;
pub const INDEX_DIR: &str = ".trigrep";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMeta {
    pub version: u32,
    pub created_at: String,
    pub repo_root: String,
    pub num_files: u32,
    pub num_trigrams: u32,
    pub index_size_bytes: u64,
    pub git_head: Option<String>,
}

impl IndexMeta {
    pub fn write(&self, index_dir: &Path) -> Result<(), IndexError> {
        let path = index_dir.join("meta.json");
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn read(index_dir: &Path) -> Result<Self, IndexError> {
        let path = index_dir.join("meta.json");
        if !path.exists() {
            return Err(IndexError::NotFound {
                path: index_dir.to_path_buf(),
            });
        }
        let json = std::fs::read_to_string(path)?;
        let meta: IndexMeta = serde_json::from_str(&json)?;
        if meta.version != INDEX_VERSION {
            return Err(IndexError::VersionMismatch {
                found: meta.version,
                expected: INDEX_VERSION,
            });
        }
        Ok(meta)
    }
}

/// Try to get the current git HEAD commit hash.
pub fn git_head(repo_root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|s| s.trim().to_string())
            } else {
                None
            }
        })
}
