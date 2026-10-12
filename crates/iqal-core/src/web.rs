//! Website rules: parsing, URL views, search-query extraction and
//! SafeSearch host mapping.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::text::{normalize, percent_decode};

/// One website rule. Written by users (and stored) as a short string:
///
/// | Text                   | Rule                                        |
/// |------------------------|---------------------------------------------|
/// | `example.com`          | the domain and all its subdomains           |
/// | `example.com/videos`   | URLs on that domain starting with the path  |
/// | `contains:casino`      | any URL containing the text                 |
/// | `search:free movies`   | searches whose query contains the text      |
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum WebRule {
    Domain(String),
    Url { host: String, path: String },
    UrlContains(String),
    SearchTerm(String),
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuleParseError {
    #[error("empty rule")]
    Empty,
    #[error("\"{0}\" is not a valid website address")]
    InvalidAddress(String),
    #[error("\"{0}\" has no domain ending (use \"contains:{0}\" to match any URL containing it)")]
    NotADomain(String),
    #[error("unknown rule prefix in \"{0}\"")]
    UnknownPrefix(String),
}

impl WebRule {
    pub fn parse(input: &str) -> Result<Self, RuleParseError> {
        let s = input.trim();
        if s.is_empty() {
            return Err(RuleParseError::Empty);
        }

        if let Some((prefix, rest)) = s.split_once(':') {
            let rest = normalize(rest);
            let make = match prefix.trim().to_ascii_lowercase().as_str() {
                "contains" | "url" => Some(WebRule::UrlContains as fn(String) -> WebRule),
                "search" | "keyword" => Some(WebRule::SearchTerm as fn(String) -> WebRule),
                "http" | "https" => None,
                _ if !prefix.contains(['.', '/']) && !prefix.chars().all(|c| c.is_ascii_digit()) => {
                    return Err(RuleParseError::UnknownPrefix(s.to_string()));
                }
                _ => None,
            };
            if let Some(make) = make {
                return if rest.is_empty() { Err(RuleParseError::Empty) } else { Ok(make(rest)) };
            }
        }

        // Adblock-style "||example.com^".
        let mut s = s;
        if let Some(rest) = s.strip_prefix("||") {
            s = rest.split(['^', '$']).next().unwrap_or_default();
        }

        // "*casino*" is a contains rule.
        if s.len() > 2 && s.starts_with('*') && s.ends_with('*') {
            let inner = normalize(&s[1..s.len() - 1]);
            if inner.is_empty() {
                return Err(RuleParseError::Empty);
            }
            return Ok(WebRule::UrlContains(inner));
        }
        let s = s.strip_prefix("*.").unwrap_or(s);

        parse_address(s)
    }

    /// True for rules that can be enforced from a host name alone
    /// (DNS level), without seeing the full URL.
    pub fn is_domain_level(&self) -> bool {
        matches!(self, WebRule::Domain(_))
    }
}

fn parse_address(s: &str) -> Result<WebRule, RuleParseError> {
    if s.chars().any(char::is_whitespace) {
        return Err(RuleParseError::InvalidAddress(s.to_string()));
    }
    let with_scheme = if s.contains("://") { s.to_string() } else { format!("http://{s}") };
    let url = url::Url::parse(&with_scheme)
        .map_err(|_| RuleParseError::InvalidAddress(s.to_string()))?;
    let host = url
        .host_str()
        .map(clean_host)
        .filter(|h| !h.is_empty())
        .ok_or_else(|| RuleParseError::InvalidAddress(s.to_string()))?;
    let is_ip = matches!(url.host(), Some(url::Host::Ipv4(_) | url::Host::Ipv6(_)));
    if !is_ip && !host.contains('.') {
        return Err(RuleParseError::NotADomain(s.to_string()));
    }
    let host = host.strip_prefix("www.").unwrap_or(&host).to_string();

    let mut path = url.path().to_lowercase();
    if let Some(q) = url.query() {
        path.push('?');
        path.push_str(&q.to_lowercase());
    }
    if path == "/" || path.is_empty() {
        Ok(WebRule::Domain(host))
    } else {
        Ok(WebRule::Url { host, path })
    }
}

/// Lower-cases a host and removes a trailing root dot.
pub fn clean_host(host: &str) -> String {
    host.trim_end_matches('.').to_ascii_lowercase()
}

impl fmt::Display for WebRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WebRule::Domain(d) => f.write_str(d),
            WebRule::Url { host, path } => write!(f, "{host}{path}"),
            WebRule::UrlContains(t) => write!(f, "contains:{t}"),
            WebRule::SearchTerm(t) => write!(f, "search:{t}"),
        }
    }
}

impl FromStr for WebRule {
    type Err = RuleParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        WebRule::parse(s)
    }
}

impl Serialize for WebRule {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for WebRule {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        WebRule::parse(&s).map_err(serde::de::Error::custom)
    }
}

