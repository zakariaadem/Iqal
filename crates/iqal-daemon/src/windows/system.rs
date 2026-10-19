//! Facts about the running Windows system, and the Windows security calls
//! the service needs.

use std::ffi::c_void;
use std::io;
use std::os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle};
use std::path::Path;

use windows_sys::Win32::Foundation::{HANDLE, LocalFree};
use windows_sys::Win32::Security::Authorization::{
    ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1,
};
use windows_sys::Win32::Security::{
    CreateWellKnownSid, DACL_SECURITY_INFORMATION, EqualSid, GetTokenInformation, LookupAccountSidW,
    PROTECTED_DACL_SECURITY_INFORMATION, PSECURITY_DESCRIPTOR, RevertToSelf, SECURITY_MAX_SID_SIZE, SID_NAME_USE, SetFileSecurityW, TOKEN_GROUPS, TOKEN_QUERY,
    TOKEN_USER, TokenGroups, TokenUser, WELL_KNOWN_SID_TYPE, WinBuiltinAdministratorsSid, WinLocalSystemSid,
};
use windows_sys::Win32::System::Pipes::{GetNamedPipeClientProcessId, ImpersonateNamedPipeClient};
use windows_sys::Win32::System::Registry::{HKEY_LOCAL_MACHINE, RRF_RT_REG_DWORD, RegGetValueW};
use windows_sys::Win32::System::Threading::{GetCurrentThread, OpenThreadToken};

/// Access for the settings folder: Windows itself and administrators only.
/// `P` stops permissions being inherited from `C:\ProgramData`; `OICI`
/// passes these on to every file created inside.
const PRIVATE_DIR_SDDL: &str = "D:P(A;OICI;FA;;;SY)(A;OICI;FA;;;BA)";

// Group attributes in a token (winnt.h).
const SE_GROUP_ENABLED: u32 = 0x4;
const SE_GROUP_USE_FOR_DENY_ONLY: u32 = 0x10;

pub fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}

/// Identifier of the current boot; changes on every restart. Windows
/// counts boots in the registry.
pub fn boot_id() -> String {
    let key = wide(r"SYSTEM\CurrentControlSet\Control\Session Manager\Memory Management\PrefetchParameters");
    let value = wide("BootId");
    let mut data = 0u32;
    let mut size = std::mem::size_of::<u32>() as u32;
    // SAFETY: the strings are NUL-terminated, data/size describe a u32.
    let status = unsafe {
        RegGetValueW(
            HKEY_LOCAL_MACHINE,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_DWORD,
            std::ptr::null_mut(),
            (&raw mut data).cast(),
            &mut size,
        )
    };
    if status == 0 { data.to_string() } else { "unknown".into() }
}

/// Who is on the other end of a pipe connection.
pub struct PipePeer {
    pub pid: u32,
    pub user: String,
    pub admin: bool,
}

