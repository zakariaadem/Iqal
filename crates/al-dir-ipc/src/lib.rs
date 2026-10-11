//! Messages exchanged between the Al-Dir service and its clients (GUI,
//! CLI, session agent), and the framing used on the wire: each message is
//! a 4-byte big-endian length followed by that many bytes of JSON.
//!
//! On Linux the transport is a Unix socket; the service identifies the
//! caller from the socket's peer credentials, so requests carry no
//! identity of their own.

use std::io::{self, Read, Write};

use al_dir_core::{BlockSet, SetId};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

pub const PROTOCOL_VERSION: u32 = 1;
pub const DEFAULT_SOCKET: &str = "/run/al-dir/al-dir.sock";
/// Overrides the socket path (development and tests).
pub const SOCKET_ENV: &str = "AL_DIR_SOCKET";
pub const MAX_FRAME: usize = 64 << 20;

pub fn socket_path() -> String {
    std::env::var(SOCKET_ENV).unwrap_or_else(|_| DEFAULT_SOCKET.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Request {
    Status,
    ListSets,
    /// Create or replace a set. Refused if it would weaken a locked set.
    UpsertSet { set: Box<BlockSet> },
    RemoveSet { id: SetId },
    Export { ids: Option<Vec<SetId>> },
    Import { json: String },
    /// Ask how a lock can be opened early.
    UnlockStart { id: SetId },
    UnlockAnswer { id: SetId, answer: String },
    CancelUnlock { id: SetId },
    /// Ask what the engine would decide (for testing rules).
    Check { query: CheckQuery, user: Option<String> },
    /// Recent things the service blocked (the caller's own, or everyone's
    /// for administrators).
    RecentEvents,
    /// Turn this connection into a feed of the caller's own block events
    /// (`Response::Event` messages, one per event, until disconnect).
    Subscribe,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CheckQuery {
    Url { url: String },
    Host { host: String },
    App { exe_path: String, cmdline: Option<String> },
    Window { title: String, exe_path: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "result", rename_all = "snake_case")]
pub enum Response {
    Status { status: Status },
    Sets { sets: Vec<SetInfo> },
    Done,
    Exported { json: String },
    Imported { added: Vec<String>, replaced: Vec<String>, rejected: Vec<(String, String)> },
    Challenge { challenge: ChallengeInfo },
    Unlock { outcome: UnlockInfo },
    Decision { decision: DecisionInfo },
    Events { events: Vec<BlockEvent> },
    Subscribed,
    Event { event: Box<BlockEvent> },
    Error { code: ErrorCode, message: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventKind {
    /// A program was stopped from starting.
    AppStart,
    /// A running program was closed.
    AppKilled,
    /// A window was closed because of its title.
    WindowClosed,
}

/// Something the service blocked.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockEvent {
    pub time: i64,
    pub user: String,
    pub kind: EventKind,
    /// Executable path, URL or host.
    pub target: String,
    pub set: SetId,
    pub set_name: String,
    pub reason: String,
}

impl Response {
    pub fn error(code: ErrorCode, message: impl Into<String>) -> Self {
        Response::Error { code, message: message.into() }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Status {
    pub version: String,
    pub protocol: u32,
    /// The account the service sees the caller as.
    pub user: String,
    /// Whether the caller may change blocks.
    pub admin: bool,
    pub sets: usize,
    pub locked_sets: usize,
    pub started_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SetInfo {
    pub set: BlockSet,
    pub locked: bool,
    /// In force right now for the calling user.
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ChallengeInfo {
    NotLocked,
    NotPossible,
    Password,
    RandomText { text: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UnlockInfo {
    Unlocked,
    Pending { at: i64 },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DecisionInfo {
    pub blocked: bool,
    pub set: Option<SetId>,
    pub set_name: Option<String>,
    pub reason: Option<String>,
    pub metered: Vec<SetId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCode {
    BadRequest,
    PermissionDenied,
    Locked,
    NotFound,
    WrongAnswer,
    CannotUnlockEarly,
    Internal,
}

pub fn write_frame<W: Write, T: Serialize>(w: &mut W, msg: &T) -> io::Result<()> {
    let body = serde_json::to_vec(msg).map_err(io::Error::other)?;
    if body.len() > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "message too large"));
    }
    let mut frame = Vec::with_capacity(4 + body.len());
    frame.extend_from_slice(&(body.len() as u32).to_be_bytes());
    frame.extend_from_slice(&body);
    w.write_all(&frame)?;
    w.flush()
}

/// Reads one message. Returns `Ok(None)` if the peer closed the
/// connection cleanly between messages.
pub fn read_frame<R: Read, T: DeserializeOwned>(r: &mut R) -> io::Result<Option<T>> {
    let mut len = [0u8; 4];
    match r.read_exact(&mut len) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
        Err(e) => return Err(e),
    }
    let len = u32::from_be_bytes(len) as usize;
    if len > MAX_FRAME {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "message too large"));
    }
    let mut body = vec![0u8; len];
    r.read_exact(&mut body)?;
    serde_json::from_slice(&body).map(Some).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(unix)]
pub mod client {
    use super::*;
    use std::os::unix::net::UnixStream;

    pub struct Client {
        stream: UnixStream,
    }

    impl Client {
        pub fn connect() -> io::Result<Self> {
            Self::connect_to(&socket_path())
        }

        pub fn connect_to(path: &str) -> io::Result<Self> {
            Ok(Self { stream: UnixStream::connect(path)? })
        }

        pub fn call(&mut self, request: &Request) -> io::Result<Response> {
            write_frame(&mut self.stream, request)?;
            self.receive()
        }

        /// Waits for the next message (used after `Request::Subscribe`).
        pub fn receive(&mut self) -> io::Result<Response> {
            read_frame(&mut self.stream)?
                .ok_or_else(|| io::Error::new(io::ErrorKind::UnexpectedEof, "service closed the connection"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frames_round_trip() {
        let mut buf = Vec::new();
        write_frame(&mut buf, &Request::Status).unwrap();
        write_frame(&mut buf, &Request::Check { query: CheckQuery::Host { host: "a.com".into() }, user: None }).unwrap();
        let mut r = buf.as_slice();
        assert!(matches!(read_frame::<_, Request>(&mut r).unwrap(), Some(Request::Status)));
        assert!(matches!(read_frame::<_, Request>(&mut r).unwrap(), Some(Request::Check { .. })));
        assert!(read_frame::<_, Request>(&mut r).unwrap().is_none());
    }

    #[test]
    fn rejects_oversized_frames() {
        let mut buf = (u32::MAX).to_be_bytes().to_vec();
        buf.extend_from_slice(b"{}");
        assert!(read_frame::<_, Request>(&mut buf.as_slice()).is_err());
    }

    #[test]
    fn json_shape() {
        let json = serde_json::to_string(&Request::RemoveSet { id: SetId::nil() }).unwrap();
        assert_eq!(json, r#"{"op":"remove_set","id":"00000000-0000-0000-0000-000000000000"}"#);
        let json = serde_json::to_string(&Response::Unlock { outcome: UnlockInfo::Pending { at: 5 } }).unwrap();
        assert_eq!(json, r#"{"result":"unlock","outcome":{"type":"pending","at":5}}"#);
    }
}
