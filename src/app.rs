//! Aplicación: navegación, explorador, búsqueda, lectura/edición y ajustes.
//!
//! La UI es inmediata (egui): cada frame se redibuja desde este estado y el
//! sistema de archivos. Las notas se **autoguardan** (sin botón de guardar);
//! el disco es la fuente de verdad.

use crate::config::{Config, EditorFont};
use crate::ext::sync::SyncState;
use crate::ext::Registry;
use crate::theme::{self, Palette, CUSTOM_ID, SYSTEM_ID};
use crate::{doc, search};
use eframe::egui;
use egui::text::{CCursor, CCursorRange, LayoutJob, TextFormat};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

/// Id estable del editor: necesario para mover el cursor desde el esquema.
const EDITOR_ID: &str = "rn-editor";
/// Espera tras la última pulsación antes de autoguardar.
const AUTOSAVE: Duration = Duration::from_millis(600);
/// Espera tras escribir en la búsqueda antes de relanzarla.
const SEARCH_DEBOUNCE: Duration = Duration::from_millis(220);

/// Sección principal mostrada en el área central.
#[derive(Clone, Copy, PartialEq, Eq)]
enum View {
    Explorer,
    Search,
    Settings,
}

/// Modo de un documento abierto: editar el Markdown o leerlo renderizado.
#[derive(Clone, Copy, PartialEq, Eq)]
enum DocMode {
    Edit,
    Read,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LogLevel {
    Info,
    Warn,
    Error,
}

struct LogEntry {
    ts: String,
    level: LogLevel,
    msg: String,
}

enum DialogKind {
    NewNote,
    NewFolder,
    /// Renombrar la ruta indicada (archivo o carpeta).
    Rename(PathBuf),
}

struct Dialog {
    kind: DialogKind,
    parent: PathBuf,
    name: String,
}

impl Dialog {
    fn new(kind: DialogKind, parent: PathBuf) -> Self {
        Self {
            kind,
            parent,
            name: String::new(),
        }
    }

    fn rename(target: PathBuf) -> Self {
        let parent = target.parent().map(Path::to_path_buf).unwrap_or_default();
        let name = target
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Self {
            kind: DialogKind::Rename(target),
            parent,
            name,
        }
    }

    fn title(&self) -> &'static str {
        match self.kind {
            DialogKind::NewNote => "Nueva nota",
            DialogKind::NewFolder => "Nueva carpeta",
            DialogKind::Rename(_) => "Renombrar",
        }
    }

    fn submit_label(&self) -> &'static str {
        match self.kind {
            DialogKind::Rename(_) => "Renombrar",
            _ => "Crear",
        }
    }
}

pub struct RustNotes {
    config: Config,
    current: Option<PathBuf>,
    buffer: String,
    dirty: bool,
    last_edit: Option<Instant>,
    mtime: Option<SystemTime>,
    status: String,
    md_cache: CommonMarkCache,
    view: View,
    doc_mode: DocMode,
    show_sidebar: bool,
    show_outline: bool,
    goto_line: Option<usize>,
    style_installed: bool,
    dialog: Option<Dialog>,
    pending_delete: Option<PathBuf>,
    registry: Registry,
    log: Vec<LogEntry>,
    query: String,
    results: search::Results,
    search_at: Option<Instant>,
}

impl RustNotes {
    pub fn new() -> Self {
        let mut app = Self {
            config: Config::load(),
            current: None,
            buffer: String::new(),
            dirty: false,
            last_edit: None,
            mtime: None,
            status: String::new(),
            md_cache: CommonMarkCache::default(),
            view: View::Explorer,
            doc_mode: DocMode::Edit,
            show_sidebar: true,
            show_outline: true,
            goto_line: None,
            style_installed: false,
            dialog: None,
            pending_delete: None,
            registry: Registry::builtin(),
            log: Vec::new(),
            query: String::new(),
            results: search::Results::default(),
            search_at: None,
        };
        app.log(LogLevel::Info, "RustNotes iniciado");
        app
    }

    // ---- registro -------------------------------------------------------

    fn log(&mut self, level: LogLevel, msg: impl Into<String>) {
        let msg = msg.into();
        self.log.push(LogEntry {
            ts: now_hms(),
            level,
            msg: msg.clone(),
        });
        if self.log.len() > 80 {
            self.log.remove(0);
        }
        self.status = msg;
    }

    // ---- acciones sobre datos -------------------------------------------

