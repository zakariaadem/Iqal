//! Importing rules from text files, and exporting/importing block sets.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::lock::{is_locked, validate_set, weakening_reason, LockContext};
use crate::model::{BlockSet, Config, SetId, CONFIG_VERSION};
use crate::web::{RuleParseError, WebRule};
use crate::CoreError;

/// Result of reading a text file of website rules.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TextImport {
    pub rules: Vec<WebRule>,
    /// 1-based line number and the problem, for lines that were skipped.
    pub errors: Vec<(usize, RuleParseError)>,
}

/// Reads website rules, one per line. Understands plain rules
/// (`example.com`, `contains:x`, ...), hosts files (`0.0.0.0 example.com`)
/// and simple Adblock lines (`||example.com^`). Blank lines and comments
/// (`#`, `!`, `//`) are skipped; duplicates are removed.
pub fn parse_rules_text(text: &str) -> TextImport {
    let mut out = TextImport::default();
    let mut seen = std::collections::HashSet::new();
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    for (idx, raw) in text.lines().enumerate() {
        let line = strip_comment(raw).trim();
        if line.is_empty() || line.starts_with('[') {
            continue; // "[Adblock Plus 2.0]" headers
        }
        for candidate in hosts_line(line).unwrap_or_else(|| vec![line]) {
            match WebRule::parse(candidate) {
                Ok(rule) => {
                    if seen.insert(rule.clone()) {
                        out.rules.push(rule);
                    }
                }
                Err(e) => out.errors.push((idx + 1, e)),
            }
        }
    }
    out
}

fn strip_comment(line: &str) -> &str {
    let t = line.trim_start();
    if t.starts_with('!') || t.starts_with("//") {
        return "";
    }
    match line.find('#') {
        // "#" starts a comment unless it is part of a URL fragment.
        Some(i) if i == 0 || line[..i].ends_with(char::is_whitespace) => &line[..i],
        _ => line,
    }
}

/// For hosts-file lines, the host names after the IP address.
fn hosts_line(line: &str) -> Option<Vec<&str>> {
    let mut parts = line.split_whitespace();
    let first = parts.next()?;
    first.parse::<std::net::IpAddr>().ok()?;
    const SKIP: &[&str] = &["localhost", "localhost.localdomain", "local", "broadcasthost", "ip6-localhost", "ip6-loopback", "0.0.0.0"];
    Some(parts.filter(|h| !SKIP.contains(h)).collect())
}

// ---------------------------------------------------------------------------
// Export files
// ---------------------------------------------------------------------------

pub const EXPORT_FORMAT: &str = "iqal-export";

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExportFile {
    pub format: String,
    pub version: u32,
    pub exported_at: i64,
    pub block_sets: Vec<BlockSet>,
}

/// Serialises the chosen sets (all when `ids` is `None`). Pending unlocks
/// are machine-local and are not exported.
pub fn export(config: &Config, ids: Option<&[SetId]>, now_unix: i64) -> String {
    let block_sets = config
        .block_sets
        .iter()
        .filter(|s| ids.is_none_or(|ids| ids.contains(&s.id)))
        .cloned()
        .map(|mut s| {
            s.pending_unlock_at = None;
            s
        })
        .collect();
    let file = ExportFile { format: EXPORT_FORMAT.into(), version: CONFIG_VERSION, exported_at: now_unix, block_sets };
    serde_json::to_string_pretty(&file).expect("export is always serialisable")
}

pub fn parse_export(json: &str) -> Result<ExportFile, CoreError> {
    let json = json.strip_prefix('\u{feff}').unwrap_or(json);
    let value: serde_json::Value = serde_json::from_str(json)?;
    if value.get("format").and_then(|f| f.as_str()) != Some(EXPORT_FORMAT) {
        return Err(CoreError::NotAnExport);
    }
    let file: ExportFile = serde_json::from_value(value)?;
    if file.version > CONFIG_VERSION {
        return Err(CoreError::UnsupportedVersion(file.version));
    }
    Ok(file)
}

#[derive(Debug, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub added: Vec<String>,
    pub replaced: Vec<String>,
    /// Set name and why it was not imported.
    pub rejected: Vec<(String, String)>,
}

