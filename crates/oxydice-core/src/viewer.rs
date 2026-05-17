//! Visor de código (T17): qué archivos, además de `.md`, se pueden abrir y
//! mostrar resaltados en **solo lectura**. El resaltado lo hace
//! CodeMirror/Lezer en el cliente según la extensión; el core solo decide
//! qué se lista y qué se considera abrible (el disco sigue siendo la verdad).

use std::path::Path;

/// Extensiones de código/texto que el visor sabe mostrar. Lista curada de
/// lenguajes y formatos habituales junto a las notas.
pub const CODE_EXTS: &[&str] = &[
    "html", "htm", "css", "scss", "sass", "less", "js", "mjs", "cjs", "ts",
    "tsx", "jsx", "json", "jsonc", "php", "rs", "py", "go", "rb", "java",
    "kt", "kts", "c", "h", "cpp", "hpp", "cc", "cs", "sh", "bash", "zsh",
    "yaml", "yml", "toml", "ini", "cfg", "sql", "xml", "svelte", "vue",
    "lua", "swift", "dart", "scala", "r", "pl", "txt", "csv", "tsv", "log",
];

fn ext_of(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
}

/// `true` si es un archivo de código/texto que el visor puede mostrar.
pub fn is_code(path: &Path) -> bool {
    ext_of(path).is_some_and(|e| CODE_EXTS.contains(&e.as_str()))
}

/// `true` si el archivo es abrible: nota Markdown **o** código del visor.
pub fn is_viewable(path: &Path) -> bool {
    ext_of(path).is_some_and(|e| e == "md" || CODE_EXTS.contains(&e.as_str()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recognises_markdown_and_code_only() {
        assert!(is_viewable(Path::new("a/n.md")));
        assert!(is_viewable(Path::new("style.CSS"))); // sin distinguir mayúsculas
        assert!(is_code(Path::new("x.php")) && is_code(Path::new("m.rs")));
        assert!(!is_code(Path::new("note.md"))); // .md no es «código»
        assert!(!is_viewable(Path::new("foto.png")));
        assert!(!is_viewable(Path::new("sin_extension")));
    }
}
