//! Location for location locks. Runs only while at least one location lock
//! exists; otherwise nothing is looked up or stored.
//!
//! Evidence, strongest first:
//!
//! * **Learned Wi-Fi networks.** The networks around a place are remembered
//!   ("I am here now", or automatically while a precise position says the
//!   machine is there). Any of them in range: *inside*. Wi-Fi working and
//!   several networks in range but none of them: *away*. Neighbours'
//!   networks count too, so switching off one router does not fake "away".
//! * **OS location service (GeoClue)**, with its Wi-Fi lookup pointed at
//!   BeaconDB, the community successor of Mozilla's retired service. Only
//!   positions more precise than 1 km count (not IP-based guesses, which a
//!   VPN can fake).
//!
//! Without evidence the machine counts as inside: locks fail closed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iqal_core::geo::{GeoArea, LocationFix, Relation};
use serde::{Deserialize, Serialize};
use zbus::blocking::{Connection, Proxy};
use zbus::proxy::CacheProperties;
use zbus::zvariant::{OwnedObjectPath, Value};

use crate::service::{LocationState, Service};

const GEOCLUE_CONF: &str = "/etc/geoclue/conf.d/90-iqal.conf";
const GEOCLUE_CONF_TEXT: &str = "# Managed by Iqal.\n\
[iqal]\nallowed=true\nsystem=true\nusers=\n\n\
[wifi]\nenable=true\nurl=https://api.beacondb.net/v1/geolocate\n";
const DESKTOP_ID: &str = "iqal";
const POLL: Duration = Duration::from_secs(20);
const MAX_ACCURACY_M: f64 = 1000.0;
const FINGERPRINTS: &str = "wifi-fingerprints.json";
const MAX_LEARNED: usize = 64;
/// "Away" needs a place with at least this many learned networks, and at
/// least this many other networks in range now.
const MIN_FOR_AWAY: usize = 2;

#[derive(Default, Serialize, Deserialize)]
struct Fingerprint {
    area: Option<GeoArea>,
    bssids: Vec<String>,
}

/// Serialises access to the fingerprint file (watcher and "learn now").
static FILE_LOCK: Mutex<()> = Mutex::new(());

pub fn start(service: Arc<Service>, state_dir: PathBuf) -> std::io::Result<()> {
    {
        let dir = state_dir.clone();
        service.set_learner(Box::new(move |area| learn_now(&dir, area)));
    }
    std::thread::Builder::new().name("iqal-location".into()).spawn(move || {
        let mut watcher = Watcher { conn: None, client: None, state_dir };
        loop {
            let areas = service.lock_summary().location_areas;
            let state = if areas.is_empty() {
                watcher.stop();
                LocationState::default()
            } else {
                watcher.observe(&areas)
            };
            service.set_location(state);
            std::thread::sleep(POLL);
        }
    })?;
    Ok(())
}

/// Removes the GeoClue configuration (uninstall).
pub fn cleanup() {
    if std::fs::remove_file(GEOCLUE_CONF).is_ok() {
        restart_geoclue();
    }
}

/// "I am at this place now": adds the networks in range to the area.
fn learn_now(state_dir: &Path, area: &GeoArea) -> std::io::Result<usize> {
    let conn = Connection::system().map_err(std::io::Error::other)?;
    let visible = wifi_networks(&conn, true);
    if visible.is_empty() {
        return Err(std::io::Error::other("no Wi-Fi networks in range (is Wi-Fi on?)"));
    }
    let _guard = FILE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut fps = load(state_dir);
    add_networks(&mut fps, area, &visible);
    save(state_dir, &fps);
    Ok(fps.iter().find(|f| f.area.as_ref() == Some(area)).map_or(0, |f| f.bssids.len()))
}

