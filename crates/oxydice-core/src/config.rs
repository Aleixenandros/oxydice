//! Configuración persistente: espacios, espacio activo y tema.

use crate::theme::{Palette, SYSTEM_ID};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Familia tipográfica del editor (guía de estilo §3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EditorFont {
    /// JetBrains Mono — monoespaciada, por defecto para Markdown.
    Mono,
    /// Inter — proporcional, lectura más cómoda en prosa.
    Sans,
}

impl EditorFont {
    pub const ALL: [EditorFont; 2] = [EditorFont::Mono, EditorFont::Sans];

    pub fn label(self) -> &'static str {
        match self {
            EditorFont::Mono => "JetBrains Mono",
            EditorFont::Sans => "Inter",
        }
    }
}

/// Ajustes del remoto de sincronización **sin secretos**: la contraseña
/// (WebDAV) y la *secret key* (S3) viven en el llavero del SO, no en el JSON.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct RemoteSettings {
    /// `"none"` (desactivado), `"webdav"` o `"s3"`.
    pub kind: String,
    pub endpoint: String,
    /// Subruta/prefijo remoto donde vive el espacio.
    pub root: String,
    /// Usuario de WebDAV.
    pub username: String,
    /// Bucket de S3.
    pub bucket: String,
    /// Región de S3.
    pub region: String,
    /// *Access key id* de S3 (el secreto va al llavero).
    pub access_key: String,
    /// Auto-sincronizar periódicamente y al recuperar el foco.
    pub auto: bool,
    /// Intervalo de auto-sync en segundos (la capa shell aplica un mínimo).
    pub interval_secs: u64,
}

impl Default for RemoteSettings {
    fn default() -> Self {
        Self {
            kind: "none".to_owned(),
            endpoint: String::new(),
            root: String::new(),
            username: String::new(),
            bucket: String::new(),
            region: String::new(),
            access_key: String::new(),
            auto: false,
            interval_secs: 300,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Carpetas raíz registradas como espacios.
    pub spaces: Vec<PathBuf>,
    /// Índice del espacio activo dentro de `spaces`.
    pub selected: Option<usize>,
    /// Id del tema activo (extensión de temas, o `System`/`Custom`).
    pub theme: String,
    /// Paleta del tema «Personalizado», editable y exportable.
    pub custom_theme: Palette,
    /// Id de la extensión de sincronización activa (catálogo de `ext`).
    pub sync: String,
    /// Configuración del remoto de sync real (OpenDAL); secretos en llavero.
    pub remote: RemoteSettings,
    /// Familia tipográfica del editor.
    pub editor_font: EditorFont,
    /// Tamaño de fuente del editor en puntos.
    pub editor_font_size: f32,
    /// Familia tipográfica del sistema para el editor (T25). Vacío = usar la
    /// fuente empotrada según `editor_font` (Mono/Sans).
    pub editor_font_family: String,
    /// Escala de la interfaz (1.0 = normal).
    pub ui_scale: f32,
    /// Idioma de la UI: `""` = seguir al sistema; o `es`/`en`/`de`/`pt`.
    pub lang: String,
    /// Carpeta destino de las copias de seguridad.
    pub backup_dir: Option<PathBuf>,
    /// Hacer una copia del espacio activo tras cada guardado.
    pub backup_on_save: bool,
    /// Ids de módulos/extensiones **desactivados** (T21). Un id ausente =
    /// habilitado; presente = el módulo no se ofrece ni actúa.
    pub disabled_ext: Vec<String>,
    /// Rutas de las notas con pestaña abierta, en orden. Se restauran al
    /// arrancar; las que ya no existan en disco se ignoran sin error.
    pub open_tabs: Vec<PathBuf>,
    /// Índice de la pestaña activa dentro de `open_tabs`.
    pub active_tab: usize,
}

impl Config {
    /// `true` si el módulo/extensión con ese id está habilitado.
    pub fn ext_enabled(&self, id: &str) -> bool {
        !self.disabled_ext.iter().any(|d| d == id)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            spaces: Vec::new(),
            selected: None,
            theme: SYSTEM_ID.to_owned(),
            custom_theme: Palette::default(),
            sync: "none".to_owned(),
            remote: RemoteSettings::default(),
            editor_font: EditorFont::Mono,
            editor_font_size: 14.0,
            editor_font_family: String::new(),
            ui_scale: 1.0,
            lang: String::new(),
            backup_dir: None,
            backup_on_save: false,
            disabled_ext: Vec::new(),
            open_tabs: Vec::new(),
            active_tab: 0,
        }
    }
}

impl Config {
    fn file() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "Aleixenandros", "Oxydice")
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
