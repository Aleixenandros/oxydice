//! Aplicación: espacios, árbol de carpetas/notas, editor y vista previa.

use crate::config::Config;
use crate::ext::sync::SyncState;
use crate::ext::Registry;
use crate::theme::{self, Palette, CUSTOM_ID, SYSTEM_ID};
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::path::{Path, PathBuf};

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

    /// Diálogo de renombrado con el nombre actual precargado.
    fn rename(target: PathBuf) -> Self {
        let parent = target
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_default();
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
    status: String,
    md_cache: CommonMarkCache,
    show_sidebar: bool,
    show_preview: bool,
    style_installed: bool,
    dialog: Option<Dialog>,
    pending_delete: Option<PathBuf>,
    show_prefs: bool,
    prefs_tab: PrefsTab,
    registry: Registry,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrefsTab {
    Appearance,
    Extensions,
    Backup,
    About,
}

impl PrefsTab {
    const ALL: [PrefsTab; 4] = [
        PrefsTab::Appearance,
        PrefsTab::Extensions,
        PrefsTab::Backup,
        PrefsTab::About,
    ];

    fn label(self) -> &'static str {
        match self {
            PrefsTab::Appearance => "Apariencia",
            PrefsTab::Extensions => "Extensiones",
            PrefsTab::Backup => "Copia de seguridad",
            PrefsTab::About => "Acerca de",
        }
    }
}

impl RustNotes {
    pub fn new() -> Self {
        Self {
            config: Config::load(),
            current: None,
            buffer: String::new(),
            dirty: false,
            status: String::new(),
            md_cache: CommonMarkCache::default(),
            show_sidebar: true,
            show_preview: true,
            style_installed: false,
            dialog: None,
            pending_delete: None,
            show_prefs: false,
            prefs_tab: PrefsTab::Appearance,
            registry: Registry::builtin(),
        }
    }

    // ---- acciones sobre datos -------------------------------------------

    fn open_note(&mut self, path: PathBuf) {
        match std::fs::read_to_string(&path) {
            Ok(content) => {
                self.buffer = content;
                self.current = Some(path);
                self.dirty = false;
                self.status = "Nota abierta".to_owned();
            }
            Err(e) => self.status = format!("Error al leer: {e}"),
        }
    }

