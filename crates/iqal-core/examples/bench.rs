//! Rough lookup benchmark: `cargo run --release --example bench -p iqal-core`

use std::time::Instant;

use iqal_core::{AppRule, BlockSet, Config, Ctx, Engine, NoLists, Now, ProcessInfo, WebRule};

fn main() {
    const DOMAINS: usize = 200_000;
    const KEYWORDS: usize = 2_000;

    let mut set = BlockSet::new("big");
    set.web.block = (0..DOMAINS)
        .map(|i| WebRule::Domain(format!("site{i}.example{}.com", i % 97)))
        .chain((0..KEYWORDS).map(|i| WebRule::UrlContains(format!("keyword{i}x"))))
        .chain((0..KEYWORDS).map(|i| WebRule::SearchTerm(format!("search term {i}"))))
        .collect();
    set.apps.block = (0..1_000).map(|i| AppRule::Name(format!("game{i}"))).collect();
    set.apps.block.push(AppRule::parse("title:youtube").unwrap());
    let config = Config { block_sets: vec![set], ..Config::default() };

    let t = Instant::now();
    let engine = Engine::new(&config, &NoLists);
    println!("compile {DOMAINS} domains + {} keywords: {:?}", KEYWORDS * 2, t.elapsed());

    let ctx = Ctx::new(Now::system(), "user");
    let urls = [
        "https://www.google.com/search?q=rust+programming+language",
        "https://a.b.c.site123.example26.com/path",
        "https://docs.rs/aho-corasick/latest/aho_corasick/struct.AhoCorasick.html",
        "https://example.org/some/long/path/with/keyword1999x/inside?x=1",
    ];
    let n = 200_000;
    let t = Instant::now();
    let mut blocked = 0;
    for i in 0..n {
        blocked += engine.check_url(urls[i % urls.len()], &ctx).is_blocked() as usize;
    }
    let per = t.elapsed() / n as u32;
    println!("check_url: {per:?} per lookup ({blocked}/{n} blocked)");

    let hosts = ["www.google.com", "site5.example5.com", "cdn.jsdelivr.net", "a.b.c.d.e.f.example.org"];
    let t = Instant::now();
    for i in 0..n {
        blocked += engine.check_host(hosts[i % hosts.len()], &ctx).is_blocked() as usize;
    }
    println!("check_host: {:?} per lookup", t.elapsed() / n as u32);

    let p = ProcessInfo { exe_path: "/usr/bin/firefox", cmdline: Some("firefox --new-window"), sha256: None };
    let t = Instant::now();
    for _ in 0..n {
        blocked += engine.check_process(&p, &ctx).is_blocked() as usize;
    }
    println!("check_process: {:?} per lookup", t.elapsed() / n as u32);

    let t = Instant::now();
    for _ in 0..n {
        blocked += engine.check_window("Some page - Mozilla Firefox", &p, &ctx).is_blocked() as usize;
    }
    println!("check_window: {:?} per lookup", t.elapsed() / n as u32);
    std::hint::black_box(blocked);
}
