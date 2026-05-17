//! Índice de sincronización: archivo *sidecar* `.oxydice/sync.json`.
//!
//! Guarda, por ruta relativa, la línea base (hash sincronizado + versión
//! remota). Vive bajo `.oxydice/` (oculto): `vault`, búsqueda y copias ya
//! omiten lo que empieza por `.`, así que no aparece en el árbol ni en los
//! backups. Se escribe de forma atómica para no corromperlo a medio sync.

use super::reconcile::Baseline;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const DIR: &str = ".oxydice";
const FILE: &str = "sync.json";

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct SyncIndex {
    /// Ruta relativa (`/`) → línea base sincronizada.
    #[serde(default)]
    pub files: BTreeMap<String, Baseline>,
}

impl SyncIndex {
    pub fn path(space: &Path) -> PathBuf {
        space.join(DIR).join(FILE)
    }

    /// Carga el índice; tolera ausencia o corrupción (cae a vacío) para que
    /// un sidecar dañado no impida sincronizar (se reconstruye la base).
    pub fn load(space: &Path) -> Self {
        std::fs::read_to_string(Self::path(space))
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    /// Guarda el índice de forma atómica (temp + rename).
    pub fn save(&self, space: &Path) -> Result<(), String> {
        let path = Self::path(space);
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("Error al crear .oxydice: {e}"))?;
        }
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Error al serializar el índice: {e}"))?;
        crate::vault::write_atomic(&path, json.as_bytes())
    }
}
