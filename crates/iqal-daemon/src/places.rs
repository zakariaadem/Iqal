//! Learned Wi-Fi networks for location locks, shared by every platform
//! (the platform scans; this remembers and judges).
//!
//! The networks around a place are remembered ("I am here now", or
//! automatically while a precise position says the machine is there).
//! Any of them in range: *inside*. Wi-Fi working and several networks in
//! range but none of them: *away*. Neighbours' networks count too, so
//! switching off one router does not fake "away". Without evidence the
//! machine counts as inside: locks fail closed.

use std::path::Path;
use std::sync::Mutex;

use iqal_core::geo::{GeoArea, LocationFix, Relation};
use serde::{Deserialize, Serialize};

use crate::service::LocationState;

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

/// "I am at this place now": adds the networks in range to the area.
/// Returns how many networks the place now has.
pub fn learn(state_dir: &Path, area: &GeoArea, visible: &[String]) -> std::io::Result<usize> {
    if visible.is_empty() {
        return Err(std::io::Error::other("no Wi-Fi networks in range (is Wi-Fi on?)"));
    }
    let _guard = FILE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut fps = load(state_dir);
    add_networks(&mut fps, area, visible);
    save(state_dir, &fps);
    Ok(fps.iter().find(|f| f.area.as_ref() == Some(area)).map_or(0, |f| f.bssids.len()))
}

/// Where the machine is, for each area, from a position (if any) and the
/// networks in range. Networks seen while the position is inside an area
/// are learned for it.
pub fn assess(state_dir: &Path, areas: &[GeoArea], fix: LocationFix, visible: &[String]) -> LocationState {
    let _guard = FILE_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let mut fps = load(state_dir);
    let mut changed = false;
    for area in areas {
        if area.relation(fix) == Relation::Inside {
            changed |= add_networks(&mut fps, area, visible);
        }
    }
    if changed {
        save(state_dir, &fps);
    }
    let mut state = LocationState { fix, ..LocationState::default() };
    for area in areas {
        let learned = fps.iter().find(|f| f.area.as_ref() == Some(area)).map_or(&[][..], |f| &f.bssids);
        match wifi_evidence(learned, visible) {
            WifiEvidence::Inside => state.inside.push(area.clone()),
            WifiEvidence::Away => state.outside.push(area.clone()),
            WifiEvidence::None => {}
        }
    }
    state
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

fn load(dir: &Path) -> Vec<Fingerprint> {
    std::fs::read_to_string(dir.join(FINGERPRINTS)).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save(dir: &Path, fps: &[Fingerprint]) {
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

    #[test]
    fn learn_then_assess() {
        let dir = std::env::temp_dir().join(format!("iqal-places-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let area = GeoArea::Circle { lat: 1.0, lon: 2.0, radius_m: 300.0 };
        assert!(learn(&dir, &area, &[]).is_err(), "Wi-Fi off: nothing to learn");
        assert_eq!(learn(&dir, &area, &s(&["a", "b"])).unwrap(), 2);
        let here = assess(&dir, std::slice::from_ref(&area), LocationFix::Unknown, &s(&["b", "q"]));
        assert_eq!(here.inside, vec![area.clone()]);
        let away = assess(&dir, std::slice::from_ref(&area), LocationFix::Unknown, &s(&["x", "y"]));
        assert_eq!(away.outside, vec![area]);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
