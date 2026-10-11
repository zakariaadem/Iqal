//! Window-title blocking on X11.
//!
//! The service itself (as root) connects to each user's X display using
//! that user's X authority cookie, so a user cannot stop title blocking by
//! killing a program of their own. For every top-level window it follows
//! title changes and closes windows whose title matches a rule: first
//! politely (`_NET_CLOSE_WINDOW`), then by disconnecting the program from
//! the display if it does not comply.

use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::os::fd::AsRawFd;
use std::os::unix::fs::MetadataExt;
use std::os::unix::net::UnixStream;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use al_dir_core::{Ctx, Engine, Hit, Now, ProcessInfo};
use al_dir_ipc::EventKind;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{
    Atom, AtomEnum, ChangeWindowAttributesAux, ClientMessageEvent, ConnectionExt as _, EventMask, Window,
};
use x11rb::protocol::Event;
use x11rb::rust_connection::{DefaultStream, RustConnection};

use super::appindex;
use super::procs::{self, Users};
use crate::service::Service;

const DISCOVER_EVERY: Duration = Duration::from_secs(5);
const RECHECK_EVERY: Duration = Duration::from_secs(2);
/// How long a window gets to close itself before it is forced.
const GRACE: Duration = Duration::from_millis(1500);

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// A local X display of a user: (uid, display number, X authority file).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct Session {
    uid: u32,
    display: u32,
    xauthority: String,
}

pub fn start(service: Arc<Service>) -> std::io::Result<()> {
    std::thread::Builder::new().name("al-dir-x11".into()).spawn(move || {
        let users = Arc::new(Users::load());
        let watching: Arc<Mutex<HashSet<(u32, u32)>>> = Arc::default();
        loop {
            for session in discover(&users) {
                let key = (session.uid, session.display);
                if !watching.lock().unwrap_or_else(|e| e.into_inner()).insert(key) {
                    continue;
                }
                let (service, users, done) = (service.clone(), users.clone(), watching.clone());
                let spawned = std::thread::Builder::new().name("al-dir-x11-watch".into()).spawn(move || {
                    match Watcher::connect(&session, &service, &users) {
                        Ok(mut w) => {
                            eprintln!("al-dir: watching window titles on :{} for {}", session.display, w.user);
                            if let Err(e) = w.run() {
                                eprintln!("al-dir: display :{} closed ({e})", session.display);
                            }
                        }
                        Err(e) => eprintln!("al-dir: cannot watch display :{} ({e})", session.display),
                    }
                    done.lock().unwrap_or_else(|e| e.into_inner()).remove(&key);
                });
                if spawned.is_err() {
                    watching.lock().unwrap_or_else(|e| e.into_inner()).remove(&key);
                }
            }
            std::thread::sleep(DISCOVER_EVERY);
        }
    })?;
    Ok(())
}

/// Finds local X displays of enforced users from their processes'
/// environment.
fn discover(users: &Users) -> HashSet<Session> {
    let mut found = HashSet::new();
    for pid in procs::pids() {
        let Some(uid) = procs::uid_of(pid) else { continue };
        if !users.enforced(uid) {
            continue;
        }
        let Ok(env) = std::fs::read(format!("/proc/{pid}/environ")) else { continue };
        let mut display = None;
        let mut xauthority = None;
        for var in env.split(|b| *b == 0) {
            let var = String::from_utf8_lossy(var);
            if let Some(d) = var.strip_prefix("DISPLAY=") {
                display = parse_display(d);
            } else if let Some(x) = var.strip_prefix("XAUTHORITY=") {
                xauthority = Some(x.to_string());
            }
        }
        let Some(display) = display else { continue };
        let xauthority = xauthority.unwrap_or_else(|| format!("{}/.Xauthority", home_of(uid)));
        found.insert(Session { uid, display, xauthority });
    }
    found
}

