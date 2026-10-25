//! The DNS filter's decisions, shared by every platform: blocked names get
//! 0.0.0.0 / ::, allowed ones are relayed unchanged.
//!
//! Also handled here:
//! * SafeSearch: Google/Bing/DuckDuckGo/Yandex resolve to their
//!   restricted-mode addresses.
//! * Encrypted-DNS bypasses: well-known DoH servers do not resolve, and
//!   Firefox's canary domain tells it to keep DoH off.

use std::collections::HashMap;
use std::io::{self, Read, Write};
use std::net::{Ipv6Addr, SocketAddr, TcpStream, UdpSocket};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use iqal_core::web::{host_suffixes, safe_search_target};
use iqal_core::{Ctx, Hit};
use iqal_ipc::EventKind;

use crate::dnsmsg::{self, Query, TYPE_A, TYPE_AAAA, TYPE_ANY};
use crate::service::Service;

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

/// Addresses of public DNS-over-HTTPS services, for the firewall.
pub const DOH_V4: &[&str] = &[
    "8.8.8.8", "8.8.4.4", "1.1.1.1", "1.0.0.1", "1.1.1.2", "1.0.0.2", "1.1.1.3", "1.0.0.3", "9.9.9.9", "9.9.9.10",
    "9.9.9.11", "149.112.112.112", "149.112.112.10", "149.112.112.11", "208.67.222.222", "208.67.220.220",
    "208.67.222.123", "208.67.220.123", "94.140.14.14", "94.140.15.15", "94.140.14.15", "94.140.15.16",
    "185.228.168.9", "185.228.169.9", "185.228.168.168", "185.228.169.168", "76.76.2.0/24", "76.76.10.0/24",
    "45.90.28.0/24", "45.90.30.0/24", "194.242.2.0/24", "185.222.222.222", "45.11.45.11", "223.5.5.5",
    "223.6.6.6", "1.12.12.12", "120.53.53.53", "8.26.56.26", "8.20.247.20",
];
pub const DOH_V6: &[&str] = &[
    "2001:4860:4860::8888", "2001:4860:4860::8844", "2606:4700:4700::1111", "2606:4700:4700::1001",
    "2606:4700:4700::1112", "2606:4700:4700::1002", "2606:4700:4700::1113", "2606:4700:4700::1003",
    "2620:fe::fe", "2620:fe::9", "2620:fe::10", "2620:fe::11", "2620:119:35::35", "2620:119:53::53",
    "2a10:50c0::ad1:ff", "2a10:50c0::ad2:ff", "2a07:a8c0::/32", "2a07:a8c1::/32", "2a07:e340::/32",
];

pub struct Answerer {
    service: Arc<Service>,
    recent: Mutex<HashMap<(String, String), Instant>>,
}

impl Answerer {
    pub fn new(service: Arc<Service>) -> Self {
        Self { service, recent: Mutex::new(HashMap::new()) }
    }

    /// Our own answer for a query from `user` (`None`: nobody whose rules
    /// apply), or `None` to relay the query upstream.
    pub fn answer(&self, user: Option<&str>, pkt: &[u8], upstream: &Upstream) -> Option<Vec<u8>> {
        let q = dnsmsg::parse_query(pkt)?;
        if q.name == FIREFOX_CANARY {
            return Some(dnsmsg::nxdomain(pkt, &q));
        }
        if host_suffixes(&q.name).any(|s| DOH_HOSTS.contains(&s)) {
            return Some(dnsmsg::blocked(pkt, &q));
        }
        let user = user?;
        let engine = self.service.engine();
        let usage = self.service.usage_for(user);
        let ctx = Ctx { now: crate::clock::now(), user, usage: usage.as_deref() };
        if let Some(hit) = engine.check_host(&q.name, &ctx).block {
            self.record(user, &q.name, &hit);
            return Some(dnsmsg::blocked(pkt, &q));
        }
        if engine.safe_search(&ctx)
            && let Some(target) = safe_search_target(&q.name)
        {
            return Some(safe_search(pkt, &q, target, upstream));
        }
        None
    }

    /// Logs a DNS block, at most once a minute per user and name (browsers
    /// look names up again and again, and prefetch links).
    fn record(&self, user: &str, name: &str, hit: &Hit) {
        let key = (user.to_string(), name.to_string());
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

fn safe_search(pkt: &[u8], q: &Query, target: &str, upstream: &Upstream) -> Vec<u8> {
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

/// The resolver allowed queries are relayed to.
pub struct Upstream {
    addr: SocketAddr,
    sock: UdpSocket,
}

impl Upstream {
    pub fn new(addr: SocketAddr) -> io::Result<Self> {
        let local: SocketAddr = if addr.is_ipv4() { ([0, 0, 0, 0], 0).into() } else { (Ipv6Addr::UNSPECIFIED, 0).into() };
        let sock = UdpSocket::bind(local)?;
        sock.connect(addr)?;
        sock.set_read_timeout(Some(UPSTREAM_TIMEOUT))?;
        Ok(Self { addr, sock })
    }

    pub fn udp(&self, pkt: &[u8]) -> Option<Vec<u8>> {
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

    pub fn tcp(&self, pkt: &[u8]) -> Option<Vec<u8>> {
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

/// Reads one DNS-over-TCP message (2-byte length prefix); `None` at the end.
pub fn read_tcp_message(conn: &mut TcpStream) -> io::Result<Option<Vec<u8>>> {
    let mut len = [0u8; 2];
    if conn.read_exact(&mut len).is_err() {
        return Ok(None);
    }
    let mut pkt = vec![0u8; u16::from_be_bytes(len) as usize];
    conn.read_exact(&mut pkt)?;
    Ok(Some(pkt))
}

pub fn write_tcp_message(conn: &mut TcpStream, msg: &[u8]) -> io::Result<()> {
    let mut framed = (msg.len() as u16).to_be_bytes().to_vec();
    framed.extend_from_slice(msg);
    conn.write_all(&framed)
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
