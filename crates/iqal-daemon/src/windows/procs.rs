//! Facts about running processes, and which accounts are enforced.

use std::collections::HashMap;
use std::ffi::c_void;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::sync::Mutex;

use windows_sys::Wdk::System::Threading::{NtQueryInformationProcess, PROCESSINFOCLASS};
use windows_sys::Win32::Foundation::{HANDLE, INVALID_HANDLE_VALUE, LocalFree, UNICODE_STRING};
use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;
use windows_sys::Win32::Security::{TOKEN_QUERY, TOKEN_USER, TokenUser};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, OpenProcessToken, PROCESS_NAME_WIN32, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_TERMINATE,
    QueryFullProcessImageNameW, TerminateProcess,
};

use super::system;

/// `ProcessCommandLineInformation` (Windows 8.1 and later).
const PROCESS_COMMAND_LINE_INFORMATION: PROCESSINFOCLASS = 60;
const STATUS_INFO_LENGTH_MISMATCH: i32 = 0xC000_0004_u32 as i32;

fn open(pid: u32, access: u32) -> Option<OwnedHandle> {
    // SAFETY: no pointers involved.
    let h = unsafe { OpenProcess(access, 0, pid) };
    if h.is_null() {
        return None;
    }
    // SAFETY: a new handle we own.
    Some(unsafe { OwnedHandle::from_raw_handle(h) })
}

/// A running process: its id and its parent's.
pub struct Entry {
    pub pid: u32,
    pub parent: u32,
}

/// All running processes.
pub fn list() -> Vec<Entry> {
    // SAFETY: no pointers involved.
    let snap = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snap == INVALID_HANDLE_VALUE {
        return Vec::new();
    }
    // SAFETY: a new handle we own.
    let snap = unsafe { OwnedHandle::from_raw_handle(snap) };
    let mut out = Vec::new();
    // SAFETY: PROCESSENTRY32W is plain data; dwSize set as required.
    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let handle = snap.as_raw_handle() as HANDLE;
    // SAFETY: entry is a valid, sized PROCESSENTRY32W.
    let mut ok = unsafe { Process32FirstW(handle, &mut entry) };
    while ok != 0 {
        out.push(Entry {
            pid: entry.th32ProcessID,
            parent: entry.th32ParentProcessID,
        });
        // SAFETY: as above.
        ok = unsafe { Process32NextW(handle, &mut entry) };
    }
    out
}

/// Full path of the program a process is running.
pub fn exe_of(pid: u32) -> Option<String> {
    let process = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let mut buf = vec![0u16; 1024];
    let mut len = buf.len() as u32;
    // SAFETY: buf holds len characters.
    let ok = unsafe {
        QueryFullProcessImageNameW(process.as_raw_handle() as HANDLE, PROCESS_NAME_WIN32, buf.as_mut_ptr(), &mut len)
    };
    (ok != 0).then(|| String::from_utf16_lossy(&buf[..len as usize]))
}

/// The command line a process was started with.
pub fn cmdline_of(pid: u32) -> Option<String> {
    let process = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let handle = process.as_raw_handle() as HANDLE;
    let mut needed = 0u32;
    // SAFETY: asking for the size only.
    let status = unsafe {
        NtQueryInformationProcess(handle, PROCESS_COMMAND_LINE_INFORMATION, std::ptr::null_mut(), 0, &mut needed)
    };
    if status != STATUS_INFO_LENGTH_MISMATCH || needed == 0 {
        return None;
    }
    // u64 for alignment: the block starts with a UNICODE_STRING.
    let mut buf = vec![0u64; (needed as usize).div_ceil(8)];
    // SAFETY: buf has room for `needed` bytes.
    let status = unsafe {
        NtQueryInformationProcess(handle, PROCESS_COMMAND_LINE_INFORMATION, buf.as_mut_ptr().cast(), needed, &mut needed)
    };
    if status < 0 {
        return None;
    }
    // SAFETY: on success buf holds a UNICODE_STRING pointing into buf.
    let text = unsafe {
        let s = &*buf.as_ptr().cast::<UNICODE_STRING>();
        if s.Buffer.is_null() {
            return None;
        }
        std::slice::from_raw_parts(s.Buffer, s.Length as usize / 2)
    };
    let s = String::from_utf16_lossy(text);
    let s = s.trim();
    (!s.is_empty()).then(|| s.to_string())
}

