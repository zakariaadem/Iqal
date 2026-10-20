//! The control pipe: one thread per connected client.

use std::fs::File;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use iqal_ipc::{read_frame, write_frame, ErrorCode, Request, Response};
use windows_sys::Win32::Foundation::{ERROR_PIPE_CONNECTED, GetLastError, HANDLE, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::Storage::FileSystem::{FILE_FLAG_FIRST_PIPE_INSTANCE, PIPE_ACCESS_DUPLEX};
use windows_sys::Win32::System::Pipes::{
    ConnectNamedPipe, CreateNamedPipeW, PIPE_READMODE_BYTE, PIPE_REJECT_REMOTE_CLIENTS, PIPE_TYPE_BYTE,
    PIPE_UNLIMITED_INSTANCES, PIPE_WAIT, PeekNamedPipe,
};

use super::system::{self, SecurityDescriptor};
use crate::service::{Caller, ExtensionSession, Service};

const MAX_CLIENTS: usize = 64;
const BUFFER: u32 = 64 * 1024;

/// Who may open the pipe: Windows itself, administrators and the pipe's
/// owner (the service's own account, which needs this to add instances)
/// fully; any signed-in user may read and write, but not create pipe
/// instances of their own (0x12019b leaves out FILE_CREATE_PIPE_INSTANCE),
/// so nobody can pose as the service. What each caller may do is decided
/// per request from their token.
const PIPE_SDDL: &str = "D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;OW)(A;;0x12019b;;;AU)";

pub struct Listener {
    name: Vec<u16>,
    security: SecurityDescriptor,
    /// The first instance, created at bind time so a second service fails
    /// early, and so nobody else can take the name.
    first: Option<File>,
}

pub fn bind(name: &str) -> io::Result<Listener> {
    let security = SecurityDescriptor::from_sddl(PIPE_SDDL)?;
    let mut listener = Listener { name: system::wide(name), security, first: None };
    listener.first = Some(listener.instance(true).map_err(|e| {
        if e.kind() == io::ErrorKind::PermissionDenied {
            io::Error::new(io::ErrorKind::AddrInUse, "another Iqal service is already running")
        } else {
            e
        }
    })?);
    Ok(listener)
}

impl Listener {
    fn instance(&self, first: bool) -> io::Result<File> {
        let sa = SECURITY_ATTRIBUTES {
            nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
            lpSecurityDescriptor: self.security.as_ptr(),
            bInheritHandle: 0,
        };
        let open_mode = PIPE_ACCESS_DUPLEX | if first { FILE_FLAG_FIRST_PIPE_INSTANCE } else { 0 };
        // SAFETY: name is NUL-terminated; sa points to a valid descriptor.
        let handle = unsafe {
            CreateNamedPipeW(
                self.name.as_ptr(),
                open_mode,
                PIPE_TYPE_BYTE | PIPE_READMODE_BYTE | PIPE_WAIT | PIPE_REJECT_REMOTE_CLIENTS,
                PIPE_UNLIMITED_INSTANCES,
                BUFFER,
                BUFFER,
                0,
                &sa,
            )
        };
        if handle == INVALID_HANDLE_VALUE {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: a new handle we own.
        Ok(unsafe { File::from_raw_handle(handle) })
    }

    /// Waits for the next client.
    fn accept(&mut self) -> io::Result<File> {
        let pipe = match self.first.take() {
            Some(p) => p,
            None => self.instance(false)?,
        };
        // SAFETY: a synchronous pipe handle; no OVERLAPPED.
        let ok = unsafe { ConnectNamedPipe(pipe.as_raw_handle() as HANDLE, std::ptr::null_mut()) };
        // A client that connected between creation and this call shows up
        // as ERROR_PIPE_CONNECTED; that is a success.
        if ok == 0 && unsafe { GetLastError() } != ERROR_PIPE_CONNECTED {
            return Err(io::Error::last_os_error());
        }
        Ok(pipe)
    }
}

pub fn serve(mut listener: Listener, service: Arc<Service>) {
    let clients = Arc::new(AtomicUsize::new(0));
    loop {
        let stream = match listener.accept() {
            Ok(s) => s,
            Err(e) => {
                eprintln!("iqal: accept failed: {e}");
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };
        if clients.load(Ordering::Relaxed) >= MAX_CLIENTS {
            continue; // dropping the handle disconnects the client
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

fn handle_client(mut stream: File, service: &Service) -> io::Result<()> {
    // Windows reveals the client's identity only once it has sent something.
    let Some(mut request) = read_frame::<_, Request>(&mut stream)? else { return Ok(()) };
    let peer = system::pipe_peer(&stream)?;
    // Everyone may change blocks on Windows (the owner's choice); locked
    // blocks protect themselves.
    let caller = Caller { user: peer.user, admin: peer.admin, editor: true };
    loop {
        if let Request::Subscribe = request {
            return feed(stream, service, &caller.user);
        }
        if let Request::ExtensionSession { browser } = request {
            return extension_session(stream, service, &caller.user, &browser, peer.pid);
        }
        if let Request::WatchWindows = request {
            if !peer.system {
                write_frame(&mut stream, &Response::error(ErrorCode::PermissionDenied, "only the service's own watchers"))?;
                return Ok(());
            }
            return super::titles::serve_watcher(stream, service);
        }
        let response = service.handle(&caller, request);
        write_frame(&mut stream, &response)?;
        match read_frame::<_, Request>(&mut stream)? {
            Some(next) => request = next,
            None => return Ok(()),
        }
    }
}

/// Sends the user's block events as they happen, until the client leaves.
fn feed(mut stream: File, service: &Service, user: &str) -> io::Result<()> {
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

/// Whether the client is still connected. Pipes have no read timeout, so
/// this peeks instead of reading.
fn connected(stream: &File) -> bool {
    let mut available = 0u32;
    // SAFETY: a valid pipe handle; only the byte count is asked for.
    unsafe {
        PeekNamedPipe(
            stream.as_raw_handle() as HANDLE,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            &mut available,
            std::ptr::null_mut(),
        ) != 0
    }
}

/// Keeps a browser extension registered for as long as its native host is
/// connected, and tells it whenever the rules may have changed.
fn extension_session(mut stream: File, service: &Service, user: &str, reported: &str, pid: u32) -> io::Result<()> {
    // The browser is identified from the process tree, not from what the
    // extension says about itself.
    let browser = super::browsers::host_browser(pid, &super::browsers::parents()).map_or("other", |(family, _)| family);
    eprintln!("iqal: extension connected in {browser} (reports {reported}) for {user}");
    let id = service.add_extension(ExtensionSession { browser: browser.into(), user: user.into(), host_pid: pid as i32 });
    if browser != "other" {
        service.mark_extension_proven(browser);
    }
    let result = (|| {
        write_frame(&mut stream, &Response::Subscribed)?;
        let mut generation = service.generation();
        let mut last = Instant::now();
        loop {
            std::thread::sleep(Duration::from_millis(500));
            if !connected(&stream) {
                return Ok(()); // host exited: the extension is gone
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
