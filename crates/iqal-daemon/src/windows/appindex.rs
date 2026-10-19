//! Knowledge about installed programs, used to keep blocking safe:
//!
//! * **Essential** programs (the desktop, sign-in, Iqal itself) are never
//!   blocked, whatever the rules say, so a rule cannot lock the user out of
//!   Windows or out of Iqal.
//! * **User apps** are what allow-only mode restricts: programs with a
//!   Start menu or desktop shortcut, Store apps, games, and anything
//!   outside Windows' and the program folders. Helpers that come with
//!   drivers and Windows (touchpad, audio, sync) keep working.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// File names (lower case, without `.exe`) that are never blocked.
const ESSENTIAL: &[&str] = &[
    // Iqal
    "iqal", "iqal-daemon", "iqal-cli", "iqal-agent", "iqal-nm-host", "iqal-win32",
    // Sign-in, desktop and shell
    "winlogon", "logonui", "userinit", "explorer", "dwm", "sihost", "ctfmon", "fontdrvhost", "taskhostw",
    "runtimebroker", "startmenuexperiencehost", "shellexperiencehost", "shellhost", "searchhost", "searchapp",
    "textinputhost", "lockapp", "applicationframehost", "dllhost", "conhost", "consent", "credentialuibroker",
    "smartscreen", "securityhealthsystray", "systemsettingsbroker", "useroobebroker", "widgets", "widgetservice",
    "openwith", "pickerhost", "rundll32", "csrss", "smss", "wininit", "services", "lsass", "svchost",
    "audiodg", "wudfhost", "tabtip", "inputapp",
];

/// Shells and script hosts: they have shortcuts but also run scripts,
/// installers and sign-in tasks behind the scenes, so allow-only mode
/// leaves them alone (the terminal window app is still an app).
const SHELLS: &[&str] = &["cmd", "powershell", "pwsh", "wscript", "cscript", "msiexec"];

/// Parts of Windows installed like Store apps (besides the
/// `MicrosoftWindows.*` packages): not apps for allow-only mode.
const WINDOWS_PARTS: &[&str] = &["microsoftstartfeedprovider", "widgetboard", "widgets", "crossdeviceresume"];

pub fn file_name(exe: &str) -> String {
    let file = exe.rsplit(['\\', '/']).next().unwrap_or(exe).to_lowercase();
    file.strip_suffix(".exe").map(str::to_string).unwrap_or(file)
}

pub fn is_essential(exe: &str) -> bool {
    ESSENTIAL.contains(&file_name(exe).as_str())
}

/// Lower case with `\`, for comparing paths.
fn key(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

fn env_dir(var: &str, default: &str) -> String {
    let dir = std::env::var(var).unwrap_or_else(|_| default.to_string());
    let mut k = key(&dir);
    if !k.ends_with('\\') {
        k.push('\\');
    }
    k
}

pub struct AppIndex {
    /// Programs that shortcuts point to (lower-case paths).
    shortcut_apps: HashSet<String>,
    windows_dir: String,
    /// Where installed programs and their helpers live.
    program_dirs: Vec<String>,
}

impl AppIndex {
    pub fn build() -> Self {
        let program_data = std::env::var("ProgramData").unwrap_or_else(|_| r"C:\ProgramData".into());
        let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into());
        let mut roots = vec![
            Path::new(&program_data).join(r"Microsoft\Windows\Start Menu\Programs"),
            PathBuf::from(format!(r"{system_drive}\Users\Public\Desktop")),
        ];
        if let Ok(homes) = std::fs::read_dir(format!(r"{system_drive}\Users")) {
            for home in homes.flatten() {
                let home = home.path();
                roots.push(home.join(r"AppData\Roaming\Microsoft\Windows\Start Menu\Programs"));
                roots.push(home.join("Desktop"));
                roots.push(home.join(r"OneDrive\Desktop"));
            }
        }
        let mut shortcut_apps = HashSet::new();
        for root in roots {
            collect_shortcuts(&root, 0, &mut shortcut_apps);
        }
        Self {
            shortcut_apps,
            windows_dir: env_dir("SystemRoot", r"C:\Windows"),
            program_dirs: vec![
                env_dir("ProgramFiles", r"C:\Program Files"),
                env_dir("ProgramFiles(x86)", r"C:\Program Files (x86)"),
                env_dir("ProgramData", r"C:\ProgramData"),
            ],
        }
    }

    pub fn is_user_app(&self, exe: &str) -> bool {
        let name = file_name(exe);
        if SHELLS.contains(&name.as_str()) || WINDOWS_PARTS.contains(&name.as_str()) {
            return false;
        }
        let exe = key(exe);
        if exe.contains(r"\windowsapps\microsoftwindows.") {
            return false;
        }
        if self.shortcut_apps.contains(&exe) {
            return true;
        }
        if exe.starts_with(&self.windows_dir) {
            return false;
        }
        // Store apps and games.
        if exe.contains(r"\windowsapps\") || exe.contains(r"\steamapps\common\") || exe.contains(r"\epic games\") {
            return true;
        }
        !self.program_dirs.iter().any(|d| exe.starts_with(d))
    }

    pub fn len(&self) -> usize {
        self.shortcut_apps.len()
    }
}

