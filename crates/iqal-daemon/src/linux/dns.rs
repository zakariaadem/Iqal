//! The DNS filter. The firewall redirects each enforced user's DNS traffic
//! (port 53, to any server) to a listener of its own on loopback, so the
//! filter knows whose query it is. Allowed queries are relayed unchanged to
//! the system resolver; blocked names get 0.0.0.0 / ::.
//!
//! Also handled here:
//! * SafeSearch: Google/Bing/DuckDuckGo/YouTube resolve to their
//!   restricted-mode addresses.
//! * Encrypted-DNS bypasses: well-known DoH servers do not resolve, and
//!   Firefox's canary domain tells it to keep DoH off.

use std::collections::{HashMap, HashSet};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iqal_core::web::{host_suffixes, safe_search_target};
use iqal_core::{Ctx, Hit};
use iqal_ipc::EventKind;

use super::procs::Users;
use crate::dnsmsg::{self, Query, TYPE_A, TYPE_AAAA, TYPE_ANY};
use crate::service::Service;

pub const BASE_PORT: u16 = 16100;
const MAX_USERS: u32 = 2000;
const UDP_WORKERS_V4: usize = 4;
const UDP_WORKERS_V6: usize = 2;
const UPSTREAM_TIMEOUT: Duration = Duration::from_secs(4);
const EVENT_QUIET: Duration = Duration::from_secs(60);

/// Firefox disables its built-in DoH when this name does not resolve.
const FIREFOX_CANARY: &str = "use-application-dns.net";

/// Public DNS-over-HTTPS/TLS services; resolving them would let a browser
/// or app bypass the filter.
const DOH_HOSTS: &[&str] = &[
    "dns.google", "dns.google.com", "8888.google", "cloudflare-dns.com", "one.one.one.one", "1dot1dot1dot1.cloudflare-dns.com",
    "dns.quad9.net", "dns9.quad9.net", "dns10.quad9.net", "dns11.quad9.net", "doh.opendns.com",
    "doh.familyshield.opendns.com", "dns.adguard.com", "dns.adguard-dns.com", "dns-family.adguard.com",
    "dns.nextdns.io", "doh.cleanbrowsing.org", "doh.mullvad.net", "dns.mullvad.net", "dns.controld.com",
    "freedns.controld.com", "doh.dns.sb", "dns.alidns.com", "doh.pub", "dns.twnic.tw", "doh.libredns.gr",
    "dns.switch.ch", "dns0.eu", "zero.dns0.eu", "kids.dns0.eu", "dns.digitale-gesellschaft.ch",
];

pub fn port_for(users: &Users, uid: u32) -> Option<u16> {
    let offset = uid.checked_sub(users.uid_min())?;
    (offset < MAX_USERS).then(|| BASE_PORT + offset as u16)
}

pub struct DnsFilter {
    service: Arc<Service>,
    users: Users,
    upstream: SocketAddr,
    listening: Mutex<HashSet<u32>>,
    recent: Mutex<HashMap<(u32, String), Instant>>,
}

impl DnsFilter {
    pub fn new(service: Arc<Service>) -> Arc<Self> {
        let upstream = system_resolver();
        eprintln!("iqal: DNS filter forwards to {upstream}");
        Arc::new(Self {
            service,
            users: Users::load(),
            upstream,
            listening: Mutex::new(HashSet::new()),
            recent: Mutex::new(HashMap::new()),
        })
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
            let reply = self.answer(uid, pkt, &upstream).or_else(|| upstream.udp(pkt));
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
        loop {
            let mut len = [0u8; 2];
            if conn.read_exact(&mut len).is_err() {
                return Ok(());
            }
            let mut pkt = vec![0u8; u16::from_be_bytes(len) as usize];
            conn.read_exact(&mut pkt)?;
            let reply = self.answer(uid, &pkt, &upstream).or_else(|| upstream.tcp(&pkt));
            let Some(reply) = reply else { return Ok(()) };
            let mut framed = (reply.len() as u16).to_be_bytes().to_vec();
            framed.extend_from_slice(&reply);
            conn.write_all(&framed)?;
        }
    }