    fn open_note(&mut self, path: PathBuf) {
        if self.current.as_deref() == Some(path.as_path()) {
            return;
        }
        self.autosave_flush();
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.buffer = content;
                self.mtime = file_mtime(&path);
                self.current = Some(path.clone());
                self.dirty = false;
                self.last_edit = None;
                self.md_cache = CommonMarkCache::default();
                self.log(LogLevel::Info, format!("Abierta {}", file_name(&path)));
            }
            Err(e) => self.log(LogLevel::Error, format!("Error al leer: {e}")),
        }
    }

    /// Escribe la nota actual a disco si hay cambios. `quiet` evita el log.
    fn write_current(&mut self, quiet: bool) {
        let Some(path) = self.current.clone() else {
            return;
        };
        match std::fs::write(&path, &self.buffer) {
            Ok(()) => {
                self.dirty = false;
                self.last_edit = None;
                self.mtime = file_mtime(&path);
                if !quiet {
                    self.log(LogLevel::Info, "Guardado");
                } else {
                    self.status = "Guardado".to_owned();
                }
                if self.config.backup_on_save {
                    self.backup_now(true);
                }
            }
            Err(e) => self.log(LogLevel::Error, format!("Error al guardar: {e}")),
        }
    }

    /// Guarda inmediatamente si hay cambios pendientes (al cambiar de nota,
    /// al cerrar o con Ctrl+S).
    fn autosave_flush(&mut self) {
        if self.dirty && self.current.is_some() {
            self.write_current(true);
        }
    }

    /// Autoguardado diferido: guarda cuando cesa la escritura.
    fn autosave_tick(&mut self) {
        if self.dirty {
            if let Some(t) = self.last_edit {
                if t.elapsed() >= AUTOSAVE {
                    self.write_current(true);
                }
            }
        }
    }

    fn add_space(&mut self) {
        if let Some(dir) = rfd::FileDialog::new().pick_folder() {
            let idx = match self.config.spaces.iter().position(|p| p == &dir) {
                Some(i) => i,
                None => {
                    self.config.spaces.push(dir);
                    self.config.spaces.len() - 1
                }
            };
            self.config.selected = Some(idx);
            self.clear_open();
            self.config.save();
        }
    }

    fn clear_open(&mut self) {
        self.autosave_flush();
        self.current = None;
        self.buffer.clear();
        self.dirty = false;
    }

    fn remove_current_space(&mut self) {
        if let Some(i) = self.config.selected {
            if i < self.config.spaces.len() {
                self.config.spaces.remove(i);
                self.config.selected = if self.config.spaces.is_empty() {
                    None
                } else {
                    Some(0)
                };
                self.clear_open();
                self.config.save();
            }
        }
    }

    fn apply_dialog(&mut self, dialog: &Dialog) {
        let name = dialog.name.trim();
        match &dialog.kind {
            DialogKind::NewFolder => {
                let path = dialog.parent.join(name);
                match std::fs::create_dir_all(&path) {
                    Ok(()) => self.log(LogLevel::Info, format!("Carpeta creada: {name}")),
                    Err(e) => self.log(LogLevel::Error, format!("Error: {e}")),
                }
            }
            DialogKind::NewNote => {
                let file = if name.ends_with(".md") {
                    name.to_owned()
                } else {
                    format!("{name}.md")
                };
                let path = dialog.parent.join(&file);
                match std::fs::write(&path, format!("# {name}\n")) {
                    Ok(()) => self.open_note(path),
                    Err(e) => self.log(LogLevel::Error, format!("Error: {e}")),
                }
            }
            DialogKind::Rename(old) => {
                let leaf = if old.is_file() && !name.ends_with(".md") {
                    format!("{name}.md")
                } else {
                    name.to_owned()
                };
                let new_path = dialog.parent.join(&leaf);
                if new_path == *old {
                    return;
                }
                if new_path.exists() {
                    self.log(LogLevel::Warn, format!("Ya existe «{leaf}»"));
                    return;
                }
                match std::fs::rename(old, &new_path) {
                    Ok(()) => {
                        self.remap_current(old, &new_path);
                        self.log(LogLevel::Info, format!("Renombrado a {leaf}"));
                    }
                    Err(e) => self.log(LogLevel::Error, format!("Error al renombrar: {e}")),
                }
            }
        }
    }

    fn remap_current(&mut self, old: &Path, new: &Path) {
        let Some(cur) = self.current.clone() else {
            return;
        };
        if cur == old {
            self.current = Some(new.to_path_buf());
        } else if let Ok(rest) = cur.strip_prefix(old) {
            self.current = Some(new.join(rest));
        }
    }

    fn delete_path(&mut self, path: &Path) {
        let res = if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        };
        match res {
            Ok(()) => {
                if self
                    .current
                    .as_deref()
                    .is_some_and(|c| c == path || c.starts_with(path))
                {
                    self.current = None;
                    self.buffer.clear();
                    self.dirty = false;
                }
                self.log(LogLevel::Info, "Eliminado");
            }
            Err(e) => self.log(LogLevel::Error, format!("Error al eliminar: {e}")),
        }
    }

    fn backup_now(&mut self, quiet: bool) {
        let Some(dest) = self.config.backup_dir.clone() else {
            if !quiet {
                self.log(
                    LogLevel::Warn,
                    "Configura una carpeta de copias en Ajustes",
                );
            }
            return;
        };
        let Some(space) = self.config.selected_space().cloned() else {
            if !quiet {
                self.log(LogLevel::Warn, "No hay espacio activo");
            }
            return;
        };
        let secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let target = dest.join(format!("rustnotes-{}-{secs}", file_name(&space)));
        match copy_dir(&space, &target) {
            Ok(()) => self.log(LogLevel::Info, format!("Copia creada: {}", target.display())),
            Err(e) => self.log(LogLevel::Error, format!("Error en la copia: {e}")),
        }
    }

    fn run_search(&mut self) {
        let root = self.config.selected_space().cloned();
        self.results = match root {
            Some(root) => search::search(&root, &self.query),
            None => search::Results::default(),
        };
    }

    // ---- barra superior -------------------------------------------------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar").show_inside(ui, |ui| {
            ui.add_space(7.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                if icon_button(ui, "▤", "Mostrar/ocultar barra lateral").clicked() {
                    self.show_sidebar = !self.show_sidebar;
                }
                ui.menu_button("☰", |ui| {
                    if ui.button("Añadir espacio…").clicked() {
                        self.add_space();
                        ui.close();
                    }
                    if ui
                        .add_enabled(
                            self.config.selected.is_some(),
                            egui::Button::new("Quitar espacio actual"),
                        )
                        .clicked()
                    {
                        self.remove_current_space();
                        ui.close();
                    }
                });

                ui.add_space(6.0);
                let crumb = match (self.config.selected_space(), &self.current) {
                    (Some(sp), Some(note)) => {
                        let rel = note.strip_prefix(sp).unwrap_or(note);
                        format!("{}   ›   {}", file_name(sp), rel.to_string_lossy())
                    }
                    (Some(sp), None) => file_name(sp),
                    _ => "Sin espacio".to_owned(),
                };
                ui.label(egui::RichText::new(crumb).color(ui.visuals().weak_text_color()));

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;

                    // Estado de sincronización (guía §6).
                    let state = self.registry.sync_by_id(&self.config.sync).state();
                    let col = match &state {
                        SyncState::Disabled => ui.visuals().weak_text_color(),
                        SyncState::Synced => egui::Color32::from_rgb(76, 175, 92),
                        SyncState::Syncing => ui.visuals().hyperlink_color,
                        SyncState::Error(_) => egui::Color32::from_rgb(201, 74, 74),
                    };
                    ui.label(egui::RichText::new(state.glyph()).color(col))
                        .on_hover_text(state.label());

                    let mut sel = self.config.theme.clone();
                    egui::ComboBox::from_id_salt("theme")
                        .selected_text(self.registry.theme_name(&sel))
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut sel, SYSTEM_ID.to_owned(), "Sistema");
                            for t in self.registry.theme_entries() {
                                ui.selectable_value(&mut sel, t.id.to_owned(), t.name);
                            }
                            ui.selectable_value(&mut sel, CUSTOM_ID.to_owned(), "Personalizado");
                        });
                    if sel != self.config.theme {
                        self.config.theme = sel;
                        self.config.save();
                    }

                    // Controles del documento (solo con nota abierta).
                    if self.view == View::Explorer && self.current.is_some() {
                        ui.add_space(2.0);
                        if icon_button(
                            ui,
                            if self.show_outline { "◧" } else { "▢" },
                            "Esquema del documento",
                        )
                        .clicked()
                        {
                            self.show_outline = !self.show_outline;
                        }
                        let mut mode = self.doc_mode;
                        segmented(ui, &["Editar", "Vista"], &mut mode);
                        self.doc_mode = mode;
                    }

                    // Estado de autoguardado.
                    let (txt, weak) = if self.dirty {
                        ("Sin guardar…".to_owned(), true)
                    } else if !self.status.is_empty() {
                        (self.status.clone(), true)
                    } else {
                        (String::new(), true)
                    };
                    if !txt.is_empty() {
                        let mut t = egui::RichText::new(txt).small();
                        if weak {
                            t = t.weak();
                        }
                        ui.label(t);
                    }
                });
            });
            ui.add_space(7.0);
        });
    }

    // ---- barra lateral (rail de navegación + explorador) ----------------

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        // Marca.
        ui.horizontal(|ui| {
            ui.add_space(4.0);
            let accent = ui.visuals().hyperlink_color;
            egui::Frame::NONE
                .fill(accent)
                .corner_radius(egui::CornerRadius::same(5))
                .inner_margin(egui::Margin::symmetric(7, 4))
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new("R")
                            .color(theme::contrast_on(accent))
                            .strong(),
                    );
                });
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("RustNotes").strong());
                ui.label(
                    egui::RichText::new(format!("v{}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .weak(),
                );
            });
        });

        ui.add_space(12.0);
        if nav_item(ui, self.view == View::Explorer, "🗀", "Explorador") {
            self.view = View::Explorer;
        }
        if nav_item(ui, self.view == View::Search, "🔍", "Buscar") {
            self.view = View::Search;
        }
        if nav_item(ui, self.view == View::Settings, "⚙", "Ajustes") {
            self.view = View::Settings;
        }

        ui.add_space(10.0);
        ui.separator();
        ui.add_space(8.0);

        if self.view != View::Explorer {
            ui.label(
                egui::RichText::new("Selecciona Explorador para ver las notas.")
                    .small()
                    .weak(),
            );
            return;
        }

        // Selector de espacio.
        let names: Vec<(usize, String)> = self
            .config
            .spaces
            .iter()
            .enumerate()
            .map(|(i, p)| (i, file_name(p)))
            .collect();
        let current_label = self
            .config
            .selected_space()
            .map(|p| file_name(p))
            .unwrap_or_else(|| "— sin espacio —".to_owned());
        let mut sel = self.config.selected;
        egui::ComboBox::from_id_salt("space")
            .selected_text(current_label)
            .width(ui.available_width() - 12.0)
            .show_ui(ui, |ui| {
                for (i, nm) in &names {
                    ui.selectable_value(&mut sel, Some(*i), nm.as_str());
                }
            });
        if sel != self.config.selected {
            self.config.selected = sel;
            self.clear_open();
            self.config.save();
        }

        let root = self.config.selected_space().cloned();
        let Some(root) = root else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No hay espacios todavía").weak());
                ui.add_space(6.0);
                if ui.button("Añadir un espacio").clicked() {
                    self.add_space();
                }
            });
            return;
        };

        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button("＋  Nota").clicked() {
                self.dialog = Some(Dialog::new(DialogKind::NewNote, root.clone()));
            }
            if ui.button("＋  Carpeta").clicked() {
                self.dialog = Some(Dialog::new(DialogKind::NewFolder, root.clone()));
            }
        });

        ui.add_space(8.0);
        section_label(ui, "NOTAS");
        ui.add_space(2.0);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                self.tree(ui, &root);
                ui.add_space(8.0);
            });
    }

    fn tree(&mut self, ui: &mut egui::Ui, dir: &Path) {
        for path in entries(dir) {
            if path.is_dir() {
                let resp = egui::CollapsingHeader::new(file_name(&path))
                    .id_salt(&path)
                    .show(ui, |ui| self.tree(ui, &path));
                resp.header_response.context_menu(|ui| {
                    if ui.button("Nueva nota").clicked() {
                        self.dialog = Some(Dialog::new(DialogKind::NewNote, path.clone()));
                        ui.close();
                    }
                    if ui.button("Nueva carpeta").clicked() {
                        self.dialog = Some(Dialog::new(DialogKind::NewFolder, path.clone()));
                        ui.close();
                    }
                    ui.separator();
                    if ui.button("Renombrar…").clicked() {
                        self.dialog = Some(Dialog::rename(path.clone()));
                        ui.close();
                    }
                    if ui.button("Eliminar").clicked() {
                        self.pending_delete = Some(path.clone());
                        ui.close();
                    }
                });
            } else {
                let selected = self.current.as_deref() == Some(path.as_path());
                let row = egui::Button::selectable(selected, file_stem(&path))
                    .min_size(egui::vec2(ui.available_width(), 27.0));
                let resp = ui.add(row);
                resp.context_menu(|ui| {
                    if ui.button("Renombrar…").clicked() {
                        self.dialog = Some(Dialog::rename(path.clone()));
                        ui.close();
                    }
                    if ui.button("Eliminar").clicked() {
                        self.pending_delete = Some(path.clone());
                        ui.close();
                    }
                });
                if resp.clicked() {
                    self.open_note(path.clone());
                    self.view = View::Explorer;
                }
            }
        }
    }

    // ---- área central ---------------------------------------------------

    fn center(&mut self, ui: &mut egui::Ui) {
        match self.view {
            View::Search => self.search_view(ui),
            View::Settings => self.settings_view(ui),
            View::Explorer => self.explorer_view(ui),
        }
    }

    fn explorer_view(&mut self, ui: &mut egui::Ui) {
        if self.config.selected_space().is_none() {
            empty_state(ui, "Añade un espacio para empezar a tomar notas");
            return;
        }
        if self.current.is_none() {
            empty_state(ui, "Selecciona o crea una nota");
            return;
        }

        self.doc_header(ui);
        match self.doc_mode {
            DocMode::Read => {
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(8.0);
                        egui::Frame::NONE
                            .inner_margin(egui::Margin::symmetric(28, 8))
                            .show(ui, |ui| {
                                CommonMarkViewer::new().show(
                                    ui,
                                    &mut self.md_cache,
                                    &self.buffer,
                                );
                            });
                        ui.add_space(24.0);
                    });
            }
            DocMode::Edit => {
                if self.show_outline {
                    egui::Panel::left("outline")
                        .resizable(true)
                        .default_size(220.0)
                        .show_inside(ui, |ui| self.outline_panel(ui));
                }
                self.editor(ui);
            }
        }
    }

    fn doc_header(&mut self, ui: &mut egui::Ui) {
        let meta = doc::meta(&self.buffer);
        let title = meta
            .title
            .clone()
            .or_else(|| self.current.as_deref().map(file_stem))
            .unwrap_or_else(|| "Sin título".to_owned());

        egui::Frame::NONE
            .inner_margin(egui::Margin {
                left: 28,
                right: 28,
                top: 18,
                bottom: 14,
            })
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.heading(title);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            let mut mode = self.doc_mode;
                            segmented(ui, &["Editar", "Vista"], &mut mode);
                            self.doc_mode = mode;
                        },
                    );
                });

                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    meta_card(ui, "MODIFICADO", &rel_time(self.mtime));
                    let estado = meta.status.clone().unwrap_or_else(|| {
                        if self.dirty {
                            "Sin guardar".to_owned()
                        } else {
                            "Guardado".to_owned()
                        }
                    });
                    meta_card(ui, "ESTADO", &estado);
                    if let Some(author) = &meta.author {
                        meta_card(ui, "AUTOR", author);
                    }
                });

                if !meta.tags.is_empty() {
                    ui.add_space(10.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(6.0, 6.0);
                        ui.label(egui::RichText::new("#").weak());
                        for t in &meta.tags {
                            chip(ui, t);
                        }
                    });
                }
            });
        ui.separator();
    }

    fn outline_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(8.0);
        section_label(ui, "ESQUEMA");
        ui.add_space(4.0);
        ui.separator();
        let headings = doc::outline(&self.buffer);
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if headings.is_empty() {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new("Sin encabezados").small().weak());
                    return;
                }
                for h in headings {
                    ui.horizontal(|ui| {
                        ui.add_space((h.level.saturating_sub(1)) as f32 * 12.0);
                        let txt = if h.level <= 2 {
                            egui::RichText::new(&h.text)
                        } else {
                            egui::RichText::new(&h.text).weak()
                        };
                        if ui
                            .add(egui::Button::new(txt).frame(false))
                            .clicked()
                        {
                            self.goto_line = Some(h.line);
                            self.doc_mode = DocMode::Edit;
                        }
                    });
                }
            });
    }

    fn editor(&mut self, ui: &mut egui::Ui) {
        let family = match self.config.editor_font {
            EditorFont::Mono => egui::FontFamily::Monospace,
            EditorFont::Sans => egui::FontFamily::Proportional,
        };
        let font = egui::FontId::new(self.config.editor_font_size, family);
        let muted = ui.visuals().weak_text_color();
        let border = ui.visuals().widgets.noninteractive.bg_stroke.color;

        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let h = ui.available_height();
            // Medianil de números de línea.
            let lines = self.buffer.lines().count().max(1);
            let digits = lines.to_string().len().max(2);
            let gutter_w = digits as f32 * (self.config.editor_font_size * 0.62) + 22.0;
            let (gutter, _) =
                ui.allocate_exact_size(egui::vec2(gutter_w, h), egui::Sense::hover());
            ui.painter().vline(
                gutter.right(),
                gutter.y_range(),
                egui::Stroke::new(1.0, border),
            );

            let ew = ui.available_width();
            let out = ui
                .allocate_ui(egui::vec2(ew, h), |ui| {
                    egui::TextEdit::multiline(&mut self.buffer)
                        .id(egui::Id::new(EDITOR_ID))
                        .font(font.clone())
                        .frame(egui::Frame::NONE)
                        .desired_width(f32::INFINITY)
                        .margin(egui::Margin {
                            left: 14,
                            right: 16,
                            top: 8,
                            bottom: 8,
                        })
                        .show(ui)
                })
                .inner;

            if out.response.changed() {
                self.dirty = true;
                self.last_edit = Some(Instant::now());
                ui.ctx().request_repaint_after(AUTOSAVE);
            }

            // Números alineados a las filas reales del galley (respeta el
            // ajuste de línea y el desplazamiento).
            let painter = ui.painter().with_clip_rect(gutter.intersect(out.text_clip_rect));
            let mut logical = 0usize;
            let mut new_line = true;
            for row in &out.galley.rows {
                if new_line {
                    logical += 1;
                    let y = out.galley_pos.y + row.min_y();
                    painter.text(
                        egui::pos2(gutter.right() - 8.0, y + font.size * 0.12),
                        egui::Align2::RIGHT_TOP,
                        logical.to_string(),
                        font.clone(),
                        muted,
                    );
                }
                new_line = row.ends_with_newline;
            }

            // Salto desde el esquema / la búsqueda.
            if let Some(line) = self.goto_line.take() {
                let id = egui::Id::new(EDITOR_ID);
                if let Some(mut st) = egui::TextEdit::load_state(ui.ctx(), id) {
                    let idx: usize = self
                        .buffer
                        .split('\n')
                        .take(line)
                        .map(|l| l.chars().count() + 1)
                        .sum();
                    let idx = idx.min(self.buffer.chars().count());
                    st.cursor
                        .set_char_range(Some(CCursorRange::one(CCursor::new(idx))));
                    st.store(ui.ctx(), id);
                    ui.ctx().memory_mut(|m| m.request_focus(id));
                }
            }
        });
    }

    // ---- búsqueda -------------------------------------------------------

    fn search_view(&mut self, ui: &mut egui::Ui) {
        egui::Frame::NONE
            .inner_margin(egui::Margin::symmetric(28, 22))
            .show(ui, |ui| {
                ui.heading("Búsqueda global");
                ui.add_space(14.0);

                let edit = ui.add(
                    egui::TextEdit::singleline(&mut self.query)
                        .hint_text("🔍  Buscar en todas las notas…")
                        .desired_width(f32::INFINITY)
                        .margin(egui::Margin::symmetric(12, 10)),
                );
                let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if edit.changed() {
                    self.search_at = Some(Instant::now());
                    ui.ctx().request_repaint_after(SEARCH_DEBOUNCE);
                }
                let due = self
                    .search_at
                    .is_some_and(|t| t.elapsed() >= SEARCH_DEBOUNCE);
                if enter || due {
                    self.search_at = None;
                    self.run_search();
                }

                ui.add_space(12.0);
                if self.config.selected_space().is_none() {
                    ui.label(
                        egui::RichText::new("Añade un espacio para poder buscar.")
                            .weak(),
                    );
                    return;
                }
                if self.query.trim().is_empty() {
                    ui.label(
                        egui::RichText::new("Escribe para buscar en las notas del espacio.")
                            .weak(),
                    );
                    return;
                }

                let summary = format!(
                    "{} resultado(s) en {} archivo(s){}",
                    self.results.hits.len(),
                    self.results.files,
                    if self.results.truncated {
                        " · truncado"
                    } else {
                        ""
                    }
                );
                ui.label(egui::RichText::new(summary).small().weak());
                ui.add_space(8.0);

                let space = self.config.selected_space().cloned();
                let hits = self.results.hits.clone();
                let mut open: Option<(PathBuf, usize)> = None;
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for hit in &hits {
                            card(ui, |ui| {
                                ui.horizontal(|ui| {
                                    let name = file_name(&hit.path);
                                    if ui
                                        .add(
                                            egui::Button::new(
                                                egui::RichText::new(name)
                                                    .color(ui.visuals().hyperlink_color)
                                                    .strong(),
                                            )
                                            .frame(false),
                                        )
                                        .clicked()
                                    {
                                        open = Some((hit.path.clone(), hit.line));
                                    }
                                    let rel = space
                                        .as_ref()
                                        .and_then(|s| hit.path.strip_prefix(s).ok())
                                        .unwrap_or(&hit.path);
                                    ui.label(
                                        egui::RichText::new(rel.to_string_lossy().into_owned())
                                            .small()
                                            .weak(),
                                    );
                                });
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(format!("{:>4}", hit.line + 1))
                                            .monospace()
                                            .weak(),
                                    );
                                    ui.add(egui::Label::new(highlight(
                                        ui,
                                        &hit.text,
                                        hit.at,
                                    )));
                                });
                            });
                            ui.add_space(8.0);
                        }
                    });
                if let Some((path, line)) = open {
                    self.open_note(path);
                    self.view = View::Explorer;
                    self.doc_mode = DocMode::Edit;
                    self.goto_line = Some(line);
                }
            });
    }

    // ---- ajustes --------------------------------------------------------

    fn settings_view(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                egui::Frame::NONE
                    .inner_margin(egui::Margin::symmetric(28, 22))
                    .show(ui, |ui| {
                        ui.heading("Ajustes");
                        ui.label(
                            egui::RichText::new(
                                "Apariencia, sincronización, copias y extensiones.",
                            )
                            .weak(),
                        );
                        ui.add_space(16.0);
                        self.settings_appearance(ui);
                        ui.add_space(14.0);
                        self.settings_sync(ui);
                        ui.add_space(14.0);
                        self.settings_backup(ui);
                        ui.add_space(14.0);
                        self.settings_extensions(ui);
                        ui.add_space(14.0);
                        self.settings_about(ui);
                    });
            });
    }

    fn settings_appearance(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            section_label(ui, "APARIENCIA");
            ui.add_space(10.0);

            ui.label("Tema");
            ui.add_space(4.0);
            let mut sel = self.config.theme.clone();
            ui.horizontal_wrapped(|ui| {
                if ui.selectable_label(sel == SYSTEM_ID, "Sistema").clicked() {
                    sel = SYSTEM_ID.to_owned();
                }
                for t in self.registry.theme_entries() {
                    if ui.selectable_label(sel == t.id, t.name).clicked() {
                        sel = t.id.to_owned();
                    }
                }
                if ui
                    .selectable_label(sel == CUSTOM_ID, "Personalizado")
                    .clicked()
                {
                    sel = CUSTOM_ID.to_owned();
                }
            });
            if sel != self.config.theme {
                self.config.theme = sel;
                self.config.save();
            }

            ui.add_space(12.0);
            ui.label("Fuente del editor");
            ui.add_space(4.0);
            let mut font = self.config.editor_font;
            egui::ComboBox::from_id_salt("editor_font")
                .selected_text(font.label())
                .show_ui(ui, |ui| {
                    for f in EditorFont::ALL {
                        ui.selectable_value(&mut font, f, f.label());
                    }
                });
            if font != self.config.editor_font {
                self.config.editor_font = font;
                self.config.save();
            }

            ui.add_space(12.0);
            ui.label("Tamaño de fuente");
            let mut size = self.config.editor_font_size;
            if ui
                .add(egui::Slider::new(&mut size, 11.0..=22.0).suffix(" px"))
                .changed()
            {
                self.config.editor_font_size = size;
                self.config.save();
            }

            ui.add_space(12.0);
            ui.label("Escala de la interfaz");
            let mut scale = self.config.ui_scale;
            if ui
                .add(egui::Slider::new(&mut scale, 0.8..=1.6).step_by(0.05))
                .changed()
            {
                self.config.ui_scale = scale;
                self.config.save();
            }

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(10.0);
            section_label(ui, "TEMA PERSONALIZADO");
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("Editar un color selecciona «Personalizado».")
                    .small()
                    .weak(),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui
                    .button("Partir del tema actual")
                    .on_hover_text("Copia la paleta visible para retocarla")
                    .clicked()
                {
                    self.config.custom_theme = self.registry.resolve_theme(
                        &self.config.theme,
                        ui.ctx(),
                        &self.config.custom_theme,
                    );
                    self.config.theme = CUSTOM_ID.to_owned();
                    self.config.save();
                }
                if ui.button("Exportar…").clicked() {
                    self.export_theme(ui.ctx());
                }
                if ui.button("Importar…").clicked() {
                    self.import_theme();
                }
            });
            ui.add_space(10.0);
            let mut changed = false;
            {
                let c = &mut self.config.custom_theme;
                changed |= color_row(ui, "Acento", &mut c.accent);
                changed |= color_row(ui, "Fondo del editor", &mut c.bg);
                changed |= color_row(ui, "Superficie (paneles)", &mut c.surface);
                changed |= color_row(ui, "Texto", &mut c.text);
                changed |= color_row(ui, "Texto atenuado", &mut c.muted);
                changed |= color_row(ui, "Bordes", &mut c.border);
                ui.add_space(4.0);
                changed |= ui
                    .checkbox(&mut c.dark, "Base oscura (sombras y contraste)")
                    .changed();
            }
            if changed {
                self.config.theme = CUSTOM_ID.to_owned();
                self.config.save();
            }
        });
    }

    fn export_theme(&mut self, ctx: &egui::Context) {
        let palette =
            self.registry
                .resolve_theme(&self.config.theme, ctx, &self.config.custom_theme);
        let suggested = format!(
            "rustnotes-{}.json",
            self.registry.theme_name(&self.config.theme).to_lowercase()
        );
        let Some(path) = rfd::FileDialog::new()
            .set_file_name(suggested)
            .add_filter("Tema JSON", &["json"])
            .save_file()
        else {
            return;
        };
        match serde_json::to_string_pretty(&palette) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => self.log(LogLevel::Info, format!("Tema exportado: {}", path.display())),
                Err(e) => self.log(LogLevel::Error, format!("Error al exportar: {e}")),
            },
            Err(e) => self.log(LogLevel::Error, format!("Error al serializar: {e}")),
        }
    }

    fn import_theme(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .add_filter("Tema JSON", &["json"])
            .pick_file()
        else {
            return;
        };
        match std::fs::read_to_string(&path)
            .ok()
            .and_then(|raw| serde_json::from_str::<Palette>(&raw).ok())
        {
            Some(p) => {
                self.config.custom_theme = p;
                self.config.theme = CUSTOM_ID.to_owned();
                self.config.save();
                self.log(LogLevel::Info, "Tema importado y aplicado");
            }
            None => self.log(LogLevel::Error, "Archivo de tema no válido"),
        }
    }

    fn settings_sync(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            section_label(ui, "SINCRONIZACIÓN");
            ui.add_space(8.0);
            let providers: Vec<(&'static str, &'static str)> = self
                .registry
                .syncs()
                .iter()
                .map(|s| (s.id(), s.name()))
                .collect();
            let mut sel = self.config.sync.clone();
            ui.horizontal_wrapped(|ui| {
                for (id, name) in providers {
                    if ui.selectable_label(sel == id, name).clicked() {
                        sel = id.to_owned();
                    }
                }
            });
            if sel != self.config.sync {
                self.config.sync = sel;
                self.config.save();
            }
            let state = self.registry.sync_by_id(&self.config.sync).state();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(format!("Estado: {}", state.label()));
                let enabled = !matches!(state, SyncState::Disabled);
                if ui
                    .add_enabled(enabled, egui::Button::new("Sincronizar ahora"))
                    .clicked()
                {
                    match self.registry.sync_now(&self.config.sync) {
                        Ok(()) => self.log(LogLevel::Info, "Sincronización completada"),
                        Err(e) => self.log(LogLevel::Error, format!("Error de sync: {e}")),
                    }
                }
            });
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(
                    "Los backends reales (Git, S3, Drive) son backlog; la \
                     interfaz ya admite añadirlos como extensión.",
                )
                .small()
                .weak(),
            );

            ui.add_space(14.0);
            section_label(ui, "REGISTRO");
            ui.add_space(6.0);
            egui::Frame::NONE
                .fill(ui.visuals().extreme_bg_color)
                .stroke(egui::Stroke::new(
                    1.0,
                    ui.visuals().widgets.noninteractive.bg_stroke.color,
                ))
                .corner_radius(egui::CornerRadius::same(4))
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .auto_shrink([false, false])
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            for e in &self.log {
                                let col = match e.level {
                                    LogLevel::Info => ui.visuals().weak_text_color(),
                                    LogLevel::Warn => ui.visuals().hyperlink_color,
                                    LogLevel::Error => egui::Color32::from_rgb(201, 74, 74),
                                };
                                ui.label(
                                    egui::RichText::new(format!("[{}] {}", e.ts, e.msg))
                                        .monospace()
                                        .size(12.0)
                                        .color(col),
                                );
                            }
                        });
                });
        });
    }

    fn settings_backup(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            section_label(ui, "COPIA DE SEGURIDAD");
            ui.add_space(8.0);
            let dest = self
                .config
                .backup_dir
                .as_ref()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "(sin configurar)".to_owned());
            ui.label(format!("Carpeta de copias: {dest}"));
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button("Elegir carpeta…").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.config.backup_dir = Some(dir);
                        self.config.save();
                    }
                }
                if self.config.backup_dir.is_some() && ui.button("Quitar").clicked() {
                    self.config.backup_dir = None;
                    self.config.save();
                }
            });
            ui.add_space(8.0);
            let mut on_save = self.config.backup_on_save;
            if ui
                .checkbox(&mut on_save, "Hacer una copia tras cada guardado")
                .changed()
            {
                self.config.backup_on_save = on_save;
                self.config.save();
            }
            ui.add_space(10.0);
            let ready =
                self.config.backup_dir.is_some() && self.config.selected_space().is_some();
            if ui
                .add_enabled(ready, egui::Button::new("Crear copia ahora"))
                .clicked()
            {
                self.backup_now(false);
            }
        });
    }

    fn settings_extensions(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            section_label(ui, "EXTENSIONES");
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(
                    "Se compilan en el binario: añadir una es implementar su \
                     trait y registrarla.",
                )
                .small()
                .weak(),
            );
            ui.add_space(10.0);
            for (kind, id, name, detail) in self.registry.listing() {
                ui.horizontal(|ui| {
                    ui.label(
                        egui::RichText::new(format!("[{}]", kind.label()))
                            .small()
                            .weak(),
                    );
                    ui.label(name);
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.label(egui::RichText::new(detail).small().weak());
                            ui.label(egui::RichText::new(id).small().weak());
                        },
                    );
                });
            }
        });
    }

    fn settings_about(&mut self, ui: &mut egui::Ui) {
        card(ui, |ui| {
            section_label(ui, "ACERCA DE");
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new(format!("RustNotes v{}", env!("CARGO_PKG_VERSION")))
                    .strong(),
            );
            ui.label(format!("Autor: {}", env!("CARGO_PKG_AUTHORS")));
            ui.label("Licencia: MIT");
            ui.add_space(6.0);
            ui.hyperlink_to(
                "Repositorio en GitHub",
                "https://github.com/Aleixenandros/RustNotes",
            );
        });
    }

    // ---- ventanas modales ----------------------------------------------

    fn dialog_window(&mut self, ui: &mut egui::Ui) {
        let Some(mut dialog) = self.dialog.take() else {
            return;
        };
        let mut keep = true;
        let mut submit = false;
        egui::Window::new(dialog.title())
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ui.ctx(), |ui| {
                let edit = ui.add(
                    egui::TextEdit::singleline(&mut dialog.name)
                        .hint_text("nombre")
                        .desired_width(240.0),
                );
                edit.request_focus();
                let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(dialog.submit_label()).clicked() || enter {
                        submit = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        keep = false;
                    }
                });
            });
        if submit && !dialog.name.trim().is_empty() {
            self.apply_dialog(&dialog);
        } else if keep && !submit {
            self.dialog = Some(dialog);
        }
    }

    fn confirm_window(&mut self, ui: &mut egui::Ui) {
        let Some(path) = self.pending_delete.clone() else {
            return;
        };
        let mut keep = true;
        let mut confirmed = false;
        let is_dir = path.is_dir();
        egui::Window::new("Eliminar")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ui.ctx(), |ui| {
                let what = if is_dir {
                    "esta carpeta y todo su contenido"
                } else {
                    "esta nota"
                };
                ui.label(format!("¿Eliminar {what}?"));
                ui.label(egui::RichText::new(file_name(&path)).small().weak());
                ui.add_space(12.0);
                ui.horizontal(|ui| {
                    let danger = egui::Color32::from_rgb(201, 74, 74);
                    let del = egui::Button::new(
                        egui::RichText::new("Eliminar").color(theme::contrast_on(danger)),
                    )
                    .fill(danger);
                    if ui.add(del).clicked() {
                        confirmed = true;
                    }
                    if ui.button("Cancelar").clicked() {
                        keep = false;
                    }
                });
            });
        if confirmed {
            self.delete_path(&path);
        } else if keep {
            self.pending_delete = Some(path);
        }
    }
}

