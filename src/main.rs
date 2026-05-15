//! RustNotes — versión mínima viable.
//!
//! Abre una carpeta (vault), lista las notas `.md`, permite editarlas con
//! guardado a disco y muestra una vista previa Markdown en vivo.

use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 720.0]),
        ..Default::default()
    };
    eframe::run_native(
        "RustNotes",
        options,
        Box::new(|_cc| Ok(Box::new(RustNotes::default()))),
    )
}

#[derive(Default)]
struct RustNotes {
    vault: Option<PathBuf>,
    notes: Vec<PathBuf>,
    current: Option<PathBuf>,
    buffer: String,
    dirty: bool,
    status: String,
    md_cache: CommonMarkCache,
}

impl RustNotes {
    fn open_vault(&mut self, dir: PathBuf) {
        let mut notes: Vec<PathBuf> = WalkDir::new(&dir)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_file())
            .map(walkdir::DirEntry::into_path)
            .filter(|p| p.extension().is_some_and(|x| x == "md"))
            .collect();
        notes.sort();
        self.status = format!("{} nota(s) en el vault", notes.len());
        self.notes = notes;
        self.vault = Some(dir);
        self.current = None;
        self.buffer.clear();
        self.dirty = false;
    }

    fn open_note(&mut self, path: PathBuf) {
        match fs::read_to_string(&path) {
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
        if let Some(path) = &self.current {
            match fs::write(path, &self.buffer) {
                Ok(()) => {
                    self.dirty = false;
                    self.status = "Guardado".to_owned();
                }
                Err(e) => self.status = format!("Error al guardar: {e}"),
            }
        }
    }
}

impl eframe::App for RustNotes {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        egui::Panel::top("top").show_inside(ui, |ui| {
            ui.horizontal(|ui| {
                if ui.button("📂 Abrir vault").clicked() {
                    if let Some(dir) = rfd::FileDialog::new().pick_folder() {
                        self.open_vault(dir);
                    }
                }
                let can_save = self.current.is_some() && self.dirty;
                if ui
                    .add_enabled(can_save, egui::Button::new("💾 Guardar"))
                    .clicked()
                {
                    self.save();
                }
                ui.separator();
                let dirty = if self.dirty { " (sin guardar)" } else { "" };
                ui.label(format!("{}{dirty}", self.status));
            });
        });

        egui::Panel::left("notes")
            .resizable(true)
            .default_size(240.0)
            .show_inside(ui, |ui| {
                ui.heading("Notas");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let vault = self.vault.clone();
                    let mut to_open = None;
                    for note in &self.notes {
                        let label = vault
                            .as_ref()
                            .and_then(|v| note.strip_prefix(v).ok())
                            .unwrap_or(note)
                            .to_string_lossy()
                            .into_owned();
                        let selected = self.current.as_ref() == Some(note);
                        if ui.selectable_label(selected, label).clicked() {
                            to_open = Some(note.clone());
                        }
                    }
                    if let Some(p) = to_open {
                        self.open_note(p);
                    }
                });
            });

        egui::Panel::right("preview")
            .resizable(true)
            .default_size(380.0)
            .show_inside(ui, |ui| {
                ui.heading("Vista previa");
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    CommonMarkViewer::new().show(ui, &mut self.md_cache, &self.buffer);
                });
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
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
                    ui.label("Abre un vault y selecciona una nota para empezar.");
                });
            }
        });
    }
}
