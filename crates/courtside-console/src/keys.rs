//! The keyboard: action keys and keys that type text. They belong to the
//! computer the scout codes on, not to a season (a Mac and a Windows
//! keyboard want different keys), so they live in a small JSON file in
//! the app's config directory. A computer without the file takes the keys
//! of the season open first, which held them before they moved here.

use std::collections::HashSet;
use std::path::Path;

use courtside_store::{KeyBinding, KeyText, Settings};
use serde::{Deserialize, Serialize};

use crate::{Error, Result};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Keys {
    /// Actions the scout moved off their default keys.
    pub keymap: Vec<KeyBinding>,
    /// Keys that type text on the command line (`]` types `#`).
    pub key_text: Vec<KeyText>,
}

impl Keys {
    /// The keys a season held before they moved to the computer.
    pub fn from_season(s: &Settings) -> Keys {
        Keys {
            keymap: s.keymap.clone(),
            key_text: s.key_text.clone(),
        }
    }

    /// What `path` holds; `None` if it is missing or unreadable.
    pub fn load(path: &Path) -> Option<Keys> {
        std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
    }

    /// Checks the keys, then writes them to a temporary file renamed over
    /// `path`, so a crash mid-write leaves the old file.
    pub fn save(&self, path: &Path) -> Result<()> {
        self.check()?;
        let write = |source| Error::Write {
            path: path.display().to_string(),
            source,
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(write)?;
        }
        let tmp = path.with_extension("tmp");
        let json = serde_json::to_vec_pretty(self).map_err(|e| write(std::io::Error::other(e)))?;
        std::fs::write(&tmp, json).map_err(write)?;
        std::fs::rename(&tmp, path).map_err(write)
    }

    /// Keys the scout can't save: a key set twice, or text the command
    /// line can't take.
    pub fn check(&self) -> Result<()> {
        let bad = |m: String| Err(Error::BadCode(m));
        let mut actions = HashSet::new();
        for b in &self.keymap {
            if !b.keys.is_empty() && !actions.insert(&b.keys) {
                return bad(format!("{} is set for more than one action.", b.keys));
            }
        }
        let mut typed = HashSet::new();
        for k in &self.key_text {
            if k.keys.is_empty() || k.text.is_empty() || k.text.chars().count() > 20 {
                return bad("A key that types text needs a key and 1 to 20 characters.".into());
            }
            if k.text.chars().any(char::is_whitespace) {
                return bad(format!("{}: the text can't have spaces.", k.keys));
            }
            if !typed.insert(&k.keys) || actions.contains(&k.keys) {
                return bad(format!("{} is set more than once.", k.keys));
            }
        }
        Ok(())
    }
}
