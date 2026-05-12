//! Main dialog WndProc, control sync, jiggle tick.
//!
//! Direct port of MouseJiggler/MainForm.cs.

use windows::Win32::Foundation::{HINSTANCE, HWND, LPARAM, WPARAM};
use windows::Win32::UI::Controls::{
    BST_CHECKED, BST_UNCHECKED, CheckDlgButton, IsDlgButtonChecked,
};
use windows::Win32::UI::WindowsAndMessaging::{
    BN_CLICKED, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL, CBN_SELCHANGE, CreateDialogParamW,
    DestroyWindow, EN_CHANGE, GWLP_USERDATA, GetDlgItem, GetDlgItemInt, GetWindowLongPtrW,
    KillTimer, PostQuitMessage, SHOW_WINDOW_CMD, SW_HIDE, SW_SHOW, SendDlgItemMessageW,
    SetDlgItemInt, SetDlgItemTextW, SetTimer, SetWindowLongPtrW, ShowWindow, WM_CLOSE,
    WM_COMMAND, WM_DESTROY, WM_INITDIALOG, WM_TIMER,
};
use windows::core::PCWSTR;

use crate::ids::{
    DISTANCE_MAX, DISTANCE_MIN, IDC_BTN_ABOUT, IDC_BTN_TRAYIFY, IDC_CB_MINIMIZE, IDC_CB_RANDOM,
    IDC_CMB_MODE, IDC_JIGGLING, IDC_LBL_PERIOD_DISPLAY, IDC_NUD_DISTANCE, IDC_NUD_PERIOD,
    IDC_PANEL_SETTINGS, IDC_SETTINGS, IDD_MAIN, IDM_TRAY_EXIT, IDM_TRAY_OPEN, IDM_TRAY_START,
    IDM_TRAY_STOP, PERIOD_MAX, PERIOD_MIN, TIMER_JIGGLE, WM_APP_TRAY,
};
use crate::jiggle::{self, Mode, PauseDetector};
use crate::rng::Rng;
use crate::settings::{self, Settings};
use crate::tray::{self, Tray, TrayEvent};
use crate::ui_about;
use crate::util::to_wide;

const MAX_TIP: usize = 63;

pub struct AppState {
    pub instance: HINSTANCE,
    pub settings: Settings,
    pub jiggling: bool,
    pub step: usize,
    pub pause: PauseDetector,
    pub rng: Rng,
    pub tray: Tray,
    pub settings_panel_visible: bool,
    pub start_jiggling_on_load: bool,
    pub minimize_on_load: bool,
    pub show_settings_on_load: bool,
    /// Suppress EN_CHANGE / CBN_SELCHANGE writes during WM_INITDIALOG.
    pub initializing: bool,
}

/// Create the modeless main dialog and return its HWND.
/// Ownership of `state` is transferred into the dialog via GWLP_USERDATA;
/// the WM_DESTROY handler frees it.
pub fn create(instance: HINSTANCE, state: Box<AppState>) -> Option<HWND> {
    let ptr = Box::into_raw(state) as isize;
    let hwnd = unsafe {
        CreateDialogParamW(
            Some(instance),
            PCWSTR(IDD_MAIN as usize as *const u16),
            None,
            Some(Some(dlg_proc)),
            LPARAM(ptr),
        )
    };
    match hwnd {
        Ok(h) if !h.is_invalid() => Some(h),
        _ => {
            // Restore ownership so we drop properly.
            let _ = unsafe { Box::from_raw(ptr as *mut AppState) };
            None
        }
    }
}

fn state_from_hwnd<'a>(hwnd: HWND) -> Option<&'a mut AppState> {
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        None
    } else {
        Some(unsafe { &mut *(ptr as *mut AppState) })
    }
}

