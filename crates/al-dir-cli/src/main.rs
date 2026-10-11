//! al-dir-cli: command-line control for the Al-Dir service.

mod parse;

use std::io::{BufRead, Write};
use std::path::PathBuf;

use al_dir_core::import::parse_rules_text;
use al_dir_core::lock::random_text;
use al_dir_core::schedule::DateRange;
use al_dir_core::{
    partial, Allowance, AppRule, BlockSet, ListMode, Lock, LockKind, PasswordHash, Schedule, SetId, UserScope,
    WebRule,
};
use al_dir_ipc::client::Client;
use al_dir_ipc::{ChallengeInfo, CheckQuery, DecisionInfo, EventKind, Request, Response, SetInfo, UnlockInfo};
use clap::{Parser, Subcommand, ValueEnum};

#[derive(Parser)]
#[command(name = "al-dir-cli", version, about = "Control the Al-Dir blocker")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Show the service status.
    Status,
    /// List all blocks.
    List,
    /// Show the details of a block.
    Show { block: String },
    /// Create a new, empty block.
    Create { name: String },
    /// Delete a block (not possible while it is locked).
    Delete { block: String },
    /// Rename a block.
    Rename { block: String, name: String },
    /// Turn a block on.
    Enable { block: String },
    /// Turn a block off (not possible while it is locked).
    Disable { block: String },
    /// Add website rules (or app rules with --app).
    ///
    /// Website rules: example.com, example.com/path, contains:TEXT, search:TEXT.
    /// App rules: NAME, /path/to/exe, /folder/, hash:SHA256, cmd:TEXT, title:TEXT.
    Add {
        block: String,
        #[arg(required = true)]
        rules: Vec<String>,
        /// Add to the allow list / exceptions instead of the block list.
        #[arg(long)]
        allow: bool,
        /// App rules instead of website rules.
        #[arg(long)]
        app: bool,
    },
    /// Remove rules.
    Remove {
        block: String,
        #[arg(required = true)]
        rules: Vec<String>,
        #[arg(long)]
        allow: bool,
        #[arg(long)]
        app: bool,
    },
    /// Add website rules from a text file (one per line; hosts and Adblock files work too).
    ImportText {
        block: String,
        file: PathBuf,
        #[arg(long)]
        allow: bool,
    },
    /// Block-list mode, or allow-only mode (block everything except the allow list).
    Mode { block: String, target: Target, mode: ModeArg },
    /// Which users the block applies to: all | except NAMES... | only NAMES...
    Users { block: String, scope: ScopeArg, names: Vec<String> },
    /// Force SafeSearch on Google, Bing, DuckDuckGo and YouTube.
    SafeSearch { block: String, state: OnOff },
    /// Add or remove a pre-made list.
    Lists { block: String, action: AddRemove, id: String },
    /// Add or remove a partial block; without arguments, show the available ones.
    Partial { block: Option<String>, action: Option<AddRemove>, id: Option<String> },
    /// When the block is in force (default: always).
    Schedule {
        block: String,
        #[command(subcommand)]
        action: ScheduleCmd,
    },
    /// Daily time allowance in minutes, or "off".
    Allowance { block: String, minutes: String },
    /// Lock a block so it cannot be weakened.
    Lock {
        block: String,
        /// Delay between solving an unlock challenge and the lock opening (e.g. 20m).
        #[arg(long)]
        delay: Option<String>,
        #[command(subcommand)]
        kind: LockCmd,
    },
    /// "I am at this place now": remember the Wi-Fi networks in range for the
    /// block's location lock (only ever adds; repeat to learn more).
    LearnLocation { block: String },
    /// Open a password or random-text lock.
    Unlock { block: String },
    /// Cancel a pending (delayed) unlock.
    CancelUnlock { block: String },
    /// Export blocks to a file (all blocks if none are named).
    Export {
        blocks: Vec<String>,
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    /// Import blocks from an export file.
    Import { file: PathBuf },
    /// Hardened mode: while anything is locked, take administrator rights
    /// (sudo, password prompts) from the people being blocked.
    Hardened { state: OnOff },
    /// Show what Al-Dir blocked recently.
    Events,
    /// Pre-made lists (use with: al-dir-cli lists <block> add <id>).
    Catalog,
    /// Statistics for the last days.
    Stats {
        #[arg(long, default_value_t = 7)]
        days: u32,
    },
    /// Test what Al-Dir would decide.
    Check {
        /// Check as another user (administrators only).
        #[arg(long, global = true)]
        user: Option<String>,
        #[command(subcommand)]
        what: CheckCmd,
    },
}

