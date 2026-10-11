//! The persisted data model: configuration, block sets and their rules.
//!
//! A [`BlockSet`] is what the user sees as one "block" (or "profile"):
//! a named group of website and app rules with its own users, schedule,
//! allowance and lock.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::apps::AppRule;
use crate::geo::GeoArea;
use crate::lock::PasswordHash;
use crate::schedule::Schedule;
use crate::web::WebRule;

pub type SetId = Uuid;

/// Current on-disk format version of [`Config`].
pub const CONFIG_VERSION: u32 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    #[serde(default)]
    pub block_sets: Vec<BlockSet>,
    /// While any block is locked, administrator rights (sudo, password
    /// prompts) are withdrawn from the people being blocked, so they cannot
    /// simply stop Al-Dir.
    #[serde(default, skip_serializing_if = "is_false")]
    pub hardened: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { version: CONFIG_VERSION, block_sets: Vec::new(), hardened: false }
    }
}

impl Config {
    pub fn from_json(json: &str) -> Result<Self, crate::CoreError> {
        let config: Config = serde_json::from_str(json)?;
        if config.version > CONFIG_VERSION {
            return Err(crate::CoreError::UnsupportedVersion(config.version));
        }
        Ok(config)
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("config is always serialisable")
    }

    pub fn get(&self, id: SetId) -> Option<&BlockSet> {
        self.block_sets.iter().find(|s| s.id == id)
    }

    pub fn get_mut(&mut self, id: SetId) -> Option<&mut BlockSet> {
        self.block_sets.iter_mut().find(|s| s.id == id)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BlockSet {
    pub id: SetId,
    pub name: String,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub users: UserScope,
    #[serde(default)]
    pub web: WebRules,
    #[serde(default)]
    pub apps: AppRules,
    /// When the set is in force. `None` means always.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub schedule: Option<Schedule>,
    /// Daily time budget. While budget remains, matching sites/apps are
    /// allowed (and metered); once it is used up they are blocked.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub allowance: Option<Allowance>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lock: Option<Lock>,
    /// Unix time at which a solved unlock challenge takes effect
    /// (set when the lock has an unlock delay).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pending_unlock_at: Option<i64>,
}

fn default_true() -> bool {
    true
}

impl BlockSet {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            enabled: true,
            users: UserScope::All,
            web: WebRules::default(),
            apps: AppRules::default(),
            schedule: None,
            allowance: None,
            lock: None,
            pending_unlock_at: None,
        }
    }
}

/// Which accounts on the machine a set applies to. Names are matched
/// case-insensitively so exported configs work across systems.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "scope", content = "names", rename_all = "snake_case")]
pub enum UserScope {
    #[default]
    All,
    AllExcept(Vec<String>),
    Only(Vec<String>),
}

impl UserScope {
    pub fn includes(&self, user: &str) -> bool {
        let listed = |names: &[String]| names.iter().any(|n| eq_ignore_case(n, user));
        match self {
            UserScope::All => true,
            UserScope::AllExcept(names) => !listed(names),
            UserScope::Only(names) => listed(names),
        }
    }
}

pub(crate) fn eq_ignore_case(a: &str, b: &str) -> bool {
    a == b || a.to_lowercase() == b.to_lowercase()
}

/// How a rule list is interpreted.
///
/// * `Block`: `block` rules are blocked, `allow` rules are exceptions that
///   win over them.
/// * `AllowOnly`: everything is blocked except `allow` rules; `block` rules
///   further restrict what is allowed (e.g. allow `youtube.com` but block
///   `youtube.com/shorts`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ListMode {
    #[default]
    Block,
    AllowOnly,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct WebRules {
    #[serde(default)]
    pub mode: ListMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub block: Vec<WebRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<WebRule>,
    /// Ids of pre-made lists whose rules are added to `block`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub lists: Vec<String>,
    /// Ids of partial-blocking rules (see [`crate::partial`]).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub partial: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub safe_search: bool,
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct AppRules {
    #[serde(default)]
    pub mode: ListMode,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub block: Vec<AppRule>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub allow: Vec<AppRule>,
}

fn is_false(b: &bool) -> bool {
    !*b
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Allowance {
    pub daily_secs: u32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Lock {
    pub kind: LockKind,
    /// For challenge locks (password, random text): seconds between solving
    /// the challenge and the lock actually being removed.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub unlock_delay_secs: u32,
}

fn is_zero(n: &u32) -> bool {
    *n == 0
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LockKind {
    /// Locked until a point in time (unix seconds).
    Until { until: i64 },
    /// Locked whenever the schedule is active.
    Schedule { schedule: Schedule },
    /// Unlocked by entering a password.
    Password { hash: PasswordHash },
    /// Unlocked by typing a freshly generated random text.
    RandomText { length: u16 },
    /// Locked until the machine restarts.
    UntilRestart { boot_id: String },
    /// Locked while the machine is inside the area (fail-closed).
    Location { area: GeoArea },
}
