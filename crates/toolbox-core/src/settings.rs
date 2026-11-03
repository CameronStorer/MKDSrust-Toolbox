use crate::{project::atomic_write, Result};
use serde::{Deserialize, Serialize};
use specta::Type;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize, Type)]
#[serde(default)]
pub struct Settings {
    pub rom_path: String,
    pub install_path: String,
    pub theme: String,
    pub navigation: String,
    pub camera_speed: f32,
    pub keybindings: BTreeMap<String, String>,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            rom_path: String::new(),
            install_path: detect_install()
                .map(|p| p.to_string_lossy().into())
                .unwrap_or_default(),
            theme: "dark".into(),
            navigation: "Blender".into(),
            camera_speed: 1.0,
            keybindings: BTreeMap::new(),
        }
    }
}
pub fn settings_path() -> Result<PathBuf> {
    directories::ProjectDirs::from("com", "mkdsrust", "Toolbox")
        .map(|p| p.config_dir().join("settings.json"))
        .ok_or_else(|| crate::Error::Invalid("user configuration folder unavailable".into()))
}
pub fn load() -> Settings {
    settings_path()
        .ok()
        .and_then(|p| std::fs::read(p).ok())
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
pub fn save(s: &Settings) -> Result<()> {
    let p = settings_path()?;
    if let Some(dir) = p.parent() {
        std::fs::create_dir_all(dir)?;
    }
    atomic_write(&p, &serde_json::to_vec_pretty(s)?)
}
pub fn is_install(path: &Path) -> bool {
    path.join("games").is_dir() && path.join("data").is_dir()
}
pub fn detect_install() -> Option<PathBuf> {
    let mut candidates = vec![];
    if let Some(p) = std::env::var_os("MKDSRUST_HOME") {
        candidates.push(PathBuf::from(p));
    }
    if let Ok(p) = std::env::current_exe() {
        for p in p.ancestors().skip(1).take(3) {
            candidates.push(p.to_path_buf());
            candidates.push(p.join("MKDSrust"));
        }
    }
    if let Some(p) = directories::UserDirs::new() {
        for folder in ["MKDSrust", "Games/MKDSrust", "Downloads/MKDSrust"] {
            candidates.push(p.home_dir().join(folder));
        }
    }
    candidates.into_iter().find(|p| is_install(p))
}
