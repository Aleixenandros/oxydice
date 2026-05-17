//! Exportador (T18). Genera un **HTML autónomo** fiel a la vista de lectura:
//! cuerpo saneado por `render` + las variables CSS del tema + los estilos de
//! la vista previa. Es la base de la exportación a HTML y, vía «imprimir a
//! PDF» del *webview*, a PDF (sin dependencias pesadas). DOCX/LaTeX seguirán
//! el mismo patrón (backlog).

use crate::render;
use crate::theme::{self, Palette};
use std::collections::BTreeMap;

fn escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Estilos del documento exportado, espejo de la vista de lectura (`.md`).
/// No hay colores fijos: todo deriva de las variables del tema.
fn doc_css(vars: &BTreeMap<String, String>) -> String {
    let root: String = vars
        .iter()
        .map(|(k, v)| format!("  {k}: {v};\n"))
        .collect();
    format!(
        ":root {{\n{root}}}\n\
         * {{ box-sizing: border-box; }}\n\
         body {{ background: var(--bg); color: var(--text); margin: 0;\n  \
         font-family: Inter, system-ui, sans-serif; line-height: 1.7; }}\n\
         main {{ max-width: 46rem; margin: 0 auto; padding: 3rem 1.5rem; }}\n\
         h1, h2, h3 {{ line-height: 1.25; margin: 1.4em 0 .5em; }}\n\
         a {{ color: var(--accent); }}\n\
         code {{ background: var(--code-bg); padding: .15em .35em;\n  \
         border-radius: 3px; font-family: 'JetBrains Mono', monospace; }}\n\
         pre {{ background: var(--code-bg); padding: 12px 14px;\n  \
         border-radius: 4px; overflow: auto; }}\n\
         pre code {{ background: none; padding: 0; }}\n\
         blockquote {{ border-left: 3px solid var(--accent); margin: .8em 0;\n  \
         padding-left: 12px; color: var(--muted); }}\n\
         table {{ border-collapse: collapse; margin: .8em 0; }}\n\
         th, td {{ border: 1px solid var(--border); padding: 6px 10px; }}\n\
         img {{ max-width: 100%; }}\n\
         hr {{ border: none; border-top: 1px solid var(--border); }}\n\
         @media print {{ body {{ background: #fff; color: #000; }}\n  \
         main {{ max-width: none; padding: 0; }} }}\n"
    )
}

/// HTML autónomo (`<!doctype html>` … ) listo para abrir o imprimir a PDF.
/// El cuerpo lo sanea `render::to_html` (sin scripts ni manejadores).
pub fn standalone_html(
    markdown: &str,
    title: &str,
    vars: &BTreeMap<String, String>,
) -> String {
    let body = render::to_html(markdown);
    let css = doc_css(vars);
    format!(
        "<!doctype html>\n<html lang=\"es\">\n<head>\n\
         <meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         <title>{}</title>\n<style>\n{css}</style>\n</head>\n\
         <body>\n<main>\n{body}</main>\n</body>\n</html>\n",
        escape(title)
    )
}

/// Conveniencia: HTML autónomo usando la paleta dada (resuelve sus vars).
pub fn html_with_palette(markdown: &str, title: &str, palette: &Palette) -> String {
    standalone_html(markdown, title, &theme::css_vars(palette))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_standalone_sanitized_document() {
        let html = html_with_palette(
            "# Hola\n\n<script>alert(1)</script>\n\ntexto",
            "Mi <nota>",
            &theme::DARK,
        );
        assert!(html.starts_with("<!doctype html>"));
        assert!(html.contains("<title>Mi &lt;nota&gt;</title>")); // título escapado
        assert!(html.contains("<h1>Hola</h1>"));
        assert!(!html.contains("<script>alert")); // saneado por render
        assert!(html.contains("--accent:")); // variables del tema embebidas
    }
}
