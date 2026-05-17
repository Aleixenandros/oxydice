//! Temas: presets curados, tema personalizado y traducción a variables CSS.
//!
//! Un *tema* es una [`Palette`] (seis colores con roles claros). El core no
//! conoce ninguna tecnología de UI: en lugar de construir estilos de un
//! framework concreto, expone la paleta y un mapa de **custom properties
//! CSS** que el frontend aplica en `:root`. Los colores derivados (hover,
//! superficies sutiles, selección, sombra) se calculan aquí para que la
//! estética sea idéntica en escritorio y móvil y no haya colores fijos
//! dispersos por la interfaz.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    accent: [206, 65, 43],    // #ce412b naranja Rust
    bg: [255, 255, 255],      // #ffffff superficie de documento/editor
    surface: [242, 244, 246], // #f2f4f6 contenedores/paneles
    text: [32, 33, 36],       // #202124
    muted: [95, 99, 104],     // #5f6368
    border: [218, 220, 224],  // #dadce0
};

// Tema Oscuro «Developer-Centric» — guía de estilo §2.1.
pub const DARK: Palette = Palette {
    dark: true,
    accent: [206, 65, 43],  // #ce412b naranja Rust
    bg: [19, 19, 19],       // #131313 fondo de superficie (editor)
    surface: [30, 30, 30],  // #1e1e1e contenedores/paneles
    text: [224, 224, 224],  // #e0e0e0
    muted: [160, 160, 160], // #a0a0a0
    border: [57, 57, 57],   // #393939
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
/// El frontend detecta la preferencia con `prefers-color-scheme` y pasa el
/// booleano. Cuando no se puede detectar (común en Linux/X11) llega `false`
/// y se cae a **claro** —no a oscuro— para no confundir «Sistema» con un
/// tema oscuro forzado: el usuario siempre puede fijar Claro u Oscuro.
pub fn system_palette(dark: bool) -> Palette {
    if dark {
        DARK
    } else {
        LIGHT
    }
}

// ---- color: utilidades puras -------------------------------------------

/// Mezcla lineal de dos colores (`t` = 0 → `a`, `t` = 1 → `b`).
fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round().clamp(0.0, 255.0) as u8;
    [l(a[0], b[0]), l(a[1], b[1]), l(a[2], b[2])]
}

/// Texto legible sobre un color de fondo dado (blanco o casi negro según
/// luma). Equivale al `contrast_on` del antiguo estilo egui.
pub fn contrast_on(rgb: Rgb) -> Rgb {
    let luma = 0.299 * rgb[0] as f32 + 0.587 * rgb[1] as f32 + 0.114 * rgb[2] as f32;
    if luma > 150.0 {
        [20, 22, 28]
    } else {
        [255, 255, 255]
    }
}

fn hex(rgb: Rgb) -> String {
    format!("#{:02x}{:02x}{:02x}", rgb[0], rgb[1], rgb[2])
}

fn rgba(rgb: Rgb, a: f32) -> String {
    format!("rgba({}, {}, {}, {:.3})", rgb[0], rgb[1], rgb[2], a)
}

// ---- paleta → variables CSS --------------------------------------------

/// Color de peligro (acción destructiva) — fijo, guía de estilo.
const DANGER: Rgb = [201, 74, 74];
/// Color de éxito (sincronización al día) — fijo, guía de estilo §6.
const SUCCESS: Rgb = [76, 175, 92];

/// Traduce la paleta a *custom properties* CSS. El frontend las fija en
/// `:root` y todo el aspecto deriva de aquí (sin colores codificados).
///
/// Los colores derivados replican el antiguo `theme::apply` de egui:
/// `--hover`/`--faint`/`--inactive` tiñen la superficie hacia el texto y
/// `--code-bg` tiñe el fondo del editor, con los mismos factores claro/oscuro.
pub fn css_vars(p: &Palette) -> BTreeMap<String, String> {
    let d = p.dark;
    let hover = mix(p.surface, p.text, if d { 0.10 } else { 0.06 });
    let faint = mix(p.surface, p.text, if d { 0.05 } else { 0.03 });
    let inactive = mix(p.surface, p.text, if d { 0.08 } else { 0.05 });
    let code_bg = mix(p.bg, p.text, if d { 0.10 } else { 0.06 });

    let mut v = BTreeMap::new();
    let mut set = |k: &str, val: String| {
        v.insert(k.to_owned(), val);
    };
    set("--accent", hex(p.accent));
    set("--accent-contrast", hex(contrast_on(p.accent)));
    set("--bg", hex(p.bg));
    set("--surface", hex(p.surface));
    set("--text", hex(p.text));
    set("--muted", hex(p.muted));
    set("--border", hex(p.border));
    set("--hover", hex(hover));
    set("--faint", hex(faint));
    set("--inactive", hex(inactive));
    set("--code-bg", hex(code_bg));
    set("--selection", rgba(p.accent, if d { 80.0 } else { 60.0 } / 255.0));
    set("--shadow", rgba([0, 0, 0], if d { 0.38 } else { 0.16 }));
    set("--danger", hex(DANGER));
    set("--danger-contrast", hex(contrast_on(DANGER)));
    set("--success", hex(SUCCESS));
    set("--color-scheme", if d { "dark" } else { "light" }.to_owned());
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contrast_picks_readable_ink() {
        assert_eq!(contrast_on([255, 255, 255]), [20, 22, 28]);
        assert_eq!(contrast_on([19, 19, 19]), [255, 255, 255]);
    }

    #[test]
    fn css_vars_cover_the_palette_roles() {
        let v = css_vars(&DARK);
        assert_eq!(v.get("--accent").map(String::as_str), Some("#ce412b"));
        assert_eq!(v.get("--bg").map(String::as_str), Some("#131313"));
        assert_eq!(v.get("--color-scheme").map(String::as_str), Some("dark"));
        assert!(v.get("--selection").unwrap().starts_with("rgba(206, 65, 43"));
        assert!(v.contains_key("--hover") && v.contains_key("--code-bg"));
    }

    #[test]
    fn system_falls_back_to_light_when_undetected() {
        assert_eq!(system_palette(false), LIGHT);
        assert_eq!(system_palette(true), DARK);
    }
}
