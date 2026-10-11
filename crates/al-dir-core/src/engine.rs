//! The compiled rule engine. Build an [`Engine`] once whenever the config
//! changes, then query it on every navigation, DNS lookup, process start
//! and window title change.
//!
//! Lookups are designed to be cheap: domains are found with one hash
//! lookup per label, and all "contains" rules of a set are searched in a
//! single Aho-Corasick pass.

use std::collections::{HashMap, HashSet};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};

use crate::apps::{exe_name, path_matches, AppRule, ProcessInfo};
use crate::model::{Allowance, BlockSet, Config, ListMode, SetId, UserScope};
use crate::partial::{self, PartialRule};
use crate::schedule::{Now, Schedule};
use crate::text::normalize;
use crate::web::{host_suffixes, UrlView, WebRule};

/// Supplies the rules of pre-made lists by id.
pub trait ListSource {
    fn list(&self, id: &str) -> Option<&[WebRule]>;
}

/// For callers that do not use pre-made lists.
pub struct NoLists;

impl ListSource for NoLists {
    fn list(&self, _: &str) -> Option<&[WebRule]> {
        None
    }
}

impl ListSource for HashMap<String, Vec<WebRule>> {
    fn list(&self, id: &str) -> Option<&[WebRule]> {
        self.get(id).map(Vec::as_slice)
    }
}

/// Who is asking, and when.
#[derive(Debug, Clone, Copy)]
pub struct Ctx<'a> {
    pub now: Now,
    pub user: &'a str,
    /// Seconds of allowance used today, per set.
    pub usage: Option<&'a HashMap<SetId, u32>>,
}

