//! Shared helpers (string conversion, console output).

use std::io::{self, Write};

use windows::Win32::Foundation::HWND;
use windows::Win32::System::Console::{ATTACH_PARENT_PROCESS, AttachConsole, FreeConsole};
use windows::Win32::UI::WindowsAndMessaging::{MB_ICONERROR, MB_OK, MessageBoxW};
use windows::core::PCWSTR;

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
    let mut stdout = io::stdout().lock();
    let _ = stdout.write_all(s.as_bytes());
    let _ = stdout.flush();
}

pub fn console_println(s: &str) {
    console_print(s);
    console_print("\r\n");
}

pub fn report_error(summary: &str, detail: &str) {
    console_println(summary);
    if !detail.is_empty() {
        console_println(detail);
    }

    let caption = to_wide("Mouse Jiggler");
    let body = if detail.is_empty() {
        summary.to_string()
    } else {
        format!("{summary}\r\n\r\n{detail}")
    };
    let body = to_wide(&body);

    unsafe {
        let _ = MessageBoxW(
            Some(HWND::default()),
            PCWSTR(body.as_ptr()),
            PCWSTR(caption.as_ptr()),
            MB_OK | MB_ICONERROR,
        );
    }
}
