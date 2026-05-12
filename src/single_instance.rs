//! Single-instance enforcement via a named mutex.
//!
//! Mirrors MouseJiggler/Program.cs:41-56 — mutex name kept identical so
//! the Rust port mutually excludes with the upstream C# build.

use windows::Win32::Foundation::{ERROR_ALREADY_EXISTS, GetLastError, HANDLE};
use windows::Win32::System::Threading::CreateMutexW;
use windows::core::PCWSTR;

use crate::util::to_wide;

const MUTEX_NAME: &str = "single instance: ArkaneSystems.MouseJiggler";

/// Handle to a held mutex. Drop on process exit; Windows releases the
/// kernel object automatically when the handle is closed.
pub struct InstanceLock {
    _handle: HANDLE,
}

pub enum AcquireResult {
    Acquired(InstanceLock),
    AlreadyRunning,
}

pub fn acquire() -> AcquireResult {
    let name = to_wide(MUTEX_NAME);
    let handle = unsafe { CreateMutexW(None, true, PCWSTR(name.as_ptr())) };

    match handle {
        Ok(h) if !h.is_invalid() => {
            let last = unsafe { GetLastError() };
            if last == ERROR_ALREADY_EXISTS {
                // Another process holds the mutex; drop our handle.
                let _ = unsafe { windows::Win32::Foundation::CloseHandle(h) };
                AcquireResult::AlreadyRunning
            } else {
                AcquireResult::Acquired(InstanceLock { _handle: h })
            }
        }
        _ => AcquireResult::AlreadyRunning,
    }
}
