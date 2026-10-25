//! Connection to the Iqal service, with translated errors.

use std::cell::RefCell;

use iqal_core::{BlockSet, SetId};
use iqal_i18n::{t, ta};
use iqal_ipc::client::Client;
use iqal_ipc::{Request, Response, SetInfo, SetState, Status};

pub struct Service {
    client: RefCell<Option<Client>>,
}

impl Service {
    pub fn new() -> Self {
        Self { client: RefCell::new(None) }
    }

    /// Sends a request; reconnects once if the service restarted.
    pub fn call(&self, request: Request) -> Result<Response, String> {
        for _ in 0..2 {
            let mut slot = self.client.borrow_mut();
            if slot.is_none() {
                *slot = Some(Client::connect().map_err(|_| unreachable())?);
            }
            match slot.as_mut().expect("connected").call(&request) {
                Ok(Response::Error { message, .. }) => return Err(localize_error(&message)),
                Ok(r) => return Ok(r),
                Err(_) => *slot = None,
            }
        }
        Err(unreachable())
    }

    pub fn sets(&self) -> Result<Vec<SetInfo>, String> {
        match self.call(Request::ListSets)? {
            Response::Sets { sets } => Ok(sets),
            _ => Err(t("error-unexpected")),
        }
    }

    /// The change counter and each block's state, without rules.
    pub fn states(&self) -> Result<(u64, Vec<SetState>), String> {
        match self.call(Request::SetStates)? {
            Response::SetStates { generation, states } => Ok((generation, states)),
            _ => Err(t("error-unexpected")),
        }
    }

    pub fn status(&self) -> Result<Box<Status>, String> {
        match self.call(Request::Status)? {
            Response::Status { status } => Ok(status),
            _ => Err(t("error-unexpected")),
        }
    }

    pub fn save(&self, set: BlockSet) -> Result<(), String> {
        self.call(Request::UpsertSet { set: Box::new(set) }).map(drop)
    }

    pub fn remove(&self, id: SetId) -> Result<(), String> {
        self.call(Request::RemoveSet { id }).map(drop)
    }
}

/// Errors are a short title, then optionally a newline and an explanation
/// (see `Ui::error`).
fn unreachable() -> String {
    format!("{}\n{}", t("error-service-unreachable"), t("error-service-unreachable-detail"))
}

fn locked(reason: String) -> String {
    format!("{}\n{}", t("error-locked-title"), ta("error-locked", &[("reason", reason.into())]))
}

/// The service reports errors in English; known ones are translated.
pub fn localize_error(message: &str) -> String {
    const LOCKED: &str = "this block is locked: the change ";
    if let Some(reason) = message.strip_prefix(LOCKED) {
        return locked(reason_text(reason));
    }
    match message {
        "wrong answer" => t("error-wrong-answer"),
        "this lock cannot be opened early" => t("error-cannot-unlock-early"),
        "only administrators can change Iqal settings" => t("error-not-admin"),
        "no Wi-Fi networks in range (is Wi-Fi on?)" => t("error-no-wifi"),
        other => other.to_string(),
    }
}

/// Why a change would weaken a locked block (from `iqal_core::lock::weakening_reason`),
/// in the app's language: "would turn SafeSearch off"...
pub fn reason_text(reason: &str) -> String {
    let key = match reason {
        "disables the block" => "reason-disables",
        "applies to fewer users" => "reason-fewer-users",
        "changes the website blocking mode" => "reason-web-mode",
        "removes blocked websites" => "reason-removes-websites",
        "adds allowed websites or exceptions" => "reason-adds-allowed-websites",
        "turns off SafeSearch" => "reason-safesearch-off",
        "changes the app blocking mode" => "reason-app-mode",
        "removes blocked apps" => "reason-removes-apps",
        "adds allowed apps or exceptions" => "reason-adds-allowed-apps",
        "limits the block to a schedule" => "reason-adds-schedule",
        "shortens the schedule" => "reason-shortens-schedule",
        "adds a time allowance" => "reason-adds-allowance",
        "increases the time allowance" => "reason-increases-allowance",
        "removes the lock" => "reason-removes-lock",
        "shortens the unlock delay" => "reason-shortens-delay",
        "weakens the lock" => "reason-weakens-lock",
        "changes a pending unlock" => "reason-pending-unlock",
        "deletes the block" => "reason-deletes",
        "turns off hardened mode" => "reason-hardened-off",
        _ => return reason.to_string(),
    };
    t(key)
}
