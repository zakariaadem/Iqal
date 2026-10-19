pub mod scm;
pub mod server;
pub mod system;

use std::fs::OpenOptions;
use std::os::windows::io::IntoRawHandle;
use std::path::Path;

use windows_sys::Win32::System::Console::{STD_ERROR_HANDLE, SetStdHandle};

const LOG: &str = "iqal.log";
/// The log is started afresh (keeping one old copy) beyond this size.
const LOG_MAX: u64 = 1 << 20;

/// A service has no terminal: sends messages to `iqal.log` in the settings
/// folder instead.
pub fn log_to_file(state_dir: &Path) -> std::io::Result<()> {
    let path = state_dir.join(LOG);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > LOG_MAX) {
        let _ = std::fs::rename(&path, state_dir.join(format!("{LOG}.old")));
    }
    let file = OpenOptions::new().create(true).append(true).open(&path)?;
    // SAFETY: hands Windows a valid handle, which stays open for good.
    unsafe { SetStdHandle(STD_ERROR_HANDLE, file.into_raw_handle()) };
    Ok(())
}
