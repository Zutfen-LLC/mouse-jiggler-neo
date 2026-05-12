//! Settings persistence — HKCU\Software\ArkaneSystems\MouseJiggler.
//!
//! Replaces the C# `Settings.Default` (user.config). Five values, loaded
//! on startup, written through on every change.

use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_READ, KEY_SET_VALUE, REG_DWORD, REG_OPENED_EXISTING_KEY,
    REG_OPTION_NON_VOLATILE, REG_SZ, RegCloseKey, RegCreateKeyExW, RegQueryValueExW,
    RegSetValueExW,
};
use windows::core::PCWSTR;

use crate::ids::{
    DISTANCE_DEFAULT, DISTANCE_MAX, DISTANCE_MIN, PERIOD_DEFAULT, PERIOD_MAX, PERIOD_MIN,
};
use crate::jiggle::Mode;
use crate::util::to_wide;

const SUBKEY: &str = "Software\\ArkaneSystems\\MouseJiggler";

const V_MINIMIZE: &str = "MinimizeOnStartup";
const V_RANDOM: &str = "RandomTimer";
const V_MODE: &str = "JiggleMode";
const V_PERIOD: &str = "JigglePeriod";
const V_DISTANCE: &str = "JiggleDistance";

#[derive(Clone, Debug)]
pub struct Settings {
    pub minimize_on_startup: bool,
    pub random_timer: bool,
    pub mode: Mode,
    pub period_secs: u32,
    pub distance: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            minimize_on_startup: false,
            random_timer: false,
            mode: Mode::Normal,
            period_secs: PERIOD_DEFAULT,
            distance: DISTANCE_DEFAULT,
        }
    }
}

fn open_or_create() -> Option<HKEY> {
    let path = to_wide(SUBKEY);
    let mut hkey = HKEY::default();
    let mut disposition = REG_OPENED_EXISTING_KEY;
    let status = unsafe {
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            PCWSTR(path.as_ptr()),
            None,
            PCWSTR::null(),
            REG_OPTION_NON_VOLATILE,
            KEY_READ | KEY_SET_VALUE,
            None,
            &mut hkey,
            Some(&mut disposition),
        )
    };
    if status.is_ok() { Some(hkey) } else { None }
}

fn close(hkey: HKEY) {
    unsafe {
        let _ = RegCloseKey(hkey);
    }
}

fn read_dword(hkey: HKEY, name: &str) -> Option<u32> {
    let name_w = to_wide(name);
    let mut ty = REG_DWORD;
    let mut buf = [0u8; 4];
    let mut cb: u32 = 4;
    let status = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr()),
            Some(&mut cb),
        )
    };
    if status.is_ok() && ty == REG_DWORD && cb == 4 {
        Some(u32::from_le_bytes(buf))
    } else {
        None
    }
}

fn read_string(hkey: HKEY, name: &str) -> Option<String> {
    let name_w = to_wide(name);
    let mut ty = REG_SZ;
    // First call: ask for required size.
    let mut cb: u32 = 0;
    let _ = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut ty),
            None,
            Some(&mut cb),
        )
    };
    if ty != REG_SZ || cb == 0 {
        return None;
    }
    let mut buf: Vec<u8> = vec![0u8; cb as usize];
    let status = unsafe {
        RegQueryValueExW(
            hkey,
            PCWSTR(name_w.as_ptr()),
            None,
            Some(&mut ty),
            Some(buf.as_mut_ptr()),
            Some(&mut cb),
        )
    };
    if !status.is_ok() {
        return None;
    }
    // cb is bytes including any trailing nulls.
    let wide: Vec<u16> = buf
        .chunks_exact(2)
        .map(|c| u16::from_le_bytes([c[0], c[1]]))
        .take_while(|&w| w != 0)
        .collect();
    Some(String::from_utf16_lossy(&wide))
}

fn write_dword(hkey: HKEY, name: &str, value: u32) {
    let name_w = to_wide(name);
    let bytes = value.to_le_bytes();
    unsafe {
        let _ = RegSetValueExW(
            hkey,
            PCWSTR(name_w.as_ptr()),
            None,
            REG_DWORD,
            Some(&bytes),
        );
    }
}

fn write_string(hkey: HKEY, name: &str, value: &str) {
    let name_w = to_wide(name);
    let wide: Vec<u16> = value.encode_utf16().chain(std::iter::once(0)).collect();
    let bytes: &[u8] = unsafe {
        std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2)
    };
    unsafe {
        let _ = RegSetValueExW(
            hkey,
            PCWSTR(name_w.as_ptr()),
            None,
            REG_SZ,
            Some(bytes),
        );
    }
}

pub fn load() -> Settings {
    let mut s = Settings::default();
    let Some(hkey) = open_or_create() else { return s; };

    if let Some(v) = read_dword(hkey, V_MINIMIZE) {
        s.minimize_on_startup = v != 0;
    }
    if let Some(v) = read_dword(hkey, V_RANDOM) {
        s.random_timer = v != 0;
    }
    if let Some(v) = read_string(hkey, V_MODE) {
        if let Some(m) = Mode::parse(&v) {
            s.mode = m;
        }
    }
    if let Some(v) = read_dword(hkey, V_PERIOD) {
        s.period_secs = v.clamp(PERIOD_MIN, PERIOD_MAX);
    }
    if let Some(v) = read_dword(hkey, V_DISTANCE) {
        s.distance = v.clamp(DISTANCE_MIN, DISTANCE_MAX);
    }

    close(hkey);
    s
}

pub fn save(s: &Settings) {
    let Some(hkey) = open_or_create() else { return; };
    write_dword(hkey, V_MINIMIZE, s.minimize_on_startup as u32);
    write_dword(hkey, V_RANDOM, s.random_timer as u32);
    write_string(hkey, V_MODE, s.mode.as_str());
    write_dword(hkey, V_PERIOD, s.period_secs);
    write_dword(hkey, V_DISTANCE, s.distance);
    close(hkey);
}