impl eframe::App for RustNotes {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        if !self.style_installed {
            theme::install_fonts(ui.ctx());
            theme::install_style(ui.ctx());
            self.style_installed = true;
        }
        let palette =
            self.registry
                .resolve_theme(&self.config.theme, ui.ctx(), &self.config.custom_theme);
        theme::apply(ui.ctx(), &palette);
        if (ui.ctx().zoom_factor() - self.config.ui_scale).abs() > f32::EPSILON {
            ui.ctx().set_zoom_factor(self.config.ui_scale);
        }

        // Ctrl+S fuerza el guardado; al cerrar, vacía lo pendiente.
        if ui.input(|i| i.modifiers.command && i.key_pressed(egui::Key::S)) {
            self.autosave_flush();
        }
        if ui.input(|i| i.viewport().close_requested()) {
            self.autosave_flush();
        }
        self.autosave_tick();

        self.top_bar(ui);
        if self.show_sidebar {
            egui::Panel::left("sidebar")
                .resizable(true)
                .default_size(266.0)
                .show_inside(ui, |ui| self.sidebar(ui));
        }

        let writing = self.view == View::Explorer
            && self.current.is_some()
            && self.doc_mode == DocMode::Edit;
        let fill = if writing {
            egui::Color32::from_rgb(palette.bg[0], palette.bg[1], palette.bg[2])
        } else {
            egui::Color32::from_rgb(
                palette.surface[0],
                palette.surface[1],
                palette.surface[2],
            )
        };
        let canvas = egui::Frame::central_panel(ui.style())
            .fill(fill)
            .inner_margin(egui::Margin::ZERO);
        egui::CentralPanel::default()
            .frame(canvas)
            .show_inside(ui, |ui| self.center(ui));

