//! Browser supervision. While a user has website rules in force:
//!
//! * browsers the extension supports must run with it: a browser that has
//!   been running for a while without a connected extension is closed
//!   (only once the extension has been seen working in that browser on
//!   this machine, so a browser it cannot be installed in is left alone);
//! * browsers it cannot supervise are not allowed to run at all.

use std::collections::{HashMap, HashSet};
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};

use windows_sys::Win32::Foundation::{FILETIME, HANDLE};
use windows_sys::Win32::System::SystemInformation::GetSystemTimeAsFileTime;
use windows_sys::Win32::System::Threading::{GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

use super::appindex::file_name;
use super::procs;

/// Browser family → executable file names (lower case, without `.exe`).
const SUPPORTED: &[(&str, &[&str])] = &[
    ("firefox", &["firefox"]),
    ("chrome", &["chrome"]),
    ("brave", &["brave"]),
    ("edge", &["msedge"]),
    ("vivaldi", &["vivaldi"]),
];

/// Browsers without Iqal support (no extension or no managed policies).
const UNSUPPORTED: &[&str] = &[
    "opera", "opera_gx", "librewolf", "waterfox", "floorp", "zen", "palemoon", "basilisk", "seamonkey", "thorium",
    "iridium", "slimjet", "maxthon", "ucbrowser", "yandex", "mullvadbrowser", "chromium", "ungoogled-chromium",
    "min", "qutebrowser", "falkon", "otter-browser", "k-meleon", "midori", "cent", "catsxp", "srware iron", "iron",
];

/// Seconds a browser may be seen without its extension before it is closed.
pub const EXTENSION_GRACE_SECS: u64 = 30;

pub fn family_of(exe: &str) -> Option<&'static str> {
    // Tor Browser is Firefox inside its own folder, without our policies.
    if exe.to_lowercase().contains(r"\tor browser\") {
        return None;
    }
    let name = file_name(exe);
    SUPPORTED.iter().find(|(_, names)| names.contains(&name.as_str())).map(|(f, _)| *f)
}

pub fn is_unsupported(exe: &str) -> bool {
    UNSUPPORTED.contains(&file_name(exe).as_str()) || exe.to_lowercase().contains(r"\tor browser\")
}

/// Parent of every running process, by process id.
pub fn parents() -> HashMap<u32, u32> {
    procs::list().into_iter().map(|e| (e.pid, e.parent)).collect()
}

/// The browser a native host belongs to: (family, main process id), found
/// by walking up the process tree.
pub fn host_browser(host_pid: u32, parents: &HashMap<u32, u32>) -> Option<(&'static str, u32)> {
    let mut pid = *parents.get(&host_pid)?;
    let mut found = None;
    for _ in 0..64 {
        if pid <= 4 {
            break;
        }
        match procs::exe_of(pid).as_deref().and_then(family_of) {
            Some(family) => found = Some((family, pid)),
            None if found.is_some() => break, // left the browser's process tree
            None => {}
        }
        let Some(&parent) = parents.get(&pid) else { break };
        pid = parent;
    }
    found
}

/// A browser's main process: a supported browser whose parent is not the
/// same browser (child processes share the executable).
pub fn is_main_process(pid: u32, family: &str, parents: &HashMap<u32, u32>) -> bool {
    let Some(&parent) = parents.get(&pid) else { return false };
    procs::exe_of(parent).as_deref().and_then(family_of) != Some(family)
}

/// Main processes that have a connected extension.
pub fn covered(host_pids: impl Iterator<Item = u32>, parents: &HashMap<u32, u32>) -> HashSet<u32> {
    host_pids.filter_map(|pid| host_browser(pid, parents)).map(|(_, main)| main).collect()
}

/// How long the process has been running.
pub fn age_secs(pid: u32) -> Option<u64> {
    // SAFETY: no pointers involved.
    let h = unsafe { OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid) };
    if h.is_null() {
        return None;
    }
    // SAFETY: a new handle we own.
    let process = unsafe { OwnedHandle::from_raw_handle(h) };
    let zero = FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 };
    let (mut created, mut exited, mut kernel, mut user) = (zero, zero, zero, zero);
    // SAFETY: all four are valid out pointers.
    let ok = unsafe {
        GetProcessTimes(process.as_raw_handle() as HANDLE, &mut created, &mut exited, &mut kernel, &mut user)
    };
    if ok == 0 {
        return None;
    }
    let mut now = zero;
    // SAFETY: a valid out pointer.
    unsafe { GetSystemTimeAsFileTime(&mut now) };
    let ticks = |t: FILETIME| (u64::from(t.dwHighDateTime) << 32) | u64::from(t.dwLowDateTime);
    // FILETIME counts 100 ns steps.
    Some(ticks(now).saturating_sub(ticks(created)) / 10_000_000)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn families() {
        assert_eq!(family_of(r"C:\Program Files\Mozilla Firefox\firefox.exe"), Some("firefox"));
        assert_eq!(family_of(r"C:\Program Files\Google\Chrome\Application\chrome.exe"), Some("chrome"));
        assert_eq!(family_of(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"), Some("edge"));
        assert_eq!(family_of(r"C:\Windows\notepad.exe"), None);
        assert_eq!(family_of(r"C:\Users\a\Desktop\Tor Browser\Browser\firefox.exe"), None);
        assert!(is_unsupported(r"C:\Users\a\Desktop\Tor Browser\Browser\firefox.exe"));
        assert!(is_unsupported(r"C:\Users\a\AppData\Local\Programs\Opera\opera.exe"));
        assert!(!is_unsupported(r"C:\Program Files\Mozilla Firefox\firefox.exe"));
    }

    #[test]
    fn own_process_age() {
        assert!(age_secs(std::process::id()).is_some_and(|a| a < 3600));
    }
}
