//! Desktop notifications (org.freedesktop.Notifications over D-Bus).

use std::collections::HashMap;

use zbus::blocking::Connection;
use zbus::zvariant::Value;

pub struct Desktop {
    conn: Connection,
}

impl Desktop {
    pub fn connect() -> Option<Self> {
        Connection::session().ok().map(|conn| Self { conn })
    }

    pub fn show(&self, summary: &str, body: &str) -> zbus::Result<()> {
        let actions: Vec<&str> = Vec::new();
        // Shown with Iqal's icon and grouped under the app.
        let hints: HashMap<&str, Value<'_>> = HashMap::from([("desktop-entry", Value::from("iqal"))]);
        self.conn.call_method(
            Some("org.freedesktop.Notifications"),
            "/org/freedesktop/Notifications",
            Some("org.freedesktop.Notifications"),
            "Notify",
            &("Iqal Blocker", 0u32, "iqal", summary, body, actions, hints, 6000i32),
        )?;
        Ok(())
    }
}