/// Merges imported sets into the config.
///
/// * A set whose id is new is added.
/// * A set with the id of an unlocked set replaces it.
/// * A set with the id of a locked set replaces it only if it is at least
///   as strict; otherwise it is added as a separate copy, so an import can
///   never weaken a locked block.
pub fn apply_import(config: &mut Config, file: ExportFile, ctx: &LockContext) -> ImportReport {
    let mut report = ImportReport::default();
    for mut set in file.block_sets {
        set.pending_unlock_at = None;
        if let Err(e) = validate_set(&set) {
            report.rejected.push((set.name.clone(), e.to_string()));
            continue;
        }
        match config.block_sets.iter_mut().find(|s| s.id == set.id) {
            None => {
                report.added.push(set.name.clone());
                config.block_sets.push(set);
            }
            Some(existing) if !is_locked(existing, ctx) || weakening_reason(existing, &set).is_none() => {
                report.replaced.push(set.name.clone());
                *existing = set;
            }
            Some(_) => {
                set.id = Uuid::new_v4();
                report.added.push(set.name.clone());
                config.block_sets.push(set);
            }
        }
    }
    report
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geo::LocationFix;
    use crate::model::{Lock, LockKind};
    use crate::schedule::{Now, Schedule};

    #[test]
    fn reads_mixed_text() {
        let text = "\u{feff}# My list\n\
                    facebook.com\n\
                    www.facebook.com   # duplicate after normalising\n\
                    \n\
                    0.0.0.0 ads.example.com tracker.example.com\n\
                    127.0.0.1 localhost\n\
                    ||doubleclick.net^\n\
                    ! adblock comment\n\
                    [Adblock Plus 2.0]\n\
                    contains:casino\n\
                    youtube.com/shorts#frag\n\
                    not a domain\n";
        let r = parse_rules_text(text);
        let got: Vec<String> = r.rules.iter().map(|r| r.to_string()).collect();
        assert_eq!(
            got,
            ["facebook.com", "ads.example.com", "tracker.example.com", "doubleclick.net", "contains:casino", "youtube.com/shorts"]
        );
        assert_eq!(r.errors.len(), 1);
        assert_eq!(r.errors[0].0, 12);
    }

    fn ctx() -> LockContext<'static> {
        LockContext {
            now: Now { unix: 0, local: jiff::civil::date(2026, 10, 12).at(10, 0, 0, 0) },
            boot_id: "b",
            location: LocationFix::Unknown,
            inside: &[],
            outside: &[],
        }
    }

    #[test]
    fn export_round_trip() {
        let mut config = Config::default();
        let mut a = BlockSet::new("A");
        a.web.block = vec![WebRule::parse("a.com").unwrap()];
        a.pending_unlock_at = Some(5);
        a.schedule = Some(Schedule::default());
        config.block_sets.push(a.clone());
        config.block_sets.push(BlockSet::new("B"));

        let json = export(&config, Some(&[a.id]), 42);
        let file = parse_export(&json).unwrap();
        assert_eq!(file.exported_at, 42);
        assert_eq!(file.block_sets.len(), 1);
        assert_eq!(file.block_sets[0].pending_unlock_at, None);
        assert_eq!(file.block_sets[0].web, a.web);

        assert!(matches!(parse_export("{\"format\":\"other\"}"), Err(CoreError::NotAnExport)));
        assert!(matches!(parse_export("nope"), Err(CoreError::Json(_))));
    }

    #[test]
    fn import_never_weakens_locked_sets() {
        let mut locked = BlockSet::new("locked");
        locked.web.block = vec![WebRule::parse("a.com").unwrap()];
        locked.lock = Some(Lock { kind: LockKind::Until { until: i64::MAX }, unlock_delay_secs: 0 });
        let unlocked = BlockSet::new("unlocked");
        let mut config = Config { block_sets: vec![locked.clone(), unlocked.clone()], ..Config::default() };

        let mut weaker = locked.clone();
        weaker.web.block.clear();
        weaker.lock = None;
        let mut renamed = unlocked.clone();
        renamed.name = "renamed".into();
        let fresh = BlockSet::new("fresh");

        let file = ExportFile {
            format: EXPORT_FORMAT.into(),
            version: 1,
            exported_at: 0,
            block_sets: vec![weaker, renamed, fresh],
        };
        let report = apply_import(&mut config, file, &ctx());
        assert_eq!(report.replaced, ["renamed"]);
        assert_eq!(report.added, ["locked", "fresh"]);
        assert_eq!(config.block_sets.len(), 4);
        assert_eq!(config.get(locked.id).unwrap(), &locked, "locked set untouched");
    }
}
