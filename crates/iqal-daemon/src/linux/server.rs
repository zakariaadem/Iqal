//! The control socket: one thread per connected client.

use std::io::{self, Read};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iqal_ipc::{read_frame, write_frame, Request, Response};

use super::system;
use crate::service::{Caller, ExtensionSession, Service};

const MAX_CLIENTS: usize = 64;

pub fn bind(path: &Path) -> io::Result<UnixListener> {
    if UnixStream::connect(path).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AddrInUse, "another Iqal service is already running"));
    }
    match std::fs::remove_file(path) {
        Err(e) if e.kind() != io::ErrorKind::NotFound => return Err(e),
        _ => {}
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let listener = UnixListener::bind(path)?;
    // Everyone may connect; what they may do is decided per request from
    // their peer credentials.
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o666))?;
    Ok(listener)
}

pub fn serve(listener: UnixListener, service: Arc<Service>) {
    let clients = Arc::new(AtomicUsize::new(0));
    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                eprintln!("iqal: accept failed: {e}");
                continue;
            }
        };
        if clients.load(Ordering::Relaxed) >= MAX_CLIENTS {
            continue; // dropping the stream closes it
        }
        clients.fetch_add(1, Ordering::Relaxed);
        let service = service.clone();
        let clients = clients.clone();
        let spawned = std::thread::Builder::new().name("iqal-client".into()).spawn(move || {
            if let Err(e) = handle_client(stream, &service) {
                eprintln!("iqal: client error: {e}");
            }
            clients.fetch_sub(1, Ordering::Relaxed);
        });
        if let Err(e) = spawned {
            eprintln!("iqal: could not start client thread: {e}");
        }
    }
}

fn handle_client(mut stream: UnixStream, service: &Service) -> io::Result<()> {
    let (uid, pid) = system::peer(&stream)?;
    let admin = system::is_admin(uid);
    let caller = Caller { user: system::user_name(uid), admin, editor: admin };
    while let Some(request) = read_frame::<_, Request>(&mut stream)? {
        if let Request::Subscribe = request {
            return feed(stream, service, &caller.user);
        }
        if let Request::ExtensionSession { browser } = request {
            return extension_session(stream, service, &caller.user, &browser, pid);
        }
        let response = service.handle(&caller, request);
        write_frame(&mut stream, &response)?;
    }
    Ok(())
}

/// Sends the user's block events as they happen, until the client leaves.
fn feed(mut stream: UnixStream, service: &Service, user: &str) -> io::Result<()> {
    let events = service.subscribe(user);
    write_frame(&mut stream, &Response::Subscribed)?;
    while let Ok(event) = events.recv() {
        write_frame(&mut stream, &Response::Event { event: Box::new(event) })?;
    }
    Ok(())
}

/// Even without changes, extensions re-check open pages this often, so
/// schedules take effect.
const RECHECK_EVERY: Duration = Duration::from_secs(15);

/// Keeps a browser extension registered for as long as its native host is
/// connected, and tells it whenever the rules may have changed.
fn extension_session(mut stream: UnixStream, service: &Service, user: &str, reported: &str, pid: i32) -> io::Result<()> {
    // The browser is identified from the process tree, not from what the
    // extension says about itself.
    let browser = super::browsers::host_browser(pid).map_or("other", |(family, _)| family);
    eprintln!("iqal: extension connected in {browser} (reports {reported}) for {user}");
    let id = service.add_extension(ExtensionSession { browser: browser.into(), user: user.into(), host_pid: pid });
    if browser != "other" {
        service.mark_extension_proven(browser);
    }
    let result = (|| {
        write_frame(&mut stream, &Response::Subscribed)?;
        stream.set_read_timeout(Some(Duration::from_millis(500)))?;
        let mut generation = service.generation();
        let mut last = Instant::now();
        let mut byte = [0u8; 1];
        loop {
            match stream.read(&mut byte) {
                Ok(0) => return Ok(()), // host exited: the extension is gone
                Ok(_) => {}
                Err(e) if matches!(e.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {}
                Err(e) => return Err(e),
            }
            let g = service.generation();
            if g != generation || last.elapsed() >= RECHECK_EVERY {
                generation = g;
                last = Instant::now();
                write_frame(&mut stream, &Response::Changed)?;
            }
        }
    })();
    service.remove_extension(id);
    result
}
