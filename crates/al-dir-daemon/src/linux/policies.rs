//! Browser policies (managed by the system, not changeable by the user).
//! For now: turn off each browser's built-in DNS-over-HTTPS, which would
//! otherwise bypass the DNS filter. The browser-extension milestone adds
//! more policies here.

use std::fs;
use std::path::Path;

use serde_json::{json, Value};

/// Managed-policy folders of Chromium-based browsers on Linux.
const CHROMIUM_DIRS: &[&str] = &[
    "/etc/opt/chrome/policies/managed",
    "/etc/chromium/policies/managed",
    "/etc/chromium-browser/policies/managed",
    "/etc/brave/policies/managed",
    "/etc/opt/edge/policies/managed",
    "/etc/opt/vivaldi/policies/managed",
];
const CHROMIUM_FILE: &str = "al-dir.json";
const FIREFOX_FILE: &str = "/etc/firefox/policies/policies.json";

fn chromium_policy() -> Value {
    json!({ "DnsOverHttpsMode": "off" })
}

fn firefox_policies() -> Value {
    json!({ "DNSOverHTTPS": { "Enabled": false, "Locked": true } })
}

/// Writes the policies; only touches files whose content differs.
pub fn apply() {
    let chromium = serde_json::to_string_pretty(&chromium_policy()).expect("static json");
    for dir in CHROMIUM_DIRS {
        let path = Path::new(dir).join(CHROMIUM_FILE);
        if fs::read_to_string(&path).ok().as_deref() != Some(chromium.as_str()) {
            let result = fs::create_dir_all(dir).and_then(|()| fs::write(&path, &chromium));
            if let Err(e) = result {
                eprintln!("al-dir: writing {}: {e}", path.display());
            }
        }
    }
    update_firefox(|policies| {
        for (k, v) in firefox_policies().as_object().expect("object") {
            policies.insert(k.clone(), v.clone());
        }
    });
}

/// Removes everything `apply` added (used when uninstalling).
pub fn remove() {
    for dir in CHROMIUM_DIRS {
        let _ = fs::remove_file(Path::new(dir).join(CHROMIUM_FILE));
    }
    update_firefox(|policies| {
        for k in firefox_policies().as_object().expect("object").keys() {
            policies.remove(k);
        }
    });
}

/// Edits Firefox's policies.json in place, keeping policies set by others.
/// A file that is not valid JSON is left alone.
fn update_firefox(edit: impl FnOnce(&mut serde_json::Map<String, Value>)) {
    let path = Path::new(FIREFOX_FILE);
    let original = fs::read_to_string(path).ok();
    let mut doc: Value = match &original {
        Some(text) => match serde_json::from_str(text) {
            Ok(v) => v,
            Err(_) => return eprintln!("al-dir: {FIREFOX_FILE} is not valid JSON; leaving it alone"),
        },
        None => json!({ "policies": {} }),
    };
    let Some(policies) = doc.get_mut("policies").and_then(Value::as_object_mut) else { return };
    edit(policies);
    let empty = policies.is_empty();
    if empty {
        if original.is_some() {
            let _ = fs::remove_file(path);
        }
        return;
    }
    let text = serde_json::to_string_pretty(&doc).expect("json");
    if original.as_deref() != Some(text.as_str()) {
        let result = fs::create_dir_all(path.parent().expect("has parent")).and_then(|()| fs::write(path, text));
        if let Err(e) = result {
            eprintln!("al-dir: writing {FIREFOX_FILE}: {e}");
        }
    }
}
