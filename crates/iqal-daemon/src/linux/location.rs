//! Location for location locks. Runs only while at least one location lock
//! exists; otherwise nothing is looked up or stored.
//!
//! Evidence, strongest first:
//!
//! * **Learned Wi-Fi networks** (see `places.rs`), from NetworkManager's
//!   scan list.
//! * **OS location service (GeoClue)**, with its Wi-Fi lookup pointed at
//!   BeaconDB, the community successor of Mozilla's retired service. Only
//!   positions more precise than 1 km count (not IP-based guesses, which a
//!   VPN can fake). Positions inside an area also teach it its networks.
//!
//! Without evidence the machine counts as inside: locks fail closed.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use iqal_core::geo::{GeoArea, LocationFix};
use zbus::blocking::{Connection, Proxy};
use zbus::proxy::CacheProperties;
use zbus::zvariant::{OwnedObjectPath, Value};

use crate::places;
use crate::service::{LocationState, Service};

const GEOCLUE_CONF: &str = "/etc/geoclue/conf.d/90-iqal.conf";
const GEOCLUE_CONF_TEXT: &str = "# Managed by Iqal.\n\
[iqal]\nallowed=true\nsystem=true\nusers=\n\n\
[wifi]\nenable=true\nurl=https://api.beacondb.net/v1/geolocate\n";
const DESKTOP_ID: &str = "iqal";
const POLL: Duration = Duration::from_secs(20);
const MAX_ACCURACY_M: f64 = 1000.0;

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
    places::learn(state_dir, area, &wifi_networks(&conn, true))
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
        places::assess(&self.state_dir, areas, fix, &visible)
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
        let last_scan = || wireless.get_property::<i64>("LastScan").ok();
        let before = last_scan();
        let mut aps = Vec::new();
        if fresh {
            // While a scan runs, the list can hold only the network in use:
            // keep what is known now, and wait for the scan to finish.
            aps.extend(wireless.call::<_, _, Vec<OwnedObjectPath>>("GetAllAccessPoints", &()).unwrap_or_default());
        }
        let _ = wireless.call_method("RequestScan", &(HashMap::<&str, Value<'_>>::new(),));
        if fresh {
            for _ in 0..30 {
                std::thread::sleep(Duration::from_millis(500));
                if last_scan() != before {
                    break;
                }
            }
        }
        aps.extend(wireless.call::<_, _, Vec<OwnedObjectPath>>("GetAllAccessPoints", &()).unwrap_or_default());
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