#[derive(Subcommand)]
enum ScheduleCmd {
    /// Add a weekly window, e.g. "mon-fri 09:00-17:00" or "daily 22:00-06:00".
    Add { spec: Vec<String> },
    /// Add a one-off window, e.g. "2026-10-20 08:00" "2026-10-22 18:00".
    AddDates { start: String, end: String },
    /// Remove the schedule (block always in force).
    Clear,
}

#[derive(Subcommand)]
enum LockCmd {
    /// Locked for a duration, e.g. 2h30m, 3d, 1w.
    For { duration: String },
    /// Locked until a date/time, e.g. "2026-10-20 18:00".
    Until { datetime: Vec<String> },
    /// Locked until a password is entered.
    Password {
        /// Generate a password for someone else to keep; it is shown once.
        #[arg(long)]
        generate: bool,
    },
    /// Locked until a random text of this length is typed.
    Random {
        #[arg(default_value_t = 200)]
        length: u16,
    },
    /// Locked until the computer restarts.
    Restart,
    /// Locked during a weekly window, e.g. "mon-fri 09:00-17:00".
    Schedule { spec: Vec<String> },
    /// Locked while this computer is inside the area.
    Location {
        #[arg(allow_negative_numbers = true)]
        lat: f64,
        #[arg(allow_negative_numbers = true)]
        lon: f64,
        /// Radius in metres.
        radius: f64,
    },
}

#[derive(Subcommand)]
enum CheckCmd {
    Url { url: String },
    Host { host: String },
    App {
        path: String,
        #[arg(long)]
        cmdline: Option<String>,
    },
    Title {
        title: String,
        #[arg(long, default_value = "")]
        exe: String,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum Target {
    Web,
    Apps,
}

#[derive(Clone, Copy, ValueEnum)]
enum ModeArg {
    Block,
    AllowOnly,
}

#[derive(Clone, Copy, ValueEnum)]
enum ScopeArg {
    All,
    Except,
    Only,
}

#[derive(Clone, Copy, ValueEnum)]
enum OnOff {
    On,
    Off,
}

#[derive(Clone, Copy, ValueEnum)]
enum AddRemove {
    Add,
    Remove,
}

type Result<T> = std::result::Result<T, String>;

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli.cmd) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

struct Conn(Client);

impl Conn {
    fn open() -> Result<Self> {
        Client::connect().map(Conn).map_err(|e| {
            format!("cannot reach the Al-Dir service at {} ({e}). Is it running?", al_dir_ipc::socket_path())
        })
    }

    fn call(&mut self, req: Request) -> Result<Response> {
        match self.0.call(&req).map_err(|e| format!("talking to the service: {e}"))? {
            Response::Error { message, .. } => Err(message),
            r => Ok(r),
        }
    }

    fn sets(&mut self) -> Result<Vec<SetInfo>> {
        match self.call(Request::ListSets)? {
            Response::Sets { sets } => Ok(sets),
            r => Err(unexpected(r)),
        }
    }

    /// Finds a block by name (case-insensitive) or by the start of its id.
    fn find(&mut self, name: &str) -> Result<SetInfo> {
        let sets = self.sets()?;
        if let Some(s) = sets.iter().find(|s| s.set.name.eq_ignore_ascii_case(name)) {
            return Ok(s.clone());
        }
        let by_id: Vec<_> = sets
            .iter()
            .filter(|s| name.len() >= 4 && s.set.id.to_string().starts_with(&name.to_ascii_lowercase()))
            .collect();
        match by_id.as_slice() {
            [one] => Ok((*one).clone()),
            [] => Err(format!("no block named \"{name}\" (see: al-dir-cli list)")),
            _ => Err(format!("\"{name}\" matches several blocks; use more of the id")),
        }
    }

    fn save(&mut self, set: BlockSet) -> Result<()> {
        self.call(Request::UpsertSet { set: Box::new(set) }).map(drop)
    }

