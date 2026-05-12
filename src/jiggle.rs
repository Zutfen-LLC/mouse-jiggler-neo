//! Jiggle patterns, SendInput, execution state, smart-pause.

use windows::Win32::Foundation::POINT;
use windows::Win32::System::Power::{
    ES_CONTINUOUS, ES_DISPLAY_REQUIRED, ES_SYSTEM_REQUIRED, SetThreadExecutionState,
};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_MOVE, MOUSEINPUT, SendInput,
};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mode {
    Normal,
    Zen,
    Circle,
    Linear,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Normal => "Normal",
            Mode::Zen => "Zen",
            Mode::Circle => "Circle",
            Mode::Linear => "Linear",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "normal" => Some(Mode::Normal),
            "zen" => Some(Mode::Zen),
            "circle" => Some(Mode::Circle),
            "linear" => Some(Mode::Linear),
            _ => None,
        }
    }

    pub fn all() -> &'static [Mode] {
        &[Mode::Normal, Mode::Zen, Mode::Circle, Mode::Linear]
    }
}

// Base patterns at distance=1, matching JigglePatterns.cs.
const NORMAL: &[(i32, i32)] = &[(4, 4), (-4, -4)];
const ZEN: &[(i32, i32)] = &[(0, 0)];
const CIRCLE: &[(i32, i32)] = &[
    (3, 2),
    (2, 3),
    (-2, 3),
    (-3, 2),
    (-3, -2),
    (-2, -3),
    (2, -3),
    (3, -2),
];
const LINEAR: &[(i32, i32)] = &[(4, 0), (-4, 0)];

pub fn base_pattern(mode: Mode) -> &'static [(i32, i32)] {
    match mode {
        Mode::Normal => NORMAL,
        Mode::Zen => ZEN,
        Mode::Circle => CIRCLE,
        Mode::Linear => LINEAR,
    }
}

/// Returns the (dx, dy) for the given mode/step at the given distance multiplier.
pub fn step_delta(mode: Mode, step: usize, distance: i32) -> (i32, i32) {
    let p = base_pattern(mode);
    let (dx, dy) = p[step % p.len()];
    (dx * distance, dy * distance)
}

pub fn pattern_len(mode: Mode) -> usize {
    base_pattern(mode).len()
}

/// SendInput with MOUSEEVENTF_MOVE for a relative cursor delta.
pub fn jiggle(dx: i32, dy: i32) {
    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx,
                dy,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_MOVE,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let inputs = [input];
    unsafe {
        SendInput(&inputs, size_of::<INPUT>() as i32);
    }
}

pub fn stay_awake() {
    unsafe {
        SetThreadExecutionState(ES_CONTINUOUS | ES_DISPLAY_REQUIRED | ES_SYSTEM_REQUIRED);
    }
}

pub fn allow_sleep() {
    unsafe {
        SetThreadExecutionState(ES_CONTINUOUS);
    }
}

/// Smart-pause: detect user-initiated cursor movement between ticks.
#[derive(Default)]
pub struct PauseDetector {
    last: Option<(i32, i32)>,
}

impl PauseDetector {
    pub fn has_mouse_moved(&mut self) -> bool {
        let mut p = POINT::default();
        if unsafe { GetCursorPos(&mut p) }.is_err() {
            return false;
        }
        let now = (p.x, p.y);
        let moved = match self.last {
            Some(prev) => prev != now,
            None => false,
        };
        self.last = Some(now);
        moved
    }

    pub fn update_position(&mut self) {
        let mut p = POINT::default();
        if unsafe { GetCursorPos(&mut p) }.is_ok() {
            self.last = Some((p.x, p.y));
        }
    }
}