impl<'a> Ctx<'a> {
    pub fn new(now: Now, user: &'a str) -> Self {
        Self { now, user, usage: None }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reason {
    Domain(String),
    Url(String),
    UrlContains(String),
    SearchTerm(String),
    Partial(&'static str),
    /// Allow-only mode and nothing on the allow list matched.
    NotAllowed,
    AppName(String),
    AppPath(String),
    AppHash,
    CmdLine(String),
    WindowTitle(String),
}

impl std::fmt::Display for Reason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Reason::Domain(d) => write!(f, "website {d}"),
            Reason::Url(u) => write!(f, "page {u}"),
            Reason::UrlContains(t) => write!(f, "address contains \"{t}\""),
            Reason::SearchTerm(t) => write!(f, "search contains \"{t}\""),
            Reason::Partial(id) => write!(f, "partial block {id}"),
            Reason::NotAllowed => f.write_str("not on the allow list"),
            Reason::AppName(n) => write!(f, "app {n}"),
            Reason::AppPath(p) => write!(f, "app path {p}"),
            Reason::AppHash => f.write_str("app file fingerprint"),
            Reason::CmdLine(t) => write!(f, "command line contains \"{t}\""),
            Reason::WindowTitle(t) => write!(f, "window title contains \"{t}\""),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub set: SetId,
    pub reason: Reason,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Decision {
    pub block: Option<Hit>,
    /// Sets whose allowance is being consumed by this (allowed) activity.
    pub metered: Vec<SetId>,
}

impl Decision {
    pub fn is_blocked(&self) -> bool {
        self.block.is_some()
    }
}

pub struct Engine {
    sets: Vec<CompiledSet>,
    wants_hash: bool,
}

struct CompiledSet {
    id: SetId,
    enabled: bool,
    users: UserScope,
    schedule: Option<Schedule>,
    allowance: Option<Allowance>,
    web_mode: ListMode,
    web_block: WebMatcher,
    web_allow: WebMatcher,
    partial: Vec<&'static PartialRule>,
    safe_search: bool,
    app_mode: ListMode,
    app_block: AppMatcher,
    app_allow: AppMatcher,
}

impl Engine {
    pub fn new(config: &Config, lists: &dyn ListSource) -> Self {
        let sets: Vec<CompiledSet> = config.block_sets.iter().map(|s| CompiledSet::new(s, lists)).collect();
        let wants_hash = sets.iter().any(|s| !s.app_block.hashes.is_empty() || !s.app_allow.hashes.is_empty());
        Self { sets, wants_hash }
    }

    /// Whether process checks need [`ProcessInfo::sha256`].
    pub fn wants_exe_hash(&self) -> bool {
        self.wants_hash
    }

    /// Checks a full URL (from the browser extension).
    pub fn check_url(&self, url: &str, ctx: &Ctx) -> Decision {
        match UrlView::parse(url) {
            Some(view) => self.check_view(&view, ctx),
            None => Decision::default(),
        }
    }

    pub fn check_view(&self, view: &UrlView, ctx: &Ctx) -> Decision {
        self.decide(ctx, |set| set.web_verdict(view))
    }

    /// Checks a bare host name (DNS level, any application).
    pub fn check_host(&self, host: &str, ctx: &Ctx) -> Decision {
        let view = UrlView::host_only(host);
        self.decide(ctx, |set| set.host_verdict(&view))
    }

    /// Checks a process that is starting.
    pub fn check_process(&self, process: &ProcessInfo, ctx: &Ctx) -> Decision {
        self.decide(ctx, |set| set.process_verdict(process))
    }

    /// Checks a window title (plus the process that owns the window).
    pub fn check_window(&self, title: &str, process: &ProcessInfo, ctx: &Ctx) -> Decision {
        let title = normalize(title);
        self.decide(ctx, |set| set.window_verdict(&title, process))
    }

    /// Whether SafeSearch must be forced for this user right now.
    pub fn safe_search(&self, ctx: &Ctx) -> bool {
        self.sets.iter().any(|s| s.safe_search && s.is_active(ctx))
    }

    /// CSS selectors the extension should hide on pages of `host`.
    pub fn hidden_selectors(&self, host: &str, ctx: &Ctx) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = Vec::new();
        for set in self.sets.iter().filter(|s| s.is_active(ctx)) {
            for rule in set.partial.iter().filter(|r| r.applies_to_host(host)) {
                for sel in rule.hide {
                    if !out.contains(sel) {
                        out.push(sel);
                    }
                }
            }
        }
        out
    }

    /// Ids of the sets currently in force for this user.
    pub fn active_sets<'s>(&'s self, ctx: &'s Ctx) -> impl Iterator<Item = SetId> + 's {
        self.sets.iter().filter(|s| s.is_active(ctx)).map(|s| s.id)
    }

    fn decide(&self, ctx: &Ctx, verdict: impl Fn(&CompiledSet) -> Option<Reason>) -> Decision {
        let mut decision = Decision::default();
        for set in self.sets.iter().filter(|s| s.is_active(ctx)) {
            let Some(reason) = verdict(set) else { continue };
            if let Some(allowance) = set.allowance {
                let used = ctx.usage.and_then(|u| u.get(&set.id)).copied().unwrap_or(0);
                if used < allowance.daily_secs {
                    decision.metered.push(set.id);
                    continue;
                }
            }
            decision.block = Some(Hit { set: set.id, reason });
            decision.metered.clear();
            break;
        }
        decision
    }
}

impl CompiledSet {
    fn new(set: &BlockSet, lists: &dyn ListSource) -> Self {
        let mut web_block = WebMatcher::default();
        let mut web_allow = WebMatcher::default();
        let list_rules = set.web.lists.iter().filter_map(|id| lists.list(id)).flatten();
        web_block.build(set.web.block.iter().chain(list_rules));
        web_allow.build(set.web.allow.iter());
        Self {
            id: set.id,
            enabled: set.enabled,
            users: set.users.clone(),
            schedule: set.schedule.clone(),
            allowance: set.allowance,
            web_mode: set.web.mode,
            web_block,
            web_allow,
            partial: set.web.partial.iter().filter_map(|id| partial::find(id)).collect(),
            safe_search: set.web.safe_search,
            app_mode: set.apps.mode,
            app_block: AppMatcher::new(&set.apps.block),
            app_allow: AppMatcher::new(&set.apps.allow),
        }
    }

    fn is_active(&self, ctx: &Ctx) -> bool {
        self.enabled
            && self.users.includes(ctx.user)
            && self.schedule.as_ref().is_none_or(|s| s.is_active(&ctx.now))
    }

    fn web_verdict(&self, view: &UrlView) -> Option<Reason> {
        let blocked = || {
            self.web_block.matches(view).or_else(|| {
                self.partial
                    .iter()
                    .find(|r| r.applies_to_host(&view.host) && r.blocks_path(&view.path))
                    .map(|r| Reason::Partial(r.id))
            })
        };
        match self.web_mode {
            ListMode::Block => blocked().filter(|_| self.web_allow.matches(view).is_none()),
            ListMode::AllowOnly => match self.web_allow.matches(view) {
                None => Some(Reason::NotAllowed),
                Some(_) => blocked(),
            },
        }
    }

    /// DNS-level verdict. A host is only blocked if no allow rule could
    /// apply to any page on it, so URL-level exceptions keep working (the
    /// extension then decides per page).
    fn host_verdict(&self, view: &UrlView) -> Option<Reason> {
        let allowed = self.web_allow.allows_host(&view.host);
        match self.web_mode {
            ListMode::Block => self.web_block.matches(view).filter(|_| !allowed),
            ListMode::AllowOnly if !allowed => Some(Reason::NotAllowed),
            ListMode::AllowOnly => self.web_block.matches(view),
        }
    }

    fn process_verdict(&self, p: &ProcessInfo) -> Option<Reason> {
        match self.app_mode {
            ListMode::Block => self.app_block.matches_process(p).filter(|_| self.app_allow.matches_process(p).is_none()),
            ListMode::AllowOnly => match self.app_allow.matches_process(p) {
                None => Some(Reason::NotAllowed),
                Some(_) => self.app_block.matches_process(p),
            },
        }
    }

    fn window_verdict(&self, title: &str, p: &ProcessInfo) -> Option<Reason> {
        let hit = self.app_block.matches_title(title)?;
        let excepted = self.app_mode == ListMode::Block
            && (self.app_allow.matches_title(title).is_some() || self.app_allow.matches_process(p).is_some());
        (!excepted).then_some(hit)
    }
}

/// Multi-pattern substring search returning the matched pattern.
struct Patterns {
    ac: AhoCorasick,
    patterns: Vec<String>,
}

impl Patterns {
    fn new(mut patterns: Vec<String>) -> Option<Self> {
        patterns.sort_unstable();
        patterns.dedup();
        if patterns.is_empty() {
            return None;
        }
        let ac = AhoCorasickBuilder::new()
            .match_kind(MatchKind::LeftmostFirst)
            .build(&patterns)
            .expect("patterns are plain strings");
        Some(Self { ac, patterns })
    }

    fn find(&self, haystack: &str) -> Option<&str> {
        self.ac.find(haystack).map(|m| self.patterns[m.pattern().as_usize()].as_str())
    }
}

fn find_in(p: &Option<Patterns>, haystack: &str) -> Option<String> {
    p.as_ref()?.find(haystack).map(str::to_string)
}

#[derive(Default)]
struct WebMatcher {
    domains: HashSet<Box<str>>,
    /// host -> path prefixes
    urls: HashMap<Box<str>, Vec<Box<str>>>,
    contains: Option<Patterns>,
    search: Option<Patterns>,
}

impl WebMatcher {
    fn build<'r>(&mut self, rules: impl Iterator<Item = &'r WebRule>) {
        let mut contains = Vec::new();
        let mut search = Vec::new();
        for rule in rules {
            match rule {
                WebRule::Domain(d) => {
                    self.domains.insert(d.as_str().into());
                }
                WebRule::Url { host, path } => {
                    self.urls.entry(host.as_str().into()).or_default().push(path.as_str().into());
                }
                WebRule::UrlContains(t) => contains.push(t.clone()),
                WebRule::SearchTerm(t) => search.push(t.clone()),
            }
        }
        self.contains = Patterns::new(contains);
        self.search = Patterns::new(search);
    }

    fn matches(&self, view: &UrlView) -> Option<Reason> {
        for suffix in host_suffixes(&view.host) {
            if self.domains.contains(suffix) {
                return Some(Reason::Domain(suffix.to_string()));
            }
            if !view.path.is_empty()
                && let Some(paths) = self.urls.get(suffix)
                && let Some(p) = paths.iter().find(|p| view.path.starts_with(&***p))
            {
                return Some(Reason::Url(format!("{suffix}{p}")));
            }
        }
        if let Some(t) = find_in(&self.contains, &view.text) {
            return Some(Reason::UrlContains(t));
        }
        if let Some(q) = &view.search
            && let Some(t) = find_in(&self.search, q)
        {
            return Some(Reason::SearchTerm(t));
        }
        None
    }

    /// Could any page on this host be allowed by these rules?
    fn allows_host(&self, host: &str) -> bool {
        host_suffixes(host).any(|s| self.domains.contains(s) || self.urls.contains_key(s))
    }
}

#[derive(Default)]
struct AppMatcher {
    names: HashSet<String>,
    paths: Vec<String>,
    hashes: HashSet<String>,
    cmd: Option<Patterns>,
    titles: Option<Patterns>,
}

impl AppMatcher {
    fn new(rules: &[AppRule]) -> Self {
        let mut m = AppMatcher::default();
        let mut cmd = Vec::new();
        let mut titles = Vec::new();
        for rule in rules {
            match rule {
                AppRule::Name(n) => {
                    m.names.insert(n.clone());
                }
                AppRule::Path(p) => m.paths.push(p.clone()),
                AppRule::Hash(h) => {
                    m.hashes.insert(h.clone());
                }
                AppRule::CmdLine(c) => cmd.push(c.clone()),
                AppRule::WindowTitle(t) => titles.push(t.clone()),
            }
        }
        m.cmd = Patterns::new(cmd);
        m.titles = Patterns::new(titles);
        m
    }

