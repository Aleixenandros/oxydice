//! Extensión de sincronización: interfaz y stub no-op.
//!
//! La sincronización real (Git, S3, Google Drive) es backlog. Aquí queda
//! definido el contrato para que añadir un backend sea implementar
//! [`SyncProvider`] y registrarlo en [`crate::ext::Registry::builtin`].

use crate::ext::{ExtKind, Extension};
use serde::{Deserialize, Serialize};

/// Estado de la nube (guía de estilo §6).
///
/// `Synced`/`Syncing`/`Error` aún no los construye ningún proveedor (el
/// stub sólo reporta `Disabled`); forman parte del contrato para los
/// backends reales del backlog, por eso se permite el código no usado.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SyncState {
    /// Sin proveedor de sincronización activo.
    Disabled,
    /// Configurado y al día.
    Synced,
    /// Sincronizando ahora.
    Syncing,
    /// Falló; el mensaje describe la causa.
    Error(String),
}

impl SyncState {
    pub fn label(&self) -> &str {
        match self {
            SyncState::Disabled => "Sin sincronización",
            SyncState::Synced => "Sincronizado",
            SyncState::Syncing => "Sincronizando…",
            SyncState::Error(_) => "Error de sincronización",
        }
    }

    /// Glifo para el indicador de la barra superior.
    pub fn glyph(&self) -> &'static str {
        match self {
            SyncState::Disabled => "○",
            SyncState::Synced => "✔",
            SyncState::Syncing => "↻",
            SyncState::Error(_) => "!",
        }
    }
}

/// Contrato de un backend de sincronización.
pub trait SyncProvider: Extension {
    fn state(&self) -> SyncState;
    /// Lanza una sincronización. El stub no hace nada.
    fn sync(&mut self) -> Result<(), String>;
}

/// Proveedor por defecto: no sincroniza nada.
pub struct NoSync;

impl Extension for NoSync {
    fn id(&self) -> &'static str {
        "none"
    }
    fn name(&self) -> &'static str {
        "Sin sincronización"
    }
    fn kind(&self) -> ExtKind {
        ExtKind::Sync
    }
}

impl SyncProvider for NoSync {
    fn state(&self) -> SyncState {
        SyncState::Disabled
    }
    fn sync(&mut self) -> Result<(), String> {
        Ok(())
    }
}
