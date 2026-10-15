//! Where the scout was: the season file and match open last, kept in a
//! small JSON file so the console reopens them after a restart or crash.
//! Every event is already saved, so reopening the match is the recovery.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Resume {
    pub season: Option<PathBuf>,
    pub match_id: Option<String>,
}

impl Resume {
    /// What `path` holds; nothing if it is missing or unreadable.
    pub fn load(path: &Path) -> Resume {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    /// Writes to a temporary file and renames it over `path`, so a crash
    /// mid-write leaves the old file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        let json = serde_json::to_vec_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(&tmp, json)?;
        std::fs::rename(&tmp, path)
    }
}
