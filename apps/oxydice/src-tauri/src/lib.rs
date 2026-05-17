//! Capa Tauri de Oxydice: envuelve `oxydice-core` en comandos tipados.
//!
//! Toda la lógica vive en el core (agnóstico de UI). Aquí solo se traducen
//! argumentos del frontend (cadenas de ruta, paleta) a llamadas al core y se
//! devuelven tipos serializables. El `Registry` es barato, se construye por
//! llamada: no hace falta estado compartido.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use oxydice_core::config::Config;
use oxydice_core::doc::{self, DocMeta, Heading, MetaEdit};
use oxydice_core::ext::sync::SyncState;
use oxydice_core::ext::theme::ThemeEntry;
use oxydice_core::ext::{ExtRow, Registry};
use oxydice_core::search::{self, Results};
use oxydice_core::sync::backend::{OpendalRemote, RemoteConfig, RemoteKind};
use oxydice_core::sync::{self, SyncReport};
use oxydice_core::theme::{self, Palette};
use oxydice_core::{render, vault};
use serde::Serialize;
use tauri::{Emitter, Manager, WindowEvent};

// ---- configuración ------------------------------------------------------

#[tauri::command]
fn load_config() -> Config {
    Config::load()
}

#[tauri::command]
fn save_config(config: Config) {
    config.save();
}

// ---- vault (sistema de archivos) ----------------------------------------

#[tauri::command]
fn list_dir(path: String) -> Vec<vault::Entry> {
    vault::entries(Path::new(&path))
}

#[tauri::command]
fn read_note(path: String) -> Result<vault::NoteData, String> {
    vault::read_note(Path::new(&path))
}

#[tauri::command]
fn write_note(path: String, content: String) -> Result<Option<u64>, String> {
    vault::write_note(Path::new(&path), &content)
}

#[tauri::command]
fn create_note(parent: String, name: String) -> Result<PathBuf, String> {
    vault::create_note(Path::new(&parent), &name)
}

#[tauri::command]
fn create_folder(parent: String, name: String) -> Result<PathBuf, String> {
    vault::create_folder(Path::new(&parent), &name)
}

#[tauri::command]
fn rename_path(old: String, name: String) -> Result<PathBuf, String> {
    vault::rename_path(Path::new(&old), &name)
}

#[tauri::command]
fn delete_path(path: String) -> Result<(), String> {
    vault::delete_path(Path::new(&path))
}

#[tauri::command]
fn backup_now(space: String, dest: String) -> Result<PathBuf, String> {
    vault::backup(Path::new(&space), Path::new(&dest))
}

// ---- documento ----------------------------------------------------------

#[tauri::command]
fn render_markdown(markdown: String) -> String {
    render::to_html(&markdown)
}

#[tauri::command]
fn doc_meta(content: String) -> DocMeta {
    doc::meta(&content)
}

#[tauri::command]
fn outline(content: String) -> Vec<Heading> {
    doc::outline(&content)
}

/// Reescribe el *frontmatter* de la nota (round-trip seguro) y devuelve su
/// nuevo `mtime`. La capa core preserva claves desconocidas y el cuerpo.
#[tauri::command]
fn write_meta(path: String, edit: MetaEdit) -> Result<Option<u64>, String> {
    let p = Path::new(&path);
    let content = vault::read_note(p)?.content;
    let updated = doc::write_meta(&content, &edit);
    vault::write_note(p, &updated)
}

// ---- búsqueda -----------------------------------------------------------

#[tauri::command]
fn notes_with_tag(root: String, tag: String) -> Vec<PathBuf> {
    search::notes_with_tag(Path::new(&root), &tag)
}

#[tauri::command]
fn search_space(root: String, query: String) -> Results {
    search::search(Path::new(&root), &query)
}

// ---- temas y extensiones ------------------------------------------------

/// Tema resuelto: paleta efectiva, variables CSS para `:root` y nombre legible.
#[derive(Serialize)]
struct ResolvedTheme {
    palette: Palette,
    css: std::collections::BTreeMap<String, String>,
    name: String,
}

#[tauri::command]
fn theme_catalog() -> Vec<ThemeEntry> {
    Registry::builtin().theme_entries()
}

#[tauri::command]
fn resolve_theme(id: String, system_dark: bool, custom: Palette) -> ResolvedTheme {
    let reg = Registry::builtin();
    let palette = reg.resolve_theme(&id, system_dark, &custom);
    ResolvedTheme {
        css: theme::css_vars(&palette),
        name: reg.theme_name(&id),
        palette,
    }
}

#[tauri::command]
fn extensions_listing() -> Vec<ExtRow> {
    Registry::builtin().listing()
}

#[tauri::command]
fn sync_now(id: String) -> Result<(), String> {
    Registry::builtin().sync_now(&id)
}

/// Exporta la nota actual como HTML autónomo (T18). El PDF se obtiene
/// imprimiendo la vista de lectura desde el frontend (sin dependencias).
#[tauri::command]
fn export_html(
    path: String,
    markdown: String,
    title: String,
    palette: Palette,
) -> Result<(), String> {
    let html = oxydice_core::export::html_with_palette(&markdown, &title, &palette);
    std::fs::write(&path, html).map_err(|e| format!("Error al exportar: {e}"))
}

#[tauri::command]
fn export_theme(path: String, palette: Palette) -> Result<(), String> {
    let json = serde_json::to_string_pretty(&palette)
        .map_err(|e| format!("Error al serializar: {e}"))?;
    std::fs::write(&path, json).map_err(|e| format!("Error al exportar: {e}"))
}

