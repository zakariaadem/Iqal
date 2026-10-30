//! Facts about the running Linux system.

use std::ffi::CString;
use std::os::fd::AsFd;
use std::os::unix::net::UnixDatagram;

use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::unistd::{getgrouplist, geteuid, Gid, Group, Uid, User};

/// Groups whose members may change Iqal settings. `sudo` (Debian,
/// Ubuntu, Mint), `wheel` (Fedora, Arch), `admin` (old Ubuntu) and our own.
const ADMIN_GROUPS: &[&str] = &["sudo", "wheel", "admin", "iqal"];

/// Identifier of the current boot; changes on every restart.
pub fn boot_id() -> String {
    std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".into())
}

/// The uid and pid of the process on the other end of a Unix socket.
pub fn peer(sock: &impl AsFd) -> std::io::Result<(u32, i32)> {
    let cred = getsockopt(sock, PeerCredentials).map_err(std::io::Error::from)?;
    Ok((cred.uid(), cred.pid()))
}

pub fn user_name(uid: u32) -> String {
    match User::from_uid(Uid::from_raw(uid)) {
        Ok(Some(u)) => u.name,
        _ => uid.to_string(),
    }
}

/// Root, the account the service itself runs as (development), and
/// members of the admin groups.
pub fn is_admin(uid: u32) -> bool {
    if uid == 0 || uid == geteuid().as_raw() {
        return true;
    }
    let Ok(Some(user)) = User::from_uid(Uid::from_raw(uid)) else { return false };
    let Ok(name) = CString::new(user.name) else { return false };
    let Ok(groups) = getgrouplist(&name, user.gid) else { return false };
    ADMIN_GROUPS.iter().any(|g| {
        matches!(Group::from_name(g), Ok(Some(group)) if groups.contains(&Gid::from_raw(group.gid.as_raw())))
    })
}

/// Tells systemd about our state (`READY=1`, `WATCHDOG=1`, ...). Does
/// nothing when not started by systemd.
pub fn sd_notify(state: &str) {
    let Some(path) = std::env::var_os("NOTIFY_SOCKET") else { return };
    let Ok(sock) = UnixDatagram::unbound() else { return };
    let path = path.to_string_lossy();
    let result = if let Some(name) = path.strip_prefix('@') {
        use std::os::linux::net::SocketAddrExt;
        std::os::unix::net::SocketAddr::from_abstract_name(name.as_bytes())
            .and_then(|addr| sock.send_to_addr(state.as_bytes(), &addr))
    } else {
        sock.send_to(state.as_bytes(), &*path)
    };
    if let Err(e) = result {
        eprintln!("iqal: could not notify systemd: {e}");
    }
}

/// The watchdog interval systemd expects pings at, if enabled.
pub fn watchdog_interval() -> Option<std::time::Duration> {
    let usec: u64 = std::env::var("WATCHDOG_USEC").ok()?.parse().ok()?;
    if let Ok(pid) = std::env::var("WATCHDOG_PID")
        && pid.parse::<u32>().ok() != Some(std::process::id())
    {
        return None;
    }
    Some(std::time::Duration::from_micros(usec / 2))
}

/// Sets or clears a file's "cannot be changed or deleted" mark (the
/// immutable attribute, `chattr +i`). Only root may, and even root must
/// clear it before deleting, renaming or changing the file.
pub fn set_immutable(path: &std::path::Path, on: bool) -> std::io::Result<()> {
    use std::os::unix::fs::OpenOptionsExt;
    const FS_IMMUTABLE_FL: libc::c_int = 0x10;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NONBLOCK | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(path)?;
    let fd = std::os::fd::AsRawFd::as_raw_fd(&file);
    let mut flags: libc::c_int = 0;
    // SAFETY: fd is open; the kernel reads and writes one int.
    if unsafe { libc::ioctl(fd, libc::FS_IOC_GETFLAGS, &mut flags) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let wanted = if on { flags | FS_IMMUTABLE_FL } else { flags & !FS_IMMUTABLE_FL };
    // SAFETY: as above.
    if wanted != flags && unsafe { libc::ioctl(fd, libc::FS_IOC_SETFLAGS, &wanted) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