    fn matches_process(&self, p: &ProcessInfo) -> Option<Reason> {
        if !self.names.is_empty() {
            let name = exe_name(p.exe_path);
            if self.names.contains(&name) {
                return Some(Reason::AppName(name));
            }
        }
        if let Some(rule) = self.paths.iter().find(|r| path_matches(r, p.exe_path)) {
            return Some(Reason::AppPath(rule.clone()));
        }
        if let Some(h) = p.sha256
            && self.hashes.contains(&h.to_ascii_lowercase())
        {
            return Some(Reason::AppHash);
        }
        if let Some(cmdline) = p.cmdline
            && self.cmd.is_some()
            && let Some(t) = find_in(&self.cmd, &normalize(cmdline))
        {
            return Some(Reason::CmdLine(t));
        }
        None
    }

    fn matches_title(&self, normalized_title: &str) -> Option<Reason> {
        find_in(&self.titles, normalized_title).map(Reason::WindowTitle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Allowance, BlockSet};
    use crate::schedule::{Day, WeeklyRange};
    use jiff::civil::date;

    fn rules(list: &[&str]) -> Vec<WebRule> {
        list.iter().map(|r| WebRule::parse(r).unwrap()).collect()
    }

    fn apps(list: &[&str]) -> Vec<AppRule> {
        list.iter().map(|r| AppRule::parse(r).unwrap()).collect()
    }

    fn engine(sets: Vec<BlockSet>) -> Engine {
        Engine::new(&Config { block_sets: sets, ..Config::default() }, &NoLists)
    }

    // Monday 2026-10-12 10:00.
    fn now() -> Now {
        Now { unix: 0, local: date(2026, 10, 12).at(10, 0, 0, 0) }
    }

    fn ctx() -> Ctx<'static> {
        Ctx::new(now(), "alice")
    }