/// The account a process runs as, as a SID string ("S-1-5-21-...").
pub fn sid_of(pid: u32) -> Option<String> {
    let process = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: token is a valid out pointer.
    if unsafe { OpenProcessToken(process.as_raw_handle() as HANDLE, TOKEN_QUERY, &mut token) } == 0 {
        return None;
    }
    // SAFETY: a new handle we own.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    token_sid(&token)
}

/// The account a token belongs to, as a SID string.
pub fn token_sid(token: &OwnedHandle) -> Option<String> {
    let info = system::token_info(token, TokenUser).ok()?;
    // SAFETY: info holds a TOKEN_USER.
    let sid = unsafe { (*info.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    sid_string(sid)
}

fn sid_string(sid: *mut c_void) -> Option<String> {
    let mut text: *mut u16 = std::ptr::null_mut();
    // SAFETY: sid is valid; text receives a LocalAlloc'd string.
    if unsafe { ConvertSidToStringSidW(sid, &mut text) } == 0 {
        return None;
    }
    // SAFETY: text is NUL-terminated.
    let s = unsafe {
        let len = (0..).take_while(|&i| *text.add(i) != 0).count();
        String::from_utf16_lossy(std::slice::from_raw_parts(text, len))
    };
    // SAFETY: allocated by ConvertSidToStringSidW.
    unsafe { LocalFree(text.cast()) };
    Some(s)
}

/// Ends a process at once. `false` if it could not be ended (already gone,
/// or protected).
pub fn kill(pid: u32) -> bool {
    let Some(process) = open(pid, PROCESS_TERMINATE) else { return false };
    // SAFETY: a handle with PROCESS_TERMINATE access.
    unsafe { TerminateProcess(process.as_raw_handle() as HANDLE, 1) != 0 }
}

/// Which accounts belong to people (and are therefore subject to blocks)
/// and what their names are. Windows itself, services and the built-in
/// Administrator account are never blocked, so a rule can never break the
/// system.
pub struct Users {
    names: Mutex<HashMap<String, Option<String>>>,
}

impl Users {
    pub fn load() -> Self {
        Self { names: Mutex::new(HashMap::new()) }
    }

    /// Accounts of people: local and domain users ("S-1-5-21-…") with a
    /// relative id of 1000 or more (500 is the built-in Administrator).
    pub fn enforced(sid: &str) -> bool {
        sid.strip_prefix("S-1-5-21-")
            .and_then(|rest| rest.rsplit('-').next())
            .and_then(|rid| rid.parse::<u32>().ok())
            .is_some_and(|rid| rid >= 1000)
    }

    /// The account name for an enforced SID; `None` for anything else.
    pub fn name(&self, sid: &str) -> Option<String> {
        if !Self::enforced(sid) {
            return None;
        }
        let mut names = self.names.lock().unwrap_or_else(|e| e.into_inner());
        names.entry(sid.to_string()).or_insert_with(|| system::account_name(sid)).clone()
    }

    /// The enforced account a process runs as, by name.
    pub fn of_process(&self, pid: u32) -> Option<String> {
        self.name(&sid_of(pid)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_own_process() {
        let pid = std::process::id();
        assert!(exe_of(pid).unwrap().to_lowercase().contains("iqal_daemon"));
        assert!(cmdline_of(pid).is_some());
        assert!(sid_of(pid).is_some_and(|s| s.starts_with("S-1-5-")));
        assert!(list().iter().any(|e| e.pid == pid));
    }

    #[test]
    fn enforced_accounts() {
        assert!(Users::enforced("S-1-5-21-1004336348-1177238915-682003330-1001"));
        assert!(!Users::enforced("S-1-5-21-1004336348-1177238915-682003330-500"));
        assert!(!Users::enforced("S-1-5-18"));
        assert!(!Users::enforced("S-1-5-90-0-1"));
    }
}
