//! Temas: presets curados, tema personalizado y estilo moderno/minimalista.
//!
//! Un *tema* es una `Palette` (seis colores con roles claros) que se traduce
//! a `egui::Visuals`. Los presets están integrados; el tema personalizado se
//! guarda en la configuración y puede exportarse/importarse como JSON.

use eframe::egui;
use serde::{Deserialize, Serialize};

/// Color RGB serializable de forma legible en el JSON del tema.
pub type Rgb = [u8; 3];

/// Paleta de un tema. Cada color tiene un rol concreto para mantener la
/// coherencia visual independientemente del tema elegido.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Palette {
    /// `true` si el tema es oscuro (afecta a sombras y contrastes base).
    pub dark: bool,
    /// Color de acento: selección, enlaces, acción primaria, foco.
    pub accent: Rgb,
    /// Fondo de la superficie de escritura (editor) y vista previa.
    pub bg: Rgb,
    /// Fondo de paneles (barra superior, lateral, ventanas).
    pub surface: Rgb,
    /// Texto principal.
    pub text: Rgb,
    /// Texto secundario / atenuado y placeholders.
    pub muted: Rgb,
    /// Bordes, separadores y trazos sutiles.
    pub border: Rgb,
}

impl Default for Palette {
    fn default() -> Self {
        DARK
    }
}

// ---- presets ------------------------------------------------------------

pub const LIGHT: Palette = Palette {
    dark: false,
    accent: [59, 111, 224],
    bg: [255, 255, 255],
    surface: [244, 245, 247],
    text: [31, 36, 48],
    muted: [110, 116, 128],
    border: [225, 228, 234],
};

pub const DARK: Palette = Palette {
    dark: true,
    accent: [108, 142, 239],
    bg: [21, 23, 30],
    surface: [28, 31, 39],
    text: [215, 218, 224],
    muted: [138, 143, 156],
    border: [42, 46, 56],
};

pub const NORD: Palette = Palette {
    dark: true,
    accent: [136, 192, 208],
    bg: [46, 52, 64],
    surface: [59, 66, 82],
    text: [236, 239, 244],
    muted: [129, 137, 154],
    border: [67, 76, 94],
};

pub const SOLARIZED: Palette = Palette {
    dark: true,
    accent: [38, 139, 210],
    bg: [0, 43, 54],
    surface: [7, 54, 66],
    text: [147, 161, 161],
    muted: [88, 110, 117],
    border: [20, 71, 86],
};

pub const DRACULA: Palette = Palette {
    dark: true,
    accent: [189, 147, 249],
    bg: [40, 42, 54],
    surface: [33, 34, 44],
    text: [248, 248, 242],
    muted: [120, 130, 170],
    border: [68, 71, 90],
};

/// Tema seleccionado. Las variantes antiguas (`Light`, `Dark`, `System`) se
/// conservan con el mismo nombre para que las configuraciones existentes
/// sigan deserializándose sin perder los espacios del usuario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemeChoice {
    Light,
    Dark,
    #[default]
    System,
    Nord,
    Solarized,
    Dracula,
    Custom,
}

impl ThemeChoice {
    pub const ALL: [ThemeChoice; 7] = [
        ThemeChoice::System,
        ThemeChoice::Light,
        ThemeChoice::Dark,
        ThemeChoice::Nord,
        ThemeChoice::Solarized,
        ThemeChoice::Dracula,
        ThemeChoice::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            ThemeChoice::Light => "Claro",
            ThemeChoice::Dark => "Oscuro",
            ThemeChoice::System => "Sistema",
            ThemeChoice::Nord => "Nord",
            ThemeChoice::Solarized => "Solarized",
            ThemeChoice::Dracula => "Dracula",
            ThemeChoice::Custom => "Personalizado",
        }
    }

    /// Paleta efectiva del tema. `System` sigue al sistema operativo;
    /// `Custom` usa la paleta editable que se le pase.
    pub fn palette(self, ctx: &egui::Context, custom: &Palette) -> Palette {
        match self {
            ThemeChoice::Light => LIGHT,
            ThemeChoice::Dark => DARK,
            ThemeChoice::Nord => NORD,
            ThemeChoice::Solarized => SOLARIZED,
            ThemeChoice::Dracula => DRACULA,
            ThemeChoice::Custom => *custom,
            ThemeChoice::System => match ctx.system_theme() {
                Some(egui::Theme::Light) => LIGHT,
                _ => DARK,
            },
        }
    }
}

// ---- aplicación de la paleta -------------------------------------------

fn c(rgb: Rgb) -> egui::Color32 {
    egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// Mezcla lineal de dos colores (`t` = 0 → `a`, `t` = 1 → `b`).
fn mix(a: Rgb, b: Rgb, t: f32) -> egui::Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    egui::Color32::from_rgb(l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2]))
}