        self.dialog_window(ui);
        self.confirm_window(ui);
    }
}

// ---- utilidades de UI ---------------------------------------------------

/// Tarjeta: superficie elevada con borde sutil y esquinas de 4px (guía §1).
fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    egui::Frame::NONE
        .fill(ui.visuals().extreme_bg_color)
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::same(16))
        .show(ui, add)
        .inner
}

/// Tarjeta compacta «ETIQUETA / valor» para los metadatos del documento.
fn meta_card(ui: &mut egui::Ui, label: &str, value: &str) {
    egui::Frame::NONE
        .fill(ui.visuals().extreme_bg_color)
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(12, 8))
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(label).small().weak());
                ui.label(egui::RichText::new(value).strong());
            });
        });
}

/// Etiqueta de sección: versalita atenuada (guía de estilo).
fn section_label(ui: &mut egui::Ui, text: &str) {
    ui.label(
        egui::RichText::new(text.to_uppercase())
            .small()
            .weak()
            .strong(),
    );
}

/// Chip de etiqueta con borde redondeado.
fn chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::NONE
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::symmetric(8, 3))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(format!("#{text}")).small());
        });
}

/// Control segmentado: pestañas mutuamente excluyentes (Editar / Vista).
fn segmented(ui: &mut egui::Ui, labels: &[&str], sel: &mut DocMode) {
    let accent = ui.visuals().hyperlink_color;
    egui::Frame::NONE
        .stroke(egui::Stroke::new(
            1.0,
            ui.visuals().widgets.noninteractive.bg_stroke.color,
        ))
        .corner_radius(egui::CornerRadius::same(4))
        .inner_margin(egui::Margin::same(2))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                for (i, l) in labels.iter().enumerate() {
                    let mode = if i == 0 { DocMode::Edit } else { DocMode::Read };
                    let active = *sel == mode;
                    let btn = if active {
                        egui::Button::new(
                            egui::RichText::new(*l).color(theme::contrast_on(accent)),
                        )
                        .fill(accent)
                        .corner_radius(egui::CornerRadius::same(3))
                    } else {
                        egui::Button::new(egui::RichText::new(*l))
                            .frame(false)
                    };
                    if ui.add(btn).clicked() {
                        *sel = mode;
                    }
                }
            });
        });
}

