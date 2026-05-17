//! Render Markdown → HTML: **única fuente de verdad** de la vista previa.
//!
//! Vive en el core para que escritorio y móvil rendericen idéntico. Usa
//! `pulldown-cmark` (CommonMark + tablas, tachado, listas de tareas y notas
//! al pie) y **sanea** el HTML resultante con `ammonia`: las notas son
//! Markdown del usuario pero pueden venir de una nube/sync; servir HTML sin
//! filtrar en un *webview* sería un vector de XSS (`<script>`, `onerror`,
//! `javascript:`…). Se permiten las etiquetas habituales en notas.

use pulldown_cmark::{html, Options, Parser};

/// Convierte Markdown a HTML saneado, listo para inyectar en la vista de
/// lectura. El HTML resultante no contiene scripts ni manejadores de eventos.
pub fn to_html(markdown: &str) -> String {
    let mut opts = Options::empty();
    opts.insert(Options::ENABLE_TABLES);
    opts.insert(Options::ENABLE_STRIKETHROUGH);
    opts.insert(Options::ENABLE_TASKLISTS);
    opts.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(markdown, opts);
    let mut raw = String::new();
    html::push_html(&mut raw, parser);

    ammonia::Builder::default()
        // Permite resaltar bloques de código por clase (`language-rust`…).
        .add_generic_attributes(["class"])
        // Casillas de las listas de tareas en estado de solo lectura.
        .add_tags(["input"])
        .add_tag_attributes("input", ["type", "checked", "disabled"])
        .clean(&raw)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headings_and_emphasis() {
        let h = to_html("# Título\n\nun *énfasis* y `código`.");
        assert!(h.contains("<h1>Título</h1>"));
        assert!(h.contains("<em>énfasis</em>"));
        assert!(h.contains("<code>código</code>"));
    }

    #[test]
    fn gfm_extensions_enabled() {
        let table = to_html("| a | b |\n|---|---|\n| 1 | 2 |");
        assert!(table.contains("<table>") && table.contains("<td>1</td>"));
        assert!(to_html("~~tachado~~").contains("<del>tachado</del>"));
    }

    #[test]
    fn fenced_code_keeps_language_class() {
        let h = to_html("```rust\nfn main() {}\n```");
        assert!(h.contains("language-rust"));
    }

    #[test]
    fn html_is_sanitized() {
        let h = to_html("texto <script>alert(1)</script> y <img src=x onerror=alert(2)>");
        assert!(!h.contains("<script>"));
        assert!(!h.contains("onerror"));
    }

    #[test]
    fn javascript_links_are_stripped() {
        let h = to_html("[clic](javascript:alert(1))");
        assert!(!h.contains("javascript:"));
    }
}