unsafe extern "system" fn dlg_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> isize {
    match msg {
        WM_INITDIALOG => {
            // Install state pointer.
            unsafe {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, lparam.0);
            }
            let state = state_from_hwnd(hwnd).expect("state must be set in WM_INITDIALOG");
            state.initializing = true;
            init_controls(hwnd, state);
            state.initializing = false;

            if state.show_settings_on_load {
                toggle_settings_panel(hwnd, state, true);
            }
            if state.start_jiggling_on_load {
                set_jiggling(hwnd, state, true);
            }
            if state.minimize_on_load {
                minimize_to_tray(hwnd, state);
            }
            1
        }
        WM_COMMAND => {
            let Some(state) = state_from_hwnd(hwnd) else {
                return 0;
            };
            handle_command(hwnd, state, wparam, lparam);
            0
        }
        WM_TIMER => {
            let Some(state) = state_from_hwnd(hwnd) else {
                return 0;
            };
            if wparam.0 == TIMER_JIGGLE {
                on_jiggle_tick(hwnd, state);
            }
            0
        }
        m if m == WM_APP_TRAY => {
            let Some(state) = state_from_hwnd(hwnd) else {
                return 0;
            };
            match tray::classify_tray_event(lparam) {
                TrayEvent::Restore => restore_from_tray(hwnd, state),
                TrayEvent::ContextMenu => {
                    let chosen = tray::show_context_menu(hwnd, state.jiggling);
                    handle_tray_command(hwnd, state, chosen);
                }
                TrayEvent::Other => {}
            }
            0
        }
        WM_CLOSE => {
            // Match upstream: closing the window exits the application.
            let Some(state) = state_from_hwnd(hwnd) else {
                return 0;
            };
            jiggle::allow_sleep();
            state.tray.remove();
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
            1
        }
        WM_DESTROY => {
            // Free the boxed state.
            let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
            if ptr != 0 {
                let _ = unsafe { Box::from_raw(ptr as *mut AppState) };
                unsafe {
                    SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                }
            }
            unsafe {
                PostQuitMessage(0);
            }
            0
        }
        _ => 0,
    }
}

// ---------- Initialization ----------

fn init_controls(hwnd: HWND, state: &mut AppState) {
    // Populate mode combo.
    for mode in Mode::all() {
        let s = to_wide(mode.as_str());
        unsafe {
            SendDlgItemMessageW(
                hwnd,
                IDC_CMB_MODE,
                CB_ADDSTRING,
                WPARAM(0),
                LPARAM(s.as_ptr() as isize),
            );
        }
    }
    let idx = Mode::all()
        .iter()
        .position(|&m| m == state.settings.mode)
        .unwrap_or(0);
    unsafe {
        SendDlgItemMessageW(
            hwnd,
            IDC_CMB_MODE,
            CB_SETCURSEL,
            WPARAM(idx),
            LPARAM(0),
        );
    }

    // Numeric edits (clamped to limits).
    let _ = unsafe { SetDlgItemInt(hwnd, IDC_NUD_PERIOD, state.settings.period_secs, false) };
    let _ = unsafe { SetDlgItemInt(hwnd, IDC_NUD_DISTANCE, state.settings.distance, false) };
    update_period_display(hwnd, state.settings.period_secs);

    // Checkboxes.
    set_check(hwnd, IDC_CB_RANDOM, state.settings.random_timer);
    set_check(hwnd, IDC_CB_MINIMIZE, state.settings.minimize_on_startup);

    // Settings panel hidden by default.
    set_settings_panel_visible(hwnd, false);
    state.settings_panel_visible = false;
}

// ---------- Command dispatch ----------

