//! *Vault*: operaciones sobre el sistema de archivos de un espacio.
//!
//! El disco es la fuente de verdad: no hay índice ni caché. Estas funciones
//! son la lógica que antes vivía mezclada con la UI egui, ahora puras y con
//! `Result<_, String>` para que la capa Tauri las exponga como comandos. El
//! comportamiento (orden del árbol, sufijo `.md`, copia que omite ocultos,
//! anti-colisión al renombrar) se mantiene idéntico.

use serde::Serialize;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

/// Una entrada del árbol del explorador.
#[derive(Debug, Clone, Serialize)]
pub struct Entry {
    /// Ruta absoluta.
    pub path: PathBuf,
    /// Nombre a mostrar: el de la carpeta, o el de la nota sin `.md`.
    pub name: String,
    pub is_dir: bool,
}

/// Contenido de una nota más su fecha de modificación (epoch en segundos).
#[derive(Debug, Clone, Serialize)]
pub struct NoteData {
    pub content: String,
    pub mtime_secs: Option<u64>,
}

fn name_of(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn stem_of(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn mtime_secs(p: &Path) -> Option<u64> {
    std::fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Carpetas y notas de `dir`: carpetas primero y orden alfabético, omitiendo
/// ocultos. Con `include_code` se listan además los archivos de código que
/// sabe mostrar el visor (T17); si el visor está desactivado (T21) se pasa
/// `false` y solo se listan `.md`.
pub fn entries_filtered(dir: &Path, include_code: bool) -> Vec<Entry> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !name.starts_with('.')
        })
        .filter(|p| {
            if p.is_dir() {
                return true;
            }
            if include_code {
                crate::viewer::is_viewable(p)
            } else {
                p.extension().is_some_and(|x| x == "md")
            }
        })
        .collect();
    v.sort_by(|a, b| (!a.is_dir(), a.file_name()).cmp(&(!b.is_dir(), b.file_name())));
    v.into_iter()
        .map(|p| {
            let is_dir = p.is_dir();
            let name = if is_dir { name_of(&p) } else { stem_of(&p) };
            Entry { path: p, name, is_dir }
        })
        .collect()
}

/// Carpetas, notas `.md` y código del visor de `dir` (atajo de
/// [`entries_filtered`] con el visor habilitado).
pub fn entries(dir: &Path) -> Vec<Entry> {
    entries_filtered(dir, true)
}

pub fn read_note(path: &Path) -> Result<NoteData, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("Error al leer: {e}"))?;
    Ok(NoteData {
        content,
        mtime_secs: mtime_secs(path),
    })
}

/// Escribe la nota y devuelve su nueva fecha de modificación (epoch s).
pub fn write_note(path: &Path, content: &str) -> Result<Option<u64>, String> {
    std::fs::write(path, content).map_err(|e| format!("Error al guardar: {e}"))?;
    Ok(mtime_secs(path))
}

/// Escribe `bytes` en `path` de forma **atómica**: archivo temporal oculto en
/// el mismo directorio + `rename` (atómico dentro del mismo sistema de
/// archivos). Así una nota nunca queda a medio escribir si el proceso muere o
/// el sync se interrumpe. Lo usa el motor de sincronización para descargas e
/// índice; el temporal empieza por `.` para que el árbol/búsqueda lo omitan.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let parent = path.parent().ok_or("Ruta sin directorio padre")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("Error al crear carpeta: {e}"))?;
    let leaf = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("oxydice");
    let tmp = parent.join(format!(".{leaf}.tmp"));
    std::fs::write(&tmp, bytes).map_err(|e| format!("Error al escribir: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("Error al renombrar: {e}")
    })
}