/// Identifies a pipe client from its access token. Windows only allows
/// this after something has been read from the pipe.
pub fn pipe_peer(pipe: &impl AsRawHandle) -> io::Result<PipePeer> {
    let handle = pipe.as_raw_handle() as HANDLE;
    let mut pid = 0u32;
    // SAFETY: handle is an open pipe; pid is a valid out pointer.
    if unsafe { GetNamedPipeClientProcessId(handle, &mut pid) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let token = client_token(handle)?;
    let user = token_user(&token)?;
    let admin = token_is_admin(&token)?;
    Ok(PipePeer { pid, user, admin })
}

/// The client's token, taken while briefly impersonating it.
fn client_token(pipe: HANDLE) -> io::Result<OwnedHandle> {
    // SAFETY: pipe is an open, connected pipe handle.
    if unsafe { ImpersonateNamedPipeClient(pipe) } == 0 {
        return Err(io::Error::last_os_error());
    }
    let mut token: HANDLE = std::ptr::null_mut();
    // SAFETY: token is a valid out pointer; OpenAsSelf checks access as the
    // service, not as the client.
    let opened = unsafe { OpenThreadToken(GetCurrentThread(), TOKEN_QUERY, 1, &mut token) };
    let error = io::Error::last_os_error();
    // SAFETY: no preconditions. Carrying on as the client would be a
    // security hole, so failing to stop is fatal.
    if unsafe { RevertToSelf() } == 0 {
        eprintln!("iqal: could not stop impersonating a client; exiting");
        std::process::abort();
    }
    if opened == 0 {
        return Err(error);
    }
    // SAFETY: OpenThreadToken succeeded, so token is an open handle we own.
    Ok(unsafe { OwnedHandle::from_raw_handle(token) })
}

/// A token information block, in a buffer aligned for the structures in it.
fn token_info(token: &OwnedHandle, class: i32) -> io::Result<Vec<u64>> {
    let handle = token.as_raw_handle() as HANDLE;
    let mut needed = 0u32;
    // SAFETY: asking for the size only.
    unsafe { GetTokenInformation(handle, class, std::ptr::null_mut(), 0, &mut needed) };
    let mut buf = vec![0u64; (needed as usize).div_ceil(8).max(1)];
    let size = (buf.len() * 8) as u32;
    // SAFETY: buf has room for `size` bytes.
    if unsafe { GetTokenInformation(handle, class, buf.as_mut_ptr().cast(), size, &mut needed) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(buf)
}

fn token_user(token: &OwnedHandle) -> io::Result<String> {
    let buf = token_info(token, TokenUser)?;
    // SAFETY: GetTokenInformation(TokenUser) filled buf with a TOKEN_USER.
    let sid = unsafe { (*buf.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    let mut name = [0u16; 256];
    let mut domain = [0u16; 256];
    let (mut name_len, mut domain_len) = (name.len() as u32, domain.len() as u32);
    let mut kind: SID_NAME_USE = 0;
    // SAFETY: sid points into buf; the lengths match the buffers.
    let ok = unsafe {
        LookupAccountSidW(
            std::ptr::null(),
            sid,
            name.as_mut_ptr(),
            &mut name_len,
            domain.as_mut_ptr(),
            &mut domain_len,
            &mut kind,
        )
    };
    if ok == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(String::from_utf16_lossy(&name[..name_len as usize]))
}

fn well_known_sid(kind: WELL_KNOWN_SID_TYPE) -> io::Result<[u8; SECURITY_MAX_SID_SIZE as usize]> {
    let mut sid = [0u8; SECURITY_MAX_SID_SIZE as usize];
    let mut size = sid.len() as u32;
    // SAFETY: sid has `size` bytes of room.
    if unsafe { CreateWellKnownSid(kind, std::ptr::null_mut(), sid.as_mut_ptr().cast(), &mut size) } == 0 {
        return Err(io::Error::last_os_error());
    }
    Ok(sid)
}

/// Windows itself, or a member of the Administrators group. With UAC an
/// administrator's everyday (not "Run as administrator") token still lists
/// the group, marked deny-only; they count as administrators too, as
/// members of `sudo` do on Linux without running sudo.
fn token_is_admin(token: &OwnedHandle) -> io::Result<bool> {
    let mut admins = well_known_sid(WinBuiltinAdministratorsSid)?;
    let mut system = well_known_sid(WinLocalSystemSid)?;
    let user = token_info(token, TokenUser)?;
    // SAFETY: buf holds a TOKEN_USER; both SIDs are valid.
    let user_sid = unsafe { (*user.as_ptr().cast::<TOKEN_USER>()).User.Sid };
    if unsafe { EqualSid(user_sid, system.as_mut_ptr().cast()) } != 0 {
        return Ok(true);
    }
    let groups = token_info(token, TokenGroups)?;
    // SAFETY: buf holds a TOKEN_GROUPS whose array has GroupCount entries.
    let list = unsafe {
        let g = &*groups.as_ptr().cast::<TOKEN_GROUPS>();
        std::slice::from_raw_parts(g.Groups.as_ptr(), g.GroupCount as usize)
    };
    Ok(list.iter().any(|g| {
        g.Attributes & (SE_GROUP_ENABLED | SE_GROUP_USE_FOR_DENY_ONLY) != 0
            // SAFETY: both are valid SIDs.
            && unsafe { EqualSid(g.Sid, admins.as_mut_ptr().cast()) } != 0
    }))
}

/// A security descriptor built from SDDL text; freed on drop.
pub struct SecurityDescriptor(PSECURITY_DESCRIPTOR);

impl SecurityDescriptor {
    pub fn from_sddl(sddl: &str) -> io::Result<Self> {
        let text = wide(sddl);
        let mut sd: PSECURITY_DESCRIPTOR = std::ptr::null_mut();
        // SAFETY: text is NUL-terminated; sd is a valid out pointer.
        let ok = unsafe {
            ConvertStringSecurityDescriptorToSecurityDescriptorW(
                text.as_ptr(),
                SDDL_REVISION_1,
                &mut sd,
                std::ptr::null_mut(),
            )
        };
        if ok == 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self(sd))
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.0
    }
}

// SAFETY: the descriptor is owned memory that is only read after creation.
unsafe impl Send for SecurityDescriptor {}

impl Drop for SecurityDescriptor {
    fn drop(&mut self) {
        // SAFETY: allocated by ConvertStringSecurityDescriptor...W.
        unsafe { LocalFree(self.0) };
    }
}

/// Limits a folder (and what is created in it) to Windows itself and
/// administrators.
pub fn protect_dir(dir: &Path) -> io::Result<()> {
    let sd = SecurityDescriptor::from_sddl(PRIVATE_DIR_SDDL)?;
    let path = wide(&dir.to_string_lossy());
    // SAFETY: path is NUL-terminated; sd is a valid descriptor.
    let ok = unsafe {
        SetFileSecurityW(path.as_ptr(), DACL_SECURITY_INFORMATION | PROTECTED_DACL_SECURITY_INFORMATION, sd.as_ptr())
    };
    if ok == 0 { Err(io::Error::last_os_error()) } else { Ok(()) }
}
