//! Knowledge about installed programs, used to keep blocking safe:
//!
//! * **Essential** programs (the desktop session, audio, Al-Dir itself) are
//!   never blocked, whatever the rules say, so a rule cannot log the user
//!   out or lock them out of Al-Dir.
//! * **User apps** are what allow-only mode restricts: programs with a menu
//!   entry, and anything outside the system directories (home, /opt,
//!   AppImages, games). Helper programs in /usr (shells, coreutils, session
//!   helpers) keep working in allow-only mode.

use std::collections::HashSet;
use std::path::Path;

/// File names (without directory) that are never blocked. Entries ending
/// in `*` are prefixes.
const ESSENTIAL: &[&str] = &[
    // Al-Dir
    "al-dir", "al-dir-daemon", "al-dir-cli", "al-dir-agent",
    // Shells used by session scripts
    "sh", "bash", "dash",
    // Display and login
    "xorg", "xwayland", "xinit", "startx", "lightdm", "lightdm-gtk-greeter", "slick-greeter", "gdm", "gdm3", "sddm",
    // Xfce
    "xfce4-session", "startxfce4", "xfwm4", "xfce4-panel", "xfdesktop", "xfsettingsd", "xfconfd",
    "xfce4-notifyd", "xfce4-power-manager", "xfce4-screensaver", "xfce4-screensaver-dialog", "light-locker",
    // Cinnamon
    "cinnamon", "cinnamon-session", "cinnamon-session-binary", "cinnamon-launcher", "cinnamon-screensaver",
    "csd-*", "muffin", "nemo-desktop",
    // MATE
    "mate-session", "mate-panel", "mate-settings-daemon", "marco",
    // GNOME / KDE
    "gnome-session-binary", "gnome-shell", "plasmashell", "kwin_x11", "kwin_wayland", "ksmserver",
    // Session services
    "systemd", "dbus-daemon", "dbus-broker", "dbus-broker-launch", "dbus-launch", "pulseaudio", "pipewire",
    "pipewire-pulse", "wireplumber", "gnome-keyring-daemon", "at-spi*", "ibus*", "fcitx*", "gvfs*",
    "xdg-desktop-portal*", "xdg-document-portal", "xdg-permission-store",
    "polkit-gnome-authentication-agent-1", "polkit-mate-authentication-agent-1",
];

/// Launchers and interpreters that appear in menu entries but are not the
/// app itself; counting them as apps would block every app they run.
const LAUNCHERS: &[&str] =
    &["env", "sh", "bash", "dash", "pkexec", "sudo", "python", "python3", "perl", "ruby", "java", "node", "gjs", "mono"];
const SANDBOX_LAUNCHERS: &[&str] = &["flatpak", "snap", "gtk-launch", "xdg-open", "exo-open"];

const SYSTEM_DIRS: &[&str] = &["/usr/", "/bin/", "/sbin/", "/lib/", "/lib32/", "/lib64/", "/libx32/", "/etc/"];
const APP_DIRS_IN_SYSTEM: &[&str] = &["/usr/games/", "/usr/local/"];

const PATH: &[&str] = &["/usr/local/sbin", "/usr/local/bin", "/usr/sbin", "/usr/bin", "/sbin", "/bin", "/usr/games", "/usr/local/games", "/snap/bin"];

pub struct AppIndex {
    /// Real paths of programs started by menu entries.
    desktop_apps: HashSet<String>,
}

pub fn file_name(exe: &str) -> String {
    exe.rsplit('/').next().unwrap_or(exe).to_lowercase()
}

pub fn is_essential(exe: &str) -> bool {
    let name = file_name(exe);
    ESSENTIAL.iter().any(|e| match e.strip_suffix('*') {
        Some(prefix) => name.starts_with(prefix),
        None => name == *e,
    })
}

