//! iqal-agent: runs in each user's desktop session and shows a
//! notification whenever Iqal blocks something for that user.
//!
//! It enforces nothing itself: blocking happens in the service, so
//! stopping the agent only hides the notifications.

#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(target_os = "linux")]
mod notify;
#[cfg(windows)]
mod windows;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use iqal_ipc::client::Client;
use iqal_ipc::{BlockEvent, EventKind, Request, Response};
use iqal_i18n::{t, ta, FluentValue};

/// The same thing blocked again within this time is not announced again
/// (e.g. a launcher retrying).
const QUIET_FOR: Duration = Duration::from_secs(10);

fn main() {
    iqal_i18n::init(&language());
    #[cfg(target_os = "linux")]
    let _instance = match single_instance() {
        Some(guard) => guard,
        None => return, // another agent already runs in this session
    };
    #[cfg(windows)]
    {
        if windows::single_instance().is_none() {
            return; // another agent already runs in this session
        }
        // The icon needs this thread for its messages; events come in on
        // another, which needs the translations too (they are per thread).
        std::thread::spawn(|| {
            iqal_i18n::init(&language());
            listen_forever();
        });
        windows::run();
    }
    #[cfg(not(windows))]
    listen_forever();
}

fn listen_forever() {
    let mut notifier = Notifier::new();
    let mut backoff = Duration::from_secs(1);
    loop {
        match listen(&mut notifier) {
            Ok(()) => backoff = Duration::from_secs(1),
            Err(e) => eprintln!("iqal-agent: {e}"),
        }
        std::thread::sleep(backoff);
        backoff = (backoff * 2).min(Duration::from_secs(15));
    }
}

fn listen(notifier: &mut Notifier) -> std::io::Result<()> {
    let mut client = Client::connect()?;
    match client.call(&Request::Subscribe)? {
        Response::Subscribed => {}
        other => return Err(std::io::Error::other(format!("unexpected reply {other:?}"))),
    }
    loop {
        if let Response::Event { event } = client.receive()? {
            notifier.show(&event);
        }
    }
}

/// Holds an abstract socket named after the user; a second agent in the
/// same session fails to bind it and exits.
#[cfg(target_os = "linux")]
fn single_instance() -> Option<std::os::unix::net::UnixListener> {
    use std::os::linux::net::SocketAddrExt;
    use std::os::unix::net::{SocketAddr, UnixListener};
    let user = std::env::var("USER").unwrap_or_default();
    let display = std::env::var("DISPLAY").unwrap_or_default();
    let addr = SocketAddr::from_abstract_name(format!("iqal-agent/{user}/{display}")).ok()?;
    UnixListener::bind_addr(&addr).ok()
}

struct Notifier {
    recent: HashMap<(EventKind, String), Instant>,
    #[cfg(target_os = "linux")]
    desktop: Option<notify::Desktop>,
}

impl Notifier {
    fn new() -> Self {
        Self {
            recent: HashMap::new(),
            #[cfg(target_os = "linux")]
            desktop: None,
        }
    }

    fn show(&mut self, event: &BlockEvent) {
        let key = (event.kind, event.target.clone());
        if self.recent.get(&key).is_some_and(|t| t.elapsed() < QUIET_FOR) {
            return;
        }
        self.recent.retain(|_, t| t.elapsed() < QUIET_FOR);
        self.recent.insert(key, Instant::now());

        let Some((summary, body)) = message(event) else { return };
        #[cfg(windows)]
        if windows::notify(&summary, &body) {
            return;
        }
        #[cfg(target_os = "linux")]
        {
            if self.desktop.is_none() {
                self.desktop = notify::Desktop::connect();
            }
            if let Some(d) = &self.desktop
                && d.show(&summary, &body).is_ok()
            {
                return;
            }
            self.desktop = None; // reconnect next time
        }
        eprintln!("iqal-agent: {summary}: {body}");
    }
}

/// Notification text, or `None` for events not worth a pop-up.
fn message(e: &BlockEvent) -> Option<(String, String)> {
    let block = || FluentValue::from(e.set_name.clone());
    let (title, body) = match e.kind {
        EventKind::AppStart => ("notify-app-blocked", ta("notify-app-blocked-body", &[("app", app_name(&e.target).into()), ("block", block())])),
        EventKind::AppKilled => ("notify-app-closed", ta("notify-app-closed-body", &[("app", app_name(&e.target).into()), ("block", block())])),
        EventKind::WindowClosed => (
            "notify-window-closed",
            ta("notify-window-closed-body", &[("title", shorten(&e.target, 80).into()), ("block", block())]),
        ),
        EventKind::AllowanceLow => ("notify-allowance-low", ta("notify-allowance-low-body", &[("item", shorten(&e.target, 60).into()), ("block", block())])),
        EventKind::AllowanceUsed => ("notify-allowance-used", ta("notify-allowance-used-body", &[("item", shorten(&e.target, 60).into()), ("block", block())])),
        // Browsers look up and prefetch many names; the browser extension
        // shows a block page for sites actually opened.
        EventKind::SiteBlocked => return None,
        // The block page in the browser says it all.
        EventKind::PageBlocked => return None,
    };
    Some((t(title), body))
}

/// "/usr/lib/firefox/firefox" → "Firefox"
fn app_name(exe: &str) -> String {
    let file = exe.rsplit(['/', '\\']).next().unwrap_or(exe);
    let file = file.strip_suffix(".exe").unwrap_or(file);
    let mut chars = file.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => exe.to_string(),
    }
}

fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}


/// The language chosen in the Iqal app (~/.config/iqal/gui.json; on
/// Windows %APPDATA%\iqal\gui.json), or the system's.
fn language() -> String {
    #[cfg(windows)]
    let path = std::env::var_os("APPDATA").map(|d| std::path::PathBuf::from(d).join(r"iqal\gui.json"));
    #[cfg(not(windows))]
    let path = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".config")))
        .map(|d| d.join("iqal/gui.json"));
    path.and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
        .and_then(|v| v.get("language")?.as_str().map(String::from))
        .unwrap_or_else(|| iqal_i18n::system_language().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names() {
        assert_eq!(app_name("/usr/games/steam"), "Steam");
        assert_eq!(app_name(r"C:\Games\doom.exe"), "Doom");
        assert_eq!(shorten("abcdef", 4), "abc…");
        assert_eq!(shorten("abc", 4), "abc");
    }
}
