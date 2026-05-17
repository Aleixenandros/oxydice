//! PLANTILLA de extensión in-tree de Oxydice.
//!
//! Copia este archivo a `crates/oxydice-core/src/ext/mi_extension.rs`,
//! renómbralo, declara `pub mod mi_extension;` en `ext/mod.rs` y registra la
//! extensión en `Registry::builtin()`. Guía completa: `docs/extensiones.md`.
//!
//! Este ejemplo implementa una `ThemeExtension` (el patrón más simple). Para
//! una clase nueva (visor de código, exportador…), añade una variante a
//! `ExtKind`, define su trait análogo a `ThemeExtension`/`SyncProvider` y su
//! `Vec` en `Registry`, y sigue exactamente esta forma.

use crate::ext::{ExtKind, Extension};
use crate::ext::theme::{ThemeEntry, ThemeExtension};
use crate::theme::Palette;

/// Tu extensión. Mantén el estado mínimo; el `Registry` la construye barata.
pub struct MiExtension;

impl Extension for MiExtension {
    fn id(&self) -> &'static str {
        // Id ESTABLE: se persiste en config.json. No lo cambies a la ligera.
        "mi-extension"
    }
    fn name(&self) -> &'static str {
        "Mi extensión"
    }
    fn kind(&self) -> ExtKind {
        ExtKind::Theme
    }
}

impl ThemeExtension for MiExtension {
    fn themes(&self) -> Vec<ThemeEntry> {
        vec![ThemeEntry {
            id: "mi-tema",
            name: "Mi tema",
            palette: Palette {
                dark: true,
                accent: [206, 65, 43],
                bg: [19, 19, 19],
                surface: [30, 30, 30],
                text: [224, 224, 224],
                muted: [160, 160, 160],
                border: [57, 57, 57],
            },
        }]
    }
}

// Registro (en `crates/oxydice-core/src/ext/mod.rs`, dentro de
// `Registry::builtin()`):
//
//     themes: vec![
//         Box::new(theme::CoreThemes),
//         Box::new(mi_extension::MiExtension), // <-- añade esto
//     ],

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expone_su_tema() {
        let e = MiExtension;
        assert_eq!(e.id(), "mi-extension");
        let ts = e.themes();
        assert_eq!(ts.len(), 1);
        assert_eq!(ts[0].id, "mi-tema");
    }
}
