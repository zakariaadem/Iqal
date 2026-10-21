//! A stopwatch for development: with IQAL_TIMING set to a file name, how
//! long opening the app, switching tabs, scrolling and opening dialogs take
//! is added to that file, one line each ("tab 12.3"), in milliseconds.
//! Without it nothing is measured (one look at the setting, at start).

use std::io::Write;
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Instant;

static FILE: OnceLock<Option<PathBuf>> = OnceLock::new();
static STARTED: OnceLock<Instant> = OnceLock::new();

/// Called first thing: notes when the program started.
pub fn init() {
    FILE.get_or_init(|| std::env::var_os("IQAL_TIMING").map(PathBuf::from));
    STARTED.get_or_init(Instant::now);
}

/// A running stopwatch, if timing is on.
pub fn start() -> Option<Instant> {
    FILE.get().and_then(Option::as_ref).map(|_| Instant::now())
}

/// When the program started, if timing is on.
pub fn program_start() -> Option<Instant> {
    FILE.get().and_then(Option::as_ref).and(STARTED.get().copied())
}

/// Records how long `what` took since `since`.
pub fn record(what: &str, since: Option<Instant>) {
    let (Some(since), Some(Some(path))) = (since, FILE.get()) else { return };
    let ms = since.elapsed().as_secs_f64() * 1000.0;
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "{what} {ms:.2}");
    }
}
