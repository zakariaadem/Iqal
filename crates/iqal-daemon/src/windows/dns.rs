//! The DNS filter on Windows. A Name Resolution Policy Table (NRPT) rule
//! for every name (".") sends all lookups of Windows' DNS client to our
//! listener on 127.0.0.1, without touching any network adapter's settings.
//! Allowed queries are relayed to the adapters' own DNS servers; what is
//! blocked is decided in `dnsfilter.rs`. The firewall (`firewall.rs`) stops
//! programs from asking other DNS servers directly.
//!
//! Windows' DNS client asks on behalf of everyone, so queries cannot be
//! told apart by user: the rules of the person using the computer apply
//! (see `sessions::active_user_sid`). The browser extension still checks
//! every page for the exact user.

use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::os::windows::process::CommandExt;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use windows_sys::Win32::NetworkManagement::IpHelper::{
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_FRIENDLY_NAME, GAA_FLAG_SKIP_MULTICAST, GAA_FLAG_SKIP_UNICAST,
    GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
};
use windows_sys::Win32::NetworkManagement::Ndis::IfOperStatusUp;
use windows_sys::Win32::Networking::WinSock::{AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6};

use super::procs::Users;
use super::{reg, sessions};
use crate::dnsfilter::{self, Answerer, Upstream};
use crate::service::Service;

const NRPT_KEY: &str = r"SYSTEM\CurrentControlSet\Services\Dnscache\Parameters\DnsPolicyConfig\Iqal";
const LISTEN_V4: Ipv4Addr = Ipv4Addr::LOCALHOST;
const UDP_WORKERS: usize = 4;
/// How often the upstream servers (Wi-Fi changes) and the person at the
/// screen are looked up again.
const REFRESH_EVERY: Duration = Duration::from_secs(5);
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

pub struct DnsFilter {
    answerer: Answerer,
    users: Users,
    upstream: RwLock<Option<SocketAddr>>,
    console_user: RwLock<Option<String>>,
}

