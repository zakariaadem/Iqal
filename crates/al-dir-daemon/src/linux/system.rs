//! Facts about the running Linux system.

use std::ffi::CString;
use std::os::fd::AsFd;
use std::os::unix::net::UnixDatagram;

use nix::sys::socket::{getsockopt, sockopt::PeerCredentials};
use nix::unistd::{getgrouplist, geteuid, Gid, Group, Uid, User};

/// Groups whose members may change Al-Dir settings. `sudo` (Debian,
/// Ubuntu, Mint), `wheel` (Fedora, Arch), `admin` (old Ubuntu) and our own.
const ADMIN_GROUPS: &[&str] = &["sudo", "wheel", "admin", "al-dir"];

/// Identifier of the current boot; changes on every restart.
pub fn boot_id() -> String {
    std::fs::read_to_string("/proc/sys/kernel/random/boot_id")
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "unknown".into())
}

/// The uid of the process on the other end of a Unix socket.
pub fn peer_uid(sock: &impl AsFd) -> std::io::Result<u32> {
    let cred = getsockopt(sock, PeerCredentials).map_err(std::io::Error::from)?;
    Ok(cred.uid())
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
        eprintln!("al-dir: could not notify systemd: {e}");
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
