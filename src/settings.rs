//! Settings persistence via a portable JSON config file.
//!
//! Seven values are loaded on startup and written through on every change.

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ids::{
    DISTANCE_DEFAULT, DISTANCE_MAX, DISTANCE_MIN, PERIOD_DEFAULT, PERIOD_MAX, PERIOD_MIN,
};
use crate::jiggle::Mode;

const CONFIG_FILE_NAME: &str = "mouse-jiggler-neo.json";
const FALLBACK_DIR_VENDOR: &str = "Zutfen-LLC";
const FALLBACK_DIR_APP: &str = "MouseJiggler";

pub const AUTO_STOP_DEFAULT_MINUTES_LOCAL: u16 = 17 * 60;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Settings {
    pub minimize_on_startup: bool,
    pub random_timer: bool,
    pub mode: Mode,
    pub period_secs: u32,
    pub distance: u32,
    pub auto_stop_enabled: bool,
    pub auto_stop_minutes_local: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            minimize_on_startup: false,
            random_timer: false,
            mode: Mode::Normal,
            period_secs: PERIOD_DEFAULT,
            distance: DISTANCE_DEFAULT,
            auto_stop_enabled: false,
            auto_stop_minutes_local: AUTO_STOP_DEFAULT_MINUTES_LOCAL,
        }
    }
}

fn clamp_auto_stop_minutes(value: u32) -> u16 {
    if value < 24 * 60 {
        value as u16
    } else {
        AUTO_STOP_DEFAULT_MINUTES_LOCAL
    }
}

fn sanitize(mut settings: Settings) -> Settings {
    settings.period_secs = settings.period_secs.clamp(PERIOD_MIN, PERIOD_MAX);
    settings.distance = settings.distance.clamp(DISTANCE_MIN, DISTANCE_MAX);
    settings.auto_stop_minutes_local =
        clamp_auto_stop_minutes(settings.auto_stop_minutes_local as u32);
    settings
}

fn primary_config_path() -> Option<PathBuf> {
    let exe = env::current_exe().ok()?;
    Some(exe.parent()?.join(CONFIG_FILE_NAME))
}

fn fallback_config_path() -> Option<PathBuf> {
    let local_app_data = env::var_os("LOCALAPPDATA")?;
    Some(
        PathBuf::from(local_app_data)
            .join(FALLBACK_DIR_VENDOR)
            .join(FALLBACK_DIR_APP)
            .join(CONFIG_FILE_NAME),
    )
}

fn read_settings_file(path: &Path) -> Option<Settings> {
    let bytes = fs::read(path).ok()?;
    let settings: Settings = serde_json::from_slice(&bytes).ok()?;
    Some(sanitize(settings))
}

fn load_from_paths(primary: Option<&Path>, fallback: Option<&Path>) -> Settings {
    if let Some(path) = primary
        && path.exists()
    {
        return read_settings_file(path).unwrap_or_default();
    }

    if let Some(path) = fallback
        && path.exists()
    {
        return read_settings_file(path).unwrap_or_default();
    }

    Settings::default()
}

fn write_settings_file(path: &Path, settings: &Settings) -> std::io::Result<()> {
    let json = serde_json::to_vec_pretty(settings)
        .map_err(|err| std::io::Error::other(err.to_string()))?;
    fs::write(path, json)
}

fn save_to_paths(settings: &Settings, primary: Option<&Path>, fallback: Option<&Path>) {
    let settings = sanitize(settings.clone());

    if let Some(path) = primary
        && write_settings_file(path, &settings).is_ok()
    {
        return;
    }

    if let Some(path) = fallback {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = write_settings_file(path, &settings);
    }
}

pub fn load() -> Settings {
    let primary = primary_config_path();
    let fallback = fallback_config_path();
    load_from_paths(primary.as_deref(), fallback.as_deref())
}

