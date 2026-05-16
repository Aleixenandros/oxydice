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

// Tema Claro «Technical Utility» — guía de estilo §2.2.
// Documento blanco (#ffffff) sobre contenedores gris claro (#f2f4f6).
pub const LIGHT: Palette = Palette {
    dark: false,
    accent: [206, 65, 43],   // #ce412b naranja Rust
    bg: [255, 255, 255],     // #ffffff superficie de documento/editor
    surface: [242, 244, 246], // #f2f4f6 contenedores/paneles
    text: [32, 33, 36],      // #202124
    muted: [95, 99, 104],    // #5f6368
    border: [218, 220, 224], // #dadce0
};

// Tema Oscuro «Developer-Centric» — guía de estilo §2.1.
pub const DARK: Palette = Palette {
    dark: true,
    accent: [206, 65, 43],   // #ce412b naranja Rust
    bg: [19, 19, 19],        // #131313 fondo de superficie (editor)
    surface: [30, 30, 30],   // #1e1e1e contenedores/paneles
    text: [224, 224, 224],   // #e0e0e0
    muted: [160, 160, 160],  // #a0a0a0
    border: [57, 57, 57],    // #393939
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

/// Identificadores especiales de tema (no son presets de paleta fija).
/// Sus nombres coinciden con las variantes del enum antiguo para que las
/// configuraciones existentes sigan cargando sin perder datos.
pub const SYSTEM_ID: &str = "System";
pub const CUSTOM_ID: &str = "Custom";

/// Paleta que sigue al tema del sistema operativo.
///
/// Si el SO declara su preferencia se respeta. Cuando no se puede detectar
/// (común en Linux/X11), se cae a **claro** —no a oscuro— para no confundir
/// «Sistema» con un tema oscuro forzado: el usuario siempre puede fijar
/// Claro u Oscuro de forma explícita.
pub fn system_palette(ctx: &egui::Context) -> Palette {
    match ctx.system_theme() {
        Some(egui::Theme::Dark) => DARK,
        _ => LIGHT,
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
    // Bloques de código con fondo distinto al cuerpo (guía §4.2).
    v.code_bg_color = mix(p.bg, p.text, if p.dark { 0.10 } else { 0.06 });

    v.hyperlink_color = accent;
    v.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(
        p.accent[0],
        p.accent[1],
        p.accent[2],
        if p.dark { 80 } else { 60 },
    );
    v.selection.stroke = Stroke::new(1.0, accent);

    // Bordes ligeramente redondeados, 4px (guía §1).
    let r = CornerRadius::same(4);
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
    // Inputs y botones secundarios con borde sutil de 1px (guía §4.3).
    w.inactive.bg_stroke = Stroke::new(1.0, border);
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

/// Empotra las fuentes de la guía de estilo §3 (Inter, Inter SemiBold y
/// JetBrains Mono), conservando las fuentes por defecto de egui como
/// respaldo para los glifos de iconos. Llamar una vez, antes del estilo.
pub fn install_fonts(ctx: &egui::Context) {
    use egui::{FontData, FontDefinitions, FontFamily};
    use std::sync::Arc;

    let mut fonts = FontDefinitions::default();

    fonts.font_data.insert(
        "Inter".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "InterSemiBold".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/Inter-SemiBold.ttf"
        ))),
    );
    fonts.font_data.insert(
        "JetBrainsMono".to_owned(),
        Arc::new(FontData::from_static(include_bytes!(
            "../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );

    // Inter/JetBrains Mono primero; las fuentes de egui quedan detrás como
    // respaldo para símbolos e iconos que Inter no cubre.
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "Inter".to_owned());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, "JetBrainsMono".to_owned());

    // Familia con nombre para encabezados (semibold) + respaldo proporcional.
    let mut heading = vec!["InterSemiBold".to_owned()];
    if let Some(prop) = fonts.families.get(&FontFamily::Proportional) {
        heading.extend(prop.iter().cloned());
    }
    fonts
        .families
        .insert(FontFamily::Name("Heading".into()), heading);

    ctx.set_fonts(fonts);
}

/// Instala tipografía y espaciado (no depende del tema). Llamar una vez.
pub fn install_style(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, Margin, TextStyle};

    ctx.all_styles_mut(|style| {
        // Jerarquía tipográfica (guía §3): Inter (UI), Inter SemiBold
        // (encabezados), JetBrains Mono (editor).
        let heading = FontFamily::Name("Heading".into());
        style.text_styles = [
            (TextStyle::Heading, FontId::new(24.0, heading.clone())),
            (TextStyle::Body, FontId::new(15.0, FontFamily::Proportional)),
            (TextStyle::Button, FontId::new(14.0, FontFamily::Proportional)),
            (TextStyle::Monospace, FontId::new(13.0, FontFamily::Monospace)),
            (TextStyle::Small, FontId::new(12.5, FontFamily::Proportional)),
        ]
        .into();

        let s = &mut style.spacing;
        // Densidad de información media-alta en escritorio (guía §1).
        s.item_spacing = egui::vec2(7.0, 7.0);
        s.button_padding = egui::vec2(10.0, 6.0);
        s.window_margin = Margin::same(14);
        s.menu_margin = Margin::same(6);
        s.indent = 18.0;
        s.scroll = egui::style::ScrollStyle::thin();
    });
}
