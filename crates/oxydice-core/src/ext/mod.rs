//! Sistema de extensiones *in-tree*.
//!
//! Las extensiones se compilan dentro del core (no hay carga dinámica de
//! `.so`/`.dll`): cada una implementa un *trait* y se registra en el
//! [`Registry`]. Los temas y la sincronización son extensiones; añadir un
//! backend nuevo es implementar el trait correspondiente y registrarlo aquí.

pub mod sync;
pub mod theme;

use crate::theme::Palette;
use serde::Serialize;

/// Clase de extensión, para listarlas en Ajustes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtKind {
    Theme,
    Sync,
}

impl ExtKind {
    pub fn label(self) -> &'static str {
        match self {
            ExtKind::Theme => "Tema",
            ExtKind::Sync => "Sincronización",
        }
    }
}

/// Metadatos comunes a toda extensión.
pub trait Extension {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn kind(&self) -> ExtKind;
}

/// Fila serializable de la pestaña de Extensiones (cruza el puente a la UI).
#[derive(Debug, Clone, Serialize)]
pub struct ExtRow {
    pub kind: String,
    pub id: String,
    pub name: String,
    pub detail: String,
    /// `false` si el usuario lo ha desactivado (T21).
    pub enabled: bool,
}

/// Registro de extensiones activas del core.
pub struct Registry {
    themes: Vec<Box<dyn theme::ThemeExtension>>,
    syncs: Vec<Box<dyn sync::SyncProvider>>,
}

impl Registry {
    /// Registra las extensiones integradas.
    pub fn builtin() -> Self {
        Self {
            themes: vec![Box::new(theme::CoreThemes)],
            syncs: vec![Box::new(sync::NoSync)],
        }
    }

    /// Catálogo plano de temas de paleta fija aportados por las extensiones.
    pub fn theme_entries(&self) -> Vec<theme::ThemeEntry> {
        self.themes.iter().flat_map(|e| e.themes()).collect()
    }

    /// Resuelve un id de tema a su paleta. Maneja los ids especiales
    /// `System` (sigue al SO, según `system_dark`) y `Custom` (paleta
    /// editable del usuario).
    pub fn resolve_theme(&self, id: &str, system_dark: bool, custom: &Palette) -> Palette {
        if id == crate::theme::CUSTOM_ID {
            return *custom;
        }
        if id == crate::theme::SYSTEM_ID {
            return crate::theme::system_palette(system_dark);
        }
        self.theme_entries()
            .into_iter()
            .find(|t| t.id == id)
            .map(|t| t.palette)
            .unwrap_or_else(|| crate::theme::system_palette(system_dark))
    }

    /// Nombre legible de un id de tema (para el selector).
    pub fn theme_name(&self, id: &str) -> String {
        if id == crate::theme::SYSTEM_ID {
            return "Sistema".to_owned();
        }
        if id == crate::theme::CUSTOM_ID {
            return "Personalizado".to_owned();
        }
        self.theme_entries()
            .into_iter()
            .find(|t| t.id == id)
            .map(|t| t.name.to_owned())
            .unwrap_or_else(|| id.to_owned())
    }

    pub fn syncs(&self) -> &[Box<dyn sync::SyncProvider>] {
        &self.syncs
    }

    /// Proveedor de sync activo según su id (cae al primero si no existe).
    pub fn sync_by_id(&self, id: &str) -> &dyn sync::SyncProvider {
        self.syncs
            .iter()
            .find(|s| s.id() == id)
            .unwrap_or(&self.syncs[0])
            .as_ref()
    }

    /// Lanza la sincronización del proveedor indicado (stub: no-op).
    pub fn sync_now(&mut self, id: &str) -> Result<(), String> {
        match self.syncs.iter_mut().find(|s| s.id() == id) {
            Some(p) => p.sync(),
            None => Ok(()),
        }
    }

    /// Filas para la pestaña de Extensiones de Ajustes. `disabled` son los
    /// ids que el usuario ha desactivado (T21); marca `enabled` en cada fila.
    pub fn listing(&self, disabled: &[String]) -> Vec<ExtRow> {
        let on = |id: &str| !disabled.iter().any(|d| d == id);
        let mut rows = Vec::new();
        for t in &self.themes {
            let n = t.themes().len();
            rows.push(ExtRow {
                kind: t.kind().label().to_owned(),
                id: t.id().to_owned(),
                name: t.name().to_owned(),
                detail: format!("{n} temas"),
                enabled: on(t.id()),
            });
        }
        for s in &self.syncs {
            rows.push(ExtRow {
                kind: s.kind().label().to_owned(),
                id: s.id().to_owned(),
                name: s.name().to_owned(),
                detail: s.state().label().to_owned(),
                enabled: on(s.id()),
            });
        }
        // Módulos de capacidad integrados (T17/T18): se listan junto al
        // resto para que la pestaña Extensiones refleje todo lo disponible.
        rows.push(ExtRow {
            kind: "Visor".to_owned(),
            id: "code-viewer".to_owned(),
            name: "Visor de código".to_owned(),
            detail: format!("{} extensiones", crate::viewer::CODE_EXTS.len()),
            enabled: on("code-viewer"),
        });
        rows.push(ExtRow {
            kind: "Exportar".to_owned(),
            id: "exporter".to_owned(),
            name: "Exportador".to_owned(),
            detail: "HTML · PDF".to_owned(),
            enabled: on("exporter"),
        });
        rows
    }
}
