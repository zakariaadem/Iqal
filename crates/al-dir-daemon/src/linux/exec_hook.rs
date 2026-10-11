//! The exec hook: the kernel asks us before any program starts
//! (`FAN_OPEN_EXEC_PERM`) and waits for allow/deny, so a blocked program
//! never runs at all.
//!
//! Safety properties:
//! * Every permission event gets an answer; on any doubt the answer is
//!   "allow". If this process dies, the kernel drops the hook and allows
//!   everything until systemd restarts us.
//! * The decision path only reads an in-memory engine snapshot; it never
//!   waits for disk writes or other clients.

use std::collections::HashSet;
use std::fs::File;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use al_dir_core::Now;
use nix::errno::Errno;
use nix::sys::fanotify::{
    EventFFlags, Fanotify, FanotifyResponse, InitFlags, MarkFlags, MaskFlags, Response,
};

use super::guard::AppGuard;
use crate::service::Service;

/// Pseudo file systems nothing is ever executed from.
const SKIP_FS: &[&str] = &[
    "proc", "sysfs", "cgroup", "cgroup2", "devpts", "mqueue", "debugfs", "tracefs", "securityfs", "pstore", "bpf",
    "configfs", "fusectl", "hugetlbfs", "autofs", "binfmt_misc", "efivarfs", "nsfs", "rpc_pipefs", "devtmpfs",
    "fuse.gvfsd-fuse", "fuse.portal",
];

pub fn start(service: Arc<Service>, guard: Arc<AppGuard>) -> io::Result<()> {
    let fan = Fanotify::init(
        InitFlags::FAN_CLASS_CONTENT
            | InitFlags::FAN_CLOEXEC
            | InitFlags::FAN_UNLIMITED_QUEUE
            | InitFlags::FAN_UNLIMITED_MARKS,
        EventFFlags::O_RDONLY | EventFFlags::O_LARGEFILE | EventFFlags::O_CLOEXEC,
    )
    .map_err(io::Error::from)?;
    let fan = Arc::new(fan);

    let mut marks = Marks::default();
    marks.update(&fan);
    eprintln!("al-dir: exec hook active on {} file systems", marks.count());

    // New mounts (USB sticks, AppImages, ...) get hooked as they appear.
    {
        let fan = fan.clone();
        std::thread::Builder::new().name("al-dir-mounts".into()).spawn(move || {
            loop {
                std::thread::sleep(Duration::from_secs(1));
                marks.update(&fan);
            }
        })?;
    }

    std::thread::Builder::new().name("al-dir-exec".into()).spawn(move || event_loop(&fan, &service, &guard))?;
    Ok(())
}

fn event_loop(fan: &Fanotify, service: &Service, guard: &AppGuard) {
    loop {
        let events = match fan.read_events() {
            Ok(ev) => ev,
            Err(Errno::EINTR | Errno::EAGAIN) => continue,
            Err(e) => {
                eprintln!("al-dir: reading exec events failed: {e}");
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };
        let engine = service.engine();
        let now = Now::system();
        for event in &events {
            let Some(fd) = event.fd() else { continue }; // queue overflow marker
            if !event.mask().contains(MaskFlags::FAN_OPEN_EXEC_PERM) {
                continue;
            }
            let allowed = !event.check_version() || guard.exec_allowed(event.pid(), fd, &engine, now);
            let verdict = if allowed { Response::FAN_ALLOW } else { Response::FAN_DENY };
            if let Err(e) = fan.write_response(FanotifyResponse::new(fd, verdict)) {
                eprintln!("al-dir: answering exec event failed: {e}");
            }
        }
        // `events` drops here, closing the event file descriptors.
    }
}

#[derive(Default)]
struct Marks {
    /// Superblocks hooked as a whole ("major:minor").
    filesystems: HashSet<String>,
    /// Mount points hooked individually (when whole-fs marks fail).
    mounts: HashSet<String>,
    failed: HashSet<String>,
}

impl Marks {
    fn count(&self) -> usize {
        self.filesystems.len() + self.mounts.len()
    }

    fn update(&mut self, fan: &Fanotify) {
        let Ok(info) = std::fs::read_to_string("/proc/self/mountinfo") else { return };
        for m in info.lines().filter_map(parse_mountinfo) {
            if SKIP_FS.contains(&m.fstype.as_str())
                || self.filesystems.contains(&m.dev)
                || self.mounts.contains(&m.point)
                || self.failed.contains(&m.point)
            {
                continue;
            }
            let Ok(dir) = File::open(&m.point) else { continue };
            let mark = |flags: MarkFlags| {
                fan.mark(MarkFlags::FAN_MARK_ADD | flags, MaskFlags::FAN_OPEN_EXEC_PERM, &dir, None::<&std::path::Path>)
            };
            if mark(MarkFlags::FAN_MARK_FILESYSTEM).is_ok() {
                self.filesystems.insert(m.dev);
            } else if mark(MarkFlags::FAN_MARK_MOUNT).is_ok() {
                self.mounts.insert(m.point);
            } else {
                eprintln!("al-dir: cannot hook {} ({})", m.point, m.fstype);
                self.failed.insert(m.point);
            }
        }
    }
}

struct MountInfo {
    dev: String,
    point: String,
    fstype: String,
}

/// `36 35 98:0 /mnt1 /mnt2 rw,noatime master:1 - ext3 /dev/root rw`
fn parse_mountinfo(line: &str) -> Option<MountInfo> {
    let (left, right) = line.split_once(" - ")?;
    let mut l = left.split(' ');
    let dev = l.nth(2)?.to_string();
    let point = unescape(l.nth(1)?);
    let fstype = right.split(' ').next()?.to_string();
    Some(MountInfo { dev, point, fstype })
}

/// Mount points escape space, tab, newline and backslash as `\ooo`.
fn unescape(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'\\'
            && i + 3 < b.len()
            && b[i + 1..i + 4].iter().all(|c| (b'0'..=b'7').contains(c))
        {
            out.push((b[i + 1] - b'0') * 64 + (b[i + 2] - b'0') * 8 + (b[i + 3] - b'0'));
            i += 4;
        } else {
            out.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8_lossy(&out).into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_mountinfo() {
        let m = parse_mountinfo("36 35 98:0 / /media/my\\040stick rw,noatime master:1 - vfat /dev/sdb1 rw").unwrap();
        assert_eq!(m.dev, "98:0");
        assert_eq!(m.point, "/media/my stick");
        assert_eq!(m.fstype, "vfat");
        let m = parse_mountinfo("29 1 8:2 / / rw,relatime shared:1 - ext4 /dev/sda2 rw,errors=remount-ro").unwrap();
        assert_eq!((m.dev.as_str(), m.point.as_str(), m.fstype.as_str()), ("8:2", "/", "ext4"));
        assert!(parse_mountinfo("garbage").is_none());
    }
}