impl AppIndex {
    pub fn build() -> Self {
        let mut dirs: Vec<String> = [
            "/usr/share/applications",
            "/usr/local/share/applications",
            "/var/lib/flatpak/exports/share/applications",
            "/var/lib/snapd/desktop/applications",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();
        if let Ok(homes) = std::fs::read_dir("/home") {
            for h in homes.flatten() {
                dirs.push(format!("{}/.local/share/applications", h.path().display()));
            }
        }
        let mut desktop_apps = HashSet::new();
        for dir in dirs {
            let Ok(entries) = std::fs::read_dir(&dir) else { continue };
            for e in entries.flatten() {
                let path = e.path();
                if path.extension().is_some_and(|x| x == "desktop")
                    && let Ok(text) = std::fs::read_to_string(&path)
                    && let Some(exe) = desktop_exec(&text)
                {
                    desktop_apps.extend(resolve(&exe));
                }
            }
        }
        Self { desktop_apps }
    }

    pub fn is_user_app(&self, exe: &str) -> bool {
        self.desktop_apps.contains(exe)
            || APP_DIRS_IN_SYSTEM.iter().any(|d| exe.starts_with(d))
            || !SYSTEM_DIRS.iter().any(|d| exe.starts_with(d))
    }

    pub fn len(&self) -> usize {
        self.desktop_apps.len()
    }
}

/// The program a `.desktop` file runs, skipping launchers and `VAR=value`.
fn desktop_exec(text: &str) -> Option<String> {
    let mut in_entry = false;
    for line in text.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry {
            continue;
        }
        if let Some(cmd) = line.strip_prefix("Exec=") {
            let tokens = cmd
                .split_whitespace()
                .map(|t| t.trim_matches(['"', '\'']))
                .filter(|t| !t.is_empty() && !t.starts_with('-') && !t.contains('='));
            for t in tokens {
                let name = file_name(t);
                if SANDBOX_LAUNCHERS.contains(&name.as_str()) || t.starts_with('%') {
                    // Flatpak/Snap apps live outside the system directories
                    // and are recognised by location instead.
                    return None;
                }
                if !LAUNCHERS.contains(&name.as_str()) {
                    return Some(t.to_string());
                }
            }
            return None;
        }
    }
    None
}

/// Absolute path(s) for a program name: as found, and with symlinks resolved.
fn resolve(exe: &str) -> Vec<String> {
    let found = if exe.starts_with('/') {
        Some(exe.to_string())
    } else {
        PATH.iter().map(|d| format!("{d}/{exe}")).find(|p| Path::new(p).is_file())
    };
    let Some(found) = found else { return Vec::new() };
    let mut out = vec![found.clone()];
    if let Ok(real) = std::fs::canonicalize(&found) {
        let real = real.to_string_lossy().into_owned();
        if real != found {
            out.push(real);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn essential_names() {
        assert!(is_essential("/usr/bin/xfce4-session"));
        assert!(is_essential("/usr/libexec/at-spi-bus-launcher"));
        assert!(is_essential("/usr/local/lib/al-dir/al-dir-daemon"));
        assert!(is_essential("/usr/bin/csd-power"));
        assert!(!is_essential("/usr/games/steam"));
        assert!(!is_essential("/usr/bin/firefox"));
    }

    #[test]
    fn parses_exec_lines() {
        let t = "[Desktop Entry]\nName=Steam\nExec=/usr/games/steam %U\n[Desktop Action x]\nExec=other\n";
        assert_eq!(desktop_exec(t).as_deref(), Some("/usr/games/steam"));
        let t = "[Desktop Entry]\nExec=env GDK_BACKEND=x11 firefox --new-window %u\n";
        assert_eq!(desktop_exec(t).as_deref(), Some("firefox"));
        let t = "[Desktop Entry]\nExec=python3 /usr/share/app/app.py\n";
        assert_eq!(desktop_exec(t).as_deref(), Some("/usr/share/app/app.py"));
        let t = "[Desktop Entry]\nExec=/usr/bin/flatpak run --branch=stable org.gimp.GIMP @@u %U @@\n";
        assert_eq!(desktop_exec(t), None);
        assert_eq!(desktop_exec("[Other]\nExec=x\n"), None);
    }

    #[test]
    fn user_app_locations() {
        let idx = AppIndex { desktop_apps: ["/usr/bin/firefox".to_string()].into() };
        assert!(idx.is_user_app("/usr/bin/firefox"));
        assert!(idx.is_user_app("/home/a/Apps/Game.AppImage"));
        assert!(idx.is_user_app("/opt/zoom/zoom"));
        assert!(idx.is_user_app("/usr/games/sol"));
        assert!(idx.is_user_app("/tmp/.mount_abc/usr/bin/app"));
        assert!(!idx.is_user_app("/usr/bin/ls"));
        assert!(!idx.is_user_app("/usr/libexec/xdg-desktop-portal"));
    }
}