pub fn save(settings: &Settings) {
    let primary = primary_config_path();
    let fallback = fallback_config_path();
    save_to_paths(settings, primary.as_deref(), fallback.as_deref());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn sample_settings() -> Settings {
        Settings {
            minimize_on_startup: true,
            random_timer: true,
            mode: Mode::Circle,
            period_secs: 42,
            distance: 7,
            auto_stop_enabled: true,
            auto_stop_minutes_local: 23 * 60 + 15,
        }
    }

    fn unique_test_dir(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time before epoch")
            .as_nanos();
        let dir = env::temp_dir().join(format!("mouse-jiggler-neo-{name}-{stamp}"));
        fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }

    #[test]
    fn json_round_trip_preserves_all_fields() {
        let settings = sample_settings();
        let json = serde_json::to_string(&settings).expect("serialize settings");
        let parsed: Settings = serde_json::from_str(&json).expect("deserialize settings");
        assert_eq!(parsed, settings);
    }

    #[test]
    fn load_clamps_out_of_range_values() {
        let dir = unique_test_dir("clamp");
        let path = dir.join(CONFIG_FILE_NAME);
        fs::write(
            &path,
            r#"{
  "minimize_on_startup": true,
  "random_timer": false,
  "mode": "Linear",
  "period_secs": 999999,
  "distance": 0,
  "auto_stop_enabled": true,
  "auto_stop_minutes_local": 9999
}"#,
        )
        .expect("write config");

        let loaded = load_from_paths(Some(&path), None);

        assert_eq!(loaded.period_secs, PERIOD_MAX);
        assert_eq!(loaded.distance, DISTANCE_MIN);
        assert_eq!(
            loaded.auto_stop_minutes_local,
            AUTO_STOP_DEFAULT_MINUTES_LOCAL
        );
    }

    #[test]
    fn primary_path_takes_precedence_over_fallback() {
        let dir = unique_test_dir("primary-wins");
        let primary = dir.join("primary.json");
        let fallback = dir.join("fallback.json");

        let mut primary_settings = sample_settings();
        primary_settings.mode = Mode::Normal;
        let mut fallback_settings = sample_settings();
        fallback_settings.mode = Mode::Zen;

        fs::write(
            &primary,
            serde_json::to_vec(&primary_settings).expect("serialize primary"),
        )
        .expect("write primary");
        fs::write(
            &fallback,
            serde_json::to_vec(&fallback_settings).expect("serialize fallback"),
        )
        .expect("write fallback");

        let loaded = load_from_paths(Some(&primary), Some(&fallback));
        assert_eq!(loaded, primary_settings);
    }

    #[test]
    fn fallback_is_used_when_primary_is_absent() {
        let dir = unique_test_dir("fallback");
        let fallback = dir.join("fallback.json");
        let settings = sample_settings();

        fs::write(
            &fallback,
            serde_json::to_vec(&settings).expect("serialize fallback"),
        )
        .expect("write fallback");

        let loaded = load_from_paths(None, Some(&fallback));
        assert_eq!(loaded, settings);
    }

    #[test]
    fn defaults_are_used_when_no_config_exists() {
        let dir = unique_test_dir("defaults");
        let primary = dir.join("missing-primary.json");
        let fallback = dir.join("missing-fallback.json");

        let loaded = load_from_paths(Some(&primary), Some(&fallback));
        assert_eq!(loaded, Settings::default());
    }

    #[test]
    fn save_prefers_primary_when_it_is_writable() {
        let dir = unique_test_dir("save-primary");
        let primary = dir.join("primary.json");
        let fallback = dir.join("fallback.json");
        let settings = sample_settings();

        save_to_paths(&settings, Some(&primary), Some(&fallback));

        assert!(primary.exists());
        assert!(!fallback.exists());
        let loaded = load_from_paths(Some(&primary), Some(&fallback));
        assert_eq!(loaded, settings);
    }

    #[test]
    fn save_falls_back_when_primary_write_fails() {
        let dir = unique_test_dir("save-fallback");
        let primary_dir = dir.join("primary-dir");
        let fallback = dir.join("fallback").join(CONFIG_FILE_NAME);
        let settings = sample_settings();

        fs::create_dir_all(&primary_dir).expect("create unwritable primary target");
        save_to_paths(&settings, Some(&primary_dir), Some(&fallback));

        assert!(fallback.exists());
        let loaded = load_from_paths(None, Some(&fallback));
        assert_eq!(loaded, settings);
    }
}