    fn blocked(e: &Engine, url: &str) -> bool {
        e.check_url(url, &ctx()).is_blocked()
    }

    #[test]
    fn domain_rules_cover_subdomains() {
        let mut s = BlockSet::new("social");
        s.web.block = rules(&["facebook.com"]);
        let e = engine(vec![s]);
        assert!(blocked(&e, "https://facebook.com/"));
        assert!(blocked(&e, "https://www.facebook.com/profile"));
        assert!(blocked(&e, "https://m.facebook.com"));
        assert!(!blocked(&e, "https://notfacebook.com"));
        assert!(!blocked(&e, "https://facebook.com.example.org"));
    }

    #[test]
    fn url_contains_and_search() {
        let mut s = BlockSet::new("words");
        s.web.block = rules(&["contains:casino", "search:free movies", "search:أفلام"]);
        let e = engine(vec![s]);
        assert!(blocked(&e, "https://best-CASINO.example/"));
        assert!(blocked(&e, "https://example.com/go/Casino%20Royale"));
        assert!(blocked(&e, "https://www.google.com/search?q=watch+free+movies+online"));
        assert!(blocked(&e, "https://www.bing.com/search?q=%D8%A3%D9%81%D9%84%D8%A7%D9%85"));
        assert!(!blocked(&e, "https://www.google.com/search?q=free+books"));
        let d = e.check_url("https://duckduckgo.com/?q=free%20movies", &ctx());
        assert_eq!(d.block.unwrap().reason, Reason::SearchTerm("free movies".into()));
    }

