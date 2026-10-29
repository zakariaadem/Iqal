//! Location for location locks on Windows. Runs only while at least one
//! location lock exists; otherwise nothing is scanned or stored.
//!
//! The evidence is the learned Wi-Fi networks (see `places.rs`), from the
//! Wi-Fi adapters' scan lists. Windows only shows those while its Location
//! services switch is on. Windows' own position is not used: places are
//! learned when locking ("Is this computer at this place now?"). Without
//! evidence (Wi-Fi or Location off, no adapter) the machine counts as
//! inside: locks fail closed.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use iqal_core::geo::LocationFix;
use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::NetworkManagement::WiFi::{
    WLAN_BSS_LIST, WLAN_INTERFACE_INFO_LIST, WlanCloseHandle, WlanEnumInterfaces, WlanFreeMemory,
    WlanGetNetworkBssList, WlanOpenHandle, WlanScan, dot11_BSS_type_any,
};

use super::reg;
use crate::places;
use crate::service::{LocationState, Service};

const POLL: Duration = Duration::from_secs(20);
/// How long a fresh scan takes ("learn now").
const SCAN_WAIT: Duration = Duration::from_secs(4);
const LOCATION_CONSENT: &str = r"SOFTWARE\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\location";

pub fn start(service: Arc<Service>, state_dir: PathBuf) -> std::io::Result<()> {
    {
        let dir = state_dir.clone();
        service.set_learner(Box::new(move |area| {
            let visible = wifi_networks(false);
            if visible.is_empty() && location_services_off() {
                return Err(std::io::Error::other(
                    "Windows hides nearby Wi-Fi networks while Location services are off. Turn them on in \
                     Settings > Privacy & security > Location, then try again.",
                ));
            }
            places::learn_now_and_later(&dir, area, visible, || wifi_networks(true))
        }));
    }
    std::thread::Builder::new().name("iqal-location".into()).spawn(move || loop {
        let areas = service.lock_summary().location_areas;
        let state = if areas.is_empty() {
            LocationState::default()
        } else {
            places::assess(&state_dir, &areas, LocationFix::Unknown, &wifi_networks(false))
        };
        service.set_location(state);
        std::thread::sleep(POLL);
    })?;
    Ok(())
}

/// Since Windows 11 24H2, nearby networks are only listed while the
/// computer's Location services switch is on.
fn location_services_off() -> bool {
    reg::get_string(LOCATION_CONSENT, "Value").as_deref() == Some("Deny")
}

/// BSSIDs ("aa:bb:cc:dd:ee:ff") of the Wi-Fi networks in range, from every
/// Wi-Fi adapter. `fresh`: wait for a new scan first (learning a place).
pub fn wifi_networks(fresh: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut version = 0u32;
    let mut client: HANDLE = std::ptr::null_mut();
    // SAFETY: valid out pointers; client version 2 is Vista and later.
    if unsafe { WlanOpenHandle(2, std::ptr::null(), &mut version, &mut client) } != 0 {
        return out; // no Wi-Fi service (desktop without Wi-Fi)
    }
    let mut list: *mut WLAN_INTERFACE_INFO_LIST = std::ptr::null_mut();
    // SAFETY: client is open; list receives memory freed below.
    if unsafe { WlanEnumInterfaces(client, std::ptr::null(), &mut list) } == 0 {
        // SAFETY: WlanEnumInterfaces returned a list of dwNumberOfItems entries.
        let interfaces = unsafe {
            std::slice::from_raw_parts((*list).InterfaceInfo.as_ptr(), (*list).dwNumberOfItems as usize)
        };
        for interface in interfaces {
            // SAFETY: a known interface; no SSID or extra data.
            unsafe {
                WlanScan(client, &interface.InterfaceGuid, std::ptr::null(), std::ptr::null(), std::ptr::null())
            };
        }
        if fresh && !interfaces.is_empty() {
            std::thread::sleep(SCAN_WAIT);
        }
        for interface in interfaces {
            let mut bss: *mut WLAN_BSS_LIST = std::ptr::null_mut();
            // SAFETY: as above; bss receives memory freed below.
            let status = unsafe {
                WlanGetNetworkBssList(
                    client,
                    &interface.InterfaceGuid,
                    std::ptr::null(),
                    dot11_BSS_type_any,
                    0,
                    std::ptr::null(),
                    &mut bss,
                )
            };
            if status != 0 {
                continue;
            }
            // SAFETY: the list holds dwNumberOfItems entries.
            let entries = unsafe { std::slice::from_raw_parts((*bss).wlanBssEntries.as_ptr(), (*bss).dwNumberOfItems as usize) };
            // Weak networks count too: neighbours' networks make the evidence
            // much harder to fake than one's own router alone.
            for entry in entries {
                let b = entry.dot11Bssid;
                out.push(format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", b[0], b[1], b[2], b[3], b[4], b[5]));
            }
            // SAFETY: allocated by WlanGetNetworkBssList.
            unsafe { WlanFreeMemory(bss.cast()) };
        }
        // SAFETY: allocated by WlanEnumInterfaces.
        unsafe { WlanFreeMemory(list.cast()) };
    }
    // SAFETY: client is open.
    unsafe { WlanCloseHandle(client, std::ptr::null()) };
    out.sort();
    out.dedup();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Scans for real: on a machine with Wi-Fi on, networks in range are
    /// listed; without Wi-Fi the list is empty (never an error).
    #[test]
    fn scan_does_not_fail() {
        let networks = wifi_networks(false);
        assert!(networks.iter().all(|b| b.len() == 17 && b.split(':').count() == 6), "{networks:?}");
        eprintln!("{} Wi-Fi networks in range", networks.len());
    }
}
