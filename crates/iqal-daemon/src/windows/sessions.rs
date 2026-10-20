//! Keeps a window watcher (`iqal-daemon --watch-session`) running on the
//! desktop of every signed-in user: started when someone signs in,
//! restarted if it is ended, ended when they sign out. The watchers run as
//! LocalSystem in the user's session and belong to a job object, so they
//! also end when the service stops.

use std::collections::HashMap;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::time::Duration;

use windows_sys::Win32::Foundation::{HANDLE, WAIT_TIMEOUT};
use windows_sys::Win32::Security::{
    DuplicateTokenEx, SecurityImpersonation, SetTokenInformation, TOKEN_ADJUST_DEFAULT, TOKEN_ADJUST_SESSIONID,
    TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE, TOKEN_QUERY, TokenPrimary, TokenSessionId,
};
use windows_sys::Win32::System::JobObjects::{
    AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation, SetInformationJobObject,
};
use windows_sys::Win32::System::RemoteDesktop::{
    WTS_CURRENT_SERVER_HANDLE, WTS_SESSION_INFOW, WTSActive, WTSDisconnected, WTSEnumerateSessionsW, WTSFreeMemory,
    WTSGetActiveConsoleSessionId, WTSQueryUserToken,
};
use windows_sys::Win32::System::Threading::{
    CREATE_NO_WINDOW, CREATE_UNICODE_ENVIRONMENT, CreateProcessAsUserW, GetCurrentProcess, OpenProcessToken, PROCESS_INFORMATION, STARTUPINFOW,
    TerminateProcess, WaitForSingleObject,
};

use windows_sys::Win32::System::Environment::{CreateEnvironmentBlock, DestroyEnvironmentBlock};

use super::system::wide;

const CHECK_EVERY: Duration = Duration::from_secs(3);

pub fn start() -> std::io::Result<()> {
    let job = kill_on_close_job()?;
    std::thread::Builder::new().name("iqal-sessions".into()).spawn(move || {
        let mut watchers: HashMap<u32, OwnedHandle> = HashMap::new();
        loop {
            let sessions = user_sessions(false);
            watchers.retain(|session, process| {
                let keep = sessions.contains(session) && running(process);
                if !keep {
                    // SAFETY: a process handle we own.
                    unsafe { TerminateProcess(process.as_raw_handle() as HANDLE, 0) };
                }
                keep
            });
            for session in sessions {
                if watchers.contains_key(&session) {
                    continue;
                }
                match spawn_watcher(session, &job) {
                    Ok(process) => {
                        watchers.insert(session, process);
                    }
                    Err(e) => eprintln!("iqal: window watcher for session {session}: {e}"),
                }
            }
            std::thread::sleep(CHECK_EVERY);
        }
    })?;
    Ok(())
}

fn running(process: &OwnedHandle) -> bool {
    // SAFETY: a valid process handle; does not wait.
    unsafe { WaitForSingleObject(process.as_raw_handle() as HANDLE, 0) == WAIT_TIMEOUT }
}

/// Sessions with a signed-in user: in use now, or (unless `active_only`)
/// switched away from.
fn user_sessions(active_only: bool) -> Vec<u32> {
    let mut list: *mut WTS_SESSION_INFOW = std::ptr::null_mut();
    let mut count = 0u32;
    // SAFETY: list and count are valid out pointers.
    if unsafe { WTSEnumerateSessionsW(WTS_CURRENT_SERVER_HANDLE, 0, 1, &mut list, &mut count) } == 0 {
        return Vec::new();
    }
    // SAFETY: WTS returned `count` entries at `list`.
    let sessions = unsafe { std::slice::from_raw_parts(list, count as usize) };
    let ids = sessions
        .iter()
        .filter(|s| s.SessionId != 0 && (s.State == WTSActive || (!active_only && s.State == WTSDisconnected)))
        .map(|s| s.SessionId)
        .filter(|&id| {
            let mut token: HANDLE = std::ptr::null_mut();
            // SAFETY: token is a valid out pointer. Succeeds only when a
            // user is signed in to the session.
            let ok = unsafe { WTSQueryUserToken(id, &mut token) } != 0;
            if ok {
                // SAFETY: a new handle we own.
                drop(unsafe { OwnedHandle::from_raw_handle(token) });
            }
            ok
        })
        .collect();
    // SAFETY: allocated by WTSEnumerateSessionsW.
    unsafe { WTSFreeMemory(list.cast()) };
    ids
}

