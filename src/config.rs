//! Sauvegarde des réglages dans %APPDATA%\ScreenCalibration\config.json
//! et lancement automatique (application de la LUT) à l'ouverture de session.

use std::collections::HashMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use winreg::RegKey;
use winreg::enums::HKEY_CURRENT_USER;

use crate::gamma::GammaSettings;

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const RUN_VALUE: &str = "ScreenCalibration";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct HdrResults {
    pub peak_10: Option<f32>,
    pub peak_full: Option<f32>,
    pub min_black: Option<f32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct MonitorConfig {
    pub gamma: GammaSettings,
    pub apply_at_startup: bool,
    pub hdr: HdrResults,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Clé : nom GDI de l'écran (\\.\DISPLAYn).
    pub monitors: HashMap<String, MonitorConfig>,
}

fn path() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|d| PathBuf::from(d).join("ScreenCalibration").join("config.json"))
}

impl Config {
    pub fn load() -> Self {
        path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        let p = path().ok_or("APPDATA introuvable")?;
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&p, json).map_err(|e| e.to_string())
    }

    pub fn monitor(&mut self, gdi_name: &str) -> &mut MonitorConfig {
        self.monitors.entry(gdi_name.to_string()).or_default()
    }
}

pub fn autostart_enabled() -> bool {
    RegKey::predef(HKEY_CURRENT_USER)
        .open_subkey(RUN_KEY)
        .and_then(|k| k.get_value::<String, _>(RUN_VALUE))
        .is_ok()
}

pub fn set_autostart(enabled: bool) -> Result<(), String> {
    let (key, _) = RegKey::predef(HKEY_CURRENT_USER).create_subkey(RUN_KEY).map_err(|e| e.to_string())?;
    if enabled {
        let exe = std::env::current_exe().map_err(|e| e.to_string())?;
        key.set_value(RUN_VALUE, &format!("\"{}\" --apply", exe.display())).map_err(|e| e.to_string())
    } else {
        match key.delete_value(RUN_VALUE) {
            Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(e.to_string()),
            _ => Ok(()),
        }
    }
}
