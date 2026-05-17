//! Backend de transporte real sobre **OpenDAL**: WebDAV y S3 con claves del
//! usuario (sin OAuth; eso es S4 y va en la capa shell). Implementa el trait
//! [`Remote`] que consume el motor de sync.
//!
//! OpenDAL es asíncrono; aquí se encapsula un runtime tokio *current-thread*
//! y se hace `block_on`, de modo que el motor sigue siendo síncrono y
//! testeable. La «versión» de un objeto (para detectar cambios remotos sin
//! descargar) es su ETag; si el proveedor no lo da, se cae a la fecha de
//! modificación y, en último término, al tamaño.

use super::{Remote, RemoteEntry};
use opendal::{services, Metadata, Operator};

/// Qué proveedor usar. Las claves las aporta el usuario (no hay OAuth).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemoteKind {
    Webdav,
    S3,
}

/// Configuración para construir el operador. Los secretos (contraseña /
/// *secret key*) los inyecta la capa shell desde el llavero del SO.
#[derive(Debug, Clone, Default)]
pub struct RemoteConfig {
    pub endpoint: String,
    pub root: String,
    // WebDAV
    pub username: String,
    pub password: String,
    // S3
    pub bucket: String,
    pub region: String,
    pub access_key: String,
    pub secret_key: String,
}

/// Remoto respaldado por OpenDAL. El runtime vive con el operador.
pub struct OpendalRemote {
    op: Operator,
    rt: tokio::runtime::Runtime,
}

impl OpendalRemote {
    pub fn new(kind: RemoteKind, cfg: &RemoteConfig) -> Result<Self, String> {
        let root = if cfg.root.is_empty() { "/" } else { &cfg.root };
        let op = match kind {
            RemoteKind::Webdav => {
                let b = services::Webdav::default()
                    .endpoint(&cfg.endpoint)
                    .username(&cfg.username)
                    .password(&cfg.password)
                    .root(root);
                Operator::new(b).map_err(err)?.finish()
            }
            RemoteKind::S3 => {
                let b = services::S3::default()
                    .endpoint(&cfg.endpoint)
                    .bucket(&cfg.bucket)
                    .region(&cfg.region)
                    .access_key_id(&cfg.access_key)
                    .secret_access_key(&cfg.secret_key)
                    .root(root);
                Operator::new(b).map_err(err)?.finish()
            }
        };
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Runtime de sync: {e}"))?;
        Ok(Self { op, rt })
    }

    fn version_of(meta: &Metadata) -> String {
        if let Some(e) = meta.etag() {
            return e.to_owned();
        }
        if let Some(lm) = meta.last_modified() {
            // Token estable de versión (no necesita formato canónico).
            return lm.to_string();
        }
        format!("len:{}", meta.content_length())
    }
}

fn err<E: std::fmt::Display>(e: E) -> String {
    format!("Remoto: {e}")
}

impl Remote for OpendalRemote {
    fn list(&mut self) -> Result<Vec<RemoteEntry>, String> {
        self.rt.block_on(async {
            let entries = self
                .op
                .list_with("")
                .recursive(true)
                .await
                .map_err(err)?;
            let mut out = Vec::new();
            for e in entries {
                if !e.metadata().is_file() {
                    continue;
                }
                let path = e.path().to_owned();
                if !path.ends_with(".md") {
                    continue;
                }
                let meta = self.op.stat(&path).await.map_err(err)?;
                out.push(RemoteEntry {
                    path,
                    version: Self::version_of(&meta),
                });
            }
            Ok(out)
        })
    }

    fn get(&mut self, path: &str) -> Result<Vec<u8>, String> {
        self.rt.block_on(async {
            let buf = self.op.read(path).await.map_err(err)?;
            Ok(buf.to_vec())
        })
    }

    fn put(&mut self, path: &str, data: &[u8]) -> Result<String, String> {
        self.rt.block_on(async {
            self.op.write(path, data.to_vec()).await.map_err(err)?;
            let meta = self.op.stat(path).await.map_err(err)?;
            Ok(Self::version_of(&meta))
        })
    }

    fn delete(&mut self, path: &str) -> Result<(), String> {
        self.rt
            .block_on(async { self.op.delete(path).await.map_err(err) })
    }
}
