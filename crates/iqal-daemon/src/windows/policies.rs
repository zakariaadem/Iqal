//! Browser configuration managed by the system (users cannot change it),
//! in the registry keys browsers read their policies from:
//!
//! * built-in DNS-over-HTTPS off, and the Chromium family's own DNS client
//!   off (either would bypass the DNS filter);
//! * while any block has website rules: no private windows in the Chromium
//!   family (a force-installed extension cannot be allowed there) and no
//!   guest mode.

use super::reg::{self, Value};

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
}

/// Removes everything `apply` added (uninstall).
pub fn remove() {
    for (key, private) in CHROMIUM {
        for name in ["DnsOverHttpsMode", "BuiltInDnsClientEnabled", private, "BrowserGuestModeEnabled"] {
            let _ = reg::delete_value(key, name);
        }
    }
    let _ = reg::delete_tree(FIREFOX_DOH);
}