/// A URL prepared for matching.
#[derive(Debug, Clone)]
pub struct UrlView {
    /// Lower-case host without trailing dot (punycode for IDNs).
    pub host: String,
    /// Lower-case path plus `?query`.
    pub path: String,
    /// Host + path, percent-decoded and [`normalize`]d, for "contains" rules.
    pub text: String,
    /// The normalised search query, if the URL looks like a search.
    pub search: Option<String>,
}

/// Query parameters that carry the search text on common search engines
/// (Google/Bing/DDG `q`, Yahoo `p`, Yandex `text`, Baidu `wd`/`word`,
/// YouTube `search_query`, Startpage `query`, ...).
const SEARCH_PARAMS: &[&str] = &[
    "q", "query", "search_query", "search", "text", "wd", "word", "p", "keyword", "keywords", "term",
];

impl UrlView {
    pub fn parse(input: &str) -> Option<Self> {
        let input = input.trim();
        let url = url::Url::parse(input)
            .ok()
            .filter(|u| u.has_host())
            .or_else(|| url::Url::parse(&format!("http://{input}")).ok())?;
        let host = clean_host(url.host_str()?);
        let mut path = url.path().to_lowercase();
        if let Some(q) = url.query() {
            path.push('?');
            path.push_str(&q.to_lowercase());
        }
        let text = normalize(&percent_decode(&format!("{host}{path}"), false));

        let mut search = String::new();
        for (key, value) in url.query_pairs() {
            if SEARCH_PARAMS.contains(&key.as_ref()) && !value.trim().is_empty() {
                if !search.is_empty() {
                    search.push(' ');
                }
                search.push_str(&value);
            }
        }
        let search = (!search.is_empty()).then(|| normalize(&search));

        Some(UrlView { host, path, text, search })
    }

    /// A view of a bare host name, as seen by DNS filtering.
    pub fn host_only(host: &str) -> Self {
        let host = clean_host(host);
        UrlView { text: host.clone(), host, path: String::new(), search: None }
    }
}

/// Cookie-consent pages that sites put in front of the page actually
/// requested. They are judged by where they lead, so a consent page on a
/// blocked domain does not hide an allowed page behind it.
const INTERSTITIALS: &[(&str, &str)] = &[("consent.youtube.com", "continue"), ("consent.google.com", "continue")];

/// For a consent page, the URL it continues to.
pub fn interstitial_target(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    let host = clean_host(parsed.host_str()?);
    let (_, param) = INTERSTITIALS.iter().find(|(h, _)| *h == host)?;
    let target = parsed.query_pairs().find(|(k, _)| k == param)?.1.into_owned();
    let target_url = url::Url::parse(&target).ok()?;
    matches!(target_url.scheme(), "http" | "https").then_some(target)
}

/// Iterates a host and its parent domains: `a.b.com`, `b.com`, `com`.
pub fn host_suffixes(host: &str) -> impl Iterator<Item = &str> {
    std::iter::once(host).chain(host.match_indices('.').map(move |(i, _)| &host[i + 1..]))
}

/// The host a search engine must be resolved to (DNS CNAME) to force
/// SafeSearch / Restricted Mode, if `host` is one of those engines.
pub fn safe_search_target(host: &str) -> Option<&'static str> {
    let host = clean_host(host);
    let bare = host.strip_prefix("www.").unwrap_or(&host);
    match bare {
        "bing.com" => Some("strict.bing.com"),
        "duckduckgo.com" | "start.duckduckgo.com" | "html.duckduckgo.com" | "lite.duckduckgo.com" => {
            Some("safe.duckduckgo.com")
        }
        "youtube.com" | "m.youtube.com" | "youtubei.googleapis.com" | "youtube.googleapis.com"
        | "youtube-nocookie.com" => Some("restrict.youtube.com"),
        _ if is_google_search_host(bare) => Some("forcesafesearch.google.com"),
        _ => None,
    }
}