    fn save(&mut self) {
        let Some(path) = self.current.clone() else {
            return;
        };
        match std::fs::write(&path, &self.buffer) {
            Ok(()) => {
                self.dirty = false;
                self.status = "Guardado".to_owned();
                if self.config.backup_on_save {
                    self.backup_now(true);
                }
            }
            Err(e) => self.status = format!("Error al guardar: {e}"),
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
            self.current = None;
            self.buffer.clear();
            self.config.save();
        }
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
                self.current = None;
                self.buffer.clear();
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
                    Ok(()) => self.status = format!("Carpeta creada: {name}"),
                    Err(e) => self.status = format!("Error: {e}"),
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
                    Err(e) => self.status = format!("Error: {e}"),
                }
            }
            DialogKind::Rename(old) => {
                // Conserva la extensión .md de las notas aunque se omita.
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
                    self.status = format!("Ya existe «{leaf}»");
                    return;
                }
                match std::fs::rename(old, &new_path) {
                    Ok(()) => {
                        self.remap_current(old, &new_path);
                        self.status = format!("Renombrado a {leaf}");
                    }
                    Err(e) => self.status = format!("Error al renombrar: {e}"),
                }
            }
        }
    }

    /// Reapunta la nota abierta tras renombrar su archivo o una carpeta padre.
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
                self.status = "Eliminado".to_owned();
            }
            Err(e) => self.status = format!("Error al eliminar: {e}"),
        }
    }

    // ---- UI --------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar").show_inside(ui, |ui| {
            ui.add_space(6.0);
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

                ui.add_space(8.0);
                let crumb = match (self.config.selected_space(), &self.current) {
                    (Some(sp), Some(note)) => {
                        let rel = note.strip_prefix(sp).unwrap_or(note);
                        format!("{}  ›  {}", file_name(sp), rel.to_string_lossy())
                    }
                    (Some(sp), None) => file_name(sp),
                    _ => "Sin espacio".to_owned(),
                };
                ui.label(
                    egui::RichText::new(crumb)
                        .color(ui.visuals().weak_text_color()),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;

                    if icon_button(ui, "⚙", "Preferencias").clicked() {
                        self.show_prefs = true;
                    }

                    // Indicador de estado de sincronización (guía §6).
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
                            ui.selectable_value(
                                &mut sel,
                                CUSTOM_ID.to_owned(),
                                "Personalizado",
                            );
                        });
                    if sel != self.config.theme {
                        self.config.theme = sel;
                        self.config.save();
                    }

                    let eye = if self.show_preview { "◑" } else { "○" };
                    if icon_button(ui, eye, "Vista previa").clicked() {
                        self.show_preview = !self.show_preview;
                    }

                    let can_save = self.current.is_some() && self.dirty;
                    let accent = ui.visuals().hyperlink_color;
                    let save = egui::Button::new(
                        egui::RichText::new("Guardar").color(theme::contrast_on(accent)),
                    )
                    .fill(accent);
                    if ui.add_enabled(can_save, save).clicked() {
                        self.save();
                    }
                    if !self.status.is_empty() {
                        let txt = if self.dirty {
                            format!("{}  ·  sin guardar", self.status)
                        } else {
                            self.status.clone()
                        };
                        ui.label(egui::RichText::new(txt).small().weak());
                    }
                });
            });
            ui.add_space(6.0);
            ui.separator();
        });
    }

    fn sidebar(&mut self, ui: &mut egui::Ui) {
        ui.add_space(6.0);

        // Selector de espacio
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
            self.current = None;
            self.buffer.clear();
            self.config.save();
        }

        if ui.button("＋  Añadir espacio").clicked() {
            self.add_space();
        }

        ui.add_space(8.0);

        let root = self.config.selected_space().cloned();
        if let Some(root) = root {
            ui.horizontal(|ui| {
                if ui.button("＋  Nota").clicked() {
                    self.dialog = Some(Dialog::new(DialogKind::NewNote, root.clone()));
                }
                if ui.button("＋  Carpeta").clicked() {
                    self.dialog = Some(Dialog::new(DialogKind::NewFolder, root.clone()));
                }
            });

            ui.add_space(6.0);
            ui.label(egui::RichText::new("NOTAS").small().weak());
            ui.separator();

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.tree(ui, &root);
                });

            ui.separator();
            ui.label(
                egui::RichText::new(root.to_string_lossy().into_owned())
                    .small()
                    .weak(),
            );
        } else {
            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                ui.label(egui::RichText::new("No hay espacios todavía").weak());
                ui.add_space(6.0);
                if ui.button("Añadir un espacio").clicked() {
                    self.add_space();
                }
            });
        }
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
                let row = egui::Button::selectable(selected, file_stem(&path));
                let resp = ui.add_sized([ui.available_width(), 26.0], row);
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
                }
            }
        }
    }

    fn center(&mut self, ui: &mut egui::Ui) {
        if self.config.selected_space().is_none() {
            ui.centered_and_justified(|ui| {
                ui.label(
                    egui::RichText::new("Añade un espacio para empezar a tomar notas")
                        .heading()
                        .weak(),
                );
            });
            return;
        }
        if self.current.is_some() {
            let resp = ui.add_sized(
                ui.available_size(),
                egui::TextEdit::multiline(&mut self.buffer)
                    .code_editor()
                    .frame(egui::Frame::NONE)
                    .margin(egui::Margin::same(16))
                    .desired_width(f32::INFINITY),
            );
            if resp.changed() {
                self.dirty = true;
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new("Selecciona o crea una nota").weak());
            });
        }
    }

    fn preview(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.label(egui::RichText::new("VISTA PREVIA").small().weak());
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                CommonMarkViewer::new().show(ui, &mut self.md_cache, &self.buffer);
            });
    }

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
                ui.add_space(6.0);
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
                ui.label(
                    egui::RichText::new(file_name(&path))
                        .small()
                        .weak(),
                );
                ui.add_space(10.0);
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

    // ---- preferencias ---------------------------------------------------

    fn preferences_window(&mut self, ui: &mut egui::Ui) {
        if !self.show_prefs {
            return;
        }
        let mut open = true;
        egui::Window::new("Preferencias")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_size([580.0, 420.0])
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ui.ctx(), |ui| {
                egui::Panel::left("prefs_nav")
                    .resizable(false)
                    .default_size(170.0)
                    .show_inside(ui, |ui| {
                        ui.add_space(4.0);
                        for tab in PrefsTab::ALL {
                            ui.selectable_value(&mut self.prefs_tab, tab, tab.label());
                        }
                    });
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| match self.prefs_tab {
                        PrefsTab::Appearance => self.prefs_appearance(ui),
                        PrefsTab::Extensions => self.prefs_extensions(ui),
                        PrefsTab::Backup => self.prefs_backup(ui),
                        PrefsTab::About => self.prefs_about(ui),
                    });
                });
            });
        if !open {
            self.show_prefs = false;
        }
    }

    fn prefs_appearance(&mut self, ui: &mut egui::Ui) {
        ui.heading("Apariencia");
        ui.add_space(12.0);

        ui.label(egui::RichText::new("TEMA").small().weak());
        ui.add_space(6.0);
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
            if ui.selectable_label(sel == CUSTOM_ID, "Personalizado").clicked() {
                sel = CUSTOM_ID.to_owned();
            }
        });
        if sel != self.config.theme {
            self.config.theme = sel;
            self.config.save();
        }

        ui.add_space(10.0);
        ui.horizontal(|ui| {
            if ui.button("Exportar tema…").clicked() {
                self.export_theme(ui.ctx());
            }
            if ui.button("Importar tema…").clicked() {
                self.import_theme();
            }
        });
        if !self.status.is_empty() {
            ui.add_space(4.0);
            ui.label(egui::RichText::new(&self.status).small().weak());
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        ui.label(egui::RichText::new("TEMA PERSONALIZADO").small().weak());
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new("Editar un color selecciona el tema «Personalizado».")
                .small()
                .weak(),
        );
        ui.add_space(8.0);

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

        ui.add_space(18.0);
        ui.separator();
        ui.add_space(12.0);

        ui.label(egui::RichText::new("INTERFAZ").small().weak());
        ui.add_space(6.0);
        ui.label("Escala");
        let mut scale = self.config.ui_scale;
        if ui
            .add(egui::Slider::new(&mut scale, 0.8..=1.6).step_by(0.05))
            .changed()
        {
            self.config.ui_scale = scale;
            self.config.save();
        }
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
        self.status = match serde_json::to_string_pretty(&palette) {
            Ok(json) => match std::fs::write(&path, json) {
                Ok(()) => format!("Tema exportado: {}", path.display()),
                Err(e) => format!("Error al exportar: {e}"),
            },
            Err(e) => format!("Error al serializar: {e}"),
        };
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
                self.status = "Tema importado y aplicado".to_owned();
            }
            None => self.status = "Archivo de tema no válido".to_owned(),
        }
    }

    fn prefs_backup(&mut self, ui: &mut egui::Ui) {
        ui.heading("Copia de seguridad");
        ui.add_space(8.0);

        let dest = self
            .config
            .backup_dir
            .as_ref()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| "(sin configurar)".to_owned());
        ui.label(format!("Carpeta de copias: {dest}"));
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

        ui.add_space(14.0);
        let ready = self.config.backup_dir.is_some() && self.config.selected_space().is_some();
        if ui
            .add_enabled(ready, egui::Button::new("Crear copia ahora"))
            .clicked()
        {
            self.backup_now(false);
        }
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "Copia el espacio activo (omitiendo archivos ocultos) a una \
                 subcarpeta con marca de tiempo.",
            )
            .small()
            .weak(),
        );
    }

    fn prefs_about(&mut self, ui: &mut egui::Ui) {
        ui.heading("RustNotes");
        ui.add_space(8.0);
        ui.label(format!("Versión {}", env!("CARGO_PKG_VERSION")));
        ui.label(format!("Autor: {}", env!("CARGO_PKG_AUTHORS")));
        ui.label("Licencia: MIT");
        ui.add_space(8.0);
        ui.hyperlink_to(
            "Repositorio en GitHub",
            "https://github.com/Aleixenandros/RustNotes",
        );
        ui.add_space(8.0);
        ui.label(egui::RichText::new("Notas Markdown con espacios, carpetas y temas.").weak());
    }

    fn prefs_extensions(&mut self, ui: &mut egui::Ui) {
        ui.heading("Extensiones");
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "Se compilan en el binario. Añadir una es implementar su \
                 trait y registrarla en el Registry.",
            )
            .small()
            .weak(),
        );
        ui.add_space(12.0);

        ui.label(egui::RichText::new("REGISTRADAS").small().weak());
        ui.add_space(4.0);
        for (kind, id, name, detail) in self.registry.listing() {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("[{}]", kind.label()))
                        .small()
                        .weak(),
                );
                ui.label(name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(detail).small().weak());
                    ui.label(egui::RichText::new(id).small().weak());
                });
            });
        }

        ui.add_space(16.0);
        ui.separator();
        ui.add_space(12.0);

        ui.label(egui::RichText::new("SINCRONIZACIÓN").small().weak());
        ui.add_space(6.0);
        let mut sel = self.config.sync.clone();
        let providers: Vec<(&'static str, &'static str)> = self
            .registry
            .syncs()
            .iter()
            .map(|s| (s.id(), s.name()))
            .collect();
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
        ui.add_space(6.0);
        ui.label(format!("Estado: {}", state.label()));
        ui.add_space(8.0);
        let enabled = !matches!(state, SyncState::Disabled);
        if ui
            .add_enabled(enabled, egui::Button::new("Sincronizar ahora"))
            .clicked()
        {
            self.status = match self.registry.sync_now(&self.config.sync) {
                Ok(()) => "Sincronización completada".to_owned(),
                Err(e) => format!("Error de sincronización: {e}"),
            };
        }
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(
                "La sincronización real (Git, S3, Drive) es backlog; la \
                 interfaz ya admite añadir backends como extensión.",
            )
            .small()
            .weak(),
        );
    }

    fn backup_now(&mut self, quiet: bool) {
        let Some(dest) = self.config.backup_dir.clone() else {
            if !quiet {
                self.status = "Configura una carpeta de copias en Preferencias".to_owned();
            }
            return;
        };
        let Some(space) = self.config.selected_space().cloned() else {
            if !quiet {
                self.status = "No hay espacio activo".to_owned();
            }
            return;
        };
        let secs = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let target = dest.join(format!("rustnotes-{}-{secs}", file_name(&space)));
        match copy_dir(&space, &target) {
            Ok(()) => self.status = format!("Copia creada: {}", target.display()),
            Err(e) => self.status = format!("Error en la copia: {e}"),
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
        let palette = self.registry.resolve_theme(
            &self.config.theme,
            ui.ctx(),
            &self.config.custom_theme,
        );
        theme::apply(ui.ctx(), &palette);
        if (ui.ctx().zoom_factor() - self.config.ui_scale).abs() > f32::EPSILON {
            ui.ctx().set_zoom_factor(self.config.ui_scale);
        }

        self.top_bar(ui);

        if self.show_sidebar {
            egui::Panel::left("sidebar")
                .resizable(true)
                .default_size(266.0)
                .show_inside(ui, |ui| self.sidebar(ui));
        }
        if self.show_preview && self.current.is_some() {
            egui::Panel::right("preview")
                .resizable(true)
                .default_size(440.0)
                .show_inside(ui, |ui| self.preview(ui));
        }
        let canvas = egui::Frame::central_panel(ui.style())
            .fill(egui::Color32::from_rgb(
                palette.bg[0],
                palette.bg[1],
                palette.bg[2],
            ))
            .inner_margin(egui::Margin::ZERO);
        egui::CentralPanel::default()
            .frame(canvas)
            .show_inside(ui, |ui| self.center(ui));

        self.dialog_window(ui);
        self.confirm_window(ui);
        self.preferences_window(ui);
    }
}

// ---- utilidades ---------------------------------------------------------

/// Fila «muestra de color + etiqueta» del editor de tema.
fn color_row(ui: &mut egui::Ui, label: &str, rgb: &mut theme::Rgb) -> bool {
    ui.horizontal(|ui| {
        let changed = ui.color_edit_button_srgb(rgb).changed();
        ui.label(label);
        changed
    })
    .inner
}

/// Botón de icono sin marco, tamaño uniforme: aspecto limpio en la barra.
fn icon_button(ui: &mut egui::Ui, glyph: &str, tip: &str) -> egui::Response {
    ui.add(
        egui::Button::new(egui::RichText::new(glyph).size(16.0))
            .frame(false)
            .min_size(egui::vec2(32.0, 28.0)),
    )
    .on_hover_text(tip)
}

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
