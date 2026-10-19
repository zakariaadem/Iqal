//! The DNS filter on Linux. The firewall redirects each enforced user's DNS
//! traffic (port 53, to any server) to a listener of its own on loopback,
//! so the filter knows whose query it is. Allowed queries are relayed to
//! the system resolver; what is blocked is decided in `dnsfilter.rs`.

use std::collections::HashSet;
use std::io;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::procs::Users;
use crate::dnsfilter::{self, Answerer, Upstream};
use crate::service::Service;

pub const BASE_PORT: u16 = 16100;
const MAX_USERS: u32 = 2000;
const UDP_WORKERS_V4: usize = 4;
const UDP_WORKERS_V6: usize = 2;

pub fn port_for(users: &Users, uid: u32) -> Option<u16> {
    let offset = uid.checked_sub(users.uid_min())?;
    (offset < MAX_USERS).then(|| BASE_PORT + offset as u16)
}

pub struct DnsFilter {
    answerer: Answerer,
    users: Users,
    upstream: SocketAddr,
    listening: Mutex<HashSet<u32>>,
}

impl DnsFilter {
    pub fn new(service: Arc<Service>) -> Arc<Self> {
        let upstream = system_resolver();
        eprintln!("iqal: DNS filter forwards to {upstream}");
        Arc::new(Self { answerer: Answerer::new(service), users: Users::load(), upstream, listening: Mutex::new(HashSet::new()) })
    }

    /// Makes sure a listener runs for every enforced account, and returns
    /// (uid, port) for each, for the firewall rules.
    pub fn ensure_listeners(self: &Arc<Self>) -> Vec<(u32, u16)> {
        let mut out = Vec::new();
        for uid in self.users.enforced_uids() {
            let Some(port) = port_for(&self.users, uid) else { continue };
            let mut listening = self.listening.lock().unwrap_or_else(|e| e.into_inner());
            if !listening.contains(&uid) {
                match self.listen(uid, port) {
                    Ok(()) => {
                        listening.insert(uid);
                    }
                    Err(e) => {
                        eprintln!("iqal: DNS listener for uid {uid} on port {port}: {e}");
                        continue;
                    }
                }
            }
            out.push((uid, port));
        }
        out
    }

    fn listen(self: &Arc<Self>, uid: u32, port: u16) -> io::Result<()> {
        let v4 = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
        let v6 = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), port);
        let udp4 = UdpSocket::bind(v4)?;
        let tcp4 = TcpListener::bind(v4)?;
        // IPv6 loopback may be disabled; IPv4 alone is enough then.
        let udp6 = UdpSocket::bind(v6).ok();
        let tcp6 = TcpListener::bind(v6).ok();

        let mut udp = vec![(udp4, UDP_WORKERS_V4)];
        udp.extend(udp6.map(|s| (s, UDP_WORKERS_V6)));
        for (sock, workers) in udp {
            for _ in 0..workers {
                let sock = sock.try_clone()?;
                let me = self.clone();
                std::thread::Builder::new().name("iqal-dns".into()).spawn(move || me.serve_udp(uid, sock))?;
            }
        }
        for listener in std::iter::once(tcp4).chain(tcp6) {
            let me = self.clone();
            std::thread::Builder::new().name("iqal-dns-tcp".into()).spawn(move || me.serve_tcp(uid, listener))?;
        }
        Ok(())
    }

    fn serve_udp(&self, uid: u32, sock: UdpSocket) {
        let upstream = match Upstream::new(self.upstream) {
            Ok(u) => u,
            Err(e) => return eprintln!("iqal: DNS upstream socket: {e}"),
        };
        let mut buf = vec![0u8; 65535];
        loop {
            let (n, from) = match sock.recv_from(&mut buf) {
                Ok(r) => r,
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => {
                    eprintln!("iqal: DNS receive: {e}");
                    std::thread::sleep(Duration::from_millis(50));
                    continue;
                }
            };
            let pkt = &buf[..n];
            let user = self.users.name(uid);
            let reply = self.answerer.answer(Some(&user), pkt, &upstream).or_else(|| upstream.udp(pkt));
            if let Some(reply) = reply {
                let _ = sock.send_to(&reply, from);
            }
        }
    }

    fn serve_tcp(self: Arc<Self>, uid: u32, listener: TcpListener) {
        for conn in listener.incoming().flatten() {
            let me = self.clone();
            let _ = std::thread::Builder::new().name("iqal-dns-conn".into()).spawn(move || {
                let _ = me.serve_tcp_conn(uid, conn);
            });
        }
    }

    fn serve_tcp_conn(&self, uid: u32, mut conn: TcpStream) -> io::Result<()> {
        conn.set_read_timeout(Some(Duration::from_secs(10)))?;
        let upstream = Upstream::new(self.upstream)?;
        while let Some(pkt) = dnsfilter::read_tcp_message(&mut conn)? {
            let user = self.users.name(uid);
            let reply = self.answerer.answer(Some(&user), &pkt, &upstream).or_else(|| upstream.tcp(&pkt));
            let Some(reply) = reply else { return Ok(()) };
            dnsfilter::write_tcp_message(&mut conn, &reply)?;
        }
        Ok(())
    }
}

/// Where the system resolver listens: the first `nameserver` in
/// /etc/resolv.conf (127.0.0.53 with systemd-resolved).
fn system_resolver() -> SocketAddr {
    let conf = std::fs::read_to_string("/etc/resolv.conf").unwrap_or_default();
    conf.lines()
        .filter_map(|l| l.trim().strip_prefix("nameserver"))
        .filter_map(|ip| ip.trim().parse::<IpAddr>().ok())
        .map(|ip| SocketAddr::new(ip, 53))
        .next()
        .unwrap_or_else(|| SocketAddr::from(([127, 0, 0, 53], 53)))
}
