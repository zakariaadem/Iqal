//! The browser extension's installed files, and a small loopback HTTP
//! server that Chromium-family browsers use to install and update it
//! (their force-install policy needs an update URL).
//!
//! Layout next to the service binary:
//! ```text
//! iqal-daemon
//! iqal-nm-host
//! extension/iqal.xpi      Firefox (signed by Mozilla)
//! extension/iqal.crx      Chromium family
//! extension/chromium-id
//! extension/version
//! ```

use std::io::{self, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;

pub const UPDATE_PORT: u16 = 16098;
pub const HOST_NAME: &str = "org.iqal.host";
/// The name used while Iqal was called Al-Dir. Extensions signed before the
/// rename still ask for it, so it is registered too.
pub const LEGACY_HOST_NAME: &str = "org.aldir.host";
pub const FIREFOX_ID: &str = "{6b0e3c7a-4f1d-4c8e-9a52-7d3e1f0b8a91}";

#[derive(Debug, Clone, Default)]
pub struct Files {
    pub host: Option<PathBuf>,
    pub xpi: Option<PathBuf>,
    pub crx: Option<PathBuf>,
    pub chromium_id: Option<String>,
    pub version: String,
}

static FILES: OnceLock<Files> = OnceLock::new();

pub fn files() -> &'static Files {
    FILES.get_or_init(locate)
}

fn locate() -> Files {
    let Some(dir) = std::env::current_exe().ok().and_then(|p| p.parent().map(PathBuf::from)) else {
        return Files::default();
    };
    let ext = dir.join("extension");
    let existing = |p: PathBuf| p.is_file().then_some(p);
    let read = |name: &str| std::fs::read_to_string(ext.join(name)).ok().map(|s| s.trim().to_string());
    let chromium_id = read("chromium-id").filter(|id| id.len() == 32 && id.bytes().all(|b| (b'a'..=b'p').contains(&b)));
    Files {
        host: existing(dir.join("iqal-nm-host")),
        xpi: existing(ext.join("iqal.xpi")),
        crx: existing(ext.join("iqal.crx")).filter(|_| chromium_id.is_some()),
        chromium_id,
        version: read("version").unwrap_or_else(|| "0.0.0".into()),
    }
}

pub fn update_url() -> String {
    format!("http://127.0.0.1:{UPDATE_PORT}/update.xml")
}

/// Serves `/update.xml` and `/iqal.crx` on 127.0.0.1.
pub fn serve_updates() -> io::Result<()> {
    let files = files();
    let (Some(crx), Some(id)) = (files.crx.clone(), files.chromium_id.clone()) else { return Ok(()) };
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, UPDATE_PORT))?;
    let manifest = format!(
        "<?xml version='1.0' encoding='UTF-8'?>\n\
         <gupdate xmlns='http://www.google.com/update2/response' protocol='2.0'>\n\
         \x20 <app appid='{id}'>\n\
         \x20   <updatecheck codebase='http://127.0.0.1:{UPDATE_PORT}/iqal.crx' version='{}' />\n\
         \x20 </app>\n\
         </gupdate>\n",
        files.version
    );
    std::thread::Builder::new().name("iqal-ext-http".into()).spawn(move || {
        for conn in listener.incoming().flatten() {
            let _ = respond(conn, &manifest, &crx);
        }
    })?;
    Ok(())
}

fn respond(mut conn: TcpStream, manifest: &str, crx: &PathBuf) -> io::Result<()> {
    conn.set_read_timeout(Some(Duration::from_secs(5)))?;
    let mut buf = [0u8; 4096];
    let n = conn.read(&mut buf)?;
    let request = String::from_utf8_lossy(&buf[..n]);
    let path = request.split_whitespace().nth(1).unwrap_or("").split('?').next().unwrap_or("");
    let (status, kind, body) = match path {
        "/update.xml" => ("200 OK", "text/xml", manifest.as_bytes().to_vec()),
        "/iqal.crx" => ("200 OK", "application/x-chrome-extension", std::fs::read(crx)?),
        _ => ("404 Not Found", "text/plain", b"not found".to_vec()),
    };
    write!(conn, "HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n", body.len())?;
    conn.write_all(&body)
}