impl DnsFilter {
    pub fn start(service: Arc<Service>) -> io::Result<Arc<Self>> {
        let me = Arc::new(Self {
            answerer: Answerer::new(service.clone()),
            users: Users::load(),
            upstream: RwLock::new(None),
            console_user: RwLock::new(None),
        });
        me.refresh();
        let v4 = SocketAddr::new(IpAddr::V4(LISTEN_V4), 53);
        let v6 = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 53);
        let udp = std::iter::once(UdpSocket::bind(v4)?).chain(UdpSocket::bind(v6).ok());
        for sock in udp {
            for _ in 0..UDP_WORKERS {
                let (sock, me) = (sock.try_clone()?, me.clone());
                std::thread::Builder::new().name("iqal-dns".into()).spawn(move || me.serve_udp(sock))?;
            }
        }
        for listener in std::iter::once(TcpListener::bind(v4)?).chain(TcpListener::bind(v6).ok()) {
            let me = me.clone();
            std::thread::Builder::new().name("iqal-dns-tcp".into()).spawn(move || me.serve_tcp(listener))?;
        }
        add_redirect()?;
        {
            let me = me.clone();
            std::thread::Builder::new().name("iqal-dns-refresh".into()).spawn(move || {
                let mut generation = service.generation();
                loop {
                    std::thread::sleep(REFRESH_EVERY);
                    me.refresh();
                    // Answers already cached by Windows would outlive a
                    // rule change.
                    let g = service.generation();
                    if g != generation {
                        generation = g;
                        flush_cache();
                    }
                    if !redirect_present() {
                        let _ = add_redirect();
                    }
                }
            })?;
        }
        let upstream = *me.upstream.read().unwrap_or_else(|e| e.into_inner());
        match upstream {
            Some(addr) => eprintln!("iqal: DNS filter active, forwarding to {addr}"),
            None => eprintln!("iqal: DNS filter active, but no DNS server known yet"),
        }
        Ok(me)
    }

    fn refresh(&self) {
        let upstream = adapter_dns_servers().into_iter().next().map(|ip| SocketAddr::new(ip, 53));
        let mut current = self.upstream.write().unwrap_or_else(|e| e.into_inner());
        if *current != upstream {
            if let Some(addr) = upstream {
                eprintln!("iqal: DNS filter now forwards to {addr}");
            }
            *current = upstream;
        }
        drop(current);
        *self.console_user.write().unwrap_or_else(|e| e.into_inner()) =
            sessions::active_user_sid().and_then(|sid| self.users.name(&sid));
    }

    fn upstream(&self) -> io::Result<Upstream> {
        let addr = *self.upstream.read().unwrap_or_else(|e| e.into_inner());
        Upstream::new(addr.ok_or_else(|| io::Error::new(io::ErrorKind::NotConnected, "no DNS server"))?)
    }

    fn user(&self) -> Option<String> {
        self.console_user.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn serve_udp(&self, sock: UdpSocket) {
        let mut buf = vec![0u8; 65535];
        loop {
            let (n, from) = match sock.recv_from(&mut buf) {
                Ok(r) => r,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                // Windows reports an earlier reply's "port unreachable" here.
                Err(e) if e.raw_os_error() == Some(10054) => continue,
                Err(e) => {
                    eprintln!("iqal: DNS receive: {e}");
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };
            let pkt = &buf[..n];
            // A fresh socket per query follows Wi-Fi changes; it is cheap.
            let Ok(upstream) = self.upstream() else { continue };
            let reply = self.answerer.answer(self.user().as_deref(), pkt, &upstream).or_else(|| upstream.udp(pkt));
            if let Some(reply) = reply {
                let _ = sock.send_to(&reply, from);
            }
        }
    }

    fn serve_tcp(self: Arc<Self>, listener: TcpListener) {
        for conn in listener.incoming().flatten() {
            let me = self.clone();
            let _ = std::thread::Builder::new().name("iqal-dns-conn".into()).spawn(move || {
                let _ = me.serve_tcp_conn(conn);
            });
        }
    }

    fn serve_tcp_conn(&self, mut conn: TcpStream) -> io::Result<()> {
        conn.set_read_timeout(Some(Duration::from_secs(10)))?;
        let upstream = self.upstream()?;
        while let Some(pkt) = dnsfilter::read_tcp_message(&mut conn)? {
            let reply = self.answerer.answer(self.user().as_deref(), &pkt, &upstream).or_else(|| upstream.tcp(&pkt));
            let Some(reply) = reply else { return Ok(()) };
            dnsfilter::write_tcp_message(&mut conn, &reply)?;
        }
        Ok(())
    }
}

/// The NRPT rule: every name (".") is looked up at our listener.
fn add_redirect() -> io::Result<()> {
    reg::set(
        NRPT_KEY,
        &[
            ("Version", reg::Value::Dword(2)),
            ("Name", reg::Value::Strs(&["."])),
            ("ConfigOptions", reg::Value::Dword(0x8)), // generic DNS servers
            ("GenericDNSServers", reg::Value::Str(&LISTEN_V4.to_string())),
            ("IPSECCARestriction", reg::Value::Str("")),
            ("Comment", reg::Value::Str("Iqal website blocking")),
        ],
    )?;
    flush_cache();
    Ok(())
}

fn redirect_present() -> bool {
    reg::get_string(NRPT_KEY, "GenericDNSServers").is_some()
}

/// Gives Windows its normal DNS back (service stopping, uninstall).
pub fn remove_redirect() {
    if let Err(e) = reg::delete_tree(NRPT_KEY) {
        eprintln!("iqal: removing the DNS redirect: {e}");
    }
    flush_cache();
}

/// Empties Windows' DNS cache, so new rules apply at once.
fn flush_cache() {
    let _ = std::process::Command::new("ipconfig").arg("/flushdns").creation_flags(CREATE_NO_WINDOW).output();
}

/// DNS servers of the network adapters that are up, IPv4 first. Loopback
/// servers (ourselves, or another local resolver) and Windows' unset
/// IPv6 placeholders are left out.
fn adapter_dns_servers() -> Vec<IpAddr> {
    let flags = GAA_FLAG_SKIP_UNICAST | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST | GAA_FLAG_SKIP_FRIENDLY_NAME;
    let mut size = 16 * 1024u32;
    let mut buf: Vec<u64> = Vec::new();
    for _ in 0..3 {
        buf = vec![0u64; (size as usize).div_ceil(8)];
        // SAFETY: buf has `size` bytes of room.
        let status = unsafe {
            GetAdaptersAddresses(AF_UNSPEC as u32, flags, std::ptr::null(), buf.as_mut_ptr().cast(), &mut size)
        };
        match status {
            0 => break,
            111 => continue, // ERROR_BUFFER_OVERFLOW: size now holds the need
            _ => return Vec::new(),
        }
    }
    let mut v4 = Vec::new();
    let mut v6 = Vec::new();
    let mut adapter = buf.as_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    // SAFETY: a linked list inside buf, as filled in by GetAdaptersAddresses.
    unsafe {
        while !adapter.is_null() {
            let a = &*adapter;
            if a.OperStatus == IfOperStatusUp {
                let mut server = a.FirstDnsServerAddress;
                while !server.is_null() {
                    let sockaddr = (*server).Address.lpSockaddr;
                    if !sockaddr.is_null() {
                        match (*sockaddr).sa_family {
                            AF_INET => {
                                let s = &*sockaddr.cast::<SOCKADDR_IN>();
                                let ip = Ipv4Addr::from(u32::from_be(s.sin_addr.S_un.S_addr));
                                if !ip.is_loopback() && !ip.is_unspecified() {
                                    v4.push(IpAddr::V4(ip));
                                }
                            }
                            AF_INET6 => {
                                let s = &*sockaddr.cast::<SOCKADDR_IN6>();
                                let ip = Ipv6Addr::from(s.sin6_addr.u.Byte);
                                // Link-local servers need a scope we do not keep.
                                let placeholder = ip.segments()[0] == 0xfec0 || ip.segments()[0] & 0xffc0 == 0xfe80;
                                if !ip.is_loopback() && !ip.is_unspecified() && !placeholder {
                                    v6.push(IpAddr::V6(ip));
                                }
                            }
                            _ => {}
                        }
                    }
                    server = (*server).Next;
                }
            }
            adapter = a.Next;
        }
    }
    v4.extend(v6);
    v4.dedup();
    v4
}
