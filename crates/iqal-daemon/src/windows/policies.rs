//! Browser configuration managed by the system (users cannot change it),
//! in the registry keys browsers read their policies from:
//!
//! * built-in DNS-over-HTTPS off, and the Chromium family's own DNS client
//!   off (either would bypass the DNS filter);
//! * while any block has website rules: no private windows in the Chromium
//!   family (a force-installed extension cannot be allowed there) and no
//!   guest mode;
//! * the Iqal extension's native host registered for every browser, and
//!   the extension force-installed: in Firefox from the signed file next
//!   to the program (private windows included), in Chrome, Brave, Vivaldi
//!   and Edge from their stores (the only source those browsers accept on
//!   computers that are not centrally managed).

use std::path::Path;

use serde_json::{Map, Value as Json, json};

use super::reg::{self, Value};
use crate::extension::{
    self, CHROME_STORE_ID, CHROME_STORE_UPDATE_URL, EDGE_STORE_ID, EDGE_STORE_UPDATE_URL, FIREFOX_ID, HOST_NAME,
    LEGACY_HOST_NAME,
};

/// Both names are registered (see `LEGACY_HOST_NAME`).
const HOST_NAMES: [&str; 2] = [HOST_NAME, LEGACY_HOST_NAME];
/// Where each browser looks for native hosts.
const FIREFOX_HOSTS: &str = r"SOFTWARE\Mozilla\NativeMessagingHosts";
const CHROMIUM_HOSTS: &[&str] = &[
    r"SOFTWARE\Google\Chrome\NativeMessagingHosts",
    r"SOFTWARE\Chromium\NativeMessagingHosts",
    r"SOFTWARE\BraveSoftware\Brave-Browser\NativeMessagingHosts",
    r"SOFTWARE\Microsoft\Edge\NativeMessagingHosts",
];
const FIREFOX_POLICIES: &str = r"SOFTWARE\Policies\Mozilla\Firefox";

/// Chromium-family policy keys, with each one's name for the private
/// window policy (Edge calls it InPrivate).
const CHROMIUM: &[(&str, &str)] = &[
    (r"SOFTWARE\Policies\Google\Chrome", "IncognitoModeAvailability"),
    (r"SOFTWARE\Policies\Chromium", "IncognitoModeAvailability"),
    (r"SOFTWARE\Policies\BraveSoftware\Brave", "IncognitoModeAvailability"),
    (r"SOFTWARE\Policies\Microsoft\Edge", "InPrivateModeAvailability"),
    (r"SOFTWARE\Policies\Vivaldi", "IncognitoModeAvailability"),
];
const FIREFOX_DOH: &str = r"SOFTWARE\Policies\Mozilla\Firefox\DNSOverHTTPS";

/// Writes all policies. `strict`: some block has website rules.
pub fn apply(strict: bool) {
    for (key, private) in CHROMIUM {
        // Without their own DNS client, these browsers look names up
        // through Windows, and so through the DNS filter.
        let mut result =
            reg::set(key, &[("DnsOverHttpsMode", Value::Str("off")), ("BuiltInDnsClientEnabled", Value::Dword(0))]);
        if strict {
            result = result.and(reg::set(key, &[(private, Value::Dword(1)), ("BrowserGuestModeEnabled", Value::Dword(0))]));
        } else {
            result = result
                .and(reg::delete_value(key, private))
                .and(reg::delete_value(key, "BrowserGuestModeEnabled"));
        }
        if let Err(e) = result {
            eprintln!("iqal: browser policies in {key}: {e}");
        }
    }
    if let Err(e) = reg::set(FIREFOX_DOH, &[("Enabled", Value::Dword(0)), ("Locked", Value::Dword(1))]) {
        eprintln!("iqal: Firefox policies: {e}");
    }
    register_hosts();
    install_in_firefox();
    for (key, id, url) in FORCE_INSTALL {
        if let Err(e) = edit_forcelist(key, id, Some(url)) {
            eprintln!("iqal: extension install policy in {key}: {e}");
        }
    }
}

/// Browsers that install the extension from a store: (policy key, the
/// extension's id there, the store's update address).
const FORCE_INSTALL: &[(&str, &str, &str)] = &[
    (r"SOFTWARE\Policies\Google\Chrome", CHROME_STORE_ID, CHROME_STORE_UPDATE_URL),
    (r"SOFTWARE\Policies\BraveSoftware\Brave", CHROME_STORE_ID, CHROME_STORE_UPDATE_URL),
    (r"SOFTWARE\Policies\Vivaldi", CHROME_STORE_ID, CHROME_STORE_UPDATE_URL),
    (r"SOFTWARE\Policies\Microsoft\Edge", EDGE_STORE_ID, EDGE_STORE_UPDATE_URL),
];

