//! Temas (claro / oscuro / sistema) y estilo visual limpio y moderno.

use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeChoice {
    Light,
    Dark,
    #[default]
    System,
}

impl ThemeChoice {
    pub const ALL: [ThemeChoice; 3] = [ThemeChoice::Light, ThemeChoice::Dark, ThemeChoice::System];

    pub fn label(self) -> &'static str {
        match self {
            ThemeChoice::Light => "Claro",
            ThemeChoice::Dark => "Oscuro",
            ThemeChoice::System => "Sistema",
        }
    }

    fn preference(self) -> egui::ThemePreference {
        match self {
            ThemeChoice::Light => egui::ThemePreference::Light,
            ThemeChoice::Dark => egui::ThemePreference::Dark,
            ThemeChoice::System => egui::ThemePreference::System,
        }
    }
}

/// Fija el tema (barato, se puede llamar cada frame; "Sistema" se sigue solo).
pub fn apply(ctx: &egui::Context, theme: ThemeChoice) {
    ctx.set_theme(theme.preference());
}

/// Instala el estilo (espaciado, tipografía y esquinas redondeadas). Una vez.
pub fn install_style(ctx: &egui::Context) {
    use egui::{CornerRadius, FontFamily, FontId, Margin, TextStyle};

    let mut style = (*ctx.global_style()).clone();

    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(21.0, FontFamily::Proportional),
        ),
        (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
        (
            TextStyle::Button,
            FontId::new(14.0, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(13.5, FontFamily::Monospace),
        ),
        (
            TextStyle::Small,
            FontId::new(11.5, FontFamily::Proportional),
        ),
    ]
    .into();

    let s = &mut style.spacing;
    s.item_spacing = egui::vec2(8.0, 8.0);
    s.button_padding = egui::vec2(10.0, 6.0);
    s.window_margin = Margin::same(12);
    s.menu_margin = Margin::same(6);
    s.indent = 18.0;

    style.visuals.window_corner_radius = CornerRadius::same(10);
    style.visuals.menu_corner_radius = CornerRadius::same(8);
    for w in [
        &mut style.visuals.widgets.noninteractive,
        &mut style.visuals.widgets.inactive,
        &mut style.visuals.widgets.hovered,
        &mut style.visuals.widgets.active,
        &mut style.visuals.widgets.open,
    ] {
        w.corner_radius = CornerRadius::same(7);
    }

    ctx.set_global_style(style);
}
