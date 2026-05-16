//! Aplicación: espacios, árbol de carpetas/notas, editor y vista previa.

use crate::config::Config;
use crate::theme::{self, ThemeChoice};
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::path::{Path, PathBuf};

enum DialogKind {
    NewNote,
    NewFolder,
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

    fn title(&self) -> &'static str {
        match self.kind {
            DialogKind::NewNote => "Nueva nota",
            DialogKind::NewFolder => "Nueva carpeta",
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
    show_prefs: bool,
    prefs_tab: PrefsTab,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PrefsTab {
    Appearance,
    Backup,
    About,
}

impl PrefsTab {
    const ALL: [PrefsTab; 3] = [PrefsTab::Appearance, PrefsTab::Backup, PrefsTab::About];

    fn label(self) -> &'static str {
        match self {
            PrefsTab::Appearance => "Apariencia",
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
            show_prefs: false,
            prefs_tab: PrefsTab::Appearance,
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
        match dialog.kind {
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
        }
    }

    // ---- UI --------------------------------------------------------------

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("topbar").show_inside(ui, |ui| {
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                if ui
                    .button("▤")
                    .on_hover_text("Mostrar/ocultar barra lateral")
                    .clicked()
                {
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

                ui.separator();
                let crumb = match (self.config.selected_space(), &self.current) {
                    (Some(sp), Some(note)) => {
                        let rel = note.strip_prefix(sp).unwrap_or(note);
                        format!("{} / {}", file_name(sp), rel.to_string_lossy())
                    }
                    (Some(sp), None) => file_name(sp),
                    _ => "Sin espacio".to_owned(),
                };
                ui.label(egui::RichText::new(crumb).weak());

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("⚙").on_hover_text("Preferencias").clicked() {
                        self.show_prefs = true;
                    }

                    let mut theme = self.config.theme;
                    egui::ComboBox::from_id_salt("theme")
                        .selected_text(theme.label())
                        .show_ui(ui, |ui| {
                            for opt in ThemeChoice::ALL {
                                ui.selectable_value(&mut theme, opt, opt.label());
                            }
                        });
                    if theme != self.config.theme {
                        self.config.theme = theme;
                        self.config.save();
                    }

                    if ui
                        .selectable_label(self.show_preview, "👁")
                        .on_hover_text("Vista previa")
                        .clicked()
                    {
                        self.show_preview = !self.show_preview;
                    }

                    let can_save = self.current.is_some() && self.dirty;
                    if ui
                        .add_enabled(can_save, egui::Button::new("Guardar"))
                        .clicked()
                    {
                        self.save();
                    }
                    if !self.status.is_empty() {
                        let txt = if self.dirty {
                            format!("{} ·  sin guardar", self.status)
                        } else {
                            self.status.clone()
                        };
                        ui.label(egui::RichText::new(txt).small().weak());
                    }
                });
            });
            ui.add_space(2.0);
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
                let title = format!("🗀  {}", file_name(&path));
                let resp = egui::CollapsingHeader::new(title)
                    .id_salt(&path)
                    .show(ui, |ui| self.tree(ui, &path));
                resp.header_response.context_menu(|ui| {
                    if ui.button("＋  Nueva nota").clicked() {
                        self.dialog = Some(Dialog::new(DialogKind::NewNote, path.clone()));
                    }
                    if ui.button("＋  Nueva carpeta").clicked() {
                        self.dialog = Some(Dialog::new(DialogKind::NewFolder, path.clone()));
                    }
                });
            } else {
                let selected = self.current.as_deref() == Some(path.as_path());
                let label = format!("📄  {}", file_stem(&path));
                if ui.selectable_label(selected, label).clicked() {
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
                    if ui.button("Crear").clicked() || enter {
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
        ui.add_space(8.0);

        ui.label("Tema");
        let mut theme = self.config.theme;
        ui.horizontal(|ui| {
            for opt in ThemeChoice::ALL {
                ui.selectable_value(&mut theme, opt, opt.label());
            }
        });
        if theme != self.config.theme {
            self.config.theme = theme;
            self.config.save();
        }

        ui.add_space(14.0);
        ui.label("Escala de la interfaz");
        let mut scale = self.config.ui_scale;
        if ui
            .add(egui::Slider::new(&mut scale, 0.8..=1.6).step_by(0.05))
            .changed()
        {
            self.config.ui_scale = scale;
            self.config.save();
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
            theme::install_style(ui.ctx());
            self.style_installed = true;
        }
        theme::apply(ui.ctx(), self.config.theme);
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
        egui::CentralPanel::default().show_inside(ui, |ui| self.center(ui));

        self.dialog_window(ui);
        self.preferences_window(ui);
    }
}

// ---- utilidades ---------------------------------------------------------

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
