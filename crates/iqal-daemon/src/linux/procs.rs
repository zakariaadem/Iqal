//! Reading process facts from /proc, and which accounts are enforced.

use std::collections::HashMap;
use std::os::fd::RawFd;
use std::os::unix::fs::MetadataExt;
use std::sync::Mutex;

/// The (effective) uid a process runs as.
pub fn uid_of(pid: i32) -> Option<u32> {
    std::fs::metadata(format!("/proc/{pid}")).ok().map(|m| m.uid())
}

/// Path of the program a process is running. `None` for kernel threads.
pub fn exe_of(pid: i32) -> Option<String> {
    let p = std::fs::read_link(format!("/proc/{pid}/exe")).ok()?;
    Some(strip_deleted(p.to_string_lossy().into_owned()))
}

/// Command line with arguments separated by spaces.
pub fn cmdline_of(pid: i32) -> Option<String> {
    let raw = std::fs::read(format!("/proc/{pid}/cmdline")).ok()?;
    let mut s = String::from_utf8_lossy(&raw).replace('\0', " ");
    s.truncate(s.trim_end().len());
    (!s.is_empty()).then_some(s)
}

/// Path of a file we hold open.
pub fn fd_path(fd: RawFd) -> Option<String> {
    let p = std::fs::read_link(format!("/proc/self/fd/{fd}")).ok()?;
    Some(strip_deleted(p.to_string_lossy().into_owned()))
}

fn strip_deleted(mut s: String) -> String {
    if let Some(stripped) = s.strip_suffix(" (deleted)") {
        s.truncate(stripped.len());
    }
    s
}

/// All process ids currently in /proc.
pub fn pids() -> Vec<i32> {
    let Ok(dir) = std::fs::read_dir("/proc") else { return Vec::new() };
    dir.filter_map(|e| e.ok()?.file_name().to_str()?.parse().ok()).collect()
}

/// Which uids belong to people (and are therefore subject to blocks) and
/// what their names are. System accounts and root are never blocked, so a
/// rule can never break the system itself.
pub struct Users {
    uid_min: u32,
    uid_max: u32,
    names: Mutex<HashMap<u32, String>>,
}

impl Users {
    pub fn load() -> Self {
        let defs = std::fs::read_to_string("/etc/login.defs").unwrap_or_default();
        let get = |key: &str, default: u32| {
            defs.lines()
                .filter_map(|l| l.trim().strip_prefix(key))
                .filter_map(|rest| rest.trim().parse().ok())
                .next_back()
                .unwrap_or(default)
        };
        Self { uid_min: get("UID_MIN", 1000), uid_max: get("UID_MAX", 60000), names: Mutex::new(HashMap::new()) }
    }

    pub fn enforced(&self, uid: u32) -> bool {
        (self.uid_min..=self.uid_max).contains(&uid)
    }

    pub fn uid_min(&self) -> u32 {
        self.uid_min
    }

    /// Enforced accounts listed in /etc/passwd.
    pub fn enforced_uids(&self) -> Vec<u32> {
        let passwd = std::fs::read_to_string("/etc/passwd").unwrap_or_default();
        let mut uids: Vec<u32> = passwd
            .lines()
            .filter_map(|l| l.split(':').nth(2)?.parse().ok())
            .filter(|uid| self.enforced(*uid))
            .collect();
        uids.sort_unstable();
        uids.dedup();
        uids
    }

    pub fn name(&self, uid: u32) -> String {
        let mut names = self.names.lock().unwrap_or_else(|e| e.into_inner());
        names.entry(uid).or_insert_with(|| super::system::user_name(uid)).clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_own_process() {
        let pid = std::process::id() as i32;
        assert!(exe_of(pid).unwrap().contains("iqal_daemon"));
        assert!(cmdline_of(pid).is_some());
        assert_eq!(uid_of(pid), Some(nix::unistd::geteuid().as_raw()));
        assert!(pids().contains(&pid));
    }

    #[test]
    fn deleted_suffix() {
        assert_eq!(strip_deleted("/usr/bin/x (deleted)".into()), "/usr/bin/x");
        assert_eq!(strip_deleted("/usr/bin/x".into()), "/usr/bin/x");
    }
}