#[tauri::command]
fn import_theme(path: String) -> Result<Palette, String> {
    let raw =
        std::fs::read_to_string(&path).map_err(|e| format!("Error al leer: {e}"))?;
    serde_json::from_str::<Palette>(&raw).map_err(|_| "Archivo de tema no válido".to_owned())
}

// ---- sincronización -----------------------------------------------------

/// Estado de sync compartido; el frontend lo consulta y recibe eventos.
struct AppSync(Mutex<SyncState>);

/// Identificador de dispositivo para los nombres de conflicto (usuario del
/// SO; `dispositivo` si no se puede determinar).
fn device_name() -> String {
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_else(|_| "dispositivo".to_owned())
}

/// Fecha local `YYYY-MM-DD` sin dependencias (algoritmo civil de Hinnant).
fn today() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let z = (secs / 86_400) as i64 + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

fn keyring_entry(kind: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new("Oxydice", &format!("sync-{kind}"))
        .map_err(|e| format!("Llavero: {e}"))
}

/// Construye el remoto OpenDAL desde la config + el secreto del llavero.
fn build_remote(cfg: &Config) -> Result<OpendalRemote, String> {
    let r = &cfg.remote;
    let kind = match r.kind.as_str() {
        "webdav" => RemoteKind::Webdav,
        "s3" => RemoteKind::S3,
        _ => return Err("Sincronización no configurada".to_owned()),
    };
    let secret = keyring_entry(&r.kind)
        .and_then(|e| e.get_password().map_err(|e| format!("Llavero: {e}")))
        .unwrap_or_default();
    let rc = RemoteConfig {
        endpoint: r.endpoint.clone(),
        root: r.root.clone(),
        username: r.username.clone(),
        password: secret.clone(),
        bucket: r.bucket.clone(),
        region: r.region.clone(),
        access_key: r.access_key.clone(),
        secret_key: secret,
    };
    OpendalRemote::new(kind, &rc)
}

fn set_state(app: &tauri::AppHandle, st: SyncState) {
    if let Some(s) = app.try_state::<AppSync>() {
        if let Ok(mut g) = s.0.lock() {
            *g = st.clone();
        }
    }
    let _ = app.emit("sync:state", &st);
}

/// Ejecuta una sincronización (bloqueante) y publica el estado resultante.
fn do_sync(app: &tauri::AppHandle) -> Result<SyncReport, String> {
    let cfg = Config::load();
    let space = cfg
        .selected_space()
        .cloned()
        .ok_or_else(|| "No hay espacio activo".to_owned())?;
    set_state(app, SyncState::Syncing);
    let mut remote = match build_remote(&cfg) {
        Ok(r) => r,
        Err(e) => {
            set_state(app, SyncState::Error(e.clone()));
            return Err(e);
        }
    };
    match sync::sync_space(&space, &mut remote, &device_name(), &today()) {
        Ok(rep) => {
            set_state(app, SyncState::Synced);
            Ok(rep)
        }
        Err(e) => {
            set_state(app, SyncState::Error(e.clone()));
            Err(e)
        }
    }
}

#[tauri::command]
async fn sync_run(app: tauri::AppHandle) -> Result<SyncReport, String> {
    tauri::async_runtime::spawn_blocking(move || do_sync(&app))
        .await
        .map_err(|e| format!("Error de tarea: {e}"))?
}

#[tauri::command]
fn sync_get_state(state: tauri::State<AppSync>) -> SyncState {
    state.0.lock().map(|g| g.clone()).unwrap_or(SyncState::Disabled)
}

#[tauri::command]
fn sync_set_secret(kind: String, secret: String) -> Result<(), String> {
    keyring_entry(&kind)?
        .set_password(&secret)
        .map_err(|e| format!("Llavero: {e}"))
}

#[tauri::command]
fn sync_clear_secret(kind: String) -> Result<(), String> {
    match keyring_entry(&kind)?.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(format!("Llavero: {e}")),
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppSync(Mutex::new(SyncState::Disabled)))
        .setup(|app| {
            // Worker de auto-sync: relee la config cada tic (intervalo
            // mínimo 60 s) y sincroniza si está activado. El *backoff* ante
            // error es esperar al siguiente intervalo (refinable a S3+).
            let handle = app.handle().clone();
            std::thread::spawn(move || loop {
                let secs = Config::load().remote.interval_secs.max(60);
                std::thread::sleep(std::time::Duration::from_secs(secs));
                let cfg = Config::load();
                if cfg.remote.auto && cfg.remote.kind != "none" {
                    let _ = do_sync(&handle);
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            // Al recuperar el foco, sincroniza si el auto-sync está activo.
            if let WindowEvent::Focused(true) = event {
                let h = window.app_handle().clone();
                std::thread::spawn(move || {
                    let cfg = Config::load();
                    if cfg.remote.auto && cfg.remote.kind != "none" {
                        let _ = do_sync(&h);
                    }
                });
            }
        })
        .invoke_handler(tauri::generate_handler![
            load_config,
            save_config,
            list_dir,
            read_note,
            write_note,
            create_note,
            create_folder,
            rename_path,
            delete_path,
            backup_now,
            render_markdown,
            doc_meta,
            outline,
            write_meta,
            notes_with_tag,
            search_space,
            theme_catalog,
            resolve_theme,
            extensions_listing,
            sync_now,
            sync_run,
            sync_get_state,
            sync_set_secret,
            sync_clear_secret,
            export_html,
            export_theme,
            import_theme
        ])
        .run(tauri::generate_context!())
        .expect("error al arrancar la aplicación Tauri");
}