/// A job whose processes end when its last handle (ours) closes.
fn kill_on_close_job() -> std::io::Result<OwnedHandle> {
    // SAFETY: an unnamed job with default security.
    let job = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
    if job.is_null() {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: a new handle we own.
    let job = unsafe { OwnedHandle::from_raw_handle(job) };
    // SAFETY: plain data.
    let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { std::mem::zeroed() };
    limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
    // SAFETY: limits is the structure this information class expects.
    let ok = unsafe {
        SetInformationJobObject(
            job.as_raw_handle() as HANDLE,
            JobObjectExtendedLimitInformation,
            (&raw const limits).cast(),
            std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(job)
}

/// Starts this program with `--watch-session` as LocalSystem on the
/// session's desktop.
fn spawn_watcher(session: u32, job: &OwnedHandle) -> std::io::Result<OwnedHandle> {
    let exe = std::env::current_exe()?;
    let mut own: HANDLE = std::ptr::null_mut();
    let access = TOKEN_DUPLICATE | TOKEN_QUERY | TOKEN_ASSIGN_PRIMARY | TOKEN_ADJUST_SESSIONID | TOKEN_ADJUST_DEFAULT;
    // SAFETY: own is a valid out pointer.
    if unsafe { OpenProcessToken(GetCurrentProcess(), access, &mut own) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: a new handle we own.
    let own = unsafe { OwnedHandle::from_raw_handle(own) };
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: duplicating our own token as a primary token.
    let ok = unsafe {
        DuplicateTokenEx(own.as_raw_handle() as HANDLE, access, std::ptr::null(), SecurityImpersonation, TokenPrimary, &mut token)
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: a new handle we own.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    // Moving a token to another session needs LocalSystem's privileges.
    // SAFETY: session is a u32, as TokenSessionId expects.
    let ok = unsafe {
        SetTokenInformation(token.as_raw_handle() as HANDLE, TokenSessionId, (&raw const session).cast(), 4)
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    let app = wide(&exe.to_string_lossy());
    let mut cmdline = wide(&format!("\"{}\" --watch-session", exe.display()));
    let mut desktop = wide(r"winsta0\default");
    // SAFETY: plain data; cb and lpDesktop set below.
    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    startup.lpDesktop = desktop.as_mut_ptr();
    // SAFETY: plain data, filled in by CreateProcessAsUserW.
    let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: all strings are NUL-terminated; cmdline is writable.
    let ok = unsafe {
        CreateProcessAsUserW(
            token.as_raw_handle() as HANDLE,
            app.as_ptr(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_NO_WINDOW,
            std::ptr::null(),
            std::ptr::null(),
            &startup,
            &mut info,
        )
    };
    if ok == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: CreateProcessAsUserW returned two new handles we own.
    let (process, _thread) =
        unsafe { (OwnedHandle::from_raw_handle(info.hProcess), OwnedHandle::from_raw_handle(info.hThread)) };
    // SAFETY: both handles are valid.
    if unsafe { AssignProcessToJobObject(job.as_raw_handle() as HANDLE, process.as_raw_handle() as HANDLE) } == 0 {
        eprintln!("iqal: window watcher not in job: {}", std::io::Error::last_os_error());
    }
    eprintln!("iqal: window watcher started for session {session}");
    Ok(process)
}

/// Starts the notification agent (iqal-agent, next to this program) for
/// everyone signed in, as themselves. At sign-in Windows starts it (the
/// Run key the installer sets); this covers those already signed in when
/// the service starts. An agent already running in a session lets the
/// new one end at once.
pub fn start_agents() {
    let Some(agent) = std::env::current_exe().ok().map(|e| e.with_file_name("iqal-agent.exe")).filter(|a| a.is_file()) else {
        return;
    };
    for session in user_sessions(false) {
        if let Err(e) = spawn_agent(session, &agent) {
            eprintln!("iqal: notifications for session {session}: {e}");
        }
    }
}

fn spawn_agent(session: u32, agent: &std::path::Path) -> std::io::Result<()> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: token is a valid out pointer.
    if unsafe { WTSQueryUserToken(session, &mut token) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    // SAFETY: a new handle we own.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    // The user's own environment (APPDATA holds their language choice).
    let mut environment: *mut core::ffi::c_void = std::ptr::null_mut();
    // SAFETY: environment is a valid out pointer; freed below.
    if unsafe { CreateEnvironmentBlock(&mut environment, token.as_raw_handle() as HANDLE, 0) } == 0 {
        environment = std::ptr::null_mut();
    }
    let app = wide(&agent.to_string_lossy());
    let mut cmdline = wide(&format!("\"{}\"", agent.display()));
    let mut desktop = wide(r"winsta0\default");
    // SAFETY: plain data; cb and lpDesktop set below.
    let mut startup: STARTUPINFOW = unsafe { std::mem::zeroed() };
    startup.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
    startup.lpDesktop = desktop.as_mut_ptr();
    // SAFETY: plain data, filled in by CreateProcessAsUserW.
    let mut info: PROCESS_INFORMATION = unsafe { std::mem::zeroed() };
    // SAFETY: all strings are NUL-terminated; cmdline is writable; the
    // environment block is the user's, as CREATE_UNICODE_ENVIRONMENT says.
    let ok = unsafe {
        CreateProcessAsUserW(
            token.as_raw_handle() as HANDLE,
            app.as_ptr(),
            cmdline.as_mut_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            0,
            CREATE_UNICODE_ENVIRONMENT,
            environment,
            std::ptr::null(),
            &startup,
            &mut info,
        )
    };
    let error = std::io::Error::last_os_error();
    if !environment.is_null() {
        // SAFETY: made by CreateEnvironmentBlock above.
        unsafe { DestroyEnvironmentBlock(environment) };
    }
    if ok == 0 {
        return Err(error);
    }
    // SAFETY: two new handles we own; the agent runs on by itself.
    drop(unsafe { (OwnedHandle::from_raw_handle(info.hProcess), OwnedHandle::from_raw_handle(info.hThread)) });
    Ok(())
}

/// The account of the person using the computer, as a SID string: whoever
/// is signed in at the screen (the console session), else the user of an
/// active remote session (Remote Desktop, Hyper-V enhanced session).
pub fn active_user_sid() -> Option<String> {
    // SAFETY: no preconditions.
    let console = unsafe { WTSGetActiveConsoleSessionId() };
    std::iter::once(console)
        .filter(|&s| s != u32::MAX)
        .chain(user_sessions(true))
        .find_map(session_user_sid)
}

fn session_user_sid(session: u32) -> Option<String> {
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: token is a valid out pointer.
    if unsafe { WTSQueryUserToken(session, &mut token) } == 0 {
        return None;
    }
    // SAFETY: a new handle we own.
    let token = unsafe { OwnedHandle::from_raw_handle(token) };
    super::procs::token_sid(&token)
}
