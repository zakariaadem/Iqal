//! New processes, the moment they start: a real-time Event Tracing for
//! Windows (ETW) session on the kernel's process provider. Each start is
//! handed to the guard, which closes the process if it is blocked.

use std::sync::mpsc::{self, Sender};
use std::sync::{Arc, OnceLock};

use windows_sys::Win32::Foundation::{ERROR_ALREADY_EXISTS, ERROR_SUCCESS};
use windows_sys::Win32::System::Diagnostics::Etw::{
    CONTROLTRACE_HANDLE, ControlTraceW, EVENT_CONTROL_CODE_ENABLE_PROVIDER, EVENT_RECORD, EVENT_TRACE_CONTROL_STOP,
    EVENT_TRACE_LOGFILEW, EVENT_TRACE_PROPERTIES, EVENT_TRACE_REAL_TIME_MODE, EnableTraceEx2, OpenTraceW,
    PROCESS_TRACE_MODE_EVENT_RECORD, PROCESS_TRACE_MODE_REAL_TIME, ProcessTrace, StartTraceW, TRACE_LEVEL_INFORMATION,
    WNODE_FLAG_TRACED_GUID,
};
use windows_sys::core::GUID;

use super::guard::AppGuard;
use super::system::wide;
use crate::service::Service;

const SESSION_NAME: &str = "Iqal process watch";
/// Microsoft-Windows-Kernel-Process.
const KERNEL_PROCESS: GUID = GUID::from_u128(0x22fb2cd6_0e7b_422b_a0c7_2fad1fd0e716);
const KEYWORD_PROCESS: u64 = 0x10;
const EVENT_PROCESS_START: u16 = 1;
/// `FlushTimer` counts milliseconds, so events arrive promptly.
const EVENT_TRACE_USE_MS_FLUSH_TIMER: u32 = 0x10;
const FLUSH_MS: u32 = 50;
/// QueryPerformanceCounter time stamps.
const CLOCK_QPC: u32 = 1;
const INVALID_PROCESSTRACE_HANDLE: u64 = u64::MAX;

static STARTS: OnceLock<Sender<u32>> = OnceLock::new();

/// The session properties, followed by room for the session name.
#[repr(C)]
struct Properties {
    props: EVENT_TRACE_PROPERTIES,
    name: [u16; 64],
}

fn properties() -> Box<Properties> {
    // SAFETY: plain data; all-zero is a valid starting point.
    let mut p: Box<Properties> = Box::new(unsafe { std::mem::zeroed() });
    p.props.Wnode.BufferSize = std::mem::size_of::<Properties>() as u32;
    p.props.Wnode.Flags = WNODE_FLAG_TRACED_GUID;
    p.props.Wnode.ClientContext = CLOCK_QPC;
    p.props.LogFileMode = EVENT_TRACE_REAL_TIME_MODE | EVENT_TRACE_USE_MS_FLUSH_TIMER;
    p.props.FlushTimer = FLUSH_MS;
    p.props.LoggerNameOffset = std::mem::offset_of!(Properties, name) as u32;
    p
}

pub fn start(service: Arc<Service>, guard: Arc<AppGuard>) -> std::io::Result<()> {
    let name = wide(SESSION_NAME);
    let mut handle = CONTROLTRACE_HANDLE { Value: 0 };
    let mut props = properties();
    // SAFETY: name is NUL-terminated; props is sized as declared.
    let mut status = unsafe { StartTraceW(&mut handle, name.as_ptr(), &mut props.props) };
    if status == ERROR_ALREADY_EXISTS {
        // Left over from a previous run of the service: replace it.
        let mut stop = properties();
        // SAFETY: as above.
        unsafe { ControlTraceW(CONTROLTRACE_HANDLE { Value: 0 }, name.as_ptr(), &mut stop.props, EVENT_TRACE_CONTROL_STOP) };
        props = properties();
        // SAFETY: as above.
        status = unsafe { StartTraceW(&mut handle, name.as_ptr(), &mut props.props) };
    }
    if status != ERROR_SUCCESS {
        return Err(std::io::Error::from_raw_os_error(status as i32));
    }
    // SAFETY: handle is a started session; the GUID outlives the call.
    let status = unsafe {
        EnableTraceEx2(
            handle,
            &KERNEL_PROCESS,
            EVENT_CONTROL_CODE_ENABLE_PROVIDER,
            TRACE_LEVEL_INFORMATION as u8,
            KEYWORD_PROCESS,
            0,
            0,
            std::ptr::null(),
        )
    };
    if status != ERROR_SUCCESS {
        return Err(std::io::Error::from_raw_os_error(status as i32));
    }

    let (tx, rx) = mpsc::channel();
    let _ = STARTS.set(tx);
    std::thread::Builder::new().name("iqal-etw".into()).spawn(move || {
        let mut name = wide(SESSION_NAME);
        // SAFETY: plain data; the fields used are set below.
        let mut log: EVENT_TRACE_LOGFILEW = unsafe { std::mem::zeroed() };
        log.LoggerName = name.as_mut_ptr();
        log.Anonymous1.ProcessTraceMode = PROCESS_TRACE_MODE_REAL_TIME | PROCESS_TRACE_MODE_EVENT_RECORD;
        log.Anonymous2.EventRecordCallback = Some(on_event);
        // SAFETY: log is filled in as OpenTraceW requires.
        let trace = unsafe { OpenTraceW(&mut log) };
        if trace.Value == INVALID_PROCESSTRACE_HANDLE {
            eprintln!("iqal: process watch: {}", std::io::Error::last_os_error());
            return;
        }
        eprintln!("iqal: watching program starts");
        // SAFETY: trace is open; blocks while the session runs.
        let status = unsafe { ProcessTrace(&trace, 1, std::ptr::null(), std::ptr::null()) };
        eprintln!("iqal: process watch ended ({status})");
    })?;

    std::thread::Builder::new().name("iqal-starts".into()).spawn(move || {
        while let Ok(pid) = rx.recv() {
            let engine = service.engine();
            guard.enforce_running(pid, &engine, crate::clock::now());
        }
    })?;
    Ok(())
}

fn same_guid(a: &GUID, b: &GUID) -> bool {
    (a.data1, a.data2, a.data3, a.data4) == (b.data1, b.data2, b.data3, b.data4)
}

/// Called by ETW for every event: passes on the new process id. Must be
/// quick, so the checking happens on another thread.
unsafe extern "system" fn on_event(record: *mut EVENT_RECORD) {
    // SAFETY: ETW passes a valid record for the duration of the call.
    let record = unsafe { &*record };
    let header = &record.EventHeader;
    if !same_guid(&header.ProviderId, &KERNEL_PROCESS)
        || header.EventDescriptor.Id != EVENT_PROCESS_START
        || record.UserDataLength < 4
        || record.UserData.is_null()
    {
        return;
    }
    // The event data starts with the new process's id.
    // SAFETY: at least 4 bytes of user data, possibly unaligned.
    let pid = unsafe { record.UserData.cast::<u32>().read_unaligned() };
    if let Some(tx) = STARTS.get() {
        let _ = tx.send(pid);
    }
}