fn home_of(uid: u32) -> String {
    nix::unistd::User::from_uid(nix::unistd::Uid::from_raw(uid))
        .ok()
        .flatten()
        .map(|u| u.dir.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// ":0", ":1.0", "unix:0" → display number. Remote displays are ignored.
fn parse_display(d: &str) -> Option<u32> {
    let rest = d.strip_prefix(':').or_else(|| d.strip_prefix("unix:"))?;
    rest.split('.').next()?.parse().ok()
}

/// The MIT-MAGIC-COOKIE for a local display from an Xauthority file,
/// which must belong to the session's user.
fn read_cookie(path: &str, uid: u32, display: u32) -> Option<(Vec<u8>, Vec<u8>)> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW)
        .open(path)
        .ok()?;
    let meta = file.metadata().ok()?;
    if !meta.is_file() || meta.uid() != uid || meta.len() > 64 * 1024 {
        return None;
    }
    let mut data = Vec::new();
    file.read_to_end(&mut data).ok()?;
    parse_xauthority(&data, &nix::unistd::gethostname().ok()?.to_string_lossy(), display)
}

fn parse_xauthority(mut data: &[u8], hostname: &str, display: u32) -> Option<(Vec<u8>, Vec<u8>)> {
    const FAMILY_LOCAL: u16 = 256;
    const FAMILY_WILD: u16 = 65535;
    fn take<'a>(data: &mut &'a [u8], n: usize) -> Option<&'a [u8]> {
        let (head, tail) = (data.get(..n)?, data.get(n..)?);
        *data = tail;
        Some(head)
    }
    fn u16_at(data: &mut &[u8]) -> Option<u16> {
        take(data, 2).map(|b| u16::from_be_bytes([b[0], b[1]]))
    }
    fn field<'a>(data: &mut &'a [u8]) -> Option<&'a [u8]> {
        let n = u16_at(data)? as usize;
        take(data, n)
    }
    let display = display.to_string();
    while !data.is_empty() {
        let family = u16_at(&mut data)?;
        let address = field(&mut data)?;
        let number = field(&mut data)?;
        let name = field(&mut data)?;
        let cookie = field(&mut data)?;
        let host_ok = family == FAMILY_WILD || (family == FAMILY_LOCAL && address == hostname.as_bytes());
        let number_ok = number.is_empty() || number == display.as_bytes();
        if host_ok && number_ok && name == b"MIT-MAGIC-COOKIE-1" {
            return Some((name.to_vec(), cookie.to_vec()));
        }
    }
    None
}

struct Atoms {
    client_list: Atom,
    net_wm_name: Atom,
    utf8_string: Atom,
    net_wm_pid: Atom,
    close_window: Atom,
}

struct Win {
    title: String,
    pid: Option<i32>,
    /// When we asked it to close.
    closing: Option<Instant>,
}

struct Watcher<'a> {
    conn: RustConnection,
    root: Window,
    atoms: Atoms,
    windows: HashMap<Window, Win>,
    service: &'a Service,
    users: &'a Users,
    uid: u32,
    user: String,
}