/// Construye `egui::Visuals` a partir de una paleta y la fija en el contexto.
/// Es barato; se puede llamar cada frame.
pub fn apply(ctx: &egui::Context, p: &Palette) {
    use egui::{CornerRadius, Stroke};

    let mut v = if p.dark {
        egui::Visuals::dark()
    } else {
        egui::Visuals::light()
    };

    let accent = c(p.accent);
    let text = c(p.text);
    let muted = c(p.muted);
    let border = c(p.border);
    // Tono hacia el texto: hover y separadores suaves sin líneas duras.
    let hover = mix(p.surface, p.text, if p.dark { 0.10 } else { 0.06 });
    let faint = mix(p.surface, p.text, if p.dark { 0.05 } else { 0.03 });

    v.dark_mode = p.dark;
    v.override_text_color = Some(text);
    v.window_fill = c(p.surface);
    v.panel_fill = c(p.surface);
    v.extreme_bg_color = c(p.bg); // fondo del editor de texto
    v.faint_bg_color = faint;
    v.code_bg_color = mix(p.bg, p.text, 0.06);

    v.hyperlink_color = accent;
    v.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(
        p.accent[0],
        p.accent[1],
        p.accent[2],
        if p.dark { 80 } else { 60 },
    );
    v.selection.stroke = Stroke::new(1.0, accent);

    let r = CornerRadius::same(8);
    v.window_corner_radius = r;
    v.menu_corner_radius = r;
    v.window_stroke = Stroke::new(1.0, border);
    v.popup_shadow.color = egui::Color32::from_black_alpha(if p.dark { 90 } else { 40 });
    v.window_shadow.color = v.popup_shadow.color;

    let w = &mut v.widgets;

    // No interactivo: textos, etiquetas y separadores (trazo = borde sutil).
    w.noninteractive.bg_fill = c(p.surface);
    w.noninteractive.weak_bg_fill = c(p.surface);
    w.noninteractive.bg_stroke = Stroke::new(1.0, border);
    w.noninteractive.fg_stroke = Stroke::new(1.0, muted);
    w.noninteractive.corner_radius = r;

    // Botones y controles en reposo: planos, sin relieve.
    w.inactive.bg_fill = mix(p.surface, p.text, if p.dark { 0.08 } else { 0.05 });
    w.inactive.weak_bg_fill = mix(p.surface, p.text, if p.dark { 0.06 } else { 0.04 });
    w.inactive.bg_stroke = Stroke::NONE;
    w.inactive.fg_stroke = Stroke::new(1.0, text);
    w.inactive.corner_radius = r;

    // Hover: ligero realce con tinte del texto.
    w.hovered.bg_fill = hover;
    w.hovered.weak_bg_fill = hover;
    w.hovered.bg_stroke = Stroke::new(1.0, mix(p.surface, p.accent, 0.4));
    w.hovered.fg_stroke = Stroke::new(1.0, text);
    w.hovered.corner_radius = r;
    w.hovered.expansion = 0.0;

    // Activo / pulsado: acento.
    w.active.bg_fill = accent;
    w.active.weak_bg_fill = accent;
    w.active.bg_stroke = Stroke::new(1.0, accent);
    w.active.fg_stroke = Stroke::new(1.0, on_accent(p.accent));
    w.active.corner_radius = r;
    w.active.expansion = 0.0;

    // Abierto (combos/menús desplegados).
    w.open.bg_fill = hover;
    w.open.weak_bg_fill = hover;
    w.open.bg_stroke = Stroke::new(1.0, border);
    w.open.fg_stroke = Stroke::new(1.0, text);
    w.open.corner_radius = r;

    v.clip_rect_margin = 2.0;

    // Escribe en los estilos claro y oscuro: la paleta los controla por
    // completo, así un cambio de tema del sistema en caliente no rompe nada.
    ctx.all_styles_mut(|s| s.visuals = v.clone());
}

/// Color de texto legible sobre el acento (blanco o casi negro según luma).
fn on_accent(rgb: Rgb) -> egui::Color32 {
    contrast_on(c(rgb))
}

/// Texto legible sobre un color de fondo dado.
pub fn contrast_on(bg: egui::Color32) -> egui::Color32 {
    let luma = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if luma > 150.0 {
        egui::Color32::from_rgb(20, 22, 28)
    } else {
        egui::Color32::WHITE
    }
}

/// Instala tipografía y espaciado (no depende del tema). Llamar una vez.
pub fn install_style(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, Margin, TextStyle};

    ctx.all_styles_mut(|style| {
        style.text_styles = [
            (TextStyle::Heading, FontId::new(22.0, FontFamily::Proportional)),
            (TextStyle::Body, FontId::new(14.5, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.5, FontFamily::Monospace)),
            (TextStyle::Small, FontId::new(11.5, FontFamily::Proportional)),
        ]
        .into();

        let s = &mut style.spacing;
        s.item_spacing = egui::vec2(8.0, 8.0);
        s.button_padding = egui::vec2(12.0, 7.0);
        s.window_margin = Margin::same(16);
        s.menu_margin = Margin::same(8);
        s.indent = 18.0;
        s.scroll = egui::style::ScrollStyle::thin();
    });
}