fn handle_command(hwnd: HWND, state: &mut AppState, wparam: WPARAM, _lparam: LPARAM) {
    let id = (wparam.0 & 0xFFFF) as i32;
    let notify = ((wparam.0 >> 16) & 0xFFFF) as u32;

    match (id, notify) {
        (IDC_JIGGLING, n) if n == BN_CLICKED => {
            let on = is_checked(hwnd, IDC_JIGGLING);
            set_jiggling(hwnd, state, on);
        }
        (IDC_SETTINGS, n) if n == BN_CLICKED => {
            let on = is_checked(hwnd, IDC_SETTINGS);
            toggle_settings_panel(hwnd, state, on);
        }
        (IDC_BTN_TRAYIFY, n) if n == BN_CLICKED => {
            minimize_to_tray(hwnd, state);
        }
        (IDC_BTN_ABOUT, n) if n == BN_CLICKED => {
            ui_about::show(hwnd, state.instance);
        }
        (IDC_CMB_MODE, n) if n == CBN_SELCHANGE && !state.initializing => {
            let idx = unsafe {
                SendDlgItemMessageW(hwnd, IDC_CMB_MODE, CB_GETCURSEL, WPARAM(0), LPARAM(0))
            }
            .0 as usize;
            if let Some(&m) = Mode::all().get(idx) {
                state.settings.mode = m;
                state.step = 0;
                settings::save(&state.settings);
                update_tooltip(state);
            }
        }
        (IDC_NUD_PERIOD, n) if n == EN_CHANGE && !state.initializing => {
            let v = read_dlg_int(hwnd, IDC_NUD_PERIOD).clamp(PERIOD_MIN, PERIOD_MAX);
            state.settings.period_secs = v;
            settings::save(&state.settings);
            update_period_display(hwnd, v);
            if state.jiggling {
                restart_timer(hwnd, state);
            }
            update_tooltip(state);
        }
        (IDC_NUD_DISTANCE, n) if n == EN_CHANGE && !state.initializing => {
            let v = read_dlg_int(hwnd, IDC_NUD_DISTANCE).clamp(DISTANCE_MIN, DISTANCE_MAX);
            state.settings.distance = v;
            state.step = 0;
            settings::save(&state.settings);
            update_tooltip(state);
        }
        (IDC_CB_RANDOM, n) if n == BN_CLICKED => {
            state.settings.random_timer = is_checked(hwnd, IDC_CB_RANDOM);
            settings::save(&state.settings);
            update_tooltip(state);
        }
        (IDC_CB_MINIMIZE, n) if n == BN_CLICKED => {
            state.settings.minimize_on_startup = is_checked(hwnd, IDC_CB_MINIMIZE);
            settings::save(&state.settings);
        }
        _ => {}
    }
}

fn handle_tray_command(hwnd: HWND, state: &mut AppState, cmd: u32) {
    match cmd {
        c if c == IDM_TRAY_OPEN => restore_from_tray(hwnd, state),
        c if c == IDM_TRAY_START => {
            set_check(hwnd, IDC_JIGGLING, true);
            set_jiggling(hwnd, state, true);
        }
        c if c == IDM_TRAY_STOP => {
            set_check(hwnd, IDC_JIGGLING, false);
            set_jiggling(hwnd, state, false);
        }
        c if c == IDM_TRAY_EXIT => {
            unsafe {
                let _ = windows::Win32::UI::WindowsAndMessaging::PostMessageW(
                    Some(hwnd),
                    WM_CLOSE,
                    WPARAM(0),
                    LPARAM(0),
                );
            }
        }
        _ => {}
    }
}

// ---------- Jiggle tick ----------

fn set_jiggling(hwnd: HWND, state: &mut AppState, on: bool) {
    state.jiggling = on;
    state.step = 0;

    if on {
        jiggle::stay_awake();
        state.pause.update_position();
        restart_timer(hwnd, state);
    } else {
        jiggle::allow_sleep();
        let _ = unsafe { KillTimer(Some(hwnd), TIMER_JIGGLE) };
    }
    update_tooltip(state);
}

fn restart_timer(hwnd: HWND, state: &AppState) {
    let interval_ms = (state.settings.period_secs as u32).saturating_mul(1000).max(1);
    unsafe {
        SetTimer(Some(hwnd), TIMER_JIGGLE, interval_ms, None);
    }
}

