//! Window-title blocking, service side. The service runs on no desktop, so
//! a watcher on each user's desktop (see `watcher.rs`) reports the windows
//! there; this decides which of them to close.

use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;

use iqal_core::{Ctx, ProcessInfo};
use iqal_ipc::{read_frame, write_frame, CloseWindow, EventKind, Request, Response};

use super::appindex;
use super::procs::{self, Users};
use crate::service::Service;

/// What is known about a window's process: user, program, command line.
type ProcessFacts = Option<(String, String, Option<String>)>;

/// Serves one watcher until it disconnects.
pub fn serve_watcher(mut stream: File, service: &Service) -> io::Result<()> {
    write_frame(&mut stream, &Response::Subscribed)?;
    let users = Users::load();
    let mut processes: HashMap<u32, ProcessFacts> = HashMap::new();
    // Windows already reported as blocked, so each is recorded once.
    let mut recorded: HashSet<(u64, String)> = HashSet::new();
    while let Some(request) = read_frame::<_, Request>(&mut stream)? {
        let Request::Windows { windows } = request else {
            return Err(io::Error::new(io::ErrorKind::InvalidData, "expected window reports"));
        };
        let engine = service.engine();
        let now = crate::clock::now();
        processes.retain(|pid, _| windows.iter().any(|w| w.pid == *pid));
        recorded.retain(|(id, title)| windows.iter().any(|w| w.id == *id && &w.title == title));
        let mut close = Vec::new();
        for window in windows {
            let facts = processes.entry(window.pid).or_insert_with(|| {
                let user = users.of_process(window.pid)?;
                let exe = procs::exe_of(window.pid)?;
                Some((user, exe, procs::cmdline_of(window.pid)))
            });
            let Some((user, exe, cmdline)) = facts else { continue };
            let usage = service.usage_for(user);
            let ctx = Ctx { now, user, usage: usage.as_deref() };
            let process = ProcessInfo { exe_path: exe, cmdline: cmdline.as_deref(), sha256: None };
            let Some(hit) = engine.check_window(&window.title, &process, &ctx).block else { continue };
            if recorded.insert((window.id, window.title.clone())) {
                service.record_block(user, EventKind::WindowClosed, &window.title, &hit);
            }
            close.push(CloseWindow { id: window.id, force: !appindex::is_essential(exe) });
        }
        write_frame(&mut stream, &Response::CloseWindows { windows: close })?;
    }
    Ok(())
}
