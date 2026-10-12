//! Time used today against daily allowances, per user and block. Resets at
//! local midnight (by the trusted clock) and survives restarts.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use iqal_core::SetId;
use serde::{Deserialize, Serialize};

const FILE: &str = "usage.json";

#[derive(Default, Serialize, Deserialize)]
pub struct Usage {
    day: String,
    per_user: HashMap<String, HashMap<SetId, u32>>,
    #[serde(skip)]
    path: Option<PathBuf>,
    #[serde(skip)]
    dirty: bool,
}

impl Usage {
    pub fn load(state_dir: &Path) -> Self {
        let path = state_dir.join(FILE);
        let mut u: Usage =
            std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        u.path = Some(path);
        u
    }

    /// Starts a new day if the date changed. Returns true if it did.
    pub fn roll(&mut self, today: &str) -> bool {
        if self.day == today {
            return false;
        }
        self.day = today.to_string();
        self.per_user.clear();
        self.dirty = true;
        true
    }

    /// Adds time; returns (before, after) for the set.
    pub fn add(&mut self, today: &str, user: &str, set: SetId, secs: u32) -> (u32, u32) {
        self.roll(today);
        let used = self.per_user.entry(user.to_string()).or_default().entry(set).or_default();
        let before = *used;
        *used = used.saturating_add(secs);
        self.dirty = true;
        (before, *used)
    }

    pub fn all(&self) -> &HashMap<String, HashMap<SetId, u32>> {
        &self.per_user
    }

    pub fn save_if_dirty(&mut self) {
        if !self.dirty {
            return;
        }
        if let (Some(path), Ok(text)) = (&self.path, serde_json::to_string(self)) {
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, path)).is_ok() {
                self.dirty = false;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_and_resets_daily() {
        let mut u = Usage::default();
        let set = SetId::nil();
        assert_eq!(u.add("2026-10-12", "ali", set, 15), (0, 15));
        assert_eq!(u.add("2026-10-12", "ali", set, 15), (15, 30));
        assert_eq!(u.add("2026-10-12", "sara", set, 15), (0, 15), "per user");
        assert_eq!(u.add("2026-10-13", "ali", set, 15), (0, 15), "new day");
        assert!(u.all().get("sara").is_none());
    }
}