/// Crea una nota en `parent` con un `# título` inicial. Añade `.md` si falta.
pub fn create_note(parent: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("El nombre no puede estar vacío".to_owned());
    }
    let file = if name.ends_with(".md") {
        name.to_owned()
    } else {
        format!("{name}.md")
    };
    let path = parent.join(&file);
    let title = name.strip_suffix(".md").unwrap_or(name);
    std::fs::write(&path, format!("# {title}\n")).map_err(|e| format!("Error: {e}"))?;
    Ok(path)
}

pub fn create_folder(parent: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("El nombre no puede estar vacío".to_owned());
    }
    let path = parent.join(name);
    std::fs::create_dir_all(&path).map_err(|e| format!("Error: {e}"))?;
    Ok(path)
}

/// Renombra `old` a un nuevo nombre dentro de su misma carpeta. Conserva la
/// extensión `.md` de las notas y evita colisiones (no sobrescribe).
pub fn rename_path(old: &Path, name: &str) -> Result<PathBuf, String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("El nombre no puede estar vacío".to_owned());
    }
    let parent = old.parent().map(Path::to_path_buf).unwrap_or_default();
    let leaf = if old.is_file() && !name.ends_with(".md") {
        format!("{name}.md")
    } else {
        name.to_owned()
    };
    let new_path = parent.join(&leaf);
    if new_path == *old {
        return Ok(new_path);
    }
    if new_path.exists() {
        return Err(format!("Ya existe «{leaf}»"));
    }
    std::fs::rename(old, &new_path).map_err(|e| format!("Error al renombrar: {e}"))?;
    Ok(new_path)
}

pub fn delete_path(path: &Path) -> Result<(), String> {
    let res = if path.is_dir() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    };
    res.map_err(|e| format!("Error al eliminar: {e}"))
}

/// Copia el espacio a `dest/oxydice-<nombre>-<epoch>/`, omitiendo ocultos.
pub fn backup(space: &Path, dest: &Path) -> Result<PathBuf, String> {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let target = dest.join(format!("oxydice-{}-{secs}", name_of(space)));
    copy_dir(space, &target).map_err(|e| format!("Error en la copia: {e}"))?;
    Ok(target)
}

/// Copia recursivamente `src` en `dst`, omitiendo archivos ocultos.
fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if from.is_dir() {
            copy_dir(&from, &to)?;
        } else {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp() -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "rn-vault-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn create_read_rename_delete_roundtrip() {
        let root = tmp();
        let note = create_note(&root, "Mi nota").unwrap();
        assert_eq!(note.extension().unwrap(), "md");
        assert!(read_note(&note).unwrap().content.starts_with("# Mi nota"));

        let renamed = rename_path(&note, "Otra").unwrap();
        assert_eq!(renamed.file_name().unwrap(), "Otra.md");
        assert!(!note.exists() && renamed.exists());

        // No sobrescribe una existente.
        let dup = create_note(&root, "Dup").unwrap();
        assert!(rename_path(&dup, "Otra").is_err());

        delete_path(&renamed).unwrap();
        assert!(!renamed.exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn entries_sorts_folders_first_then_alpha() {
        let root = tmp();
        create_note(&root, "b").unwrap();
        create_note(&root, "a").unwrap();
        create_folder(&root, "zeta").unwrap();
        std::fs::write(root.join(".oculto.md"), "x").unwrap();
        let e = entries(&root);
        let names: Vec<_> = e.iter().map(|x| (x.is_dir, x.name.as_str())).collect();
        assert_eq!(names, vec![(true, "zeta"), (false, "a"), (false, "b")]);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn backup_skips_hidden_and_is_timestamped() {
        let root = tmp();
        create_note(&root, "nota").unwrap();
        std::fs::write(root.join(".secreto"), "x").unwrap();
        let dest = tmp();
        let out = backup(&root, &dest).unwrap();
        assert!(out.file_name().unwrap().to_string_lossy().starts_with("oxydice-"));
        assert!(out.join("nota.md").exists());
        assert!(!out.join(".secreto").exists());
        std::fs::remove_dir_all(&root).ok();
        std::fs::remove_dir_all(&dest).ok();
    }
}