/// Adds networks to an area's fingerprint (never removes any). Returns
/// whether anything was new.
fn add_networks(fps: &mut Vec<Fingerprint>, area: &GeoArea, visible: &[String]) -> bool {
    let fp = match fps.iter().position(|f| f.area.as_ref() == Some(area)) {
        Some(i) => &mut fps[i],
        None => {
            fps.push(Fingerprint { area: Some(area.clone()), bssids: Vec::new() });
            fps.last_mut().expect("just pushed")
        }
    };
    let mut changed = false;
    for b in visible {
        if !fp.bssids.contains(b) {
            fp.bssids.push(b.clone());
            changed = true;
        }
    }
    if fp.bssids.len() > MAX_LEARNED {
        let excess = fp.bssids.len() - MAX_LEARNED;
        fp.bssids.drain(..excess);
    }
    changed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WifiEvidence {
    Inside,
    Away,
    None,
}

fn wifi_evidence(learned: &[String], visible: &[String]) -> WifiEvidence {
    if learned.iter().any(|b| visible.contains(b)) {
        WifiEvidence::Inside
    } else if learned.len() >= MIN_FOR_AWAY && visible.len() >= MIN_FOR_AWAY {
        WifiEvidence::Away
    } else {
        WifiEvidence::None
    }
}

struct Watcher {
    conn: Option<Connection>,
    client: Option<OwnedObjectPath>,
    state_dir: PathBuf,
}

impl Watcher {
    fn observe(&mut self, areas: &[GeoArea]) -> LocationState {
        ensure_geoclue_config();
        if self.conn.is_none() {
            self.conn = connect_with_agent()
                .map_err(|e| eprintln!("iqal: location service connection: {e}"))
                .ok();
        }
        let fix = match self.position() {
            Ok(fix) => fix,
            Err(e) => {
                eprintln!("iqal: location service: {e}");
                // GeoClue may have restarted: register the agent again.
                self.client = None;
                self.conn = None;
                LocationFix::Unknown
            }
        };
        let visible = self.conn.as_ref().map(|c| wifi_networks(c, false)).unwrap_or_default();

        let _guard = FILE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let mut fps = load(&self.state_dir);
        let mut changed = false;
        for area in areas {
            if area.relation(fix) == Relation::Inside {
                changed |= add_networks(&mut fps, area, &visible);
            }
        }
        if changed {
            save(&self.state_dir, &fps);
        }
        let mut state = LocationState { fix, ..LocationState::default() };
        for area in areas {
            let learned = fps.iter().find(|f| f.area.as_ref() == Some(area)).map_or(&[][..], |f| &f.bssids);
            match wifi_evidence(learned, &visible) {
                WifiEvidence::Inside => state.inside.push(area.clone()),
                WifiEvidence::Away => state.outside.push(area.clone()),
                WifiEvidence::None => {}
            }
        }
        state
    }

    fn position(&mut self) -> zbus::Result<LocationFix> {
        let Some(conn) = &self.conn else { return Ok(LocationFix::Unknown) };
        if self.client.is_none() {
            let manager = proxy(conn, "org.freedesktop.GeoClue2", "/org/freedesktop/GeoClue2/Manager", "org.freedesktop.GeoClue2.Manager")?;
            let path: OwnedObjectPath = manager.call("GetClient", &())?;
            let client = proxy(conn, "org.freedesktop.GeoClue2", path.as_str(), "org.freedesktop.GeoClue2.Client")?;
            client.set_property("DesktopId", DESKTOP_ID)?;
            client.set_property("RequestedAccuracyLevel", 8u32)?; // exact
            client.set_property("DistanceThreshold", 0u32)?;
            client.call_method("Start", &())?;
            drop(client);
            self.client = Some(path);
        }
        let path = self.client.as_ref().expect("set above");
        let client = proxy(conn, "org.freedesktop.GeoClue2", path.as_str(), "org.freedesktop.GeoClue2.Client")?;
        let location: OwnedObjectPath = client.get_property("Location")?;
        if location.as_str() == "/" {
            return Ok(LocationFix::Unknown);
        }
        let l = proxy(conn, "org.freedesktop.GeoClue2", location.as_str(), "org.freedesktop.GeoClue2.Location")?;
        let (lat, lon, accuracy_m): (f64, f64, f64) =
            (l.get_property("Latitude")?, l.get_property("Longitude")?, l.get_property("Accuracy")?);
        if !(accuracy_m.is_finite() && accuracy_m <= MAX_ACCURACY_M) {
            return Ok(LocationFix::Unknown);
        }
        Ok(LocationFix::Known { lat, lon, accuracy_m })
    }

    fn stop(&mut self) {
        if let (Some(conn), Some(path)) = (&self.conn, self.client.take())
            && let Ok(client) = proxy(conn, "org.freedesktop.GeoClue2", path.as_str(), "org.freedesktop.GeoClue2.Client")
        {
            let _ = client.call_method("Stop", &());
        }
    }
}

/// GeoClue serves a user's clients only once that user has a location
/// agent (normally part of the desktop). The service runs as root, which
/// has none, so it provides its own agent that approves only Iqal.
struct Agent;

#[zbus::interface(name = "org.freedesktop.GeoClue2.Agent")]
impl Agent {
    fn authorize_app(&self, desktop_id: &str, req_accuracy_level: u32) -> (bool, u32) {
        (desktop_id == DESKTOP_ID, req_accuracy_level)
    }

    #[zbus(property)]
    fn max_accuracy_level(&self) -> u32 {
        8 // exact
    }
}

/// GeoClue only accepts agents whose id is on its whitelist; this one is in
/// the default configuration.
const AGENT_ID: &str = "geoclue-demo-agent";

fn connect_with_agent() -> zbus::Result<Connection> {
    let conn = zbus::blocking::connection::Builder::system()?
        .serve_at("/org/freedesktop/GeoClue2/Agent", Agent)?
        .build()?;
    {
        let manager =
            proxy(&conn, "org.freedesktop.GeoClue2", "/org/freedesktop/GeoClue2/Manager", "org.freedesktop.GeoClue2.Manager")?;
        manager.call_method("AddAgent", &(AGENT_ID,))?;
    }
    Ok(conn)
}

fn proxy<'a>(conn: &'a Connection, dest: &'a str, path: &'a str, iface: &'a str) -> zbus::Result<Proxy<'a>> {
    zbus::blocking::proxy::Builder::new(conn)
        .destination(dest)?
        .path(path)?
        .interface(iface)?
        .cache_properties(CacheProperties::No)
        .build()
}

/// BSSIDs of Wi-Fi networks in range (from NetworkManager's scan list).
/// `fresh`: wait for a new scan first (used when learning a place).
fn wifi_networks(conn: &Connection, fresh: bool) -> Vec<String> {
    let mut out = Vec::new();
    let nm = "org.freedesktop.NetworkManager";
    let Ok(manager) = proxy(conn, nm, "/org/freedesktop/NetworkManager", nm) else { return out };
    let Ok(devices) = manager.call::<_, _, Vec<OwnedObjectPath>>("GetDevices", &()) else { return out };
    for dev in devices {
        let Ok(device) = proxy(conn, nm, dev.as_str(), "org.freedesktop.NetworkManager.Device") else { continue };
        if device.get_property::<u32>("DeviceType").ok() != Some(2) {
            continue; // not Wi-Fi
        }
        let Ok(wireless) = proxy(conn, nm, dev.as_str(), "org.freedesktop.NetworkManager.Device.Wireless") else { continue };
        let _ = wireless.call_method("RequestScan", &(HashMap::<&str, Value<'_>>::new(),));
        if fresh {
            std::thread::sleep(Duration::from_secs(4));
        }
        let Ok(aps) = wireless.call::<_, _, Vec<OwnedObjectPath>>("GetAllAccessPoints", &()) else { continue };
        for ap in aps {
            let Ok(ap) = proxy(conn, nm, ap.as_str(), "org.freedesktop.NetworkManager.AccessPoint") else { continue };
            // Weak networks count too: neighbours' networks make the evidence
            // much harder to fake than one's own router alone.
            if let Ok(bssid) = ap.get_property::<String>("HwAddress") {
                out.push(bssid.to_ascii_lowercase());
            }
        }
    }
    out
}

fn ensure_geoclue_config() {
    if std::fs::read_to_string(GEOCLUE_CONF).ok().as_deref() == Some(GEOCLUE_CONF_TEXT) {
        return;
    }
    let result = std::fs::create_dir_all("/etc/geoclue/conf.d").and_then(|()| std::fs::write(GEOCLUE_CONF, GEOCLUE_CONF_TEXT));
    match result {
        Ok(()) => restart_geoclue(),
        Err(e) => eprintln!("iqal: writing {GEOCLUE_CONF}: {e}"),
    }
}

fn restart_geoclue() {
    let _ = std::process::Command::new("systemctl").args(["try-restart", "geoclue.service"]).status();
}

fn load(dir: &std::path::Path) -> Vec<Fingerprint> {
    std::fs::read_to_string(dir.join(FINGERPRINTS)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save(dir: &std::path::Path, fps: &[Fingerprint]) {
    if let Ok(text) = serde_json::to_string(fps) {
        let tmp = dir.join(format!(".{FINGERPRINTS}.tmp"));
        if std::fs::write(&tmp, text).and_then(|()| std::fs::rename(&tmp, dir.join(FINGERPRINTS))).is_err() {
            eprintln!("iqal: could not save learned Wi-Fi networks");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(v: &[&str]) -> Vec<String> {
        v.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn evidence() {
        let home = s(&["a", "b", "c", "d"]);
        assert_eq!(wifi_evidence(&home, &s(&["x", "c"])), WifiEvidence::Inside, "a neighbour's network is enough");
        assert_eq!(wifi_evidence(&home, &s(&["x", "y", "z"])), WifiEvidence::Away);
        assert_eq!(wifi_evidence(&home, &s(&["x"])), WifiEvidence::None, "too few networks to be sure");
        assert_eq!(wifi_evidence(&home, &s(&["x", "y"])), WifiEvidence::Away);
        assert_eq!(wifi_evidence(&home, &[]), WifiEvidence::None, "Wi-Fi off");
        assert_eq!(wifi_evidence(&s(&["a"]), &s(&["x", "y", "z"])), WifiEvidence::None, "place barely learned");
    }

    #[test]
    fn learning_only_adds() {
        let area = GeoArea::Circle { lat: 1.0, lon: 2.0, radius_m: 300.0 };
        let mut fps = Vec::new();
        assert!(add_networks(&mut fps, &area, &s(&["a", "b"])));
        assert!(!add_networks(&mut fps, &area, &s(&["a"])));
        assert!(add_networks(&mut fps, &area, &s(&["c"])));
        assert_eq!(fps[0].bssids, s(&["a", "b", "c"]));
    }
}
