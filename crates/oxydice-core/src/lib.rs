//! Oxydice core — lógica agnóstica de UI.
//!
//! Este crate no conoce ninguna tecnología de interfaz: solo el documento
//! Markdown, la búsqueda, la configuración persistente, los temas (como
//! paleta + variables CSS), el render Markdown→HTML (única fuente de verdad
//! de la vista previa en escritorio y móvil), las operaciones de *vault*
//! sobre el sistema de archivos y el motor de sincronización (índice
//! baseline + diff a 3 bandas). La capa Tauri lo envuelve en comandos.

pub mod config;
pub mod doc;
pub mod export;
pub mod ext;
pub mod render;
pub mod search;
pub mod sync;
pub mod theme;
pub mod vault;
pub mod viewer;
