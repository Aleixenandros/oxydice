//! Motor de sincronización: índice baseline + reconciliación a 3 bandas +
//! conflicto por **bifurcación** + escrituras **atómicas**.
//!
//! El transporte (WebDAV, S3, Drive…) se abstrae tras [`Remote`]; un backend
//! OpenDAL lo implementará (Fase 2, S2) envolviendo un operador bloqueante,
//! por eso la API es síncrona. El motor en sí es síncrono y testeable
//! *headless*: es el punto donde se pierden datos, así que la lógica se
//! prueba sin red. *Worker* async (tokio), *backoff* y cableado a la UI son
//! el siguiente incremento (S3); aquí queda el núcleo seguro.

pub mod backend;
pub mod index;
pub mod reconcile;

use index::SyncIndex;
use reconcile::{reconcile, Action, Baseline, FileState};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

/// Una entrada del remoto: ruta relativa y su versión nativa (ETag de
/// S3/WebDAV, rev de Dropbox…), que detecta cambios remotos sin descargar.
#[derive(Debug, Clone)]
pub struct RemoteEntry {
    pub path: String,
    pub version: String,
}

/// Transporte hacia un remoto. Lo implementará un backend OpenDAL. Métodos
/// síncronos: el backend puede envolver un operador bloqueante de OpenDAL.
pub trait Remote {
    fn list(&mut self) -> Result<Vec<RemoteEntry>, String>;
    fn get(&mut self, path: &str) -> Result<Vec<u8>, String>;
    /// Sube `data` y devuelve la **nueva versión** del objeto.
    fn put(&mut self, path: &str, data: &[u8]) -> Result<String, String>;
    fn delete(&mut self, path: &str) -> Result<(), String>;
}

/// Resumen de una sincronización (para el registro y el `SyncState`).
#[derive(Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct SyncReport {
    pub uploaded: usize,
    pub downloaded: usize,
    pub deleted_local: usize,
    pub deleted_remote: usize,
    /// Copias de conflicto creadas: nunca hay pérdida silenciosa de datos.
    pub conflicts: Vec<String>,
}

/// SHA-256 hex del contenido (línea base e identidad de contenido).
pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

/// Nombre determinista de la copia de conflicto, preservando carpeta y
/// extensión: `dir/nota (conflicto <disp> <fecha>).md`. Determinista para
/// que dos pasadas no creen copias distintas de la misma divergencia.
pub fn conflict_name(rel: &str, device: &str, date: &str) -> String {
    let (dir, file) = match rel.rsplit_once('/') {
        Some((d, f)) => (format!("{d}/"), f.to_owned()),
        None => (String::new(), rel.to_owned()),
    };
    let (stem, ext) = match file.rsplit_once('.') {
        Some((s, e)) => (s.to_owned(), format!(".{e}")),
        None => (file.clone(), String::new()),
    };
    format!("{dir}{stem} (conflicto {device} {date}){ext}")
}

/// Rutas relativas (`/`) de los `.md` bajo `space`, omitiendo ocultos
/// (incluido `.oxydice` y los temporales `.… .tmp` de escritura atómica).
fn local_files(space: &Path) -> Vec<String> {
    let mut out = Vec::new();
    walk(space, space, &mut out);
    out.sort();
    out
}

fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for e in rd.filter_map(Result::ok) {
        let p = e.path();
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name.starts_with('.') {
            continue;
        }
        if p.is_dir() {
            walk(root, &p, out);
        } else if p.extension().is_some_and(|x| x == "md") {
            if let Ok(rel) = p.strip_prefix(root) {
                out.push(rel.to_string_lossy().replace('\\', "/"));
            }
        }
    }
}