/// Puts our entry ("id;update url") into a browser's
/// ExtensionInstallForcelist, or takes it out (`url` None). The list is
/// values named 1, 2, 3...; entries of others are kept, in order.
fn edit_forcelist(policy_key: &str, id: &str, url: Option<&str>) -> std::io::Result<()> {
    let list_key = format!(r"{policy_key}\ExtensionInstallForcelist");
    let current: Vec<String> =
        (1..).map_while(|i| reg::get_string(&list_key, &i.to_string())).collect();
    let mut wanted: Vec<String> =
        current.iter().filter(|e| e.split(';').next() != Some(id)).cloned().collect();
    if let Some(url) = url {
        wanted.push(format!("{id};{url}"));
    }
    if wanted == current {
        return Ok(());
    }
    reg::delete_tree(&list_key)?;
    if wanted.is_empty() {
        return Ok(());
    }
    let names: Vec<String> = (1..=wanted.len()).map(|i| i.to_string()).collect();
    let values: Vec<(&str, Value)> =
        names.iter().zip(&wanted).map(|(name, entry)| (name.as_str(), Value::Str(entry))).collect();
    reg::set(&list_key, &values)
}

/// Native host manifests next to the program (browsers read them as the
/// user, so they cannot live in the private settings folder), and the
/// registry entries pointing to them.
fn register_hosts() {
    let files = extension::files();
    let Some(host) = &files.host else { return };
    let Some(dir) = host.parent() else { return };
    for name in HOST_NAMES {
        let firefox = json!({
            "name": name,
            "description": "Iqal",
            "path": host,
            "type": "stdio",
            "allowed_extensions": [FIREFOX_ID],
        });
        register(&dir.join(format!("{name}.firefox.json")), &firefox, &[FIREFOX_HOSTS], name);
        // The store versions, and our own signed one (development).
        let ids = [CHROME_STORE_ID, EDGE_STORE_ID].into_iter().chain(files.chromium_id.as_deref());
        let chromium = json!({
            "name": name,
            "description": "Iqal",
            "path": host,
            "type": "stdio",
            "allowed_origins": ids.map(|id| format!("chrome-extension://{id}/")).collect::<Vec<_>>(),
        });
        register(&dir.join(format!("{name}.chromium.json")), &chromium, CHROMIUM_HOSTS, name);
    }
}

fn register(manifest: &Path, content: &Json, keys: &[&str], name: &str) {
    let text = serde_json::to_string_pretty(content).expect("json");
    if std::fs::read_to_string(manifest).ok().as_deref() != Some(text.as_str())
        && let Err(e) = std::fs::write(manifest, &text)
    {
        return eprintln!("iqal: writing {}: {e}", manifest.display());
    }
    let path = manifest.to_string_lossy();
    for key in keys {
        if let Err(e) = reg::set(&format!(r"{key}\{name}"), &[("", Value::Str(&path))]) {
            eprintln!("iqal: registering the native host in {key}: {e}");
        }
    }
}

/// Firefox's ExtensionSettings policy (JSON in the registry), keeping
/// settings made by others.
fn install_in_firefox() {
    let Some(xpi) = &extension::files().xpi else { return };
    let url = format!("file:///{}", xpi.to_string_lossy().replace('\\', "/").replace(' ', "%20"));
    edit_firefox_extensions(|settings| {
        settings.insert(
            FIREFOX_ID.into(),
            json!({ "installation_mode": "force_installed", "install_url": url, "private_browsing": true }),
        );
    });
}

fn edit_firefox_extensions(edit: impl FnOnce(&mut Map<String, Json>)) {
    let original = reg::get_string(FIREFOX_POLICIES, "ExtensionSettings");
    let mut settings = match &original {
        Some(text) => match serde_json::from_str::<Json>(text) {
            Ok(Json::Object(map)) => map,
            _ => return eprintln!("iqal: Firefox's ExtensionSettings policy is not valid JSON; leaving it alone"),
        },
        None => Map::new(),
    };
    edit(&mut settings);
    let result = if settings.is_empty() {
        reg::delete_value(FIREFOX_POLICIES, "ExtensionSettings")
    } else {
        let text = Json::Object(settings).to_string();
        if original.as_deref() == Some(text.as_str()) {
            return;
        }
        reg::set(FIREFOX_POLICIES, &[("ExtensionSettings", Value::Str(&text))])
    };
    if let Err(e) = result {
        eprintln!("iqal: Firefox extension policy: {e}");
    }
}

/// Removes everything `apply` added (uninstall).
pub fn remove() {
    for (key, private) in CHROMIUM {
        for name in ["DnsOverHttpsMode", "BuiltInDnsClientEnabled", private, "BrowserGuestModeEnabled"] {
            let _ = reg::delete_value(key, name);
        }
    }
    let _ = reg::delete_tree(FIREFOX_DOH);
    edit_firefox_extensions(|settings| {
        settings.remove(FIREFOX_ID);
    });
    for (key, id, _) in FORCE_INSTALL {
        let _ = edit_forcelist(key, id, None);
    }
    for name in HOST_NAMES {
        for key in std::iter::once(&FIREFOX_HOSTS).chain(CHROMIUM_HOSTS) {
            let _ = reg::delete_tree(&format!(r"{key}\{name}"));
        }
        if let Some(dir) = extension::files().host.as_ref().and_then(|h| h.parent()) {
            let _ = std::fs::remove_file(dir.join(format!("{name}.firefox.json")));
            let _ = std::fs::remove_file(dir.join(format!("{name}.chromium.json")));
        }
    }
}
