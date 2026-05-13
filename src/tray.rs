//! System tray icon (Shell_NotifyIconW) + context menu.

use windows::Win32::Foundation::{HWND, LPARAM, POINT};
use windows::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    Shell_NotifyIconW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyMenu, GetCursorPos, HICON, HMENU, MF_SEPARATOR, MF_STRING,
    SetForegroundWindow, TPM_BOTTOMALIGN, TPM_RIGHTBUTTON, TrackPopupMenu, WM_LBUTTONDBLCLK,
    WM_RBUTTONUP,
};
use windows::core::PCWSTR;

use crate::ids::{IDM_TRAY_EXIT, IDM_TRAY_OPEN, IDM_TRAY_START, IDM_TRAY_STOP, WM_APP_TRAY};
use crate::util::to_wide;

const TRAY_ICON_UID: u32 = 1;

pub struct Tray {
    owner: HWND,
    icon: HICON,
    /// Whether the icon is currently registered with the shell.
    pub registered: bool,
}

impl Tray {
    pub fn new(owner: HWND, icon: HICON) -> Self {
        Self {
            owner,
            icon,
            registered: false,
        }
    }

    /// Update the owner HWND post-construction (used because the dialog HWND
    /// doesn't exist until CreateDialogParamW returns, after AppState is built).
    pub fn set_owner(&mut self, owner: HWND) {
        self.owner = owner;
    }

    fn build_nid(&self, tip: &str, flags: u32) -> NOTIFYICONDATAW {
        let mut nid = NOTIFYICONDATAW {
            cbSize: size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.owner,
            uID: TRAY_ICON_UID,
            uFlags: windows::Win32::UI::Shell::NOTIFY_ICON_DATA_FLAGS(flags),
            uCallbackMessage: WM_APP_TRAY,
            hIcon: self.icon,
            ..Default::default()
        };
        // szTip is [u16; 128]. Truncate to 127 to leave room for null.
        let wide: Vec<u16> = tip
            .encode_utf16()
            .take(127)
            .chain(std::iter::once(0))
            .collect();
        nid.szTip[..wide.len()].copy_from_slice(&wide);
        nid
    }

    pub fn add(&mut self, tip: &str) -> bool {
        let nid = self.build_nid(tip, (NIF_ICON | NIF_MESSAGE | NIF_TIP).0);
        let ok = unsafe { Shell_NotifyIconW(NIM_ADD, &nid).as_bool() };
        self.registered = ok;
        ok
    }

    pub fn ensure_added(&mut self, tip: &str) -> bool {
        if self.registered {
            self.update_tip(tip)
        } else {
            self.add(tip)
        }
    }

    pub fn update_tip(&mut self, tip: &str) -> bool {
        if !self.registered {
            return false;
        }
        let nid = self.build_nid(tip, (NIF_TIP).0);
        let ok = unsafe { Shell_NotifyIconW(NIM_MODIFY, &nid).as_bool() };
        if !ok {
            self.registered = false;
        }
        ok
    }

    pub fn remove(&mut self) -> bool {
        if !self.registered {
            return false;
        }
        let nid = self.build_nid("", 0);
        let ok = unsafe { Shell_NotifyIconW(NIM_DELETE, &nid).as_bool() };
        self.registered = false;
        ok
    }
}

/// Context menu shown when the user right-clicks the tray icon.
/// `jiggling` controls which of Start/Stop is shown.
pub fn show_context_menu(owner: HWND, jiggling: bool) -> u32 {
    unsafe {
        let menu: HMENU = match CreatePopupMenu() {
            Ok(m) => m,
            Err(_) => return 0,
        };

        let open = to_wide("&Open Mouse Jiggler Neo");
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TRAY_OPEN as usize,
            PCWSTR(open.as_ptr()),
        );
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

        if !jiggling {
            let s = to_wide("&Start Jiggling");
            let _ = AppendMenuW(menu, MF_STRING, IDM_TRAY_START as usize, PCWSTR(s.as_ptr()));
        } else {
            let s = to_wide("S&top Jiggling");
            let _ = AppendMenuW(menu, MF_STRING, IDM_TRAY_STOP as usize, PCWSTR(s.as_ptr()));
        }

        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
        let exit = to_wide("E&xit");
        let _ = AppendMenuW(
            menu,
            MF_STRING,
            IDM_TRAY_EXIT as usize,
            PCWSTR(exit.as_ptr()),
        );

        let mut pt = POINT::default();
        let _ = GetCursorPos(&mut pt);
        // TPM_RETURNCMD asks TrackPopupMenu to return the chosen id rather
        // than posting WM_COMMAND. Simpler dispatch for the caller.
        let _ = SetForegroundWindow(owner);
        let chosen = TrackPopupMenu(
            menu,
            TPM_RIGHTBUTTON
                | TPM_BOTTOMALIGN
                | windows::Win32::UI::WindowsAndMessaging::TPM_RETURNCMD,
            pt.x,
            pt.y,
            Some(0),
            owner,
            None,
        );
        let _ = DestroyMenu(menu);
        chosen.0 as u32
    }
}

/// Decode the lParam of a WM_APP_TRAY notification.
pub fn classify_tray_event(lparam: LPARAM) -> TrayEvent {
    let msg = lparam.0 as u32 & 0xFFFF;
    match msg {
        WM_LBUTTONDBLCLK => TrayEvent::Restore,
        WM_RBUTTONUP => TrayEvent::ContextMenu,
        _ => TrayEvent::Other,
    }
}

pub enum TrayEvent {
    Restore,
    ContextMenu,
    Other,
}