/// Sincroniza `space` contra `remote`. `device`/`date` hacen deterministas
/// los nombres de conflicto. Nunca pierde datos: ante divergencia real
/// bifurca y conserva ambas versiones (la local sigue siendo la canónica;
/// la remota se guarda como copia de conflicto y el remoto converge a la
/// local). Devuelve un resumen para el registro/estado de la UI.
pub fn sync_space(
    space: &Path,
    remote: &mut dyn Remote,
    device: &str,
    date: &str,
) -> Result<SyncReport, String> {
    let mut idx = SyncIndex::load(space);
    let mut report = SyncReport::default();

    // Hashes locales.
    let mut local: BTreeMap<String, String> = BTreeMap::new();
    for rel in local_files(space) {
        let bytes = std::fs::read(space.join(&rel))
            .map_err(|e| format!("Error al leer {rel}: {e}"))?;
        local.insert(rel, hash_bytes(&bytes));
    }
    // Versiones remotas.
    let remote_map: BTreeMap<String, String> = remote
        .list()?
        .into_iter()
        .map(|r| (r.path, r.version))
        .collect();

    // Unión de claves: local ∪ remoto ∪ índice.
    let mut keys: BTreeSet<String> = local.keys().cloned().collect();
    keys.extend(remote_map.keys().cloned());
    keys.extend(idx.files.keys().cloned());

    for key in keys {
        // Se clona la base para no mantener un préstamo de `idx` mientras se
        // muta `idx.files` en las acciones.
        let base = idx.files.get(&key).cloned();
        let action = reconcile(&FileState {
            local_hash: local.get(&key).map(String::as_str),
            remote_version: remote_map.get(&key).map(String::as_str),
            base: base.as_ref(),
        });

        match action {
            Action::Noop => {}
            Action::DropIndex => {
                idx.files.remove(&key);
            }
            Action::Upload => {
                let bytes = std::fs::read(space.join(&key))
                    .map_err(|e| format!("Error al leer {key}: {e}"))?;
                let version = remote.put(&key, &bytes)?;
                idx.files.insert(
                    key.clone(),
                    Baseline { hash: hash_bytes(&bytes), remote_version: version },
                );
                report.uploaded += 1;
            }
            Action::Download => {
                let bytes = remote.get(&key)?;
                crate::vault::write_atomic(&space.join(&key), &bytes)?;
                let version = remote_map.get(&key).cloned().unwrap_or_default();
                idx.files.insert(
                    key.clone(),
                    Baseline { hash: hash_bytes(&bytes), remote_version: version },
                );
                report.downloaded += 1;
            }
            Action::DeleteLocal => {
                let _ = std::fs::remove_file(space.join(&key));
                idx.files.remove(&key);
                report.deleted_local += 1;
            }
            Action::DeleteRemote => {
                remote.delete(&key)?;
                idx.files.remove(&key);
                report.deleted_remote += 1;
            }
            Action::CompareContent => {
                let rbytes = remote.get(&key)?;
                let lbytes = std::fs::read(space.join(&key))
                    .map_err(|e| format!("Error al leer {key}: {e}"))?;
                let lhash = hash_bytes(&lbytes);
                if hash_bytes(&rbytes) == lhash {
                    // Convergieron a lo mismo: solo registrar la base.
                    let version =
                        remote_map.get(&key).cloned().unwrap_or_default();
                    idx.files.insert(
                        key.clone(),
                        Baseline { hash: lhash, remote_version: version },
                    );
                } else {
                    // Conflicto real: conservar el remoto como copia
                    // bifurcada (nunca se pisa) y dejar el local como
                    // canónico; subir el local para que el remoto converja.
                    let cname = conflict_name(&key, device, date);
                    crate::vault::write_atomic(&space.join(&cname), &rbytes)?;
                    let version = remote.put(&key, &lbytes)?;
                    idx.files.insert(
                        key.clone(),
                        Baseline { hash: lhash, remote_version: version },
                    );
                    report.conflicts.push(cname);
                }
            }
        }
    }

    idx.save(space)?;
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// Remoto en memoria: cada `put` asigna una versión nueva (como un ETag).
    #[derive(Default)]
    struct MockRemote {
        files: BTreeMap<String, (Vec<u8>, String)>,
        ver: u64,
    }
    impl MockRemote {
        fn next(&mut self) -> String {
            self.ver += 1;
            format!("v{}", self.ver)
        }
        fn seed(&mut self, path: &str, data: &str) -> String {
            let v = self.next();
            self.files
                .insert(path.to_owned(), (data.as_bytes().to_vec(), v.clone()));
            v
        }
        fn body(&self, path: &str) -> Option<String> {
            self.files
                .get(path)
                .map(|(b, _)| String::from_utf8_lossy(b).into_owned())
        }
    }
    impl Remote for MockRemote {
        fn list(&mut self) -> Result<Vec<RemoteEntry>, String> {
            Ok(self
                .files
                .iter()
                .map(|(p, (_, v))| RemoteEntry { path: p.clone(), version: v.clone() })
                .collect())
        }
        fn get(&mut self, path: &str) -> Result<Vec<u8>, String> {
            self.files
                .get(path)
                .map(|(b, _)| b.clone())
                .ok_or_else(|| format!("no existe {path}"))
        }
        fn put(&mut self, path: &str, data: &[u8]) -> Result<String, String> {
            let v = self.next();
            self.files.insert(path.to_owned(), (data.to_vec(), v.clone()));
            Ok(v)
        }
        fn delete(&mut self, path: &str) -> Result<(), String> {
            self.files.remove(path);
            Ok(())
        }
    }

    fn tmp() -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!(
            "ox-sync-{}",
            SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos()
        ));
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn write(space: &Path, rel: &str, body: &str) {
        let p = space.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, body).unwrap();
    }
    fn read(space: &Path, rel: &str) -> Option<String> {
        std::fs::read_to_string(space.join(rel)).ok()
    }
    fn sync(space: &Path, r: &mut MockRemote) -> SyncReport {
        sync_space(space, r, "portatil", "2026-05-17").unwrap()
    }

    #[test]
    fn fresh_local_uploads_then_stable() {
        let s = tmp();
        let mut r = MockRemote::default();
        write(&s, "a.md", "# A");
        write(&s, "sub/b.md", "# B");

        let rep = sync(&s, &mut r);
        assert_eq!(rep.uploaded, 2);
        assert_eq!(r.body("a.md").as_deref(), Some("# A"));
        assert_eq!(r.body("sub/b.md").as_deref(), Some("# B"));

        // Segunda pasada sin cambios: nada que hacer.
        assert_eq!(sync(&s, &mut r), SyncReport::default());
        std::fs::remove_dir_all(&s).ok();
    }

    #[test]
    fn fresh_remote_downloads() {
        let s = tmp();
        let mut r = MockRemote::default();
        r.seed("remota.md", "contenido remoto");

        let rep = sync(&s, &mut r);
        assert_eq!(rep.downloaded, 1);
        assert_eq!(read(&s, "remota.md").as_deref(), Some("contenido remoto"));
        assert_eq!(sync(&s, &mut r), SyncReport::default());
        std::fs::remove_dir_all(&s).ok();
    }

    #[test]
    fn one_sided_edits_propagate_each_way() {
        let s = tmp();
        let mut r = MockRemote::default();
        write(&s, "n.md", "v1");
        sync(&s, &mut r);

        // Edición local → sube.
        write(&s, "n.md", "v2 local");
        assert_eq!(sync(&s, &mut r).uploaded, 1);
        assert_eq!(r.body("n.md").as_deref(), Some("v2 local"));

        // Edición remota → baja.
        r.put("n.md", b"v3 remoto").unwrap();
        let rep = sync(&s, &mut r);
        assert_eq!(rep.downloaded, 1);
        assert_eq!(read(&s, "n.md").as_deref(), Some("v3 remoto"));
        std::fs::remove_dir_all(&s).ok();
    }

    #[test]
    fn identical_both_new_converges_without_conflict() {
        let s = tmp();
        let mut r = MockRemote::default();
        write(&s, "dup.md", "mismo texto");
        r.seed("dup.md", "mismo texto");

        let rep = sync(&s, &mut r);
        assert!(rep.conflicts.is_empty());
        assert_eq!(rep.uploaded, 0);
        assert_eq!(rep.downloaded, 0);
        assert_eq!(sync(&s, &mut r), SyncReport::default());
        std::fs::remove_dir_all(&s).ok();
    }

    #[test]
    fn real_divergence_forks_and_loses_nothing() {
        let s = tmp();
        let mut r = MockRemote::default();
        write(&s, "doc.md", "base");
        sync(&s, &mut r);

        // Divergen: edición local y edición remota distintas.
        write(&s, "doc.md", "edición LOCAL");
        r.put("doc.md", b"edicion REMOTA").unwrap();

        let rep = sync(&s, &mut r);
        assert_eq!(rep.conflicts.len(), 1);
        let cname = &rep.conflicts[0];
        assert_eq!(
            cname,
            "doc (conflicto portatil 2026-05-17).md"
        );
        // Nada se pierde: local canónico intacto, remoto preservado en la
        // copia de conflicto, y el remoto converge al local.
        assert_eq!(read(&s, "doc.md").as_deref(), Some("edición LOCAL"));
        assert_eq!(read(&s, cname).as_deref(), Some("edicion REMOTA"));
        assert_eq!(r.body("doc.md").as_deref(), Some("edición LOCAL"));

        // La copia de conflicto se sincroniza como archivo nuevo y estabiliza.
        let rep2 = sync(&s, &mut r);
        assert!(rep2.conflicts.is_empty());
        assert_eq!(r.body(cname).as_deref(), Some("edicion REMOTA"));
        std::fs::remove_dir_all(&s).ok();
    }

    #[test]
    fn deletions_propagate_both_ways() {
        let s = tmp();
        let mut r = MockRemote::default();
        write(&s, "x.md", "x");
        write(&s, "y.md", "y");
        sync(&s, &mut r);

        // Borrado local → se borra en el remoto.
        std::fs::remove_file(s.join("x.md")).unwrap();
        let rep = sync(&s, &mut r);
        assert_eq!(rep.deleted_remote, 1);
        assert!(r.body("x.md").is_none());

        // Borrado remoto → se borra en local.
        r.delete("y.md").unwrap();
        let rep = sync(&s, &mut r);
        assert_eq!(rep.deleted_local, 1);
        assert!(read(&s, "y.md").is_none());
        std::fs::remove_dir_all(&s).ok();
    }

    #[test]
    fn delete_versus_edit_resurrects_no_loss() {
        let s = tmp();
        let mut r = MockRemote::default();
        write(&s, "k.md", "base");
        sync(&s, &mut r);

        // Borro local pero el remoto fue editado: debe resucitar el remoto.
        std::fs::remove_file(s.join("k.md")).unwrap();
        r.put("k.md", b"editado en remoto").unwrap();
        let rep = sync(&s, &mut r);
        assert_eq!(rep.downloaded, 1);
        assert_eq!(read(&s, "k.md").as_deref(), Some("editado en remoto"));
        std::fs::remove_dir_all(&s).ok();
    }
}
