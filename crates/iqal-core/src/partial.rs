//! Partial blocking: block or hide distracting parts of a site while
//! leaving the rest usable. URL parts are enforced by the engine; element
//! hiding is applied by the browser extension using the CSS selectors.
//!
//! Sites change their markup over time, so this table is expected to be
//! updated together with the pre-made lists.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialRule {
    pub id: &'static str,
    /// The site this rule is about; also matches subdomains.
    pub domain: &'static str,
    /// Paths that are blocked outright. A path ending in `*` is a prefix,
    /// otherwise the path must match exactly (query ignored).
    pub paths: &'static [&'static str],
    /// Elements the extension hides on this site.
    pub hide: &'static [&'static str],
}

pub const BUILTIN: &[PartialRule] = &[
    PartialRule {
        id: "youtube-shorts",
        domain: "youtube.com",
        paths: &["/shorts*"],
        hide: &[
            "ytd-reel-shelf-renderer",
            "ytd-rich-shelf-renderer[is-shorts]",
            "ytd-guide-entry-renderer:has(a[title='Shorts'])",
            "ytd-mini-guide-entry-renderer[aria-label='Shorts']",
            "a[href^='/shorts/']",
        ],
    },
    PartialRule {
        id: "youtube-comments",
        domain: "youtube.com",
        paths: &[],
        hide: &["ytd-comments#comments"],
    },
    PartialRule {
        id: "youtube-recommendations",
        domain: "youtube.com",
        paths: &[],
        hide: &[
            "#related",
            "ytd-browse[page-subtype='home'] ytd-rich-grid-renderer",
            ".ytp-endscreen-content",
            ".ytp-ce-element",
        ],
    },
    PartialRule {
        id: "reddit-home",
        domain: "reddit.com",
        paths: &["/", "/r/all*", "/r/popular*"],
        hide: &[],
    },
    PartialRule {
        id: "facebook-feed",
        domain: "facebook.com",
        paths: &["/watch*", "/reel*"],
        hide: &["[role='feed']"],
    },
    PartialRule {
        id: "instagram-reels",
        domain: "instagram.com",
        paths: &["/reels*", "/reel/*"],
        hide: &["a[href^='/reels/']"],
    },
    PartialRule {
        id: "instagram-explore",
        domain: "instagram.com",
        paths: &["/explore*"],
        hide: &["a[href='/explore/']"],
    },
    PartialRule {
        id: "x-trends",
        domain: "x.com",
        paths: &["/explore*"],
        hide: &["[aria-label='Timeline: Trending now']", "a[href='/explore']"],
    },
];

pub fn find(id: &str) -> Option<&'static PartialRule> {
    BUILTIN.iter().find(|r| r.id == id)
}

impl PartialRule {
    pub fn applies_to_host(&self, host: &str) -> bool {
        host == self.domain || host.strip_suffix(self.domain).is_some_and(|rest| rest.ends_with('.'))
    }

    /// Is this path (lower-case, may include `?query`) blocked by the rule?
    pub fn blocks_path(&self, path: &str) -> bool {
        let path = path.split('?').next().unwrap_or(path);
        let path = if path.is_empty() { "/" } else { path };
        self.paths.iter().any(|p| match p.strip_suffix('*') {
            Some(prefix) => path.starts_with(prefix),
            None => path == *p,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_are_unique() {
        for (i, a) in BUILTIN.iter().enumerate() {
            assert!(BUILTIN[i + 1..].iter().all(|b| b.id != a.id), "{}", a.id);
        }
    }

    #[test]
    fn shorts() {
        let r = find("youtube-shorts").unwrap();
        assert!(r.applies_to_host("www.youtube.com"));
        assert!(r.applies_to_host("youtube.com"));
        assert!(!r.applies_to_host("notyoutube.com"));
        assert!(r.blocks_path("/shorts/abc123"));
        assert!(!r.blocks_path("/watch?v=abc"));
    }

    #[test]
    fn reddit_front_page_only() {
        let r = find("reddit-home").unwrap();
        assert!(r.blocks_path("/"));
        assert!(r.blocks_path(""));
        assert!(r.blocks_path("/?feed=home"));
        assert!(r.blocks_path("/r/popular/"));
        assert!(!r.blocks_path("/r/rust/"));
    }
}
