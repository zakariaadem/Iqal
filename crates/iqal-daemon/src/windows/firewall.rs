//! Firewall rules (Windows Filtering Platform) that stop programs from
//! going around the DNS filter, while any website rules exist:
//!
//! * DNS (TCP/UDP port 53) is refused to every program except our own
//!   service, which relays allowed lookups, and except to this computer
//!   itself (127.0.0.1 / ::1, where the DNS filter listens);
//! * DNS-over-TLS/QUIC (port 853) is refused;
//! * HTTPS/QUIC to well-known DNS-over-HTTPS servers is refused.
//!
//! The rules live in a dynamic WFP session: Windows deletes them by itself
//! as soon as the service ends, so a crash never leaves the computer
//! without internet.

use std::io;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::sync::Arc;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::NetworkManagement::WindowsFilteringPlatform::{
    FWP_ACTION_BLOCK, FWP_ACTION_PERMIT, FWP_BYTE_BLOB, FWP_BYTE_BLOB_TYPE, FWP_CONDITION_FLAG_IS_LOOPBACK,
    FWP_CONDITION_VALUE0, FWP_CONDITION_VALUE0_0, FWP_MATCH_EQUAL, FWP_MATCH_FLAGS_ALL_SET, FWP_UINT8, FWP_UINT16,
    FWP_UINT32, FWP_V4_ADDR_AND_MASK, FWP_V4_ADDR_MASK, FWP_V6_ADDR_AND_MASK, FWP_V6_ADDR_MASK, FWP_VALUE0,
    FWP_VALUE0_0, FWPM_ACTION0, FWPM_CONDITION_ALE_APP_ID, FWPM_CONDITION_FLAGS, FWPM_CONDITION_IP_PROTOCOL,
    FWPM_CONDITION_IP_REMOTE_ADDRESS, FWPM_CONDITION_IP_REMOTE_PORT, FWPM_DISPLAY_DATA0, FWPM_FILTER_CONDITION0,
    FWPM_FILTER0, FWPM_LAYER_ALE_AUTH_CONNECT_V4, FWPM_LAYER_ALE_AUTH_CONNECT_V6, FWPM_PROVIDER0,
    FWPM_SESSION_FLAG_DYNAMIC, FWPM_SESSION0, FWPM_SUBLAYER0, FwpmEngineClose0, FwpmEngineOpen0, FwpmFilterAdd0,
    FwpmFilterGetById0, FwpmFreeMemory0, FwpmGetAppIdFromFileName0, FwpmProviderAdd0, FwpmSubLayerAdd0,
    FwpmTransactionAbort0, FwpmTransactionBegin0, FwpmTransactionCommit0,
};
use windows_sys::Win32::Networking::WinSock::{IPPROTO_TCP, IPPROTO_UDP};
use windows_sys::core::GUID;

use crate::dnsfilter::{DOH_V4, DOH_V6};
use crate::service::Service;

const PROVIDER: GUID = GUID::from_u128(0x6b1f3c52_9a4e_4d71_b0a8_2c5e7d9f1a01);
const SUBLAYER: GUID = GUID::from_u128(0x6b1f3c52_9a4e_4d71_b0a8_2c5e7d9f1a02);
/// Windows' own authentication for the (local) filtering engine.
const RPC_C_AUTHN_WINNT: u32 = 10;
/// Within our sublayer the heavier filter wins: the exceptions outweigh
/// the blocks.
const BLOCK_WEIGHT: u8 = 1;
const PERMIT_WEIGHT: u8 = 10;
/// How often the rules are checked for having been removed by someone.
const CHECK_EVERY: Duration = Duration::from_secs(30);

