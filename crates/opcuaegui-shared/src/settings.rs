use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::theme::ThemeMode;

#[derive(Serialize, Deserialize)]
pub struct WindowSettings {
    pub width: f32,
    pub height: f32,
    /// Persisted theme preference. `serde(default)` keeps existing JSON files
    /// (which lack this field) deserialisable.
    #[serde(default)]
    pub theme: ThemeMode,
}

pub fn settings_path(name: &str) -> Option<PathBuf> {
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    let dir = PathBuf::from(home).join(".opcuasim");
    std::fs::create_dir_all(&dir).ok()?;
    Some(dir.join(format!("{name}-window.json")))
}

pub fn load(name: &str) -> Option<WindowSettings> {
    let path = settings_path(name)?;
    let json = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&json).ok()
}

pub fn save(name: &str, s: &WindowSettings) {
    let Some(path) = settings_path(name) else {
        return;
    };
    if let Ok(json) = serde_json::to_string(s) {
        let _ = std::fs::write(path, json);
    }
}