/// `google.com`, `google.com.sa`, `google.co.uk`, ... (not `mail.google.com`).
fn is_google_search_host(bare: &str) -> bool {
    let Some(tld) = bare.strip_prefix("google.") else { return false };
    !tld.is_empty()
        && tld.len() <= 6
        && tld.split('.').all(|label| (2..=3).contains(&label.len()))
        && tld.chars().all(|c| c.is_ascii_lowercase() || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> WebRule {
        WebRule::parse(s).unwrap_or_else(|e| panic!("{s}: {e}"))
    }

    #[test]
    fn parses_domains() {
        assert_eq!(p("example.com"), WebRule::Domain("example.com".into()));
        assert_eq!(p("  WWW.Example.COM  "), WebRule::Domain("example.com".into()));
        assert_eq!(p("https://example.com/"), WebRule::Domain("example.com".into()));
        assert_eq!(p("*.example.com"), WebRule::Domain("example.com".into()));
        assert_eq!(p("||example.com^"), WebRule::Domain("example.com".into()));
        assert_eq!(p("example.com."), WebRule::Domain("example.com".into()));
        assert_eq!(p("10.0.0.1"), WebRule::Domain("10.0.0.1".into()));
    }

    #[test]
    fn parses_unicode_domains_to_punycode() {
        assert_eq!(p("مثال.إختبار"), WebRule::Domain("xn--mgbh0fb.xn--kgbechtv".into()));
    }

    #[test]
    fn parses_urls() {
        assert_eq!(
            p("youtube.com/Shorts"),
            WebRule::Url { host: "youtube.com".into(), path: "/shorts".into() }
        );
        assert_eq!(
            p("https://www.reddit.com/r/all?sort=new"),
            WebRule::Url { host: "reddit.com".into(), path: "/r/all?sort=new".into() }
        );
    }

    #[test]
    fn parses_prefixed_rules() {
        assert_eq!(p("contains:Casino"), WebRule::UrlContains("casino".into()));
        assert_eq!(p("*poker*"), WebRule::UrlContains("poker".into()));
        assert_eq!(p("search: Free   Movies"), WebRule::SearchTerm("free movies".into()));
        assert_eq!(p("keyword:أفلام"), WebRule::SearchTerm("افلام".into()));
    }

    #[test]
    fn rejects_bad_rules() {
        assert_eq!(WebRule::parse("   "), Err(RuleParseError::Empty));
        assert_eq!(WebRule::parse("contains:  "), Err(RuleParseError::Empty));
        assert!(matches!(WebRule::parse("facebook"), Err(RuleParseError::NotADomain(_))));
        assert!(matches!(WebRule::parse("exa mple.com"), Err(RuleParseError::InvalidAddress(_))));
        assert!(matches!(WebRule::parse("foo:bar"), Err(RuleParseError::UnknownPrefix(_))));
    }

    #[test]
    fn display_round_trips() {
        for s in ["example.com", "youtube.com/shorts", "contains:casino", "search:free movies"] {
            assert_eq!(p(s).to_string(), s);
            assert_eq!(p(&p(s).to_string()), p(s));
        }
    }

    #[test]
    fn serde_uses_strings() {
        let rules = vec![p("example.com"), p("search:x")];
        let json = serde_json::to_string(&rules).unwrap();
        assert_eq!(json, r#"["example.com","search:x"]"#);
        assert_eq!(serde_json::from_str::<Vec<WebRule>>(&json).unwrap(), rules);
    }

    #[test]
    fn url_view_extracts_search() {
        let v = UrlView::parse("https://www.google.com/search?q=Free+Movies&hl=en").unwrap();
        assert_eq!(v.host, "www.google.com");
        assert_eq!(v.search.as_deref(), Some("free movies"));

        let v = UrlView::parse("https://www.youtube.com/results?search_query=%D8%A3%D9%81%D9%84%D8%A7%D9%85").unwrap();
        assert_eq!(v.search.as_deref(), Some("افلام"));

        let v = UrlView::parse("https://search.yahoo.com/search?p=poker").unwrap();
        assert_eq!(v.search.as_deref(), Some("poker"));

        let v = UrlView::parse("example.com/page").unwrap();
        assert_eq!(v.host, "example.com");
        assert_eq!(v.search, None);
    }

    #[test]
    fn url_view_text_is_decoded() {
        let v = UrlView::parse("https://Example.com/Watch%20Free%20Movies").unwrap();
        assert_eq!(v.text, "example.com/watch free movies");
    }

    #[test]
    fn consent_pages_lead_to_their_target() {
        let u = "https://consent.youtube.com/m?continue=https%3A%2F%2Fwww.youtube.com%2F%40university%3Fcbrd%3D1&gl=NO";
        assert_eq!(interstitial_target(u).as_deref(), Some("https://www.youtube.com/@university?cbrd=1"));
        assert_eq!(interstitial_target("https://consent.youtube.com/m?continue=javascript:alert(1)"), None);
        assert_eq!(interstitial_target("https://www.youtube.com/?continue=https://x.com"), None);
    }

    #[test]
    fn suffixes() {
        let s: Vec<_> = host_suffixes("a.b.example.com").collect();
        assert_eq!(s, ["a.b.example.com", "b.example.com", "example.com", "com"]);
    }

    #[test]
    fn safe_search_hosts() {
        assert_eq!(safe_search_target("www.google.com"), Some("forcesafesearch.google.com"));
        assert_eq!(safe_search_target("google.com.sa"), Some("forcesafesearch.google.com"));
        assert_eq!(safe_search_target("www.google.co.uk"), Some("forcesafesearch.google.com"));
        assert_eq!(safe_search_target("mail.google.com"), None);
        assert_eq!(safe_search_target("www.bing.com"), Some("strict.bing.com"));
        assert_eq!(safe_search_target("duckduckgo.com."), Some("safe.duckduckgo.com"));
        assert_eq!(safe_search_target("m.youtube.com"), Some("restrict.youtube.com"));
        assert_eq!(safe_search_target("example.com"), None);
    }
}
