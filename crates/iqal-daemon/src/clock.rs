//! The trusted clock. Lock deadlines must not end early because someone
//! moves the system clock forward, so the service keeps its own idea of the
//! time, in this order of preference:
//!
//! 1. **Network time**: the median of several NTP servers, carried forward
//!    with the kernel's boot clock (which counts suspend and cannot be set).
//! 2. **System time**, but only while the kernel says it is synchronised
//!    (setting the clock by hand clears that flag).
//! 3. **Counted time**: the last trusted time saved on disk, carried forward
//!    with the boot clock. Time spent switched off is not counted, so with
//!    no network a lock can only last longer than planned, never shorter.
//!
//! The trusted time also never goes back below the last saved value.

use std::net::{ToSocketAddrs, UdpSocket};
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use iqal_core::Now;

const NTP_SERVERS: &[&str] = &["time.cloudflare.com:123", "pool.ntp.org:123", "time.google.com:123", "time.apple.com:123"];
/// Seconds between 1900-01-01 (NTP era) and 1970-01-01.
const NTP_UNIX_OFFSET: f64 = 2_208_988_800.0;
const SAVE_FILE: &str = "clock";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Network,
    System,
    Counted,
}

impl Source {
    /// The apps translate these texts (iqal_app::describe::time_source):
    /// change both together.
    pub fn describe(self) -> &'static str {
        match self {
            Source::Network => "network time",
            Source::System => "synchronised system time",
            Source::Counted => "counted from last trusted time (offline)",
        }
    }
}

struct State {
    /// trusted unix time = boot clock + offset, once network time is known.
    network_offset: Option<f64>,
    /// Counted time: trusted unix time `base` at boot clock `base_boot`.
    base: f64,
    base_boot: f64,
    /// Never report less than this (last saved trusted time).
    floor: f64,
}

pub struct Clock {
    state: Mutex<State>,
    save_path: Option<PathBuf>,
}

static CLOCK: OnceLock<Clock> = OnceLock::new();

/// Starts the trusted clock, persisting to `state_dir`.
pub fn init(state_dir: PathBuf) {
    let path = state_dir.join(SAVE_FILE);
    let saved: Option<f64> = std::fs::read_to_string(&path).ok().and_then(|s| s.trim().parse().ok());
    let system = system_unix();
    let floor = saved.unwrap_or(0.0);
    // Counted time starts from the saved value if there is one (we cannot
    // know how long we were off), else from the system clock.
    let base = saved.unwrap_or(system).max(floor);
    let _ = CLOCK.set(Clock {
        state: Mutex::new(State { network_offset: None, base, base_boot: boot_clock(), floor }),
        save_path: Some(path),
    });
}

fn clock() -> Option<&'static Clock> {
    CLOCK.get()
}

/// The trusted current time. Falls back to the system clock if the trusted
/// clock was never started (tools, tests).
pub fn now() -> Now {
    match clock() {
        Some(c) => now_at(c.unix().0),
        None => Now::system(),
    }
}

pub fn source() -> Source {
    clock().map_or(Source::System, |c| c.unix().1)
}

fn now_at(unix: f64) -> Now {
    let secs = unix.floor() as i64;
    match jiff::Timestamp::from_second(secs) {
        Ok(ts) => {
            let zoned = ts.to_zoned(jiff::tz::TimeZone::system());
            Now { unix: secs, local: zoned.datetime() }
        }
        Err(_) => Now::system(),
    }
}

impl Clock {
    fn unix(&self) -> (f64, Source) {
        let s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let boot = boot_clock();
        let (t, source) = if let Some(offset) = s.network_offset {
            (boot + offset, Source::Network)
        } else if kernel_synchronised() {
            (system_unix(), Source::System)
        } else {
            (s.base + (boot - s.base_boot), Source::Counted)
        };
        (t.max(s.floor), source)
    }

    fn set_network(&self, offset: f64) {
        let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
        s.network_offset = Some(offset);
        let now = boot_clock() + offset;
        s.base = now.max(s.floor);
        s.base_boot = boot_clock();
    }

    /// Re-bases counted time on synchronised system time when available, and
    /// saves the trusted time.
    fn checkpoint(&self) {
        let (t, source) = self.unix();
        {
            let mut s = self.state.lock().unwrap_or_else(|e| e.into_inner());
            if source != Source::Counted {
                s.base = t;
                s.base_boot = boot_clock();
            }
            s.floor = s.floor.max(t);
        }
        if let Some(path) = &self.save_path {
            let tmp = path.with_extension("tmp");
            if std::fs::write(&tmp, format!("{t:.0}")).and_then(|()| std::fs::rename(&tmp, path)).is_err() {
                eprintln!("iqal: could not save trusted time");
            }
        }
    }
}