    #[test]
    fn url_prefix_and_exceptions() {
        let mut s = BlockSet::new("yt");
        s.web.block = rules(&["youtube.com"]);
        s.web.allow = rules(&["youtube.com/watch?v=lecture1", "youtube.com/@university"]);
        let e = engine(vec![s]);
        assert!(blocked(&e, "https://www.youtube.com/"));
        assert!(blocked(&e, "https://www.youtube.com/watch?v=cats"));
        assert!(!blocked(&e, "https://www.youtube.com/watch?v=lecture1"));
        assert!(!blocked(&e, "https://www.youtube.com/@university/videos"));
        // DNS must let youtube.com resolve so the exceptions can work.
        assert!(!e.check_host("www.youtube.com", &ctx()).is_blocked());
    }

    #[test]
    fn dns_level() {
        let mut s = BlockSet::new("dns");
        s.web.block = rules(&["reddit.com", "youtube.com/shorts", "contains:casino", "search:x"]);
        let e = engine(vec![s]);
        assert!(e.check_host("old.reddit.com.", &ctx()).is_blocked());
        assert!(!e.check_host("youtube.com", &ctx()).is_blocked(), "URL rules are not domain level");
        assert!(e.check_host("casino-royale.net", &ctx()).is_blocked());
        assert!(!e.check_host("example.com", &ctx()).is_blocked());
    }

    #[test]
    fn allow_only_mode() {
        let mut s = BlockSet::new("work");
        s.web.mode = ListMode::AllowOnly;
        s.web.allow = rules(&["wikipedia.org", "github.com/rust-lang"]);
        s.web.block = rules(&["en.wikipedia.org/wiki/special:random"]);
        let e = engine(vec![s]);
        assert!(!blocked(&e, "https://ar.wikipedia.org/wiki/x"));
        assert!(blocked(&e, "https://en.wikipedia.org/wiki/Special:Random"));
        assert!(!blocked(&e, "https://github.com/rust-lang/rust"));
        assert!(blocked(&e, "https://github.com/torvalds"));
        assert!(blocked(&e, "https://example.com"));
        let d = e.check_host("example.com", &ctx());
        assert_eq!(d.block.unwrap().reason, Reason::NotAllowed);
        assert!(!e.check_host("github.com", &ctx()).is_blocked());
    }

    #[test]
    fn partial_blocking() {
        let mut s = BlockSet::new("partial");
        s.web.partial = vec!["youtube-shorts".into(), "youtube-comments".into(), "nonexistent".into()];
        let e = engine(vec![s]);
        assert!(blocked(&e, "https://www.youtube.com/shorts/abc"));
        assert!(!blocked(&e, "https://www.youtube.com/watch?v=abc"));
        let sel = e.hidden_selectors("www.youtube.com", &ctx());
        assert!(sel.contains(&"ytd-comments#comments"));
        assert!(sel.contains(&"ytd-reel-shelf-renderer"));
        assert!(e.hidden_selectors("example.com", &ctx()).is_empty());
    }

    #[test]
    fn users_and_schedule() {
        let mut kids = BlockSet::new("kids");
        kids.users = UserScope::Only(vec!["bob".into()]);
        kids.web.block = rules(&["games.com"]);
        let mut work = BlockSet::new("work hours");
        work.schedule = Some(Schedule {
            weekly: vec![WeeklyRange { days: vec![Day::Mon], start: 9 * 60, end: 17 * 60 }],
            dates: vec![],
        });
        work.web.block = rules(&["news.com"]);
        let e = engine(vec![kids, work]);

        assert!(!blocked(&e, "games.com"), "alice is not in scope");
        assert!(e.check_url("games.com", &Ctx::new(now(), "Bob")).is_blocked());
        assert!(blocked(&e, "news.com"), "Monday 10:00 is in work hours");
        let evening = Now { unix: 0, local: date(2026, 10, 12).at(20, 0, 0, 0) };
        assert!(!e.check_url("news.com", &Ctx::new(evening, "alice")).is_blocked());
    }

    #[test]
    fn disabled_sets_do_nothing() {
        let mut s = BlockSet::new("off");
        s.enabled = false;
        s.web.block = rules(&["example.com"]);
        assert!(!blocked(&engine(vec![s]), "example.com"));
    }

    #[test]
    fn allowance_meters_then_blocks() {
        let mut s = BlockSet::new("yt budget");
        s.web.block = rules(&["youtube.com"]);
        s.allowance = Some(Allowance { daily_secs: 1800 });
        let id = s.id;
        let e = engine(vec![s]);

        let mut usage = HashMap::new();
        usage.insert(id, 600);
        let c = Ctx { usage: Some(&usage), ..ctx() };
        let d = e.check_url("youtube.com", &c);
        assert!(!d.is_blocked());
        assert_eq!(d.metered, vec![id]);

        usage.insert(id, 1800);
        let c = Ctx { usage: Some(&usage), ..ctx() };
        assert!(e.check_url("youtube.com", &c).is_blocked());
        assert!(e.check_url("example.com", &c).metered.is_empty());
    }

