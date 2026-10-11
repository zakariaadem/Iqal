//! Browser configuration managed by the system (users cannot change it):
//!
//! * built-in DNS-over-HTTPS off (it would bypass the DNS filter);
//! * the Al-Dir extension force-installed, and its native host registered;
//! * while any block has website rules: private windows either covered by
//!   the extension (Firefox) or unavailable (Chromium family, where a
//!   force-installed extension cannot be allowed in incognito), and no
//!   guest mode.

use std::fs;
use std::path::Path;

use serde_json::{json, Map, Value};

use super::extension::{self, FIREFOX_ID, HOST_NAME};

/// (managed policy folder, native messaging host folder) per browser.
const CHROMIUM: &[(&str, &str)] = &[
    ("/etc/opt/chrome/policies/managed", "/etc/opt/chrome/native-messaging-hosts"),
    ("/etc/chromium/policies/managed", "/etc/chromium/native-messaging-hosts"),
    ("/etc/chromium-browser/policies/managed", "/etc/chromium-browser/native-messaging-hosts"),
    ("/etc/brave/policies/managed", "/etc/brave/native-messaging-hosts"),
    ("/etc/opt/edge/policies/managed", "/etc/opt/edge/native-messaging-hosts"),
    ("/etc/opt/vivaldi/policies/managed", "/etc/opt/vivaldi/native-messaging-hosts"),
];
const POLICY_FILE: &str = "al-dir.json";
const FIREFOX_POLICIES: &str = "/etc/firefox/policies/policies.json";
const FIREFOX_HOSTS: &str = "/usr/lib/mozilla/native-messaging-hosts";

/// Writes all policies. `strict`: some block has website rules.
pub fn apply(strict: bool) {
    let files = extension::files();
    let host_file = format!("{HOST_NAME}.json");

    let mut chromium = Map::new();
    chromium.insert("DnsOverHttpsMode".into(), json!("off"));
    if let Some(id) = &files.chromium_id {
        chromium.insert("ExtensionInstallForcelist".into(), json!([format!("{id};{}", extension::update_url())]));
    }
    if strict {
        chromium.insert("IncognitoModeAvailability".into(), json!(1));
        chromium.insert("BrowserGuestModeEnabled".into(), json!(false));
    }
    let chromium = pretty(&Value::Object(chromium));

    for (policy_dir, host_dir) in CHROMIUM {
        write_if_changed(&Path::new(policy_dir).join(POLICY_FILE), &chromium);
        if let (Some(host), Some(id)) = (&files.host, &files.chromium_id) {
            let manifest = json!({
                "name": HOST_NAME,
                "description": "Al-Dir",
                "path": host,
                "type": "stdio",
                "allowed_origins": [format!("chrome-extension://{id}/")],
            });
            write_if_changed(&Path::new(host_dir).join(&host_file), &pretty(&manifest));
        }
    }

    if let Some(host) = &files.host {
        let manifest = json!({
            "name": HOST_NAME,
            "description": "Al-Dir",
            "path": host,
            "type": "stdio",
            "allowed_extensions": [FIREFOX_ID],
        });
        write_if_changed(&Path::new(FIREFOX_HOSTS).join(&host_file), &pretty(&manifest));
    }
    update_firefox(|policies| {
        policies.insert("DNSOverHTTPS".into(), json!({ "Enabled": false, "Locked": true }));
        if let Some(xpi) = &files.xpi {
            let settings = policies.entry("ExtensionSettings").or_insert_with(|| json!({}));
            if let Some(settings) = settings.as_object_mut() {
                settings.insert(
                    FIREFOX_ID.into(),
                    json!({
                        "installation_mode": "force_installed",
                        "install_url": format!("file://{}", xpi.display()),
                        "private_browsing": true,
                    }),
                );
            }
        }
    });
}

/// Removes everything `apply` added (uninstall).
pub fn remove() {
    let host_file = format!("{HOST_NAME}.json");
    for (policy_dir, host_dir) in CHROMIUM {
        let _ = fs::remove_file(Path::new(policy_dir).join(POLICY_FILE));
        let _ = fs::remove_file(Path::new(host_dir).join(&host_file));
    }
    let _ = fs::remove_file(Path::new(FIREFOX_HOSTS).join(&host_file));
    update_firefox(|policies| {
        policies.remove("DNSOverHTTPS");
        let now_empty = policies
            .get_mut("ExtensionSettings")
            .and_then(Value::as_object_mut)
            .map(|s| {
                s.remove(FIREFOX_ID);
                s.is_empty()
            })
            .unwrap_or(false);
        if now_empty {
            policies.remove("ExtensionSettings");
        }
    });
}

fn pretty(v: &Value) -> String {
    serde_json::to_string_pretty(v).expect("json")
}

fn write_if_changed(path: &Path, text: &str) {
    if fs::read_to_string(path).ok().as_deref() == Some(text) {
        return;
    }
    let result = fs::create_dir_all(path.parent().expect("has parent")).and_then(|()| fs::write(path, text));
    if let Err(e) = result {
        eprintln!("al-dir: writing {}: {e}", path.display());
    }
}

/// Edits Firefox's policies.json in place, keeping policies set by others.
/// A file that is not valid JSON is left alone.
fn update_firefox(edit: impl FnOnce(&mut Map<String, Value>)) {
    let path = Path::new(FIREFOX_POLICIES);
    let original = fs::read_to_string(path).ok();
    let mut doc: Value = match &original {
        Some(text) => match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return eprintln!("al-dir: {FIREFOX_POLICIES} is not valid JSON; leaving it alone"),
        },
        None => json!({ "policies": {} }),
    };
    let Some(policies) = doc.get_mut("policies").and_then(Value::as_object_mut) else { return };
    edit(policies);
    if policies.is_empty() {
        if original.is_some() {
            let _ = fs::remove_file(path);
        }
        return;
    }
    let text = pretty(&doc);
    if original.as_deref() != Some(text.as_str()) {
        write_if_changed(path, &text);
    }
}
