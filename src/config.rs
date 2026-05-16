//! Configuración persistente: espacios, espacio activo y tema.

use crate::theme::ThemeChoice;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Carpetas raíz registradas como espacios.
    pub spaces: Vec<PathBuf>,
    /// Índice del espacio activo dentro de `spaces`.
    pub selected: Option<usize>,
    pub theme: ThemeChoice,
    /// Escala de la interfaz (1.0 = normal).
    pub ui_scale: f32,
    /// Carpeta destino de las copias de seguridad.
    pub backup_dir: Option<PathBuf>,
    /// Hacer una copia del espacio activo tras cada guardado.
    pub backup_on_save: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spaces: Vec::new(),
            selected: None,
            theme: ThemeChoice::default(),
            ui_scale: 1.0,
            backup_dir: None,
            backup_on_save: false,
        }
    }
}

impl Config {
    fn file() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "Aleixenandros", "RustNotes")
            .map(|d| d.config_dir().join("config.json"))
    }

    pub fn load() -> Self {
        let Some(path) = Self::file() else {
            return Self::default();
        };
        std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self) {
        let Some(path) = Self::file() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write(path, json);
        }
    }

    pub fn selected_space(&self) -> Option<&PathBuf> {
        self.selected.and_then(|i| self.spaces.get(i))
    }
}
