//! Shared helpers (string conversion, console output).

use windows::Win32::Foundation::HANDLE;
use windows::Win32::System::Console::{
    ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole, GetStdHandle, STD_OUTPUT_HANDLE,
    WriteConsoleW,
};

/// Convert a Rust &str to a null-terminated UTF-16 buffer.
pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Attach to the parent process's console (so a GUI-subsystem .exe can
/// print --help / --version output when launched from cmd.exe).
/// Returns true if attachment succeeded.
pub fn attach_parent_console() -> bool {
    unsafe { AttachConsole(ATTACH_PARENT_PROCESS) }.is_ok()
}

pub fn free_console() {
    unsafe {
        let _ = FreeConsole();
    }
}

/// Write a UTF-8 string to the currently attached console, line-by-line.
/// Falls back silently if no console is attached.
pub fn console_print(s: &str) {
    let h: HANDLE = match unsafe { GetStdHandle(STD_OUTPUT_HANDLE) } {
        Ok(h) if !h.is_invalid() => h,
        _ => return,
    };
    let wide = to_wide(s);
    // Length excludes the terminating null we appended.
    let len = (wide.len() - 1) as u32;
    let mut written: u32 = 0;
    unsafe {
        let _ = WriteConsoleW(h, &wide[..len as usize], Some(&mut written), None);
    }
}

pub fn console_println(s: &str) {
    console_print(s);
    console_print("\r\n");
}
