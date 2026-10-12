//! nftables rules, in a table of our own (`inet iqal`) that is replaced
//! atomically on every change:
//!
//! * each enforced user's DNS (UDP/TCP port 53, to any server) is
//!   redirected to that user's DNS filter listener;
//! * DNS-over-TLS/QUIC (port 853) is refused for enforced users;
//! * HTTPS/QUIC to well-known DNS-over-HTTPS servers is refused for them.
//!
//! Root and system services are never affected.

use std::io::{self, Write};
use std::process::{Command, Stdio};

const NFT: &str = "/usr/sbin/nft";
const TABLE: &str = "iqal";

/// Addresses of public DNS-over-HTTPS services.
const DOH_V4: &[&str] = &[
    "8.8.8.8", "8.8.4.4", "1.1.1.1", "1.0.0.1", "1.1.1.2", "1.0.0.2", "1.1.1.3", "1.0.0.3", "9.9.9.9", "9.9.9.10",
    "9.9.9.11", "149.112.112.112", "149.112.112.10", "149.112.112.11", "208.67.222.222", "208.67.220.220",
    "208.67.222.123", "208.67.220.123", "94.140.14.14", "94.140.15.15", "94.140.14.15", "94.140.15.16",
    "185.228.168.9", "185.228.169.9", "185.228.168.168", "185.228.169.168", "76.76.2.0/24", "76.76.10.0/24",
    "45.90.28.0/24", "45.90.30.0/24", "194.242.2.0/24", "185.222.222.222", "45.11.45.11", "223.5.5.5",
    "223.6.6.6", "1.12.12.12", "120.53.53.53", "8.26.56.26", "8.20.247.20",
];
const DOH_V6: &[&str] = &[
    "2001:4860:4860::8888", "2001:4860:4860::8844", "2606:4700:4700::1111", "2606:4700:4700::1001",
    "2606:4700:4700::1112", "2606:4700:4700::1002", "2606:4700:4700::1113", "2606:4700:4700::1003",
    "2620:fe::fe", "2620:fe::9", "2620:fe::10", "2620:fe::11", "2620:119:35::35", "2620:119:53::53",
    "2a10:50c0::ad1:ff", "2a10:50c0::ad2:ff", "2a07:a8c0::/32", "2a07:a8c1::/32", "2a07:e340::/32",
];

/// The complete ruleset for the given (uid, DNS filter port) pairs.
pub fn ruleset(users: &[(u32, u16)]) -> String {
    let mut s = String::new();
    // Declaring then deleting makes the first run (no table yet) work too;
    // the whole file is applied as one transaction.
    s.push_str(&format!("table inet {TABLE}\ndelete table inet {TABLE}\ntable inet {TABLE} {{\n"));
    s.push_str(&format!("\tset doh4 {{ type ipv4_addr; flags interval; elements = {{ {} }} }}\n", DOH_V4.join(", ")));
    s.push_str(&format!("\tset doh6 {{ type ipv6_addr; flags interval; elements = {{ {} }} }}\n", DOH_V6.join(", ")));
    s.push_str("\tchain dns {\n\t\ttype nat hook output priority -100; policy accept;\n");
    for (uid, port) in users {
        s.push_str(&format!("\t\tmeta skuid {uid} meta l4proto {{ tcp, udp }} th dport 53 redirect to :{port}\n"));
    }
    s.push_str("\t}\n");
    s.push_str("\tchain out {\n\t\ttype filter hook output priority 0; policy accept;\n");
    if !users.is_empty() {
        let uids = users.iter().map(|(u, _)| u.to_string()).collect::<Vec<_>>().join(", ");
        s.push_str(&format!("\t\tmeta skuid {{ {uids} }} meta l4proto {{ tcp, udp }} th dport 853 reject\n"));
        s.push_str(&format!(
            "\t\tmeta skuid {{ {uids} }} ip daddr @doh4 meta l4proto {{ tcp, udp }} th dport 443 reject\n"
        ));
        s.push_str(&format!(
            "\t\tmeta skuid {{ {uids} }} ip6 daddr @doh6 meta l4proto {{ tcp, udp }} th dport 443 reject\n"
        ));
    }
    s.push_str("\t}\n}\n");
    s
}

pub fn apply(users: &[(u32, u16)]) -> io::Result<()> {
    let mut child = Command::new(NFT).args(["-f", "-"]).stdin(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    child.stdin.take().expect("piped").write_all(ruleset(users).as_bytes())?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        return Err(io::Error::other(format!("nft: {}", String::from_utf8_lossy(&out.stderr).trim())));
    }
    Ok(())
}

/// Whether our table exists with a redirect for each of `users` (someone
/// may have deleted or replaced it).
pub fn intact(users: &[(u32, u16)]) -> bool {
    let Ok(out) = Command::new(NFT).args(["list", "table", "inet", TABLE]).stderr(Stdio::null()).output() else {
        return false;
    };
    let listing = String::from_utf8_lossy(&out.stdout);
    out.status.success()
        && listing.contains("chain out")
        && users.iter().all(|(uid, port)| {
            listing.contains(&format!("meta skuid {uid} ")) && listing.contains(&format!("redirect to :{port}"))
        })
}

pub fn remove() {
    let _ = Command::new(NFT)
        .args(["delete", "table", "inet", TABLE])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ruleset_shape() {
        let r = ruleset(&[(1000, 16100), (1001, 16101)]);
        assert!(r.contains("meta skuid 1000 meta l4proto { tcp, udp } th dport 53 redirect to :16100"));
        assert!(r.contains("meta skuid 1001 meta l4proto { tcp, udp } th dport 53 redirect to :16101"));
        assert!(r.contains("meta skuid { 1000, 1001 } meta l4proto { tcp, udp } th dport 853 reject"));
        assert!(r.starts_with("table inet iqal\ndelete table inet iqal\n"));
        let empty = ruleset(&[]);
        assert!(!empty.contains("skuid"));
    }
}