impl<'a> Watcher<'a> {
    fn connect(s: &Session, service: &'a Service, users: &'a Users) -> Result<Self> {
        let (name, cookie) = read_cookie(&s.xauthority, s.uid, s.display).unwrap_or_default();
        let stream = UnixStream::connect(format!("/tmp/.X11-unix/X{}", s.display))?;
        let (stream, _) = DefaultStream::from_unix_stream(stream)?;
        let conn = RustConnection::connect_to_stream_with_auth_info(stream, 0, name, cookie)?;
        let root = conn.setup().roots[0].root;
        let atom = |n: &[u8]| -> Result<Atom> { Ok(conn.intern_atom(false, n)?.reply()?.atom) };
        let atoms = Atoms {
            client_list: atom(b"_NET_CLIENT_LIST")?,
            net_wm_name: atom(b"_NET_WM_NAME")?,
            utf8_string: atom(b"UTF8_STRING")?,
            net_wm_pid: atom(b"_NET_WM_PID")?,
            close_window: atom(b"_NET_CLOSE_WINDOW")?,
        };
        conn.change_window_attributes(root, &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE))?
            .check()?;
        Ok(Self {
            conn,
            root,
            atoms,
            windows: HashMap::new(),
            service,
            users,
            uid: s.uid,
            user: users.name(s.uid),
        })
    }

    fn run(&mut self) -> Result<()> {
        self.sync_client_list()?;
        self.check_all()?;
        let mut generation = self.service.generation();
        let mut last_check = Instant::now();
        loop {
            while let Some(event) = self.conn.poll_for_event()? {
                self.handle(event)?;
            }
            self.conn.flush()?;
            wait_readable(self.conn.stream().as_raw_fd(), 500)?;

            let g = self.service.generation();
            let closing = self.windows.values().any(|w| w.closing.is_some());
            if g != generation || closing || last_check.elapsed() >= RECHECK_EVERY {
                generation = g;
                last_check = Instant::now();
                self.check_all()?;
            }
        }
    }

    fn handle(&mut self, event: Event) -> Result<()> {
        match event {
            Event::PropertyNotify(e) if e.window == self.root && e.atom == self.atoms.client_list => {
                self.sync_client_list()?;
            }
            Event::PropertyNotify(e)
                if e.atom == self.atoms.net_wm_name || e.atom == u32::from(AtomEnum::WM_NAME) =>
            {
                if let Some(title) = self.title(e.window)?
                    && let Some(w) = self.windows.get_mut(&e.window)
                {
                    w.title = title;
                    let engine = self.service.engine();
                    self.check(e.window, &engine, crate::clock::now())?;
                }
            }
            Event::DestroyNotify(e) => {
                self.windows.remove(&e.window);
            }
            _ => {}
        }
        Ok(())
    }

    fn sync_client_list(&mut self) -> Result<()> {
        let reply = self.conn.get_property(false, self.root, self.atoms.client_list, AtomEnum::WINDOW, 0, 4096)?.reply()?;
        let current: HashSet<Window> = reply.value32().map(|v| v.collect()).unwrap_or_default();
        self.windows.retain(|w, _| current.contains(w));
        let engine = self.service.engine();
        let now = crate::clock::now();
        for &w in &current {
            if self.windows.contains_key(&w) {
                continue;
            }
            self.conn
                .change_window_attributes(
                    w,
                    &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE | EventMask::STRUCTURE_NOTIFY),
                )?
                .ignore_error();
            let title = self.title(w)?.unwrap_or_default();
            let pid = self.pid(w)?;
            self.windows.insert(w, Win { title, pid, closing: None });
            self.check(w, &engine, now)?;
        }
        Ok(())
    }

    fn title(&self, w: Window) -> Result<Option<String>> {
        let Ok(cookie) = self.conn.get_property(false, w, self.atoms.net_wm_name, self.atoms.utf8_string, 0, 1024)
        else {
            return Ok(None);
        };
        if let Ok(r) = cookie.reply()
            && !r.value.is_empty()
        {
            return Ok(Some(String::from_utf8_lossy(&r.value).into_owned()));
        }
        let r = self.conn.get_property(false, w, AtomEnum::WM_NAME, AtomEnum::ANY, 0, 1024)?.reply();
        Ok(r.ok().map(|r| legacy_title(&r.value)))
    }

    fn pid(&self, w: Window) -> Result<Option<i32>> {
        let r = self.conn.get_property(false, w, self.atoms.net_wm_pid, AtomEnum::CARDINAL, 0, 1)?.reply();
        Ok(r.ok().and_then(|r| r.value32().and_then(|mut v| v.next())).map(|p| p as i32).filter(|p| *p > 0))
    }

    fn check_all(&mut self) -> Result<()> {
        let engine = self.service.engine();
        let now = crate::clock::now();
        let ids: Vec<Window> = self.windows.keys().copied().collect();
        for w in ids {
            self.check(w, &engine, now)?;
        }
        Ok(())
    }

    fn decide(&self, win: &Win, engine: &Engine, now: Now) -> Option<Hit> {
        if win.title.is_empty() {
            return None;
        }
        let (uid, exe) = match win.pid {
            Some(pid) => (procs::uid_of(pid).unwrap_or(self.uid), procs::exe_of(pid).unwrap_or_default()),
            None => (self.uid, String::new()),
        };
        if !self.users.enforced(uid) || (!exe.is_empty() && appindex::is_essential(&exe)) {
            return None;
        }
        let process = ProcessInfo { exe_path: &exe, ..Default::default() };
        engine.check_window(&win.title, &process, &Ctx::new(now, &self.user)).block
    }

    fn check(&mut self, w: Window, engine: &Engine, now: Now) -> Result<()> {
        let Some(win) = self.windows.get(&w) else { return Ok(()) };
        let Some(hit) = self.decide(win, engine, now) else {
            if let Some(win) = self.windows.get_mut(&w) {
                win.closing = None;
            }
            return Ok(());
        };
        match win.closing {
            None => {
                self.service.record_block(&self.user, EventKind::WindowClosed, &win.title, &hit);
                let msg = ClientMessageEvent::new(32, w, self.atoms.close_window, [0u32, 2, 0, 0, 0]);
                self.conn.send_event(
                    false,
                    self.root,
                    EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                    msg,
                )?;
                if let Some(win) = self.windows.get_mut(&w) {
                    win.closing = Some(Instant::now());
                }
            }
            Some(since) if since.elapsed() >= GRACE => {
                // It did not close (or asked "are you sure?"): disconnect it.
                self.conn.kill_client(w)?.ignore_error();
                self.windows.remove(&w);
            }
            Some(_) => {}
        }
        self.conn.flush()?;
        Ok(())
    }
}

