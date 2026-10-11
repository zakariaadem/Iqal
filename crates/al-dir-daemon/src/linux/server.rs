//! The control socket: one thread per connected client.

use std::io;
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use al_dir_ipc::{read_frame, write_frame, Request};

use super::system;
use crate::service::{Caller, Service};

const MAX_CLIENTS: usize = 64;

pub fn bind(path: &Path) -> io::Result<UnixListener> {
    if UnixStream::connect(path).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AddrInUse, "another Al-Dir service is already running"));
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
                eprintln!("al-dir: accept failed: {e}");
                continue;
            }
        };
        if clients.load(Ordering::Relaxed) >= MAX_CLIENTS {
            continue; // dropping the stream closes it
        }
        clients.fetch_add(1, Ordering::Relaxed);
        let service = service.clone();
        let clients = clients.clone();
        let spawned = std::thread::Builder::new().name("al-dir-client".into()).spawn(move || {
            if let Err(e) = handle_client(stream, &service) {
                eprintln!("al-dir: client error: {e}");
            }
            clients.fetch_sub(1, Ordering::Relaxed);
        });
        if let Err(e) = spawned {
            eprintln!("al-dir: could not start client thread: {e}");
        }
    }
}

fn handle_client(mut stream: UnixStream, service: &Service) -> io::Result<()> {
    let uid = system::peer_uid(&stream)?;
    let caller = Caller { user: system::user_name(uid), admin: system::is_admin(uid) };
    while let Some(request) = read_frame::<_, Request>(&mut stream)? {
        let response = service.handle(&caller, request);
        write_frame(&mut stream, &response)?;
    }
    Ok(())
}