/// Fila de navegación a ancho completo, con realce de acento si está activa.
fn nav_item(ui: &mut egui::Ui, selected: bool, glyph: &str, label: &str) -> bool {
    let w = ui.available_width();
    ui.add(
        egui::Button::selectable(selected, format!("  {glyph}   {label}"))
            .min_size(egui::vec2(w, 34.0)),
    )
    .clicked()
}

/// Texto del fragmento de búsqueda con el término resaltado.
fn highlight(ui: &egui::Ui, text: &str, at: (usize, usize)) -> LayoutJob {
    let body = egui::FontId::new(13.5, egui::FontFamily::Monospace);
    let fg = ui.visuals().text_color();
    let mut job = LayoutJob::default();
    let plain = TextFormat {
        font_id: body.clone(),
        color: fg,
        ..Default::default()
    };
    let (a, b) = at;
    if a < b && b <= text.len() && text.is_char_boundary(a) && text.is_char_boundary(b) {
        let accent = ui.visuals().hyperlink_color;
        let hl = TextFormat {
            font_id: body.clone(),
            color: theme::contrast_on(accent),
            background: accent,
            ..Default::default()
        };
        job.append(&text[..a], 0.0, plain.clone());
        job.append(&text[a..b], 0.0, hl);
        job.append(&text[b..], 0.0, plain);
    } else {
        job.append(text, 0.0, plain);
    }
    job
}