    fn modify(&mut self, block: &str, f: impl FnOnce(&mut BlockSet) -> Result<()>) -> Result<()> {
        let mut set = self.find(block)?.set;
        f(&mut set)?;
        let name = set.name.clone();
        self.save(set)?;
        println!("Saved \"{name}\".");
        Ok(())
    }
}

fn unexpected(r: Response) -> String {
    format!("unexpected reply from the service: {r:?}")
}

fn run(cmd: Cmd) -> Result<()> {
    if let Cmd::Partial { block: None, .. } = cmd {
        for r in partial::BUILTIN {
            println!("{:<26} {}", r.id, r.domain);
        }
        return Ok(());
    }
    let mut c = Conn::open()?;
    match cmd {
        Cmd::Status => {
            let Response::Status { status: s } = c.call(Request::Status)? else { return Err("bad reply".into()) };
            println!("Al-Dir service {} (protocol {})", s.version, s.protocol);
            println!("running since {}", fmt_time(s.started_at));
            println!("you are {}{}", s.user, if s.admin { " (administrator)" } else { "" });
            println!("{} blocks, {} locked", s.sets, s.locked_sets);
            println!("time: {}", s.clock);
            println!("location: {}", s.location);
            println!("hardened mode: {}", if s.hardened { "on" } else { "off" });
            if !s.extensions.is_empty() {
                println!("browser extensions connected: {}", s.extensions.join(", "));
            }
        }
        Cmd::List => {
            let sets = c.sets()?;
            if sets.is_empty() {
                println!("No blocks yet. Create one with: al-dir-cli create \"My block\"");
            }
            for s in sets {
                let state = match (s.set.enabled, s.active) {
                    (false, _) => "off",
                    (true, true) => "ACTIVE",
                    (true, false) => "scheduled",
                };
                println!(
                    "{}  {:<24} {:<9} {:<8} {} web, {} app rules",
                    &s.set.id.to_string()[..8],
                    s.set.name,
                    state,
                    if s.locked { "LOCKED" } else { "" },
                    s.set.web.block.len() + s.set.web.allow.len(),
                    s.set.apps.block.len() + s.set.apps.allow.len(),
                );
            }
        }
        Cmd::Show { block } => show(&c.find(&block)?),
        Cmd::Create { name } => {
            if c.find(&name).is_ok() {
                return Err(format!("a block named \"{name}\" already exists"));
            }
            c.save(BlockSet::new(name.clone()))?;
            println!("Created \"{name}\".");
        }
        Cmd::Delete { block } => {
            let s = c.find(&block)?;
            c.call(Request::RemoveSet { id: s.set.id })?;
            println!("Deleted \"{}\".", s.set.name);
        }
        Cmd::Rename { block, name } => c.modify(&block, |s| {
            s.name = name;
            Ok(())
        })?,
        Cmd::Enable { block } => c.modify(&block, |s| {
            s.enabled = true;
            Ok(())
        })?,
        Cmd::Disable { block } => c.modify(&block, |s| {
            s.enabled = false;
            Ok(())
        })?,
        Cmd::Add { block, rules, allow, app } => c.modify(&block, |s| edit_rules(s, &rules, allow, app, true))?,
        Cmd::Remove { block, rules, allow, app } => c.modify(&block, |s| edit_rules(s, &rules, allow, app, false))?,
        Cmd::ImportText { block, file, allow } => {
            let text = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let parsed = parse_rules_text(&text);
            for (line, err) in &parsed.errors {
                eprintln!("{}:{line}: skipped: {err}", file.display());
            }
            let mut added = 0;
            c.modify(&block, |s| {
                let list = if allow { &mut s.web.allow } else { &mut s.web.block };
                let existing: std::collections::HashSet<WebRule> = list.iter().cloned().collect();
                for r in parsed.rules {
                    if !existing.contains(&r) {
                        list.push(r);
                        added += 1;
                    }
                }
                Ok(())
            })?;
            println!("Added {added} rules ({} lines skipped).", parsed.errors.len());
        }
        Cmd::Mode { block, target, mode } => c.modify(&block, |s| {
            let mode = match mode {
                ModeArg::Block => ListMode::Block,
                ModeArg::AllowOnly => ListMode::AllowOnly,
            };
            match target {
                Target::Web => s.web.mode = mode,
                Target::Apps => s.apps.mode = mode,
            }
            Ok(())
        })?,
        Cmd::Users { block, scope, names } => c.modify(&block, |s| {
            s.users = match scope {
                ScopeArg::All => UserScope::All,
                _ if names.is_empty() => return Err("name at least one user".into()),
                ScopeArg::Except => UserScope::AllExcept(names),
                ScopeArg::Only => UserScope::Only(names),
            };
            Ok(())
        })?,
        Cmd::SafeSearch { block, state } => c.modify(&block, |s| {
            s.web.safe_search = matches!(state, OnOff::On);
            Ok(())
        })?,
        Cmd::Lists { ref id, .. } if al_dir_core::lists::find(id).is_none() => {
            return Err(format!("unknown list \"{id}\" (see: al-dir-cli catalog)"));
        }
        Cmd::Lists { block, action, id } => c.modify(&block, |s| {
            toggle(&mut s.web.lists, id, action);
            Ok(())
        })?,
        Cmd::Partial { block: Some(block), action: Some(action), id: Some(id) } => {
            if partial::find(&id).is_none() {
                return Err(format!("unknown partial block \"{id}\" (see: al-dir-cli partial)"));
            }
            c.modify(&block, |s| {
                toggle(&mut s.web.partial, id, action);
                Ok(())
            })?
        }
        Cmd::Partial { .. } => return Err("usage: al-dir-cli partial <block> add|remove <id>".into()),
        Cmd::Schedule { block, action } => c.modify(&block, |s| {
            match action {
                ScheduleCmd::Clear => s.schedule = None,
                ScheduleCmd::Add { spec } => {
                    let range = parse::weekly(&spec.join(" "))?;
                    s.schedule.get_or_insert_with(Schedule::default).weekly.push(range);
                }
                ScheduleCmd::AddDates { start, end } => {
                    let range = DateRange { start: parse::datetime(&start)?, end: parse::datetime(&end)? };
                    s.schedule.get_or_insert_with(Schedule::default).dates.push(range);
                }
            }
            Ok(())
        })?,
        Cmd::Allowance { block, minutes } => c.modify(&block, |s| {
            s.allowance = match minutes.as_str() {
                "off" => None,
                m => {
                    let m: u32 = m.parse().map_err(|_| "allowance must be a number of minutes or \"off\"")?;
                    Some(Allowance { daily_secs: m * 60 })
                }
            };
            Ok(())
        })?,
        Cmd::Lock { block, delay, kind } => lock(&mut c, &block, delay, kind)?,
        Cmd::LearnLocation { block } => {
            let s = c.find(&block)?;
            let Response::Learned { networks } = c.call(Request::LearnLocation { id: s.set.id })? else {
                return Err("bad reply".into());
            };
            println!("This place now has {networks} Wi-Fi network(s) remembered for \"{}\".", s.set.name);
        }
        Cmd::Unlock { block } => unlock(&mut c, &block)?,
        Cmd::CancelUnlock { block } => {
            let s = c.find(&block)?;
            c.call(Request::CancelUnlock { id: s.set.id })?;
            println!("Pending unlock cancelled.");
        }
        Cmd::Export { blocks, output } => {
            let ids = if blocks.is_empty() {
                None
            } else {
                Some(blocks.iter().map(|b| c.find(b).map(|s| s.set.id)).collect::<Result<Vec<SetId>>>()?)
            };
            let Response::Exported { json } = c.call(Request::Export { ids })? else { return Err("bad reply".into()) };
            match output {
                Some(path) => {
                    std::fs::write(&path, json).map_err(|e| format!("{}: {e}", path.display()))?;
                    println!("Exported to {}.", path.display());
                }
                None => println!("{json}"),
            }
        }
        Cmd::Import { file } => {
            let json = std::fs::read_to_string(&file).map_err(|e| format!("{}: {e}", file.display()))?;
            let Response::Imported { added, replaced, rejected } = c.call(Request::Import { json })? else {
                return Err("bad reply".into());
            };
            for n in added {
                println!("added     {n}");
            }
            for n in replaced {
                println!("replaced  {n}");
            }
            for (n, why) in rejected {
                println!("rejected  {n}: {why}");
            }
        }
        Cmd::Hardened { state } => {
            let on = matches!(state, OnOff::On);
            c.call(Request::SetHardened { on })?;
            println!("Hardened mode {}.", if on { "on" } else { "off" });
            if on {
                println!("While a block is locked, sudo and administrator password prompts will not work");
                println!("for the people it applies to. Emergency access: boot into recovery mode.");
            }
        }
        Cmd::Catalog => {
            let Response::Catalog { lists } = c.call(Request::Catalog)? else { return Err("bad reply".into()) };
            for l in lists {
                let updated = l.updated_at.map(fmt_time).unwrap_or_else(|| "built-in only".into());
                println!("{:<10} {:>7} rules   {updated}", l.id, l.rules);
            }
        }
        Cmd::Stats { days } => {
            let Response::Stats { stats } = c.call(Request::Stats { days })? else { return Err("bad reply".into()) };
            println!("Blocked per day:");
            for d in &stats.days {
                let mins = d.allowance_secs / 60;
                let bar = "#".repeat((d.blocked as usize).min(50));
                println!("  {}  {:>5}  {bar}{}", d.date, d.blocked, if mins > 0 { format!("  ({mins} min allowance used)") } else { String::new() });
            }
            if !stats.top_targets.is_empty() {
                println!("Most blocked:");
                for (t, n) in &stats.top_targets {
                    println!("  {n:>5}  {t}");
                }
            }
            if !stats.by_block.is_empty() {
                println!("By block:");
                for (b, n) in &stats.by_block {
                    println!("  {n:>5}  {b}");
                }
            }
        }
        Cmd::Events => {
            let Response::Events { events } = c.call(Request::RecentEvents)? else { return Err("bad reply".into()) };
            if events.is_empty() {
                println!("Nothing blocked yet.");
            }
            for e in events {
                let what = match e.kind {
                    EventKind::AppStart => "stopped",
                    EventKind::AppKilled => "closed",
                    EventKind::WindowClosed => "closed",
                    EventKind::SiteBlocked => "blocked",
                    EventKind::PageBlocked => "page",
                    EventKind::AllowanceLow => "5 min left",
                    EventKind::AllowanceUsed => "time up",
                };
                println!("{}  {:<10} {what:<8} {}  [{}: {}]", fmt_time(e.time), e.user, e.target, e.set_name, e.reason);
            }
        }
        Cmd::Check { user, what } => {
            let query = match what {
                CheckCmd::Url { url } => CheckQuery::Url { url },
                CheckCmd::Host { host } => CheckQuery::Host { host },
                CheckCmd::App { path, cmdline } => CheckQuery::App { exe_path: path, cmdline },
                CheckCmd::Title { title, exe } => CheckQuery::Window { title, exe_path: exe },
            };
            let Response::Decision { decision } = c.call(Request::Check { query, user })? else {
                return Err("bad reply".into());
            };
            print_decision(&decision);
        }
    }
    Ok(())
}

fn toggle(list: &mut Vec<String>, id: String, action: AddRemove) {
    match action {
        AddRemove::Add if !list.contains(&id) => list.push(id),
        AddRemove::Add => {}
        AddRemove::Remove => list.retain(|x| *x != id),
    }
}

fn edit_rules(s: &mut BlockSet, rules: &[String], allow: bool, app: bool, add: bool) -> Result<()> {
    if app {
        let parsed = rules.iter().map(|r| AppRule::parse(r).map_err(|e| e.to_string())).collect::<Result<Vec<_>>>()?;
        let list = if allow { &mut s.apps.allow } else { &mut s.apps.block };
        apply(list, parsed, add);
    } else {
        let parsed = rules.iter().map(|r| WebRule::parse(r).map_err(|e| e.to_string())).collect::<Result<Vec<_>>>()?;
        let list = if allow { &mut s.web.allow } else { &mut s.web.block };
        apply(list, parsed, add);
    }
    Ok(())
}

fn apply<T: PartialEq>(list: &mut Vec<T>, rules: Vec<T>, add: bool) {
    for r in rules {
        if add {
            if !list.contains(&r) {
                list.push(r);
            }
        } else {
            list.retain(|x| *x != r);
        }
    }
}

fn lock(c: &mut Conn, block: &str, delay: Option<String>, kind: LockCmd) -> Result<()> {
    let unlock_delay_secs = match delay {
        Some(d) => u32::try_from(parse::duration_secs(&d)?).map_err(|_| "delay too long")?,
        None => 0,
    };
    let now = jiff::Timestamp::now().as_second();
    let mut shown_password = None;
    let kind = match kind {
        LockCmd::For { duration } => {
            let secs = i64::try_from(parse::duration_secs(&duration)?).map_err(|_| "duration too long")?;
            LockKind::Until { until: now.saturating_add(secs) }
        }
        LockCmd::Until { datetime } => {
            let dt = parse::datetime(&datetime.join(" "))?;
            let ts = dt.to_zoned(jiff::tz::TimeZone::system()).map_err(|e| e.to_string())?.timestamp();
            if ts.as_second() <= now {
                return Err("that time is in the past".into());
            }
            LockKind::Until { until: ts.as_second() }
        }
        LockCmd::Password { generate } => {
            let password = if generate {
                let p = random_text(16).map_err(|e| e.to_string())?;
                shown_password = Some(p.clone());
                p
            } else {
                let p = rpassword::prompt_password("New lock password: ").map_err(|e| e.to_string())?;
                if p.is_empty() {
                    return Err("empty password".into());
                }
                if rpassword::prompt_password("Repeat password: ").map_err(|e| e.to_string())? != p {
                    return Err("the passwords do not match".into());
                }
                p
            };
            LockKind::Password { hash: PasswordHash::new(&password).map_err(|e| e.to_string())? }
        }
        LockCmd::Random { length } => {
            if !(10..=5000).contains(&length) {
                return Err("length must be between 10 and 5000".into());
            }
            LockKind::RandomText { length }
        }
        LockCmd::Restart => LockKind::UntilRestart { boot_id: String::new() },
        LockCmd::Schedule { spec } => LockKind::Schedule {
            schedule: Schedule { weekly: vec![parse::weekly(&spec.join(" "))?], dates: vec![] },
        },
        LockCmd::Location { lat, lon, radius } => {
            let area = al_dir_core::GeoArea::Circle { lat, lon, radius_m: radius };
            area.validate().map_err(|e| e.to_string())?;
            LockKind::Location { area }
        }
    };
    let name = c.find(block)?.set.name;
    c.modify(block, |s| {
        s.lock = Some(Lock { kind, unlock_delay_secs });
        Ok(())
    })?;
    println!("\"{name}\" is now locked.");
    if let Some(Lock { kind: LockKind::Location { .. }, .. }) = c.find(block)?.set.lock {
        println!("If you are at that place now, run: al-dir-cli learn-location \"{name}\"");
    }
    if let Some(p) = shown_password {
        println!();
        println!("Lock password: {p}");
        println!("Give it to someone you trust. It will not be shown again.");
    }
    Ok(())
}

fn unlock(c: &mut Conn, block: &str) -> Result<()> {
    let s = c.find(block)?;
    let id = s.set.id;
    let Response::Challenge { challenge } = c.call(Request::UnlockStart { id })? else {
        return Err("bad reply".into());
    };
    let answer = match challenge {
        ChallengeInfo::NotLocked => {
            println!("\"{}\" is not locked.", s.set.name);
            return Ok(());
        }
        ChallengeInfo::NotPossible => {
            return Err(format!("this lock cannot be opened early ({})", describe_lock(s.set.lock.as_ref())));
        }
        ChallengeInfo::Password => rpassword::prompt_password("Lock password: ").map_err(|e| e.to_string())?,
        ChallengeInfo::RandomText { text } => {
            println!("Type this text exactly, then press Enter:\n\n{text}\n");
            print!("> ");
            std::io::stdout().flush().ok();
            let mut line = String::new();
            std::io::stdin().lock().read_line(&mut line).map_err(|e| e.to_string())?;
            line.trim_end_matches(['\r', '\n']).to_string()
        }
    };
    let Response::Unlock { outcome } = c.call(Request::UnlockAnswer { id, answer })? else {
        return Err("bad reply".into());
    };
    match outcome {
        UnlockInfo::Unlocked => println!("Unlocked."),
        UnlockInfo::Pending { at } => println!("Correct. The lock opens at {}.", fmt_time(at)),
    }
    Ok(())
}

fn print_decision(d: &DecisionInfo) {
    if d.blocked {
        println!(
            "BLOCKED by \"{}\": {}",
            d.set_name.as_deref().unwrap_or("?"),
            d.reason.as_deref().unwrap_or("?")
        );
    } else if !d.metered.is_empty() {
        println!("allowed (uses time allowance)");
    } else {
        println!("allowed");
    }
}

fn fmt_time(unix: i64) -> String {
    jiff::Timestamp::from_second(unix)
        .map(|t| t.to_zoned(jiff::tz::TimeZone::system()).strftime("%Y-%m-%d %H:%M").to_string())
        .unwrap_or_else(|_| unix.to_string())
}

fn fmt_minutes(m: u16) -> String {
    format!("{:02}:{:02}", m / 60, m % 60)
}

fn describe_schedule(s: &Schedule) -> Vec<String> {
    let mut out: Vec<String> = s
        .weekly
        .iter()
        .map(|w| {
            let days: Vec<String> = w.days.iter().map(|d| format!("{d:?}").to_lowercase()).collect();
            format!("{} {}-{}", days.join(","), fmt_minutes(w.start), fmt_minutes(w.end))
        })
        .collect();
    out.extend(s.dates.iter().map(|d| format!("{} to {}", d.start, d.end)));
    out
}

fn describe_lock(lock: Option<&Lock>) -> String {
    let Some(lock) = lock else { return "none".into() };
    let mut s = match &lock.kind {
        LockKind::Until { until } => format!("until {}", fmt_time(*until)),
        LockKind::Schedule { schedule } => format!("during {}", describe_schedule(schedule).join("; ")),
        LockKind::Password { .. } => "password".into(),
        LockKind::RandomText { length } => format!("random text ({length} characters)"),
        LockKind::UntilRestart { .. } => "until the computer restarts".into(),
        LockKind::Location { area } => match area {
            al_dir_core::GeoArea::Circle { lat, lon, radius_m } => {
                format!("while within {radius_m:.0} m of {lat:.5}, {lon:.5}")
            }
            al_dir_core::GeoArea::Polygon { points } => format!("while inside an area of {} points", points.len()),
        },
    };
    if lock.unlock_delay_secs > 0 {
        s.push_str(&format!(", unlock delay {} min", lock.unlock_delay_secs / 60));
    }
    s
}

fn show(info: &SetInfo) {
    let s = &info.set;
    let mode = |m: ListMode| match m {
        ListMode::Block => "block list",
        ListMode::AllowOnly => "ALLOW-ONLY",
    };
    println!("{}  ({})", s.name, s.id);
    println!(
        "  state:      {}{}{}",
        if s.enabled { "on" } else { "off" },
        if info.active { ", in force now" } else { "" },
        if info.locked { ", LOCKED" } else { "" }
    );
    println!(
        "  users:      {}",
        match &s.users {
            UserScope::All => "everyone".to_string(),
            UserScope::AllExcept(n) => format!("everyone except {}", n.join(", ")),
            UserScope::Only(n) => format!("only {}", n.join(", ")),
        }
    );
    println!("  websites:   {}", mode(s.web.mode));
    for r in &s.web.block {
        println!("    block     {r}");
    }
    for r in &s.web.allow {
        println!("    allow     {r}");
    }
    for l in &s.web.lists {
        println!("    list      {l}");
    }
    for p in &s.web.partial {
        println!("    partial   {p}");
    }
    if s.web.safe_search {
        println!("    SafeSearch forced");
    }
    println!("  apps:       {}", mode(s.apps.mode));
    for r in &s.apps.block {
        println!("    block     {r}");
    }
    for r in &s.apps.allow {
        println!("    allow     {r}");
    }
    match &s.schedule {
        None => println!("  schedule:   always"),
        Some(sch) => {
            println!("  schedule:");
            for line in describe_schedule(sch) {
                println!("    {line}");
            }
        }
    }
    if let Some(a) = s.allowance {
        println!("  allowance:  {} min/day", a.daily_secs / 60);
    }
    println!("  lock:       {}", describe_lock(s.lock.as_ref()));
    if let Some(at) = s.pending_unlock_at {
        println!("  unlocks at: {}", fmt_time(at));
    }
}
