//! Small helpers for the machine-wide registry (HKLM).

use std::io;

use windows_sys::Win32::Foundation::ERROR_FILE_NOT_FOUND;
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_LOCAL_MACHINE, KEY_READ, KEY_WRITE, REG_DWORD, REG_MULTI_SZ, REG_OPTION_NON_VOLATILE, REG_SZ,
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegGetValueW, RegSetValueExW, RRF_RT_REG_SZ,
};

use super::system::wide;

pub enum Value<'a> {
    Dword(u32),
    Str(&'a str),
    /// A list of strings (REG_MULTI_SZ).
    Strs(&'a [&'a str]),
}

/// Creates the key if needed and sets the given values.
pub fn set(path: &str, values: &[(&str, Value)]) -> io::Result<()> {
    let key = wide(path);
    let mut handle: HKEY = std::ptr::null_mut();
    // SAFETY: key is NUL-terminated; handle is a valid out pointer.
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            0,
            std::ptr::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_WRITE,
            std::ptr::null(),
            &mut handle,
            std::ptr::null_mut(),
        )
    };
    if status != 0 {
        return Err(io::Error::from_raw_os_error(status as i32));
    }
    let mut result = Ok(());
    for (name, value) in values {
        let name = wide(name);
        let (kind, data): (u32, Vec<u8>) = match value {
            Value::Dword(d) => (REG_DWORD, d.to_le_bytes().to_vec()),
            Value::Str(s) => (REG_SZ, utf16_bytes(&wide(s))),
            Value::Strs(list) => {
                let mut units: Vec<u16> = Vec::new();
                for s in *list {
                    units.extend(s.encode_utf16());
                    units.push(0);
                }
                units.push(0);
                (REG_MULTI_SZ, utf16_bytes(&units))
            }
        };
        // SAFETY: handle is open; data holds data.len() bytes.
        let status = unsafe { RegSetValueExW(handle, name.as_ptr(), 0, kind, data.as_ptr(), data.len() as u32) };
        if status != 0 {
            result = Err(io::Error::from_raw_os_error(status as i32));
        }
    }
    // SAFETY: handle is open.
    unsafe { RegCloseKey(handle) };
    result
}

fn utf16_bytes(units: &[u16]) -> Vec<u8> {
    units.iter().flat_map(|u| u.to_le_bytes()).collect()
}

/// Deletes a key with everything below it. A missing key is fine.
pub fn delete_tree(path: &str) -> io::Result<()> {
    let key = wide(path);
    // SAFETY: key is NUL-terminated.
    match unsafe { RegDeleteTreeW(HKEY_LOCAL_MACHINE, key.as_ptr()) } {
        0 | ERROR_FILE_NOT_FOUND => Ok(()),
        status => Err(io::Error::from_raw_os_error(status as i32)),
    }
}

/// Deletes one value. A missing value is fine.
pub fn delete_value(path: &str, name: &str) -> io::Result<()> {
    let (key, name) = (wide(path), wide(name));
    // SAFETY: both strings are NUL-terminated.
    match unsafe { windows_sys::Win32::System::Registry::RegDeleteKeyValueW(HKEY_LOCAL_MACHINE, key.as_ptr(), name.as_ptr()) } {
        0 | ERROR_FILE_NOT_FOUND => Ok(()),
        status => Err(io::Error::from_raw_os_error(status as i32)),
    }
}

pub fn get_string(path: &str, name: &str) -> Option<String> {
    let (key, name) = (wide(path), wide(name));
    let mut buf = vec![0u16; 1024];
    let mut size = (buf.len() * 2) as u32;
    // SAFETY: buf has `size` bytes of room.
    let status = unsafe {
        RegGetValueW(HKEY_LOCAL_MACHINE, key.as_ptr(), name.as_ptr(), RRF_RT_REG_SZ, std::ptr::null_mut(), buf.as_mut_ptr().cast(), &mut size)
    };
    if status != 0 {
        return None;
    }
    let len = (size as usize / 2).saturating_sub(1);
    Some(String::from_utf16_lossy(&buf[..len]))
}
