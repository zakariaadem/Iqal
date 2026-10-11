//! al-dir-agent: runs in each user's desktop session and shows a
//! notification whenever Al-Dir blocks something for that user.
//!
//! It enforces nothing itself: blocking happens in the service, so
//! stopping the agent only hides the notifications.

#[cfg(target_os = "linux")]
mod notify;

use std::collections::HashMap;
use std::time::{Duration, Instant};

use al_dir_ipc::client::Client;
use al_dir_ipc::{BlockEvent, EventKind, Request, Response};

/// The same thing blocked again within this time is not announced again
/// (e.g. a launcher retrying).
const QUIET_FOR: Duration = Duration::from_secs(10);

fn main() {
    #[cfg(target_os = "linux")]
    let _instance = match single_instance() {
        Some(guard) => guard,
        None => return, // another agent already runs in this session
    };

    let mut notifier = Notifier::new();
    let mut backoff = Duration::from_secs(1);
    loop {
        match listen(&mut notifier) {
            Ok(()) => backoff = Duration::from_secs(1),
            Err(e) => eprintln!("al-dir-agent: {e}"),
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
    let addr = SocketAddr::from_abstract_name(format!("al-dir-agent/{user}/{display}")).ok()?;
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

        let (summary, body) = message(event);
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
        eprintln!("al-dir-agent: {summary}: {body}");
    }
}

/// Notification text. (Translations arrive with the GUI milestone.)
fn message(e: &BlockEvent) -> (String, String) {
    let app = app_name(&e.target);
    match e.kind {
        EventKind::AppStart => ("App blocked".into(), format!("{app} was stopped by “{}”.", e.set_name)),
        EventKind::AppKilled => ("App closed".into(), format!("{app} was closed by “{}”.", e.set_name)),
        EventKind::WindowClosed => {
            ("Window closed".into(), format!("“{}” was closed by “{}”.", shorten(&e.target, 80), e.set_name))
        }
    }
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
