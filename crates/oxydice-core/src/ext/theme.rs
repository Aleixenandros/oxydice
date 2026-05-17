//! Extensión de temas: aporta un catálogo de paletas con nombre.
//!
//! Añadir un pack de temas nuevo = implementar [`ThemeExtension`] y
//! registrarlo en [`crate::ext::Registry::builtin`].

use crate::ext::{ExtKind, Extension};
use crate::theme::{self, Palette};
use serde::Serialize;

/// Un tema de paleta fija ofrecido por una extensión.
#[derive(Debug, Clone, Serialize)]
pub struct ThemeEntry {
    /// Id estable y persistible (coincide con el guardado en la config).
    pub id: &'static str,
    /// Nombre visible en el selector.
    pub name: &'static str,
    pub palette: Palette,
}

/// Extensión que publica uno o más temas.
pub trait ThemeExtension: Extension {
    fn themes(&self) -> Vec<ThemeEntry>;
}

/// Temas integrados de Oxydice (guía de estilo §2 + extras).
pub struct CoreThemes;

impl Extension for CoreThemes {
    fn id(&self) -> &'static str {
        "core-themes"
    }
    fn name(&self) -> &'static str {
        "Temas integrados"
    }
    fn kind(&self) -> ExtKind {
        ExtKind::Theme
    }
}

impl ThemeExtension for CoreThemes {
    fn themes(&self) -> Vec<ThemeEntry> {
        vec![
            ThemeEntry { id: "Light", name: "Claro", palette: theme::LIGHT },
            ThemeEntry { id: "Dark", name: "Oscuro", palette: theme::DARK },
            ThemeEntry { id: "Nord", name: "Nord", palette: theme::NORD },
            ThemeEntry {
                id: "Solarized",
                name: "Solarized",
                palette: theme::SOLARIZED,
            },
            ThemeEntry { id: "Dracula", name: "Dracula", palette: theme::DRACULA },
        ]
    }
}
