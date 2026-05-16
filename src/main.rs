//! RustNotes — notas Markdown con espacios, carpetas y temas.

mod app;
mod config;
mod doc;
mod ext;
mod search;
mod theme;

use eframe::egui;

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 760.0])
            .with_min_inner_size([760.0, 480.0])
            .with_title("RustNotes"),
        ..Default::default()
    };
    eframe::run_native(
        "RustNotes",
        options,
        Box::new(|_cc| Ok(Box::new(app::RustNotes::new()))),
    )
}