/// Decodes an old-style `WM_NAME`. Programs that predate `_NET_WM_NAME`
/// often send COMPOUND_TEXT, which wraps UTF-8 in `ESC % G` … `ESC % @`;
/// those markers and other control characters are dropped.
fn legacy_title(raw: &[u8]) -> String {
    let text = String::from_utf8_lossy(raw).replace("\u{1b}%G", "").replace("\u{1b}%@", "");
    text.chars().filter(|c| !c.is_control()).collect()
}

fn wait_readable(fd: i32, timeout_ms: i32) -> std::io::Result<()> {
    let mut pfd = libc::pollfd { fd, events: libc::POLLIN, revents: 0 };
    // SAFETY: pfd is a valid pollfd for the duration of the call.
    let rc = unsafe { libc::poll(&mut pfd, 1, timeout_ms) };
    if rc < 0 {
        let e = std::io::Error::last_os_error();
        if e.kind() != std::io::ErrorKind::Interrupted {
            return Err(e);
        }
    }
    if pfd.revents & (libc::POLLHUP | libc::POLLERR) != 0 {
        return Err(std::io::Error::new(std::io::ErrorKind::BrokenPipe, "display connection lost"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displays() {
        assert_eq!(parse_display(":0"), Some(0));
        assert_eq!(parse_display(":1.0"), Some(1));
        assert_eq!(parse_display("unix:2"), Some(2));
        assert_eq!(parse_display("localhost:10.0"), None);
    }

    #[test]
    fn compound_text_titles() {
        assert_eq!(legacy_title("\u{1b}%Gقناة\u{1b}%@ \u{1b}%Gيوتيوب\u{1b}%@".as_bytes()), "قناة يوتيوب");
        assert_eq!(legacy_title(b"Plain title"), "Plain title");
    }

    fn entry(family: u16, addr: &[u8], num: &[u8], name: &[u8], data: &[u8]) -> Vec<u8> {
        let mut v = family.to_be_bytes().to_vec();
        for f in [addr, num, name, data] {
            v.extend_from_slice(&(f.len() as u16).to_be_bytes());
            v.extend_from_slice(f);
        }
        v
    }

    #[test]
    fn xauthority() {
        let mut file = entry(256, b"otherhost", b"0", b"MIT-MAGIC-COOKIE-1", &[1; 16]);
        file.extend(entry(256, b"minto", b"1", b"MIT-MAGIC-COOKIE-1", &[2; 16]));
        file.extend(entry(256, b"minto", b"0", b"MIT-MAGIC-COOKIE-1", &[3; 16]));
        let (name, cookie) = parse_xauthority(&file, "minto", 0).unwrap();
        assert_eq!(name, b"MIT-MAGIC-COOKIE-1");
        assert_eq!(cookie, vec![3; 16]);
        assert_eq!(parse_xauthority(&file, "minto", 1).unwrap().1, vec![2; 16]);
        assert!(parse_xauthority(&file, "minto", 5).is_none());
        let wild = entry(65535, b"", b"", b"MIT-MAGIC-COOKIE-1", &[4; 16]);
        assert_eq!(parse_xauthority(&wild, "x", 7).unwrap().1, vec![4; 16]);
        assert!(parse_xauthority(&file[..10], "minto", 0).is_none());
    }
}