fn empty_state(ui: &mut egui::Ui, msg: &str) {
    ui.centered_and_justified(|ui| {
        ui.label(egui::RichText::new(msg).heading().weak());
    });
}

/// Botón de icono sin marco, tamaño uniforme.
fn icon_button(ui: &mut egui::Ui, glyph: &str, tip: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(glyph).size(16.0))
            .frame(false)
            .min_size(egui::vec2(32.0, 28.0)),
    )
    .on_hover_text(tip)
}

/// Fila «muestra de color + etiqueta» del editor de tema.
fn color_row(ui: &mut egui::Ui, label: &str, rgb: &mut theme::Rgb) -> bool {
    ui.horizontal(|ui| {
        let changed = ui.color_edit_button_srgb(rgb).changed();
        ui.label(label);
        changed
    })
    .inner
}

// ---- utilidades de datos ------------------------------------------------

fn file_name(p: &Path) -> String {
    p.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn file_stem(p: &Path) -> String {
    p.file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn file_mtime(p: &Path) -> Option<SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

/// Tiempo relativo legible («hace 5 min», «hace 2 h», «hace 3 d»).
fn rel_time(t: Option<SystemTime>) -> String {
    let Some(t) = t else {
        return "—".to_owned();
    };
    let Ok(d) = SystemTime::now().duration_since(t) else {
        return "ahora".to_owned();
    };
    let s = d.as_secs();
    if s < 60 {
        "hace un momento".to_owned()
    } else if s < 3600 {
        format!("hace {} min", s / 60)
    } else if s < 86400 {
        format!("hace {} h", s / 3600)
    } else {
        format!("hace {} d", s / 86400)
    }
}

/// Hora del día (UTC) `HH:MM:SS` para el registro, sin dependencias.
fn now_hms() -> String {
    let secs = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let d = secs % 86_400;
    format!("{:02}:{:02}:{:02}", d / 3600, (d % 3600) / 60, d % 60)
}

/// Carpetas y notas `.md` de `dir`, carpetas primero y orden alfabético.
fn entries(dir: &Path) -> Vec<PathBuf> {
    let mut v: Vec<PathBuf> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            !name.starts_with('.')
        })
        .filter(|p| p.is_dir() || p.extension().is_some_and(|x| x == "md"))
        .collect();
    v.sort_by(|a, b| (!a.is_dir(), a.file_name()).cmp(&(!b.is_dir(), b.file_name())));
    v
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
