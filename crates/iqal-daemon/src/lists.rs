//! Pre-made lists: the curated starter files shipped with Iqal, merged
//! with upstream lists downloaded daily (only for lists some block uses).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use iqal_core::import::parse_rules_text;
use iqal_core::lists::{self, CATALOG};
use iqal_core::WebRule;
use iqal_ipc::ListSummary;

/// Downloads larger than this are refused.
const MAX_DOWNLOAD: u64 = 30 << 20;
/// A download with fewer rules than this is treated as broken.
const MIN_RULES: usize = 10;
pub const UPDATE_EVERY_SECS: i64 = 24 * 3600;

pub type ListRules = Arc<HashMap<String, Vec<WebRule>>>;

pub struct ListStore {
    /// Shipped starter files (`<id>.txt`).
    bundled: Option<PathBuf>,
    /// Downloaded files (`<id>.txt`, `<id>.updated`).
    downloads: PathBuf,
}

impl ListStore {
    pub fn new(state_dir: &Path) -> Self {
        let downloads = state_dir.join("lists");
        let _ = std::fs::create_dir_all(&downloads);
        Self { bundled: find_bundled(), downloads }
    }

    /// All lists, starter rules plus downloaded rules.
    pub fn load(&self) -> (ListRules, Vec<ListSummary>) {
        let mut rules = HashMap::new();
        let mut summaries = Vec::new();
        for def in CATALOG {
            let mut text = String::new();
            if let Some(dir) = &self.bundled {
                text.push_str(&std::fs::read_to_string(dir.join(format!("{}.txt", def.id))).unwrap_or_default());
                text.push('\n');
            }
            text.push_str(&std::fs::read_to_string(self.downloads.join(format!("{}.txt", def.id))).unwrap_or_default());
            let parsed = parse_rules_text(&text).rules;
            summaries.push(ListSummary { id: def.id.into(), rules: parsed.len(), updated_at: self.updated_at(def.id) });
            rules.insert(def.id.to_string(), parsed);
        }
        (Arc::new(rules), summaries)
    }

    fn updated_at(&self, id: &str) -> Option<i64> {
        std::fs::read_to_string(self.downloads.join(format!("{id}.updated"))).ok()?.trim().parse().ok()
    }

    /// Downloads the lists in `ids` that have upstream sources and are older
    /// than a day. Returns whether anything changed.
    pub fn update(&self, ids: &[String]) -> bool {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64);
        let mut changed = false;
        for id in ids {
            let Some(def) = lists::find(id) else { continue };
            if def.sources.is_empty() || self.updated_at(id).is_some_and(|t| now - t < UPDATE_EVERY_SECS) {
                continue;
            }
            let mut text = String::new();
            for url in def.sources {
                match download(url) {
                    Ok(body) => {
                        text.push_str(&body);
                        text.push('\n');
                    }
                    Err(e) => {
                        eprintln!("iqal: list {id}: {e}");
                        text.clear();
                        break;
                    }
                }
            }
            let count = parse_rules_text(&text).rules.len();
            if count < MIN_RULES {
                continue; // keep the previous copy
            }
            let path = self.downloads.join(format!("{id}.txt"));
            let tmp = self.downloads.join(format!(".{id}.tmp"));
            if std::fs::write(&tmp, &text).and_then(|()| std::fs::rename(&tmp, &path)).is_ok() {
                let _ = std::fs::write(self.downloads.join(format!("{id}.updated")), now.to_string());
                eprintln!("iqal: list {id} updated ({count} rules)");
                changed = true;
            }
        }
        changed
    }
}

/// Where the starter files are: next to the installed program
/// (`<prefix>/share/iqal/lists`), or `lists/` in a source checkout.
fn find_bundled() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut candidates = Vec::new();
    // Windows: <Program Files>\Iqal\iqal-daemon.exe -> <Program Files>\Iqal\lists
    #[cfg(windows)]
    if let Some(dir) = exe.parent() {
        candidates.push(dir.join("lists"));
    }
    // <prefix>/lib/iqal/iqal-daemon -> <prefix>/share/iqal/lists
    if let Some(prefix) = exe.parent().and_then(Path::parent).and_then(Path::parent) {
        candidates.push(prefix.join("share/iqal/lists"));
    }
    // <checkout>/target/{debug,release}/iqal-daemon -> <checkout>/lists
    if let Some(root) = exe.parent().and_then(Path::parent).and_then(Path::parent) {
        candidates.push(root.join("lists"));
    }
    candidates.push(PathBuf::from("/usr/share/iqal/lists"));
    candidates.into_iter().find(|p| p.join("social.txt").is_file())
}

fn download(url: &str) -> std::io::Result<String> {
    let out = Command::new("curl")
        .args(["--fail", "--silent", "--show-error", "--location", "--proto", "=https", "--proto-redir", "=https"])
        .args(["--max-time", "120", "--max-filesize", &MAX_DOWNLOAD.to_string(), url])
        .output()?;
    if !out.status.success() {
        return Err(std::io::Error::other(String::from_utf8_lossy(&out.stderr).trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}
