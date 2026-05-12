// Mirrors resources/resource.h — keep in sync by hand.

// Resources
pub const IDI_APP: u16 = 1;

// Dialogs
pub const IDD_MAIN: u16 = 100;
pub const IDD_ABOUT: u16 = 101;

// Main dialog controls
pub const IDC_JIGGLING: i32 = 1001;
pub const IDC_SETTINGS: i32 = 1002;
pub const IDC_PANEL_SETTINGS: i32 = 1003;
pub const IDC_CMB_MODE: i32 = 1011;
pub const IDC_NUD_PERIOD: i32 = 1021;
pub const IDC_LBL_PERIOD_DISPLAY: i32 = 1023;
pub const IDC_NUD_DISTANCE: i32 = 1031;
pub const IDC_CB_RANDOM: i32 = 1040;
pub const IDC_CB_MINIMIZE: i32 = 1041;
pub const IDC_BTN_TRAYIFY: i32 = 1050;
pub const IDC_BTN_ABOUT: i32 = 1051;

// Tray menu items
pub const IDM_TRAY_OPEN: u32 = 3001;
pub const IDM_TRAY_START: u32 = 3002;
pub const IDM_TRAY_STOP: u32 = 3003;
pub const IDM_TRAY_EXIT: u32 = 3004;

// User-defined window messages
// (WM_APP is 0x8000; we offset to keep room for future use.)
pub const WM_APP_TRAY: u32 = 0x8000 + 1;

// Timer IDs
pub const TIMER_JIGGLE: usize = 1;

// Defaults / clamps
pub const PERIOD_MIN: u32 = 1;
pub const PERIOD_MAX: u32 = 10_800;
pub const DISTANCE_MIN: u32 = 1;
pub const DISTANCE_MAX: u32 = 120;
pub const PERIOD_DEFAULT: u32 = 60;
pub const DISTANCE_DEFAULT: u32 = 1;
