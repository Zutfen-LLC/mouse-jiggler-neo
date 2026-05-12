//! About dialog (modal).

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    DialogBoxParamW, EndDialog, IDCANCEL, IDOK, WM_COMMAND, WM_INITDIALOG,
};
use windows::core::PCWSTR;

use crate::ids::IDD_ABOUT;

pub fn show(parent: HWND, instance: HINSTANCE) {
    unsafe {
        let _ = DialogBoxParamW(
            Some(instance),
            PCWSTR(IDD_ABOUT as usize as *const u16),
            Some(parent),
            Some(Some(about_proc)),
            LPARAM(0),
        );
    }
}

unsafe extern "system" fn about_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    _lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => 1,
        WM_COMMAND => {
            let id = (wparam.0 & 0xFFFF) as i32;
            if id == IDOK.0 || id == IDCANCEL.0 {
                unsafe {
                    let _ = EndDialog(hwnd, id as isize);
                }
                return 1;
            }
            0
        }
        _ => 0,
    }
}
