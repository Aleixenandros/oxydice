//! Búsqueda de texto en todas las notas del espacio activo.
//!
//! Recorrido síncrono y directo del sistema de archivos, coherente con el
//! resto de la app (el disco es la verdad, sin índice). El número de
//! resultados se limita para que la pasada por frame sea barata.

use std::path::{Path, PathBuf};

/// Una coincidencia: archivo, línea y el texto de esa línea con la posición
/// del término para poder resaltarlo.
#[derive(Debug, Clone)]
pub struct Hit {
    pub path: PathBuf,
    /// Línea (base 0).
    pub line: usize,
    /// Texto de la línea (recortado si es muy largo).
    pub text: String,
    /// Rango `[inicio, fin)` del término dentro de `text`, en bytes.
    pub at: (usize, usize),
}

/// Resultado de una búsqueda: coincidencias y si se truncó por el límite.
#[derive(Debug, Default)]
pub struct Results {
    pub hits: Vec<Hit>,
    pub files: usize,
    pub truncated: bool,
}

const MAX_HITS: usize = 200;
const SNIPPET: usize = 160;

/// Busca `query` (sin distinguir mayúsculas) en los `.md` bajo `root`.
pub fn search(root: &Path, query: &str) -> Results {
    let mut res = Results::default();
    let needle = query.trim().to_lowercase();
    if needle.is_empty() {
        return res;
    }
    walk(root, &needle, &mut res);
    res
}

fn walk(dir: &Path, needle: &str, res: &mut Results) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| !n.starts_with('.'))
        })
        .collect();
    entries.sort();
    for path in entries {
        if res.hits.len() >= MAX_HITS {
            res.truncated = true;
            return;
        }
        if path.is_dir() {
            walk(&path, needle, res);
        } else if path.extension().is_some_and(|x| x == "md") {
            scan_file(&path, needle, res);
        }
    }
}

fn scan_file(path: &Path, needle: &str, res: &mut Results) {
    let Ok(content) = std::fs::read_to_string(path) else {
        return;
    };
    res.files += 1;
    for (i, line) in content.lines().enumerate() {
        if res.hits.len() >= MAX_HITS {
            res.truncated = true;
            return;
        }
        let Some(byte) = line.to_lowercase().find(needle) else {
            continue;
        };
        // La posición en minúsculas coincide en bytes con el original
        // mientras el término sea ASCII; si no, se cae al inicio de línea.
        let (start, end) = if line.is_char_boundary(byte)
            && line.is_char_boundary(byte + needle.len())
        {
            (byte, byte + needle.len())
        } else {
            (0, 0)
        };
        let (text, shift) = snippet(line, start);
        res.hits.push(Hit {
            path: path.to_path_buf(),
            line: i,
            text,
            at: (start.saturating_sub(shift), end.saturating_sub(shift)),
        });
    }
}

/// Recorta una línea larga centrada en el término. Devuelve el texto y
/// cuántos bytes se quitaron por la izquierda (para reubicar el resaltado).
fn snippet(line: &str, at: usize) -> (String, usize) {
    if line.len() <= SNIPPET {
        return (line.to_owned(), 0);
    }
    let mut start = at.saturating_sub(SNIPPET / 3);
    while start > 0 && !line.is_char_boundary(start) {
        start -= 1;
    }
    let mut end = (start + SNIPPET).min(line.len());
    while end < line.len() && !line.is_char_boundary(end) {
        end += 1;
    }
    let mut s = String::new();
    if start > 0 {
        s.push('…');
    }
    s.push_str(&line[start..end]);
    if end < line.len() {
        s.push('…');
    }
    // El «…» inicial ocupa 3 bytes; compénsalo en el desfase.
    let shift = if start > 0 { start - '…'.len_utf8() } else { 0 };
    (s, shift)
}
