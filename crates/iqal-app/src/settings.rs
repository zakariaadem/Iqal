//! Per-user app settings (`~/.config/iqal/gui.json`; on Windows
//! `%APPDATA%\iqal\gui.json`).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct GuiSettings {
    /// Language code; `None` follows the system.
    pub language: Option<String>,
}

#[cfg(not(windows))]
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".config")))
}

#[cfg(windows)]
fn config_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(PathBuf::from)
}

fn path() -> Option<PathBuf> {
    Some(config_dir()?.join("iqal/gui.json"))
}

impl GuiSettings {
    pub fn load() -> Self {
        // Before the rename the app was called Al-Dir.
        let legacy = config_dir().map(|d| d.join("al-dir/gui.json"));
        [path(), legacy]
            .into_iter()
            .flatten()
            .find_map(|p| std::fs::read_to_string(p).ok())
            .and_then(|t| serde_json::from_str(&t).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(text) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(p, text);
        }
    }
}
