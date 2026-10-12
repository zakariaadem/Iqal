//! System settings that protect locks, applied only while they are needed
//! and removed again automatically:
//!
//! * while anything is locked: users cannot change the date, time, time
//!   zone or time synchronisation (polkit);
//! * while a location lock holds: users cannot switch Wi-Fi or networking
//!   off (polkit) — not that it would help, as unknown location keeps the
//!   lock closed;
//! * hardened mode, while anything is locked: administrator password
//!   prompts accept only root (polkit), and sudo is refused for the people
//!   being blocked (sudoers). Emergency access: recovery mode.
//!
//! Root itself is never restricted.

use std::fs;
use std::io;
use std::process::Command;

use super::procs::Users;
use super::system;
use crate::service::LockSummary;

const POLKIT_FILE: &str = "/etc/polkit-1/rules.d/10-iqal.rules";
const SUDOERS_FILE: &str = "/etc/sudoers.d/zz-iqal-hardened";

const TIME_ACTIONS: &[&str] = &[
    "org.freedesktop.timedate1.set-time",
    "org.freedesktop.timedate1.set-timezone",
    "org.freedesktop.timedate1.set-ntp",
    "org.freedesktop.timedate1.set-local-rtc",
];
const NETWORK_ACTIONS: &[&str] = &[
    "org.freedesktop.NetworkManager.enable-disable-wifi",
    "org.freedesktop.NetworkManager.enable-disable-network",
    "org.freedesktop.NetworkManager.enable-disable-wwan",
];


pub fn polkit_rules(summary: &LockSummary) -> Option<String> {
    let mut deny: Vec<&str> = Vec::new();
    if summary.any_locked {
        deny.extend(TIME_ACTIONS);
    }
    if summary.location_locked {
        deny.extend(NETWORK_ACTIONS);
    }
    let hardened = summary.hardened && summary.any_locked;
    if deny.is_empty() && !hardened {
        return None;
    }
    let mut js = String::from("// Managed by Iqal while blocks are locked; removed automatically.\n");
    if !deny.is_empty() {
        let list = deny.iter().map(|a| format!("\"{a}\"")).collect::<Vec<_>>().join(", ");
        js.push_str(&format!(
            "polkit.addRule(function (action, subject) {{\n    if (subject.user !== \"root\" && [{list}].indexOf(action.id) >= 0) {{\n        return polkit.Result.NO;\n    }}\n}});\n"
        ));
    }
    if hardened {
        js.push_str("polkit.addAdminRule(function (action, subject) {\n    return [\"unix-user:0\"];\n});\n");
    }
    Some(js)
}

pub fn sudoers(summary: &LockSummary, users: &Users) -> Option<String> {
    if !(summary.hardened && summary.any_locked) {
        return None;
    }
    let names: Vec<String> =
        users.enforced_uids().into_iter().filter(|uid| system::is_admin(*uid)).map(|uid| users.name(uid)).collect();
    if names.is_empty() {
        return None;
    }
    let mut text = String::from("# Managed by Iqal (hardened mode) while blocks are locked; removed automatically.\n");
    for n in names {
        text.push_str(&format!("{n} ALL=(ALL:ALL) !ALL\n"));
    }
    Some(text)
}

pub fn apply(summary: &LockSummary, users: &Users) {
    // Compared with what is on disk (not with what we wrote last), so files
    // changed or left behind by anyone else are corrected too.
    let polkit = polkit_rules(summary);
    if current(POLKIT_FILE) != polkit {
        match &polkit {
            Some(js) => {
                if let Err(e) = write_root_file(POLKIT_FILE, js, 0o644) {
                    eprintln!("iqal: writing {POLKIT_FILE}: {e}");
                }
            }
            None => {
                let _ = fs::remove_file(POLKIT_FILE);
            }
        }
    }
    let sudo = sudoers(summary, users);
    if current(SUDOERS_FILE) != sudo {
        match &sudo {
            Some(text) => {
                if let Err(e) = install_sudoers(text) {
                    eprintln!("iqal: hardened mode: {e}");
                }
            }
            None => {
                let _ = fs::remove_file(SUDOERS_FILE);
            }
        }
    }
}

fn current(path: &str) -> Option<String> {
    fs::read_to_string(path).ok()
}

/// Removes everything (uninstall).
pub fn remove() {
    let _ = fs::remove_file(POLKIT_FILE);
    let _ = fs::remove_file(SUDOERS_FILE);
}

fn write_root_file(path: &str, text: &str, mode: u32) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    let tmp = format!("{path}.iqal-tmp");
    fs::write(&tmp, text)?;
    fs::set_permissions(&tmp, fs::Permissions::from_mode(mode))?;
    fs::rename(&tmp, path)
}

/// Installs a sudoers fragment only after `visudo` accepts it, so a bad
/// file can never break sudo for everyone.
fn install_sudoers(text: &str) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    // Outside /etc/sudoers.d while being checked: sudo ignores names with '.'.
    let tmp = "/etc/sudoers.d/.iqal-check";
    fs::write(tmp, text)?;
    fs::set_permissions(tmp, fs::Permissions::from_mode(0o440))?;
    let ok = Command::new("visudo").args(["-c", "-q", "-f", tmp]).status().is_ok_and(|s| s.success());
    if !ok {
        let _ = fs::remove_file(tmp);
        return Err(io::Error::other("visudo rejected the generated rules"));
    }
    fs::rename(tmp, SUDOERS_FILE)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_follow_lock_state() {
        let none = LockSummary::default();
        assert!(polkit_rules(&none).is_none());

        let locked = LockSummary { any_locked: true, ..LockSummary::default() };
        let js = polkit_rules(&locked).unwrap();
        assert!(js.contains("org.freedesktop.timedate1.set-time"));
        assert!(!js.contains("NetworkManager"));
        assert!(!js.contains("addAdminRule"));

        let location = LockSummary { any_locked: true, location_locked: true, ..LockSummary::default() };
        assert!(polkit_rules(&location).unwrap().contains("enable-disable-wifi"));

        let hardened = LockSummary { any_locked: true, hardened: true, ..LockSummary::default() };
        assert!(polkit_rules(&hardened).unwrap().contains("addAdminRule"));

        // Hardened but nothing locked: no restrictions at all.
        let idle = LockSummary { hardened: true, ..LockSummary::default() };
        assert!(polkit_rules(&idle).is_none());
    }
}
