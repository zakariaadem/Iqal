//! Watches every program start through the kernel's process connector, to
//! apply rules that can only be checked once a program is running (its
//! command line, e.g. `java -jar Minecraft.jar`) and to catch programs on
//! file systems the exec hook could not cover.

use std::io;
use std::mem::size_of;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
use std::sync::Arc;


use super::guard::AppGuard;
use crate::service::Service;

const NETLINK_CONNECTOR: libc::c_int = 11;
const CN_IDX_PROC: u32 = 1;
const CN_VAL_PROC: u32 = 1;
const PROC_CN_MCAST_LISTEN: u32 = 1;
const PROC_EVENT_EXEC: u32 = 0x0000_0002;
const NLMSG_HDR: usize = 16;
const CN_MSG: usize = 20;

pub fn start(service: Arc<Service>, guard: Arc<AppGuard>) -> io::Result<()> {
    let sock = subscribe()?;
    std::thread::Builder::new().name("al-dir-procs".into()).spawn(move || {
        let mut buf = [0u8; 4096];
        loop {
            // SAFETY: buf is valid for writes of its length.
            let n = unsafe { libc::recv(sock.as_raw_fd(), buf.as_mut_ptr().cast(), buf.len(), 0) };
            if n < 0 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::Interrupted {
                    continue;
                }
                // ENOBUFS: we fell behind; the sweep will catch up.
                if err.raw_os_error() != Some(libc::ENOBUFS) {
                    eprintln!("al-dir: process events: {err}");
                    std::thread::sleep(std::time::Duration::from_millis(100));
                }
                continue;
            }
            if let Some(pid) = exec_pid(&buf[..n as usize]) {
                guard.enforce_running(pid, &service.engine(), crate::clock::now());
            }
        }
    })?;
    Ok(())
}

fn subscribe() -> io::Result<OwnedFd> {
    // SAFETY: plain socket(2) call; the result is checked.
    let fd = unsafe { libc::socket(libc::AF_NETLINK, libc::SOCK_DGRAM | libc::SOCK_CLOEXEC, NETLINK_CONNECTOR) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    // SAFETY: fd is a fresh, owned socket.
    let sock = unsafe { OwnedFd::from_raw_fd(fd) };

    // SAFETY: sockaddr_nl is plain data; zeroed is a valid value.
    let mut addr: libc::sockaddr_nl = unsafe { std::mem::zeroed() };
    addr.nl_family = libc::AF_NETLINK as u16;
    addr.nl_pid = 0; // let the kernel assign
    addr.nl_groups = CN_IDX_PROC;
    // SAFETY: addr is a valid sockaddr_nl of the given size.
    let rc = unsafe {
        libc::bind(sock.as_raw_fd(), (&addr as *const libc::sockaddr_nl).cast(), size_of::<libc::sockaddr_nl>() as u32)
    };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }

    // nlmsghdr + cn_msg + op
    let total = NLMSG_HDR + CN_MSG + 4;
    let mut msg = Vec::with_capacity(total);
    msg.extend_from_slice(&(total as u32).to_ne_bytes()); // nlmsg_len
    msg.extend_from_slice(&(libc::NLMSG_DONE as u16).to_ne_bytes()); // nlmsg_type
    msg.extend_from_slice(&0u16.to_ne_bytes()); // nlmsg_flags
    msg.extend_from_slice(&0u32.to_ne_bytes()); // nlmsg_seq
    msg.extend_from_slice(&0u32.to_ne_bytes()); // nlmsg_pid
    msg.extend_from_slice(&CN_IDX_PROC.to_ne_bytes()); // cb_id.idx
    msg.extend_from_slice(&CN_VAL_PROC.to_ne_bytes()); // cb_id.val
    msg.extend_from_slice(&0u32.to_ne_bytes()); // seq
    msg.extend_from_slice(&0u32.to_ne_bytes()); // ack
    msg.extend_from_slice(&4u16.to_ne_bytes()); // len
    msg.extend_from_slice(&0u16.to_ne_bytes()); // flags
    msg.extend_from_slice(&PROC_CN_MCAST_LISTEN.to_ne_bytes());
    // SAFETY: msg is a valid buffer of its length.
    let rc = unsafe { libc::send(sock.as_raw_fd(), msg.as_ptr().cast(), msg.len(), 0) };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(sock)
}

/// The process id from a PROC_EVENT_EXEC message, if that is what it is.
fn exec_pid(msg: &[u8]) -> Option<i32> {
    let u32_at = |off: usize| msg.get(off..off + 4).map(|b| u32::from_ne_bytes(b.try_into().unwrap()));
    let ev = NLMSG_HDR + CN_MSG; // struct proc_event
    if u32_at(ev)? != PROC_EVENT_EXEC {
        return None;
    }
    // what(4) cpu(4) timestamp(8), then exec_proc_event { pid, tgid }
    let tgid = u32_at(ev + 16 + 4)? as i32;
    (tgid > 0).then_some(tgid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exec_event() {
        let mut msg = vec![0u8; NLMSG_HDR + CN_MSG];
        msg.extend_from_slice(&PROC_EVENT_EXEC.to_ne_bytes());
        msg.extend_from_slice(&0u32.to_ne_bytes()); // cpu
        msg.extend_from_slice(&0u64.to_ne_bytes()); // timestamp
        msg.extend_from_slice(&1234i32.to_ne_bytes()); // pid
        msg.extend_from_slice(&1200i32.to_ne_bytes()); // tgid
        assert_eq!(exec_pid(&msg), Some(1200));
        msg[NLMSG_HDR + CN_MSG] = 1; // PROC_EVENT_FORK
        assert_eq!(exec_pid(&msg), None);
        assert_eq!(exec_pid(&[0; 8]), None);
    }
}
