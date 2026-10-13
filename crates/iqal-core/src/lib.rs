//! Platform-independent core of Iqal: the data model, rule parsing,
//! the matching engine, schedules, locks and import/export.
//!
//! Nothing in this crate touches the operating system except for reading
//! the clock ([`Now::system`]) and the system random generator. The
//! platform daemons feed it facts (URLs, host names, processes, window
//! titles, location fixes) and enforce its decisions.

pub mod apps;
pub mod engine;
pub mod geo;
pub mod import;
pub mod lists;
pub mod lock;
pub mod model;
pub mod schedule;
pub mod text;
pub mod web;

pub use apps::{AppRule, ProcessInfo};
pub use engine::{Ctx, Decision, Engine, Hit, ListSource, NoLists, Reason};
pub use geo::{GeoArea, LocationFix};
pub use lock::{Challenge, LockContext, PasswordHash, UnlockOutcome};
pub use model::{
    Allowance, AppRules, BlockSet, Config, ListMode, Lock, LockKind, SetId, UserScope, WebRules,
};
pub use schedule::{Now, Schedule};
pub use web::{UrlView, WebRule};

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("file version {0} is newer than this version of Iqal supports")]
    UnsupportedVersion(u32),
    #[error("not an Iqal export file")]
    NotAnExport,
    #[error("this block is locked: the change {0}")]
    Locked(&'static str),
    #[error("this lock cannot be opened early")]
    CannotUnlockEarly,
    #[error("wrong answer")]
    WrongAnswer,
    #[error("no block with that id")]
    NotFound,
    #[error(transparent)]
    Schedule(#[from] schedule::ScheduleError),
    #[error(transparent)]
    Geo(#[from] geo::GeoError),
    #[error("password hashing failed")]
    Hash,
    #[error("the system random number generator failed")]
    Random,
}
