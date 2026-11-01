//! The catalog of pre-made lists. Each list has a curated starter file
//! shipped with Iqal (`lists/<id>.txt`) and optionally upstream sources
//! that the service downloads daily and merges in.
//!
//! Upstream sources must be compatible with GPL-3.0; StevenBlack/hosts is
//! MIT-licensed.

pub struct ListDef {
    pub id: &'static str,
    /// Hosts-file or plain lists, downloaded over HTTPS.
    pub sources: &'static [&'static str],
    /// Programs the list also blocks (app rules), for lists whose websites
    /// come with programs that would get around them.
    pub apps: &'static [&'static str],
}

pub const CATALOG: &[ListDef] = &[
    ListDef { id: "social", sources: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/social-only/hosts"], apps: &[] },
    ListDef { id: "adult", sources: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn-only/hosts"], apps: &[] },
    ListDef { id: "gambling", sources: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/gambling-only/hosts"], apps: &[] },
    ListDef { id: "search", sources: &[], apps: &[] },
    ListDef { id: "news", sources: &[], apps: &[] },
    ListDef { id: "video", sources: &[], apps: &[] },
    ListDef { id: "games", sources: &[], apps: &[] },
    ListDef { id: "shopping", sources: &[], apps: &[] },
    // VPN and proxy services hide where the computer connects to: their
    // websites (no downloading or signing up) and their programs, including
    // the general VPN tools OpenVPN and WireGuard.
    ListDef { id: "vpn", sources: &[], apps: VPN_APPS },
];

/// Programs of VPN and proxy services (Windows and Linux file names).
const VPN_APPS: &[&str] = &[
    "nordvpn", "nordvpn-service", "nordvpnd",
    "expressvpn", "expressvpnd",
    "protonvpn", "protonvpn.client", "protonvpnservice", "protonvpn-app",
    "surfshark", "surfshark.service",
    "windscribe", "windscribeservice", "windscribe-cli",
    "cyberghost", "cyberghost.service",
    "pia-client", "pia-service", "pia-daemon",
    "mullvad vpn", "mullvad-gui", "mullvad-daemon", "mullvad",
    "tunnelbear",
    "hsscp",
    "ipvanish",
    "vyprvpn",
    "psiphon3", "psiphon-tunnel-core",
    "lantern",
    "tor",
    "cloudflare warp", "warp-svc", "warp-cli",
    "openvpn", "openvpn-gui", "openvpnserv",
    "wireguard", "wireguard-go",
];

pub fn find(id: &str) -> Option<&'static ListDef> {
    CATALOG.iter().find(|l| l.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_is_consistent() {
        for (i, a) in CATALOG.iter().enumerate() {
            assert!(CATALOG[i + 1..].iter().all(|b| b.id != a.id));
            for s in a.sources {
                assert!(s.starts_with("https://"), "{s}");
            }
        }
        assert!(find("adult").is_some());
        assert!(find("nope").is_none());
        let vpn = crate::import::parse_rules_text(include_str!("../../../lists/vpn.txt"));
        assert!(vpn.errors.is_empty(), "{:?}", vpn.errors);
        assert!(vpn.rules.len() > 50);
        for l in CATALOG {
            for a in l.apps {
                assert!(matches!(crate::AppRule::parse(&format!("name:{a}")), Ok(crate::AppRule::Name(_))), "{a}");
            }
        }
    }
}