/// Applies the rules while website rules exist. Only call this once the
/// DNS filter runs: without it, refusing outside DNS servers would leave
/// the computer unable to look up any name.
pub fn start(service: Arc<Service>) -> io::Result<()> {
    std::thread::Builder::new().name("iqal-firewall".into()).spawn(move || {
        let mut rules: Option<Rules> = None;
        let mut last_try: Option<Instant> = None;
        let mut last_check = Instant::now();
        loop {
            let wanted = service.engine().has_any_web_rules();
            if !wanted {
                // Closing the session removes every filter.
                rules = None;
                last_try = None;
            } else if let Some(r) = &rules
                && last_check.elapsed() >= CHECK_EVERY
            {
                last_check = Instant::now();
                if !r.intact() {
                    eprintln!("iqal: firewall rules were removed; restoring them");
                    rules = None;
                    last_try = None;
                }
            }
            // At once when website rules appear; after a failure, now and then.
            if wanted && rules.is_none() && last_try.is_none_or(|t| t.elapsed() >= CHECK_EVERY) {
                last_try = Some(Instant::now());
                match Rules::apply() {
                    Ok(r) => {
                        rules = Some(r);
                        last_check = Instant::now();
                    }
                    Err(e) => eprintln!("iqal: firewall rules unavailable ({e}); DNS bypasses are possible"),
                }
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    })?;
    Ok(())
}

/// An open dynamic session holding our filters; dropping it removes them.
struct Rules {
    engine: HANDLE,
    filters: Vec<u64>,
}

// SAFETY: the engine handle may be used from any thread.
unsafe impl Send for Rules {}

impl Drop for Rules {
    fn drop(&mut self) {
        // SAFETY: a handle we opened and close once.
        unsafe { FwpmEngineClose0(self.engine) };
    }
}

impl Rules {
    fn apply() -> io::Result<Self> {
        let mut session = FWPM_SESSION0 { flags: FWPM_SESSION_FLAG_DYNAMIC, ..Default::default() };
        let mut name = wide("Iqal");
        session.displayData.name = name.as_mut_ptr();
        let mut engine: HANDLE = null_mut();
        // SAFETY: valid session and out pointer; `name` outlives the call.
        check("opening the filtering engine", unsafe {
            FwpmEngineOpen0(null(), RPC_C_AUTHN_WINNT, null(), &session, &mut engine)
        })?;
        let mut me = Self { engine, filters: Vec::new() };
        let app_id = AppId::of_this_program()?;
        // SAFETY: an open engine handle.
        check("starting a transaction", unsafe { FwpmTransactionBegin0(engine, 0) })?;
        match me.add_all(&app_id) {
            // SAFETY: an open engine handle with a transaction in progress.
            Ok(()) => check("saving the rules", unsafe { FwpmTransactionCommit0(engine) })?,
            Err(e) => {
                // SAFETY: as above.
                unsafe { FwpmTransactionAbort0(engine) };
                return Err(e);
            }
        }
        Ok(me)
    }

    fn add_all(&mut self, app_id: &AppId) -> io::Result<()> {
        let mut provider_name = wide("Iqal");
        let provider = FWPM_PROVIDER0 {
            providerKey: PROVIDER,
            displayData: FWPM_DISPLAY_DATA0 { name: provider_name.as_mut_ptr(), description: null_mut() },
            ..Default::default()
        };
        // SAFETY: a valid provider; its name outlives the call.
        check("adding the provider", unsafe { FwpmProviderAdd0(self.engine, &provider, null_mut()) })?;
        let mut provider_key = PROVIDER;
        let mut sublayer_name = wide("Iqal DNS protection");
        let sublayer = FWPM_SUBLAYER0 {
            subLayerKey: SUBLAYER,
            displayData: FWPM_DISPLAY_DATA0 { name: sublayer_name.as_mut_ptr(), description: null_mut() },
            providerKey: &mut provider_key,
            weight: 0x8000,
            ..Default::default()
        };
        // SAFETY: a valid sublayer; what it points to outlives the call.
        check("adding the sublayer", unsafe { FwpmSubLayerAdd0(self.engine, &sublayer, null_mut()) })?;

        let doh4: Vec<_> = DOH_V4.iter().filter_map(|a| v4_net(a)).collect();
        let doh6: Vec<_> = DOH_V6.iter().filter_map(|a| v6_net(a)).collect();
        for (layer, v4) in [(FWPM_LAYER_ALE_AUTH_CONNECT_V4, true), (FWPM_LAYER_ALE_AUTH_CONNECT_V6, false)] {
            self.add(layer, "Iqal: block outside DNS", FWP_ACTION_BLOCK, Conditions::port(53))?;
            let mut own = Conditions::port(53);
            own.app_id(app_id);
            self.add(layer, "Iqal: allow its own DNS relay", FWP_ACTION_PERMIT, own)?;
            let mut local = Conditions::port(53);
            local.loopback();
            self.add(layer, "Iqal: allow DNS on this computer", FWP_ACTION_PERMIT, local)?;
            self.add(layer, "Iqal: block DNS over TLS", FWP_ACTION_BLOCK, Conditions::port(853))?;
            let mut doh = Conditions::port(443);
            if v4 {
                doh4.iter().for_each(|&(addr, mask)| doh.v4(addr, mask));
            } else {
                doh6.iter().for_each(|&(addr, prefix)| doh.v6(addr, prefix));
            }
            self.add(layer, "Iqal: block DNS over HTTPS", FWP_ACTION_BLOCK, doh)?;
        }
        Ok(())
    }

    fn add(&mut self, layer: GUID, name: &str, action: u32, mut conditions: Conditions) -> io::Result<()> {
        let mut name = wide(name);
        let mut provider_key = PROVIDER;
        let weight = if action == FWP_ACTION_PERMIT { PERMIT_WEIGHT } else { BLOCK_WEIGHT };
        let filter = FWPM_FILTER0 {
            displayData: FWPM_DISPLAY_DATA0 { name: name.as_mut_ptr(), description: null_mut() },
            providerKey: &mut provider_key,
            layerKey: layer,
            subLayerKey: SUBLAYER,
            weight: FWP_VALUE0 { r#type: FWP_UINT8, Anonymous: FWP_VALUE0_0 { uint8: weight } },
            numFilterConditions: conditions.list.len() as u32,
            filterCondition: conditions.list.as_mut_ptr(),
            action: FWPM_ACTION0 { r#type: action, ..Default::default() },
            ..Default::default()
        };
        let mut id = 0;
        // SAFETY: a valid filter; everything it points to (name, key,
        // conditions and their values) outlives the call.
        check(name_of(&name), unsafe { FwpmFilterAdd0(self.engine, &filter, null_mut(), &mut id) })?;
        self.filters.push(id);
        Ok(())
    }

    /// Whether all our filters are still there.
    fn intact(&self) -> bool {
        self.filters.iter().all(|&id| {
            let mut filter: *mut FWPM_FILTER0 = null_mut();
            // SAFETY: an open engine handle and a valid out pointer.
            let found = unsafe { FwpmFilterGetById0(self.engine, id, &mut filter) } == 0;
            if found {
                // SAFETY: memory Windows handed us, freed once.
                unsafe { FwpmFreeMemory0(&mut filter as *mut _ as *mut *mut core::ffi::c_void) };
            }
            found
        })
    }
}

/// Filter conditions and the values they point to. Conditions on the same
/// field match if any of them does; different fields must all match.
struct Conditions {
    list: Vec<FWPM_FILTER_CONDITION0>,
    v4: Vec<Box<FWP_V4_ADDR_AND_MASK>>,
    v6: Vec<Box<FWP_V6_ADDR_AND_MASK>>,
}

impl Conditions {
    /// TCP or UDP to the given remote port.
    fn port(port: u16) -> Self {
        let mut me = Self { list: Vec::new(), v4: Vec::new(), v6: Vec::new() };
        for proto in [IPPROTO_TCP, IPPROTO_UDP] {
            me.push(
                FWPM_CONDITION_IP_PROTOCOL,
                FWP_MATCH_EQUAL,
                FWP_UINT8,
                FWP_CONDITION_VALUE0_0 { uint8: proto as u8 },
            );
        }
        me.push(FWPM_CONDITION_IP_REMOTE_PORT, FWP_MATCH_EQUAL, FWP_UINT16, FWP_CONDITION_VALUE0_0 { uint16: port });
        me
    }

    fn app_id(&mut self, app_id: &AppId) {
        let value = FWP_CONDITION_VALUE0_0 { byteBlob: app_id.0 };
        self.push(FWPM_CONDITION_ALE_APP_ID, FWP_MATCH_EQUAL, FWP_BYTE_BLOB_TYPE, value);
    }

    fn loopback(&mut self) {
        let value = FWP_CONDITION_VALUE0_0 { uint32: FWP_CONDITION_FLAG_IS_LOOPBACK };
        self.push(FWPM_CONDITION_FLAGS, FWP_MATCH_FLAGS_ALL_SET, FWP_UINT32, value);
    }

    fn v4(&mut self, addr: u32, mask: u32) {
        let mut value = Box::new(FWP_V4_ADDR_AND_MASK { addr, mask });
        let ptr = &mut *value as *mut _;
        self.v4.push(value);
        self.push(
            FWPM_CONDITION_IP_REMOTE_ADDRESS,
            FWP_MATCH_EQUAL,
            FWP_V4_ADDR_MASK,
            FWP_CONDITION_VALUE0_0 { v4AddrMask: ptr },
        );
    }

    fn v6(&mut self, addr: [u8; 16], prefix: u8) {
        let mut value = Box::new(FWP_V6_ADDR_AND_MASK { addr, prefixLength: prefix });
        let ptr = &mut *value as *mut _;
        self.v6.push(value);
        self.push(
            FWPM_CONDITION_IP_REMOTE_ADDRESS,
            FWP_MATCH_EQUAL,
            FWP_V6_ADDR_MASK,
            FWP_CONDITION_VALUE0_0 { v6AddrMask: ptr },
        );
    }

    fn push(&mut self, field: GUID, match_type: i32, kind: i32, value: FWP_CONDITION_VALUE0_0) {
        self.list.push(FWPM_FILTER_CONDITION0 {
            fieldKey: field,
            matchType: match_type,
            conditionValue: FWP_CONDITION_VALUE0 { r#type: kind, Anonymous: value },
        });
    }
}

/// How the firewall names this program (iqal-daemon.exe).
struct AppId(*mut FWP_BYTE_BLOB);

impl AppId {
    fn of_this_program() -> io::Result<Self> {
        let path = wide(std::env::current_exe()?.as_os_str());
        let mut blob: *mut FWP_BYTE_BLOB = null_mut();
        // SAFETY: a NUL-terminated path and a valid out pointer.
        check("naming this program", unsafe { FwpmGetAppIdFromFileName0(path.as_ptr(), &mut blob) })?;
        Ok(Self(blob))
    }
}

impl Drop for AppId {
    fn drop(&mut self) {
        // SAFETY: memory Windows handed us, freed once.
        unsafe { FwpmFreeMemory0(&mut self.0 as *mut _ as *mut *mut core::ffi::c_void) };
    }
}

/// "1.2.3.4" or "1.2.3.0/24" as address and mask (host byte order, as
/// WFP wants them).
fn v4_net(s: &str) -> Option<(u32, u32)> {
    let (addr, bits) = s.split_once('/').unwrap_or((s, "32"));
    let bits: u32 = bits.parse().ok().filter(|b| *b <= 32)?;
    let mask = if bits == 0 { 0 } else { u32::MAX << (32 - bits) };
    Some((u32::from(addr.parse::<Ipv4Addr>().ok()?) & mask, mask))
}

/// "2620:fe::fe" or "2a07:a8c0::/32" as address and prefix length.
fn v6_net(s: &str) -> Option<([u8; 16], u8)> {
    let (addr, bits) = s.split_once('/').unwrap_or((s, "128"));
    let bits: u8 = bits.parse().ok().filter(|b| *b <= 128)?;
    Some((addr.parse::<Ipv6Addr>().ok()?.octets(), bits))
}

fn wide(s: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    s.as_ref().encode_wide().chain(std::iter::once(0)).collect()
}

fn name_of(wide: &[u16]) -> String {
    String::from_utf16_lossy(&wide[..wide.len() - 1])
}

fn check(what: impl std::fmt::Display, code: u32) -> io::Result<()> {
    if code == 0 { Ok(()) } else { Err(io::Error::other(format!("{what}: error 0x{code:08x}"))) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn networks() {
        assert_eq!(v4_net("1.1.1.1"), Some((0x0101_0101, u32::MAX)));
        assert_eq!(v4_net("76.76.2.0/24"), Some((0x4C4C_0200, 0xFFFF_FF00)));
        assert_eq!(v6_net("2a07:a8c0::/32").map(|(_, p)| p), Some(32));
        assert!(DOH_V4.iter().all(|a| v4_net(a).is_some()));
        assert!(DOH_V6.iter().all(|a| v6_net(a).is_some()));
    }
}
