#![windows_subsystem = "windows"]

mod cli;
mod ids;
mod jiggle;
mod rng;
mod settings;
mod single_instance;
mod tray;
mod ui_about;
mod ui_main;
mod util;

use windows::Win32::Foundation::HINSTANCE;
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Controls::{
    ICC_LINK_CLASS, ICC_STANDARD_CLASSES, ICC_UPDOWN_CLASS, INITCOMMONCONTROLSEX,
    InitCommonControlsEx,
};
use windows::Win32::UI::HiDpi::{
    DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext,
};
use windows::Win32::UI::WindowsAndMessaging::{
    DispatchMessageW, GetMessageW, HICON, IsDialogMessageW, LoadIconW, MSG, SW_SHOW, ShowWindow,
    TranslateMessage,
};
use windows::core::PCWSTR;

use crate::cli::{Args, ParseOutcome};
use crate::ids::IDI_APP;
use crate::single_instance::AcquireResult;
use crate::ui_main::AppState;

fn main() -> std::process::ExitCode {
    // Attach to parent console so --help / --version / single-instance errors
    // print where the user can see them when launched from a terminal.
    let attached = util::attach_parent_console();

    // Parse CLI before doing anything else.
    let args = match cli::parse() {
        ParseOutcome::Run(a) => a,
        ParseOutcome::PrintAndExit(code) => {
            if attached {
                util::free_console();
            }
            return std::process::ExitCode::from(code as u8);
        }
        ParseOutcome::Error(msg) => {
            util::report_error(
                &format!("error: {msg}"),
                "Run with --help for usage information.",
            );
            if attached {
                util::free_console();
            }
            return std::process::ExitCode::from(1);
        }
    };

    // Single-instance enforcement.
    let _lock = match single_instance::acquire() {
        AcquireResult::Acquired(l) => l,
        AcquireResult::AlreadyRunning => {
            util::report_error("Mouse Jiggler is already running. Aborting.", "");
            if attached {
                util::free_console();
            }
            return std::process::ExitCode::from(1);
        }
    };

    // We're committed to running the GUI now; release the console.
    if attached {
        util::free_console();
    }

    // Per-monitor DPI awareness (manifest also declares this, belt and suspenders).
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }

    // Common Controls v6: needed for SysLink (about dialog) and UpDown (settings).
    let icc = INITCOMMONCONTROLSEX {
        dwSize: size_of::<INITCOMMONCONTROLSEX>() as u32,
        dwICC: ICC_STANDARD_CLASSES | ICC_UPDOWN_CLASS | ICC_LINK_CLASS,
    };
    unsafe {
        let _ = InitCommonControlsEx(&icc);
    }

    let hmodule = unsafe { GetModuleHandleW(None) }.expect("GetModuleHandleW");
    let instance = HINSTANCE(hmodule.0);

    let icon: HICON = unsafe { LoadIconW(Some(instance), PCWSTR(IDI_APP as usize as *const u16)) }
        .unwrap_or_default();

    // Resolve effective initial state from (settings ⊕ CLI overrides).
    let stored = settings::load();
    let effective = resolve_initial(&args, stored);

    let state = Box::new(AppState {
        instance,
        settings: effective.settings.clone(),
        jiggling: false,
        step: 0,
        pause: jiggle::PauseDetector::default(),
        rng: rng::Rng::new(),
        tray: tray::Tray::new(windows::Win32::Foundation::HWND::default(), icon),
        settings_panel_visible: false,
        start_jiggling_on_load: args.jiggle,
        minimize_on_load: effective.minimize,
        show_settings_on_load: args.settings_panel,
        initializing: false,
    });

    let hwnd = match ui_main::create(instance, state) {
        Ok(hwnd) => hwnd,
        Err(err) => {
            let detail = format!("Failed to create the main dialog.\r\nWin32 error: {err}");
            let attached = util::attach_parent_console();
            util::report_error("Mouse Jiggler could not start.", &detail);
            if attached {
                util::free_console();
            }
            return std::process::ExitCode::from(2);
        }
    };

    // The Tray's owner HWND was created with a placeholder; fix it now.
    fix_tray_owner(hwnd);

    if !effective.minimize {
        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
    }

    // Modeless message loop.
    let mut msg = MSG::default();
    loop {
        let r = unsafe { GetMessageW(&mut msg, None, 0, 0) };
        if r.0 <= 0 {
            break;
        }
        unsafe {
            if !IsDialogMessageW(hwnd, &msg).as_bool() {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }

    std::process::ExitCode::from(0)
}

struct Effective {
    settings: settings::Settings,
    minimize: bool,
}

/// Merge stored settings with explicit CLI overrides. The C# build resolves
/// these defaults inside System.CommandLine; we do it inline.
fn resolve_initial(args: &Args, stored: settings::Settings) -> Effective {
    let mut s = stored;
    if let Some(m) = args.mode {
        s.mode = m;
    }
    if let Some(p) = args.seconds {
        s.period_secs = p;
    }
    if let Some(d) = args.distance {
        s.distance = d;
    }
    if let Some(r) = args.random {
        s.random_timer = r;
    }
    let minimize = args.minimized.unwrap_or(s.minimize_on_startup);
    Effective {
        settings: s,
        minimize,
    }
}

/// After the main dialog HWND exists, retroactively give the Tray the right owner.
/// (We construct Tray before the HWND is available; this is the cleanup.)
fn fix_tray_owner(hwnd: windows::Win32::Foundation::HWND) {
    // Reach into the boxed AppState via GWLP_USERDATA and patch the owner.
    use windows::Win32::UI::WindowsAndMessaging::{GWLP_USERDATA, GetWindowLongPtrW};
    let ptr = unsafe { GetWindowLongPtrW(hwnd, GWLP_USERDATA) };
    if ptr == 0 {
        return;
    }
    let state = unsafe { &mut *(ptr as *mut AppState) };
    state.tray.set_owner(hwnd);
}
