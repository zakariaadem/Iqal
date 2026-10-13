//! iqal-nm-host: started by the browser for the Iqal extension
//! (native messaging), and relays between the two:
//!
//! * extension → host: `{"type":"hello"}`, `{"type":"check","id":N,"url":"..."}`
//! * host → extension: `{"type":"result","id":N,"blocked":..,...}`,
//!   `{"type":"changed"}` whenever rules may have changed.
//!
//! Browser messages are a 4-byte native-endian length followed by JSON.
//! The service sees this process as the browser's user, and recognises the
//! browser from the process tree; while this host is connected, the
//! browser counts as supervised.

use std::io::{self, Read, Write};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use iqal_ipc::client::Client;
use iqal_ipc::{Request, Response};
use serde_json::{json, Value};

/// Browsers accept at most 1 MB per message from a host.
const MAX_OUT: usize = 1 << 20;
const MAX_IN: usize = 4 << 20;

type Out = Arc<Mutex<io::Stdout>>;

fn main() {
    let out: Out = Arc::new(Mutex::new(io::stdout()));
    let mut stdin = io::stdin().lock();
    let mut client: Option<Client> = None;
    let mut session_started = false;

    while let Ok(Some(msg)) = read_message(&mut stdin) {
        match msg.get("type").and_then(Value::as_str) {
            Some("hello") => {
                if !session_started {
                    session_started = true;
                    let browser = msg.get("browser").and_then(Value::as_str).unwrap_or("").to_string();
                    let out = out.clone();
                    std::thread::spawn(move || session(&browser, &out));
                }
                send(&out, &json!({ "type": "hello", "version": env!("CARGO_PKG_VERSION") }));
            }
            Some("active") => {
                // The page in front of the user, for time allowances.
                if let Some(url) = msg.get("url").and_then(Value::as_str) {
                    let request = Request::ActiveTab { url: url.to_string() };
                    if client.as_mut().is_none_or(|c| c.call(&request).is_err()) {
                        client = Client::connect().ok();
                        if let Some(c) = client.as_mut() {
                            let _ = c.call(&request);
                        }
                    }
                }
            }
            Some("check") => {
                let id = msg.get("id").cloned().unwrap_or(Value::Null);
                let url = msg.get("url").and_then(Value::as_str).unwrap_or("").to_string();
                let reply = match check(&mut client, url) {
                    Ok(page) => json!({
                        "type": "result",
                        "id": id,
                        "blocked": page.blocked,
                        "set": page.set_name,
                        "reason": page.reason,
                        "metered": page.metered,
                    }),
                    // Service unreachable: do not block (an uninstalled or
                    // restarting service must not break browsing).
                    Err(e) => json!({ "type": "result", "id": id, "blocked": false, "error": e.to_string() }),
                };
                send(&out, &reply);
            }
            _ => {}
        }
    }
    // stdin closed: the browser or the extension went away.
}

/// Asks the service; reconnects once if the connection went stale (the
/// service restarted).
fn check(client: &mut Option<Client>, url: String) -> io::Result<iqal_ipc::PageInfo> {
    let mut last_error = None;
    for _ in 0..2 {
        let c = match client {
            Some(c) => c,
            None => client.insert(Client::connect()?),
        };
        match c.call(&Request::CheckPage { url: url.clone() }) {
            Ok(Response::Page { page }) => return Ok(page),
            Ok(other) => return Err(io::Error::other(format!("unexpected reply {other:?}"))),
            Err(e) => {
                *client = None;
                last_error = Some(e);
            }
        }
    }
    Err(last_error.expect("loop ran"))
}

/// Registers with the service and forwards "rules changed" notices, for as
/// long as this host lives. Reconnects if the service restarts.
fn session(browser: &str, out: &Out) {
    loop {
        if let Ok(mut c) = Client::connect()
            && let Ok(Response::Subscribed) = c.call(&Request::ExtensionSession { browser: browser.to_string() })
        {
            send(out, &json!({ "type": "changed" }));
            while let Ok(Response::Changed) = c.receive() {
                send(out, &json!({ "type": "changed" }));
            }
        }
        std::thread::sleep(Duration::from_secs(2));
    }
}

fn read_message(r: &mut impl Read) -> io::Result<Option<Value>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_ne_bytes(len) as usize;
    if len > MAX_IN {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    serde_json::from_slice(&body).map(Some).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn encode(msg: &Value) -> Option<Vec<u8>> {
    let body = serde_json::to_vec(msg).ok()?;
    if body.len() > MAX_OUT {
        return None;
    }
    let mut frame = (body.len() as u32).to_ne_bytes().to_vec();
    frame.extend_from_slice(&body);
    Some(frame)
}

fn send(out: &Out, msg: &Value) {
    if let Some(frame) = encode(msg) {
        let mut out = out.lock().unwrap_or_else(|e| e.into_inner());
        if out.write_all(&frame).and_then(|()| out.flush()).is_err() {
            std::process::exit(0); // browser closed the pipe
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn framing_round_trip() {
        let msg = json!({ "type": "check", "id": 3, "url": "https://example.com/" });
        let frame = encode(&msg).unwrap();
        assert_eq!(read_message(&mut frame.as_slice()).unwrap().unwrap(), msg);
        assert!(read_message(&mut [].as_slice()).unwrap().is_none());
    }
}
