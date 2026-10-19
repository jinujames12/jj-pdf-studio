//! The settings file: theme choice and window geometry.
//!
//! Lives in `%APPDATA%\JJPDFStudio\settings.json`. If a file named `portable`
//! sits next to the exe, `settings.json` is kept beside the exe instead
//! (SPEC section 4). A missing or unreadable file just means defaults.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

/// Last known position and size of the window, in physical pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct WindowState {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub maximized: bool,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// "light" or "dark"; `None` follows Windows.
    pub theme: Option<String>,
    pub window: Option<WindowState>,
}

impl Settings {
    /// Reads the file; anything missing, damaged or unexpected gives defaults.
    pub fn load(path: &Path) -> Self {
        let mut settings: Self = fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default();
        if !matches!(settings.theme.as_deref(), Some("light" | "dark")) {
            settings.theme = None;
        }
        settings
    }

    /// Writes via a temp file and rename, so a crash never leaves half a file.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("json.tmp");
        fs::write(&tmp, serde_json::to_vec_pretty(self)?)?;
        fs::rename(&tmp, path)
    }
}

/// Where `settings.json` lives, given the exe's folder and `%APPDATA%`.
pub fn settings_path(exe_dir: &Path, appdata: Option<&Path>) -> PathBuf {
    match appdata {
        Some(appdata) if !exe_dir.join("portable").exists() => {
            appdata.join("JJPDFStudio").join("settings.json")
        }
        _ => exe_dir.join("settings.json"),
    }
}

/// Settings held in memory, with the file they are persisted to.
pub struct Store {
    path: PathBuf,
    settings: Mutex<Settings>,
}

impl Store {
    pub fn open() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_default();
        let appdata = std::env::var_os("APPDATA").map(PathBuf::from);
        let path = settings_path(&exe_dir, appdata.as_deref());
        let settings = Settings::load(&path);
        Self {
            path,
            settings: Mutex::new(settings),
        }
    }

    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// Applies `change` and persists. Saving is best effort: a read-only
    /// folder must not break the app, the choice just won't be remembered.
    pub fn update(&self, change: impl FnOnce(&mut Settings)) {
        let mut guard = self.settings.lock().unwrap();
        change(&mut guard);
        let _ = guard.save(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("jjpdf-settings-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn round_trips_and_creates_folders() {
        let dir = scratch("roundtrip");
        let path = dir.join("nested").join("settings.json");
        let settings = Settings {
            theme: Some("dark".into()),
            window: Some(WindowState {
                x: -10,
                y: 20,
                width: 1280,
                height: 800,
                maximized: true,
            }),
        };
        settings.save(&path).unwrap();
        assert_eq!(Settings::load(&path), settings);
        assert!(!path.with_extension("json.tmp").exists());
    }

    #[test]
    fn missing_damaged_or_unknown_values_fall_back_to_defaults() {
        let dir = scratch("defaults");
        let path = dir.join("settings.json");
        assert_eq!(Settings::load(&path), Settings::default());

        fs::write(&path, b"{ not json").unwrap();
        assert_eq!(Settings::load(&path), Settings::default());

        fs::write(&path, br#"{"theme":"purple","extra":1}"#).unwrap();
        assert_eq!(Settings::load(&path).theme, None);
    }

    #[test]
    fn portable_marker_keeps_settings_beside_the_exe() {
        let dir = scratch("portable");
        let appdata = dir.join("appdata");
        let normal = settings_path(&dir, Some(&appdata));
        assert_eq!(normal, appdata.join("JJPDFStudio").join("settings.json"));

        fs::write(dir.join("portable"), b"").unwrap();
        assert_eq!(
            settings_path(&dir, Some(&appdata)),
            dir.join("settings.json")
        );
        assert_eq!(settings_path(&dir, None), dir.join("settings.json"));
    }
}