fn on_jiggle_tick(hwnd: HWND, state: &mut AppState) {
    // Smart pause — skip this tick if the user moved the cursor.
    if state.pause.has_mouse_moved() {
        return;
    }

    let (dx, dy) = jiggle::step_delta(
        state.settings.mode,
        state.step,
        state.settings.distance as i32,
    );
    state.step = (state.step + 1) % jiggle::pattern_len(state.settings.mode);

    jiggle::jiggle(dx, dy);
    state.pause.update_position();

    // Apply random variation if enabled, matching MainForm.cs:193-200.
    let next_secs = if state.settings.random_timer {
        let v = state.rng.range_inclusive(1, state.settings.period_secs);
        update_period_display(hwnd, v);
        v
    } else {
        state.settings.period_secs
    };
    let next_ms = (next_secs as u32).saturating_mul(1000).max(1);
    unsafe {
        SetTimer(Some(hwnd), TIMER_JIGGLE, next_ms, None);
    }
}

// ---------- Settings panel / tray ----------

fn toggle_settings_panel(hwnd: HWND, state: &mut AppState, on: bool) {
    set_settings_panel_visible(hwnd, on);
    state.settings_panel_visible = on;
}

fn set_settings_panel_visible(hwnd: HWND, on: bool) {
    // Hide/show the groupbox + every settings control inside it.
    // (Dialog itself is not resized in this initial framework.)
    let cmd: SHOW_WINDOW_CMD = if on { SW_SHOW } else { SW_HIDE };
    for id in [
        IDC_PANEL_SETTINGS,
        IDC_CMB_MODE,
        IDC_NUD_PERIOD,
        IDC_NUD_DISTANCE,
        IDC_CB_RANDOM,
        IDC_CB_MINIMIZE,
        IDC_LBL_PERIOD_DISPLAY,
    ] {
        if let Ok(h) = unsafe { GetDlgItem(Some(hwnd), id) } {
            unsafe {
                let _ = ShowWindow(h, cmd);
            }
        }
    }
}

fn minimize_to_tray(hwnd: HWND, state: &mut AppState) {
    if !state.tray.visible {
        state.tray.add(&tooltip_text(state));
    } else {
        update_tooltip(state);
    }
    unsafe {
        let _ = ShowWindow(hwnd, SW_HIDE);
    }
}

fn restore_from_tray(hwnd: HWND, state: &mut AppState) {
    state.tray.remove();
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOW);
    }
}

fn update_tooltip(state: &mut AppState) {
    if state.tray.visible {
        let text = tooltip_text(state);
        state.tray.update_tip(&text);
    }
}

fn tooltip_text(state: &AppState) -> String {
    if !state.jiggling {
        return "Not jiggling the mouse.".to_string();
    }
    let rnd = if state.settings.random_timer {
        " with random variation,"
    } else {
        ""
    };
    let text = format!(
        "Jiggling mouse every {} s,{} mode: {} (Δ {}).",
        state.settings.period_secs,
        rnd,
        state.settings.mode.as_str(),
        state.settings.distance
    );
    if text.chars().count() > MAX_TIP {
        let mut t: String = text.chars().take(MAX_TIP - 3).collect();
        t.push_str("...");
        t
    } else {
        text
    }
}

// ---------- Small control helpers ----------

fn is_checked(hwnd: HWND, id: i32) -> bool {
    let r = unsafe { IsDlgButtonChecked(hwnd, id) };
    r == BST_CHECKED.0
}

fn set_check(hwnd: HWND, id: i32, on: bool) {
    let v = if on { BST_CHECKED } else { BST_UNCHECKED };
    unsafe {
        let _ = CheckDlgButton(hwnd, id, v);
    }
}

fn read_dlg_int(hwnd: HWND, id: i32) -> u32 {
    unsafe { GetDlgItemInt(hwnd, id, None, false) }
}

fn update_period_display(hwnd: HWND, secs: u32) {
    let text = format!("{} s", secs);
    let wide = to_wide(&text);
    unsafe {
        let _ = SetDlgItemTextW(hwnd, IDC_LBL_PERIOD_DISPLAY, PCWSTR(wide.as_ptr()));
    }
}