/// Keeps the clock synchronised with the network and saved to disk.
pub fn start_sync() -> std::io::Result<()> {
    let Some(c) = clock() else { return Ok(()) };
    std::thread::Builder::new().name("iqal-clock".into()).spawn(move || {
        let mut next_sync = std::time::Instant::now();
        let mut warned = false;
        loop {
            if std::time::Instant::now() >= next_sync {
                match network_offset() {
                    Some(offset) => {
                        c.set_network(offset);
                        let skew = system_unix() - (boot_clock() + offset);
                        if skew.abs() > 120.0 && !warned {
                            eprintln!("iqal: the system clock is off by {skew:.0} s; using network time");
                            warned = true;
                        }
                        next_sync = std::time::Instant::now() + Duration::from_secs(600);
                    }
                    None => next_sync = std::time::Instant::now() + Duration::from_secs(60),
                }
            }
            c.checkpoint();
            std::thread::sleep(Duration::from_secs(30));
        }
    })?;
    Ok(())
}

fn system_unix() -> f64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs_f64()).unwrap_or(0.0)
}

/// Seconds since boot, including suspend; cannot be changed by users.
#[cfg(target_os = "linux")]
fn boot_clock() -> f64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    // SAFETY: ts is a valid timespec.
    unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, &mut ts) };
    ts.tv_sec as f64 + ts.tv_nsec as f64 / 1e9
}

/// Seconds since boot, including sleep; cannot be changed by users.
#[cfg(windows)]
fn boot_clock() -> f64 {
    // SAFETY: no preconditions.
    unsafe { windows_sys::Win32::System::SystemInformation::GetTickCount64() as f64 / 1000.0 }
}

#[cfg(not(any(target_os = "linux", windows)))]
fn boot_clock() -> f64 {
    static START: OnceLock<std::time::Instant> = OnceLock::new();
    START.get_or_init(std::time::Instant::now).elapsed().as_secs_f64()
}

/// Whether the kernel considers the clock synchronised (by NTP). Setting
/// the time by hand marks it unsynchronised.
#[cfg(target_os = "linux")]
fn kernel_synchronised() -> bool {
    // SAFETY: timex is plain data; modes = 0 only reads.
    let mut tx: libc::timex = unsafe { std::mem::zeroed() };
    let state = unsafe { libc::adjtimex(&mut tx) };
    state >= 0 && state != libc::TIME_ERROR && tx.status & libc::STA_UNSYNC == 0
}

#[cfg(not(target_os = "linux"))]
fn kernel_synchronised() -> bool {
    false
}

/// Network time as an offset from the boot clock: the median over the NTP
/// servers that answered sensibly.
fn network_offset() -> Option<f64> {
    let mut offsets: Vec<f64> = NTP_SERVERS.iter().filter_map(|s| sntp(s)).collect();
    if offsets.is_empty() {
        return None;
    }
    offsets.sort_by(|a, b| a.total_cmp(b));
    Some(offsets[offsets.len() / 2])
}

fn sntp(server: &str) -> Option<f64> {
    let addr = server.to_socket_addrs().ok()?.next()?;
    let sock = UdpSocket::bind(if addr.is_ipv4() { "0.0.0.0:0" } else { "[::]:0" }).ok()?;
    sock.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    let mut req = [0u8; 48];
    req[0] = 0x23; // LI 0, version 4, client mode
    let sent = boot_clock();
    sock.send_to(&req, addr).ok()?;
    let mut resp = [0u8; 48];
    let (n, _) = sock.recv_from(&mut resp).ok()?;
    let received = boot_clock();
    parse_sntp(&resp[..n]).map(|server_time| server_time - (sent + received) / 2.0)
}

/// The server's transmit time (unix seconds) from an SNTP reply.
fn parse_sntp(resp: &[u8]) -> Option<f64> {
    if resp.len() < 48 || resp[0] & 0x07 != 4 || !(1..=15).contains(&resp[1]) {
        return None;
    }
    let secs = u32::from_be_bytes(resp[40..44].try_into().ok()?) as f64;
    let frac = u32::from_be_bytes(resp[44..48].try_into().ok()?) as f64 / 4_294_967_296.0;
    if secs == 0.0 {
        return None;
    }
    Some(secs + frac - NTP_UNIX_OFFSET)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sntp_reply() {
        let mut r = [0u8; 48];
        r[0] = 0x24; // server mode
        r[1] = 2; // stratum
        let ntp_secs: u32 = (1_800_000_000u64 + NTP_UNIX_OFFSET as u64) as u32;
        r[40..44].copy_from_slice(&ntp_secs.to_be_bytes());
        r[44..48].copy_from_slice(&(1u32 << 31).to_be_bytes()); // .5 s
        assert_eq!(parse_sntp(&r), Some(1_800_000_000.5));
        r[1] = 0; // "kiss of death"
        assert_eq!(parse_sntp(&r), None);
    }

    #[test]
    fn counted_time_never_goes_back() {
        let c = Clock {
            state: Mutex::new(State { network_offset: None, base: 1000.0, base_boot: boot_clock(), floor: 5000.0 }),
            save_path: None,
        };
        assert!(c.unix().0 >= 5000.0);
    }

    #[test]
    fn boot_clock_moves() {
        let a = boot_clock();
        std::thread::sleep(Duration::from_millis(20));
        assert!(boot_clock() > a);
    }
}