    #[test]
    fn pre_made_lists() {
        let mut lists = HashMap::new();
        lists.insert("social".to_string(), rules(&["facebook.com", "tiktok.com"]));
        let mut s = BlockSet::new("social");
        s.web.lists = vec!["social".into(), "missing".into()];
        let e = Engine::new(&Config { block_sets: vec![s], ..Config::default() }, &lists);
        assert!(e.check_url("https://www.tiktok.com/@x", &ctx()).is_blocked());
    }

    #[test]
    fn safe_search_flag() {
        let mut s = BlockSet::new("safe");
        s.web.safe_search = true;
        s.users = UserScope::AllExcept(vec!["admin".into()]);
        let e = engine(vec![s]);
        assert!(e.safe_search(&ctx()));
        assert!(!e.safe_search(&Ctx::new(now(), "admin")));
    }

    fn proc(exe: &str) -> ProcessInfo<'_> {
        ProcessInfo { exe_path: exe, ..Default::default() }
    }

    #[test]
    fn app_rules() {
        let mut s = BlockSet::new("games");
        s.apps.block = apps(&["steam", "path:/opt/games/", "cmd:minecraft", &format!("hash:{}", "a".repeat(64))]);
        s.apps.allow = apps(&["path:/opt/games/chess"]);
        let e = engine(vec![s]);
        assert!(e.wants_exe_hash());
        let c = ctx();
        assert!(e.check_process(&proc("/usr/games/steam"), &c).is_blocked());
        assert!(e.check_process(&proc(r"C:\Steam\Steam.exe"), &c).is_blocked());
        assert!(e.check_process(&proc("/opt/games/doom"), &c).is_blocked());
        assert!(!e.check_process(&proc("/opt/games/chess"), &c).is_blocked(), "exception");
        assert!(!e.check_process(&proc("/usr/bin/firefox"), &c).is_blocked());

        let java = ProcessInfo { exe_path: "/usr/bin/java", cmdline: Some("java -jar Minecraft.jar"), sha256: None };
        assert!(e.check_process(&java, &c).is_blocked());

        let renamed = ProcessInfo { exe_path: "/tmp/notes", cmdline: None, sha256: Some(&"A".repeat(64)) };
        assert_eq!(e.check_process(&renamed, &c).block.unwrap().reason, Reason::AppHash);
    }

    #[test]
    fn app_allow_only() {
        let mut s = BlockSet::new("focus");
        s.apps.mode = ListMode::AllowOnly;
        s.apps.allow = apps(&["code", "firefox"]);
        let e = engine(vec![s]);
        assert!(!e.check_process(&proc("/usr/bin/firefox"), &ctx()).is_blocked());
        assert!(e.check_process(&proc("/usr/bin/steam"), &ctx()).is_blocked());
        assert!(!e.wants_exe_hash());
    }

    #[test]
    fn window_titles() {
        let mut s = BlockSet::new("titles");
        s.apps.block = apps(&["title:YouTube", "title:يوتيوب"]);
        s.apps.allow = apps(&["title:YouTube Studio"]);
        let e = engine(vec![s]);
        let ff = proc("/usr/bin/firefox");
        assert!(e.check_window("Cats - YouTube — Mozilla Firefox", &ff, &ctx()).is_blocked());
        assert!(e.check_window("قناة يوتيوب", &ff, &ctx()).is_blocked());
        assert!(!e.check_window("Channel content - YouTube Studio", &ff, &ctx()).is_blocked());
        assert!(!e.check_window("Inbox - Mail", &ff, &ctx()).is_blocked());
    }

    #[test]
    fn first_blocking_set_wins_over_metered() {
        let mut metered = BlockSet::new("budget");
        metered.web.block = rules(&["youtube.com"]);
        metered.allowance = Some(Allowance { daily_secs: 3600 });
        let mut hard = BlockSet::new("hard");
        hard.web.block = rules(&["youtube.com/shorts"]);
        let hard_id = hard.id;
        let e = engine(vec![metered, hard]);
        let d = e.check_url("youtube.com/shorts/x", &ctx());
        assert_eq!(d.block.unwrap().set, hard_id);
        assert!(d.metered.is_empty());
    }
}