    /// Our own answer, or `None` to relay the query upstream.
    fn answer(&self, uid: u32, pkt: &[u8], upstream: &Upstream) -> Option<Vec<u8>> {
        let q = dnsmsg::parse_query(pkt)?;
        if q.name == FIREFOX_CANARY {
            return Some(dnsmsg::nxdomain(pkt, &q));
        }
        if host_suffixes(&q.name).any(|s| DOH_HOSTS.contains(&s)) {
            return Some(dnsmsg::blocked(pkt, &q));
        }
        let user = self.users.name(uid);
        let engine = self.service.engine();
        let usage = self.service.usage_for(&user);
        let ctx = Ctx { now: crate::clock::now(), user: &user, usage: usage.as_deref() };
        if let Some(hit) = engine.check_host(&q.name, &ctx).block {
            self.record(uid, &user, &q.name, &hit);
            return Some(dnsmsg::blocked(pkt, &q));
        }
        if engine.safe_search(&ctx)
            && let Some(target) = safe_search_target(&q.name)
        {
            return Some(self.safe_search(pkt, &q, target, upstream));
        }
        None
    }

    fn safe_search(&self, pkt: &[u8], q: &Query, target: &str, upstream: &Upstream) -> Vec<u8> {
        let qtype = match q.qtype {
            TYPE_A | TYPE_ANY => TYPE_A,
            TYPE_AAAA => TYPE_AAAA,
            // HTTPS/SVCB records could point around the restriction.
            _ => return dnsmsg::no_data(pkt, q),
        };
        let lookup = dnsmsg::build_query(q.id ^ 0x5a5a, target, qtype);
        match upstream.udp(&lookup).and_then(|r| dnsmsg::address_records(&r)) {
            Some(records) => dnsmsg::with_addresses(pkt, q, &records),
            // Never fall back to the unrestricted address.
            None => dnsmsg::no_data(pkt, q),
        }
    }

    /// Logs a DNS block, at most once a minute per user and name (browsers
    /// look names up again and again, and prefetch links).
    fn record(&self, uid: u32, user: &str, name: &str, hit: &Hit) {
        let key = (uid, name.to_string());
        let mut recent = self.recent.lock().unwrap_or_else(|e| e.into_inner());
        if recent.get(&key).is_some_and(|t| t.elapsed() < EVENT_QUIET) {
            return;
        }
        if recent.len() > 10_000 {
            recent.retain(|_, t| t.elapsed() < EVENT_QUIET);
        }
        recent.insert(key, Instant::now());
        drop(recent);
        self.service.record_block(user, EventKind::SiteBlocked, name, hit);
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

struct Upstream {
    addr: SocketAddr,
    sock: UdpSocket,
}

impl Upstream {
    fn new(addr: SocketAddr) -> io::Result<Self> {
        let local: SocketAddr = if addr.is_ipv4() { ([0, 0, 0, 0], 0).into() } else { (Ipv6Addr::UNSPECIFIED, 0).into() };
        let sock = UdpSocket::bind(local)?;
        sock.connect(addr)?;
        sock.set_read_timeout(Some(UPSTREAM_TIMEOUT))?;
        Ok(Self { addr, sock })
    }

    fn udp(&self, pkt: &[u8]) -> Option<Vec<u8>> {
        let id = pkt.get(..2)?;
        self.sock.send(pkt).ok()?;
        let mut buf = vec![0u8; 65535];
        let deadline = Instant::now() + UPSTREAM_TIMEOUT;
        while Instant::now() < deadline {
            let n = self.sock.recv(&mut buf).ok()?;
            // Skip late answers to earlier, timed-out queries.
            if n >= 2 && &buf[..2] == id {
                buf.truncate(n);
                return Some(buf);
            }
        }
        None
    }

    fn tcp(&self, pkt: &[u8]) -> Option<Vec<u8>> {
        let mut conn = TcpStream::connect_timeout(&self.addr, UPSTREAM_TIMEOUT).ok()?;
        conn.set_read_timeout(Some(UPSTREAM_TIMEOUT)).ok()?;
        let mut framed = (pkt.len() as u16).to_be_bytes().to_vec();
        framed.extend_from_slice(pkt);
        conn.write_all(&framed).ok()?;
        let mut len = [0u8; 2];
        conn.read_exact(&mut len).ok()?;
        let mut reply = vec![0u8; u16::from_be_bytes(len) as usize];
        conn.read_exact(&mut reply).ok()?;
        Some(reply)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn doh_hosts_match_subdomains() {
        let hit = |n: &str| host_suffixes(n).any(|s| DOH_HOSTS.contains(&s));
        assert!(hit("dns.google"));
        assert!(hit("mozilla.cloudflare-dns.com"));
        assert!(!hit("google.com"));
        assert!(!hit("example.com"));
    }
}
