//! Browser supervision. While a user has website rules in force:
//!
//! * browsers the extension supports must run with it: a browser that has
//!   been running for a while without a connected extension is closed
//!   (only once the extension has been seen working in that browser on
//!   this machine, so a failed install cannot lock anyone out);
//! * browsers it cannot supervise are not allowed to run at all.

use std::collections::HashSet;

use super::procs;

/// Browser family → executable file names.
const SUPPORTED: &[(&str, &[&str])] = &[
    ("firefox", &["firefox", "firefox-bin", "firefox-esr"]),
    ("chrome", &["chrome", "google-chrome", "google-chrome-stable", "google-chrome-beta"]),
    ("chromium", &["chromium", "chromium-browser"]),
    ("brave", &["brave", "brave-browser"]),
    ("edge", &["msedge", "microsoft-edge", "microsoft-edge-stable"]),
    ("vivaldi", &["vivaldi-bin", "vivaldi"]),
];

/// Browsers without Iqal support (no extension or no managed policies).
const UNSUPPORTED: &[&str] = &[
    "opera", "epiphany", "epiphany-browser", "midori", "falkon", "qutebrowser", "palemoon", "librewolf",
    "waterfox", "floorp", "zen", "zen-bin", "konqueror", "min", "seamonkey", "dillo", "netsurf", "netsurf-gtk",
    "links", "links2", "lynx", "w3m", "elinks", "firefox.real", "thorium", "yandex-browser", "yandex_browser",
    "ungoogled-chromium", "librewolf-bin", "mullvad-browser", "badwolf", "luakit", "surf", "nyxt", "otter-browser",
];

/// Seconds a browser may run before its extension must have connected.
pub const EXTENSION_GRACE_SECS: u64 = 30;

fn file_name(exe: &str) -> &str {
    exe.rsplit('/').next().unwrap_or(exe)
}

pub fn family_of(exe: &str) -> Option<&'static str> {
    let name = file_name(exe);
    SUPPORTED.iter().find(|(_, names)| names.contains(&name)).map(|(f, _)| *f)
}

pub fn is_unsupported(exe: &str) -> bool {
    UNSUPPORTED.contains(&file_name(exe))
}

/// Parent pid and start time (seconds after boot) from /proc/<pid>/stat.
fn stat(pid: i32) -> Option<(i32, f64)> {
    let text = std::fs::read_to_string(format!("/proc/{pid}/stat")).ok()?;
    let fields: Vec<&str> = text.rsplit_once(')')?.1.split_whitespace().collect();
    let ppid = fields.get(1)?.parse().ok()?;
    let start_ticks: f64 = fields.get(19)?.parse().ok()?;
    // SAFETY: sysconf has no preconditions.
    let hz = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as f64;
    Some((ppid, start_ticks / hz))
}

pub fn parent(pid: i32) -> Option<i32> {
    stat(pid).map(|(p, _)| p)
}

/// How long the process has been running.
pub fn age_secs(pid: i32) -> Option<u64> {
    let (_, started) = stat(pid)?;
    let uptime: f64 = std::fs::read_to_string("/proc/uptime").ok()?.split_whitespace().next()?.parse().ok()?;
    Some((uptime - started).max(0.0) as u64)
}

/// The browser a native host belongs to: (family, main process pid),
/// found by walking up the process tree.
pub fn host_browser(host_pid: i32) -> Option<(&'static str, i32)> {
    let mut pid = parent(host_pid)?;
    let mut found = None;
    for _ in 0..64 {
        if pid <= 1 {
            break;
        }
        match procs::exe_of(pid).as_deref().and_then(family_of) {
            Some(family) => found = Some((family, pid)),
            None if found.is_some() => break, // left the browser's process tree
            None => {}
        }
        pid = parent(pid)?;
    }
    found
}

/// A browser's main process: a supported browser whose parent is not the
/// same browser (child processes share the executable).
pub fn is_main_process(pid: i32, family: &str) -> bool {
    let Some(ppid) = parent(pid) else { return false };
    procs::exe_of(ppid).as_deref().and_then(family_of) != Some(family)
}

/// Main processes that have a connected extension.
pub fn covered(host_pids: impl Iterator<Item = i32>) -> HashSet<i32> {
    host_pids.filter_map(host_browser).map(|(_, main)| main).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families() {
        assert_eq!(family_of("/usr/lib/firefox/firefox"), Some("firefox"));
        assert_eq!(family_of("/opt/google/chrome/chrome"), Some("chrome"));
        assert_eq!(family_of("/opt/brave.com/brave/brave"), Some("brave"));
        assert_eq!(family_of("/usr/bin/xed"), None);
        assert!(is_unsupported("/usr/bin/epiphany-browser"));
        assert!(is_unsupported("/home/a/tor-browser/Browser/firefox.real"));
        assert!(!is_unsupported("/usr/lib/firefox/firefox"));
    }

    #[test]
    fn own_process_stat() {
        let me = std::process::id() as i32;
        assert!(parent(me).is_some_and(|p| p > 0));
        assert!(age_secs(me).is_some_and(|a| a < 3600));
    }
}
