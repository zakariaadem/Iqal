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
}

pub const CATALOG: &[ListDef] = &[
    ListDef { id: "social", sources: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/social-only/hosts"] },
    ListDef { id: "adult", sources: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/porn-only/hosts"] },
    ListDef { id: "gambling", sources: &["https://raw.githubusercontent.com/StevenBlack/hosts/master/alternates/gambling-only/hosts"] },
    ListDef { id: "search", sources: &[] },
    ListDef { id: "news", sources: &[] },
    ListDef { id: "video", sources: &[] },
    ListDef { id: "games", sources: &[] },
    ListDef { id: "shopping", sources: &[] },
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
    }
}
