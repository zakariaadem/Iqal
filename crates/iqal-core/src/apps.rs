//! Application rules.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::text::normalize;

/// One application rule, written and stored as a short string:
///
/// | Text                        | Matches                                             |
/// |-----------------------------|-----------------------------------------------------|
/// | `steam` / `name:steam.exe`  | executable file name (case-insensitive, `.exe` optional) |
/// | `/opt/games/` / `path:...`  | exact executable path, or every program under a folder ending in `/` |
/// | `hash:<sha256>`             | executable contents, so renaming or moving it does not help |
/// | `cmd:minecraft`             | command line contains the text (Java, Python, Electron apps) |
/// | `title:YouTube`             | a window whose title contains the text              |
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AppRule {
    Name(String),
    Path(String),
    Hash(String),
    CmdLine(String),
    WindowTitle(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum AppRuleParseError {
    #[error("empty rule")]
    Empty,
    #[error("\"{0}\" is not a SHA-256 hash (64 hex characters)")]
    InvalidHash(String),
    #[error("unknown rule prefix in \"{0}\"")]
    UnknownPrefix(String),
}

impl AppRule {
    pub fn parse(input: &str) -> Result<Self, AppRuleParseError> {
        let s = input.trim();
        if s.is_empty() {
            return Err(AppRuleParseError::Empty);
        }
        let (prefix, rest) = match s.split_once(':') {
            // "C:\..." is a Windows path, not a prefix.
            Some((p, r)) if p.len() > 1 => (Some(p.to_ascii_lowercase()), r.trim()),
            _ => (None, s),
        };
        if rest.is_empty() {
            return Err(AppRuleParseError::Empty);
        }
        let rule = match prefix.as_deref() {
            Some("name") => AppRule::Name(clean_name(rest)),
            Some("path") => AppRule::Path(clean_path(rest)),
            Some("hash" | "sha256") => {
                let h = rest.to_ascii_lowercase();
                if h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err(AppRuleParseError::InvalidHash(rest.to_string()));
                }
                AppRule::Hash(h)
            }
            Some("cmd" | "cmdline") => AppRule::CmdLine(normalize(rest)),
            Some("title") => AppRule::WindowTitle(normalize(rest)),
            Some(_) => return Err(AppRuleParseError::UnknownPrefix(s.to_string())),
            None if s.contains(['/', '\\']) => AppRule::Path(clean_path(s)),
            None => AppRule::Name(clean_name(s)),
        };
        Ok(rule)
    }
}

fn clean_name(s: &str) -> String {
    strip_exe(&s.to_lowercase()).to_string()
}

fn strip_exe(s: &str) -> &str {
    s.strip_suffix(".exe").unwrap_or(s)
}

fn clean_path(s: &str) -> String {
    s.replace('\\', "/")
}

/// Windows paths (`C:/...`, `//server/...`) compare case-insensitively.
fn path_is_case_insensitive(p: &str) -> bool {
    !p.starts_with('/') || p.starts_with("//")
}

/// Does `exe_path` match the path rule `rule` (already cleaned)?
pub(crate) fn path_matches(rule: &str, exe_path: &str) -> bool {
    let exe = clean_path(exe_path);
    let (rule, exe) = if path_is_case_insensitive(rule) {
        (rule.to_lowercase(), exe.to_lowercase())
    } else {
        (rule.to_string(), exe)
    };
    if rule.ends_with('/') { exe.starts_with(&rule) } else { exe == rule }
}

/// The comparable name of an executable path: lower-case file name
/// without `.exe`.
pub(crate) fn exe_name(exe_path: &str) -> String {
    let file = exe_path.rsplit(['/', '\\']).next().unwrap_or(exe_path);
    clean_name(file)
}

impl fmt::Display for AppRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppRule::Name(n) => write!(f, "name:{n}"),
            AppRule::Path(p) => write!(f, "path:{p}"),
            AppRule::Hash(h) => write!(f, "hash:{h}"),
            AppRule::CmdLine(c) => write!(f, "cmd:{c}"),
            AppRule::WindowTitle(t) => write!(f, "title:{t}"),
        }
    }
}

impl FromStr for AppRule {
    type Err = AppRuleParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        AppRule::parse(s)
    }
}

impl Serialize for AppRule {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for AppRule {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        AppRule::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// What the platform layer knows about a starting process.
#[derive(Debug, Clone, Copy, Default)]
pub struct ProcessInfo<'a> {
    pub exe_path: &'a str,
    pub cmdline: Option<&'a str>,
    /// Lower-case hex SHA-256 of the executable. Only needed when
    /// [`crate::Engine::wants_exe_hash`] is true, as hashing costs I/O.
    pub sha256: Option<&'a str>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> AppRule {
        AppRule::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn parses_rules() {
        assert_eq!(p("Steam.EXE"), AppRule::Name("steam".into()));
        assert_eq!(p("name: Discord"), AppRule::Name("discord".into()));
        assert_eq!(p("/usr/bin/steam"), AppRule::Path("/usr/bin/steam".into()));
        assert_eq!(p(r"C:\Games\x.exe"), AppRule::Path("C:/Games/x.exe".into()));
        assert_eq!(p("cmd:Minecraft"), AppRule::CmdLine("minecraft".into()));
        assert_eq!(p("title:  YouTube  Music"), AppRule::WindowTitle("youtube music".into()));
        let h = "AB".repeat(32);
        assert_eq!(p(&format!("hash:{h}")), AppRule::Hash(h.to_lowercase()));
    }

    #[test]
    fn rejects_bad_rules() {
        assert_eq!(AppRule::parse(""), Err(AppRuleParseError::Empty));
        assert_eq!(AppRule::parse("title:"), Err(AppRuleParseError::Empty));
        assert!(matches!(AppRule::parse("hash:123"), Err(AppRuleParseError::InvalidHash(_))));
        assert!(matches!(AppRule::parse("foo:bar"), Err(AppRuleParseError::UnknownPrefix(_))));
    }

    #[test]
    fn display_round_trips() {
        for s in ["name:steam", "path:/opt/games/", "cmd:minecraft", "title:youtube"] {
            assert_eq!(p(s).to_string(), s);
        }
    }

    #[test]
    fn matches_paths() {
        assert!(path_matches("/opt/games/", "/opt/games/doom/doom"));
        assert!(!path_matches("/opt/games/", "/opt/gamesx/doom"));
        assert!(path_matches("/usr/bin/steam", "/usr/bin/steam"));
        assert!(!path_matches("/usr/bin/steam", "/usr/bin/Steam"));
        assert!(path_matches("C:/Games/", r"c:\games\x.exe"));
    }

    #[test]
    fn exe_names() {
        assert_eq!(exe_name("/usr/bin/Firefox"), "firefox");
        assert_eq!(exe_name(r"C:\Program Files\Steam\steam.exe"), "steam");
    }
}
