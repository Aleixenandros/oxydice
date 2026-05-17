//! Reconciliación a **3 bandas** (base / local / remoto), pura y testeable.
//!
//! Sin esta comparación a tres bandas no se puede distinguir «lo cambié yo»
//! de «lo cambiaron ellos» de «ambos»: es donde se pierden datos. Por eso
//! vive aislada del transporte y de la E/S, y se prueba exhaustivamente. Las
//! decisiones que mueven datos nunca descartan una versión sin preservarla.

use serde::{Deserialize, Serialize};

/// Línea base sincronizada de un archivo: su hash cuando se sincronizó por
/// última vez y la versión remota (ETag/rev) que se vio entonces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Baseline {
    pub hash: String,
    pub remote_version: String,
}

/// Estado observado de un archivo en esta pasada de sincronización.
pub struct FileState<'a> {
    /// Hash del contenido local, o `None` si no existe en disco.
    pub local_hash: Option<&'a str>,
    /// Versión remota (ETag/rev), o `None` si no existe en el remoto.
    pub remote_version: Option<&'a str>,
    /// Línea base del índice, o `None` si nunca se sincronizó.
    pub base: Option<&'a Baseline>,
}

/// Decisión de la reconciliación.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// Nada que hacer.
    Noop,
    /// Subir el local al remoto y registrar la base.
    Upload,
    /// Descargar el remoto al local y registrar la base.
    Download,
    /// Borrado remoto consolidado: eliminar el local (sin cambios locales).
    DeleteLocal,
    /// Borrado local consolidado: eliminar el remoto (sin cambios remotos).
    DeleteRemote,
    /// Ambos desaparecieron: olvidar la entrada del índice.
    DropIndex,
    /// Ambos lados presentes y divergentes respecto de la base (o sin base):
    /// hay que comparar contenidos para decidir convergencia o conflicto.
    CompareContent,
}

fn local_unchanged(s: &FileState, b: &Baseline) -> bool {
    s.local_hash == Some(b.hash.as_str())
}

fn remote_unchanged(s: &FileState, b: &Baseline) -> bool {
    s.remote_version == Some(b.remote_version.as_str())
}

/// Decide la acción a partir del estado a 3 bandas. Reglas clave:
/// - Sin línea base: presencia nueva ⇒ subir/descargar; ambos ⇒ comparar.
/// - Con base: si solo cambió un lado, propágalo; si cambiaron ambos, hay
///   que comparar contenidos (puede ser convergencia o conflicto real).
/// - Borrados: solo se propagan si el otro lado **no** cambió; si cambió,
///   se «resucita» (descarga/sube) para no perder la edición.
pub fn reconcile(s: &FileState) -> Action {
    match (s.local_hash, s.remote_version) {
        (None, None) => Action::DropIndex,

        (Some(_), None) => match s.base {
            None => Action::Upload, // nuevo local
            Some(b) if local_unchanged(s, b) => Action::DeleteLocal, // borrado remoto consolidado
            Some(_) => Action::Upload, // local editado y remoto borrado: re-crear (sin pérdida)
        },

        (None, Some(_)) => match s.base {
            None => Action::Download, // nuevo remoto
            Some(b) if remote_unchanged(s, b) => Action::DeleteRemote, // borrado local consolidado
            Some(_) => Action::Download, // remoto editado y local borrado: resucitar (sin pérdida)
        },

        (Some(_), Some(_)) => match s.base {
            None => Action::CompareContent, // ambos nuevos
            Some(b) => match (!local_unchanged(s, b), !remote_unchanged(s, b)) {
                (false, false) => Action::Noop,
                (true, false) => Action::Upload,
                (false, true) => Action::Download,
                (true, true) => Action::CompareContent, // posible conflicto
            },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(h: &str, v: &str) -> Baseline {
        Baseline { hash: h.to_owned(), remote_version: v.to_owned() }
    }

    fn act(l: Option<&str>, r: Option<&str>, b: Option<&Baseline>) -> Action {
        reconcile(&FileState { local_hash: l, remote_version: r, base: b })
    }

    #[test]
    fn no_base_presence_decides() {
        assert_eq!(act(Some("h"), None, None), Action::Upload);
        assert_eq!(act(None, Some("v"), None), Action::Download);
        assert_eq!(act(Some("h"), Some("v"), None), Action::CompareContent);
        assert_eq!(act(None, None, None), Action::DropIndex);
    }

    #[test]
    fn synced_and_untouched_is_noop() {
        let b = base("h", "v");
        assert_eq!(act(Some("h"), Some("v"), Some(&b)), Action::Noop);
    }

    #[test]
    fn one_sided_change_propagates() {
        let b = base("h", "v");
        assert_eq!(act(Some("h2"), Some("v"), Some(&b)), Action::Upload);
        assert_eq!(act(Some("h"), Some("v2"), Some(&b)), Action::Download);
    }

    #[test]
    fn both_changed_needs_content_compare() {
        let b = base("h", "v");
        assert_eq!(act(Some("h2"), Some("v2"), Some(&b)), Action::CompareContent);
    }

    #[test]
    fn deletions_propagate_only_when_other_side_unchanged() {
        let b = base("h", "v");
        // Borrado local, remoto intacto ⇒ borrar remoto.
        assert_eq!(act(None, Some("v"), Some(&b)), Action::DeleteRemote);
        // Borrado remoto, local intacto ⇒ borrar local.
        assert_eq!(act(Some("h"), None, Some(&b)), Action::DeleteLocal);
        // Ambos desaparecieron ⇒ olvidar índice.
        assert_eq!(act(None, None, Some(&b)), Action::DropIndex);
    }

    #[test]
    fn deletion_against_edit_resurrects_no_loss() {
        let b = base("h", "v");
        // Borré local pero el remoto fue editado ⇒ descargar (no perder).
        assert_eq!(act(None, Some("v2"), Some(&b)), Action::Download);
        // Borraron remoto pero edité local ⇒ subir (no perder).
        assert_eq!(act(Some("h2"), None, Some(&b)), Action::Upload);
    }
}