fn collect_shortcuts(dir: &Path, depth: u32, out: &mut HashSet<String>) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let Ok(kind) = entry.file_type() else { continue };
        if kind.is_dir() {
            collect_shortcuts(&path, depth + 1, out);
        } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("lnk"))
            && let Ok(data) = std::fs::read(&path)
            && let Some(target) = lnk_target(&data)
            && target.to_lowercase().ends_with(".exe")
        {
            out.insert(key(&target));
        }
    }
}

/// The local file a shortcut (`.lnk`, MS-SHLLINK) points to. Installer
/// ("advertised") shortcuts carry no path and give `None`.
fn lnk_target(data: &[u8]) -> Option<String> {
    let u32_at = |at: usize| Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?) as usize);
    let u16_at = |at: usize| Some(u16::from_le_bytes(data.get(at..at + 2)?.try_into().ok()?) as usize);
    const HAS_ID_LIST: usize = 0x1;
    const HAS_LINK_INFO: usize = 0x2;
    const LOCAL_PATH: usize = 0x1;
    if u32_at(0)? != 0x4C {
        return None;
    }
    let flags = u32_at(0x14)?;
    let mut at = 0x4C;
    if flags & HAS_ID_LIST != 0 {
        at += 2 + u16_at(at)?;
    }
    if flags & HAS_LINK_INFO == 0 {
        return None;
    }
    let info = at;
    let header_size = u32_at(info + 4)?;
    if u32_at(info + 8)? & LOCAL_PATH == 0 {
        return None;
    }
    let ansi = |offset: usize| -> Option<String> {
        let bytes = data.get(info + offset..)?;
        let end = bytes.iter().position(|&b| b == 0)?;
        Some(String::from_utf8_lossy(&bytes[..end]).into_owned())
    };
    let unicode = |offset: usize| -> Option<String> {
        let bytes = data.get(info + offset..)?;
        let units: Vec<u16> =
            bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).take_while(|&u| u != 0).collect();
        Some(String::from_utf16_lossy(&units))
    };
    let (base, suffix) = if header_size >= 0x24 {
        (unicode(u32_at(info + 0x1C)?)?, unicode(u32_at(info + 0x20)?).unwrap_or_default())
    } else {
        (ansi(u32_at(info + 0x10)?)?, ansi(u32_at(info + 0x18)?).unwrap_or_default())
    };
    Some(base + &suffix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn essential_names() {
        assert!(is_essential(r"C:\Windows\explorer.exe"));
        assert!(is_essential(r"C:\Program Files\Iqal\iqal-daemon.exe"));
        assert!(!is_essential(r"C:\Program Files\Steam\steam.exe"));
    }

    #[test]
    fn user_apps() {
        let index = AppIndex {
            shortcut_apps: [key(r"C:\Program Files\Mozilla Firefox\firefox.exe")].into(),
            windows_dir: r"c:\windows\".into(),
            program_dirs: vec![r"c:\program files\".into(), r"c:\program files (x86)\".into()],
        };
        assert!(index.is_user_app(r"C:\Program Files\Mozilla Firefox\firefox.exe"));
        assert!(index.is_user_app(r"C:\Users\a\Downloads\game.exe"));
        assert!(index.is_user_app(r"D:\SteamLibrary\steamapps\common\Doom\doom.exe"));
        assert!(index.is_user_app(r"C:\Program Files\WindowsApps\Spotify_1.2\Spotify.exe"));
        assert!(!index.is_user_app(r"C:\Program Files\Synaptics\SynTPEnh.exe"));
        assert!(!index.is_user_app(
            r"C:\Program Files\WindowsApps\MicrosoftWindows.Client.WebExperience_526.21100.40.0_x64__cw5n1h2txyewy\WidgetBoard.exe"
        ));
        assert!(!index.is_user_app(r"C:\Windows\System32\notepad.exe"));
        let index = AppIndex { shortcut_apps: [key(r"C:\Windows\System32\cmd.exe")].into(), ..index };
        assert!(!index.is_user_app(r"C:\Windows\System32\cmd.exe"));
    }

    #[test]
    fn reads_shortcuts() {
        // Shortcuts from this machine's Start menu, where there are any.
        let mut found = HashSet::new();
        let pd = std::env::var("ProgramData").unwrap_or_default();
        collect_shortcuts(&Path::new(&pd).join(r"Microsoft\Windows\Start Menu\Programs"), 0, &mut found);
        assert!(found.iter().all(|p| p.ends_with(".exe") && p.contains(":\\")), "{found:?}");
    }
}
