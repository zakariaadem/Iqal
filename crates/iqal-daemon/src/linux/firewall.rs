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

use crate::dnsfilter::{DOH_V4, DOH_V6};

const NFT: &str = "/usr/sbin/nft";
const TABLE: &str = "iqal";

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
