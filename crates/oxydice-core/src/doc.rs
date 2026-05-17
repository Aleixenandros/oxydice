//! Análisis del documento: *frontmatter*, metadatos y esquema de encabezados.
//!
//! Todo se deriva del contenido Markdown en memoria (más la fecha del
//! archivo en disco); no hay índice ni estado persistido. Es barato y se
//! recalcula bajo demanda al abrir o editar una nota.

use serde::{Deserialize, Serialize};

/// Metadatos mostrados en la cabecera de la vista de lectura.
#[derive(Debug, Default, Clone, Serialize)]
pub struct DocMeta {
    /// Título: `title:` del frontmatter, primer `# H1`, o nombre del archivo.
    pub title: Option<String>,
    /// Etiquetas del frontmatter (`tags: [a, b]`, lista `- a`, o `a, b`).
    pub tags: Vec<String>,
    /// Estado declarado (`status:`), p. ej. «Borrador» o «Publicado».
    pub status: Option<String>,
    /// Autor declarado (`author:`).
    pub author: Option<String>,
}

/// Un encabezado del documento para el panel de esquema.
#[derive(Debug, Clone, Serialize)]
pub struct Heading {
    /// Nivel 1..=6 (número de `#`).
    pub level: usize,
    /// Texto del encabezado, sin los `#`.
    pub text: String,
    /// Línea (base 0) donde aparece, para saltar el cursor.
    pub line: usize,
}

/// Devuelve el bloque de *frontmatter* (sin los `---`) y la línea donde
/// empieza el cuerpo, si el documento abre con `---` en la primera línea.
fn split_frontmatter(content: &str) -> (Option<&str>, usize) {
    let mut lines = content.lines();
    if lines.next().map(str::trim_end) != Some("---") {
        return (None, 0);
    }
    let mut body_line = 1;
    let after_marker = content.split_inclusive('\n').skip(1);
    let mut end_byte = None;
    let mut start_byte = None;
    let mut offset = content.split_inclusive('\n').next().map_or(0, str::len);
    for chunk in after_marker {
        body_line += 1;
        if start_byte.is_none() {
            start_byte = Some(offset);
        }
        if chunk.trim_end() == "---" {
            end_byte = Some(offset);
            break;
        }
        offset += chunk.len();
    }
    match (start_byte, end_byte) {
        (Some(s), Some(e)) if e >= s => (Some(&content[s..e]), body_line),
        _ => (None, 0),
    }
}

/// Normaliza una lista de etiquetas escrita en una sola línea:
/// `[a, b]`, `a, b` o `"a" 'b'` → `["a", "b"]`.
fn parse_tag_list(raw: &str) -> Vec<String> {
    raw.trim()
        .trim_start_matches('[')
        .trim_end_matches(']')
        .split(',')
        .map(|t| t.trim().trim_matches(['"', '\'', '#']).trim())
        .filter(|t| !t.is_empty())
        .map(str::to_owned)
        .collect()
}

/// Extrae los metadatos del documento a partir de su contenido.
pub fn meta(content: &str) -> DocMeta {
    let mut m = DocMeta::default();
    let (front, _) = split_frontmatter(content);

    if let Some(front) = front {
        let mut collecting_tags = false;
        for line in front.lines() {
            let trimmed = line.trim();
            // Elemento de lista YAML bajo `tags:`.
            if collecting_tags {
                if let Some(item) = trimmed.strip_prefix('-') {
                    let t = item.trim().trim_matches(['"', '\'', '#']).trim();
                    if !t.is_empty() {
                        m.tags.push(t.to_owned());
                    }
                    continue;
                }
                collecting_tags = false;
            }
            let Some((key, value)) = trimmed.split_once(':') else {
                continue;
            };
            let key = key.trim().to_ascii_lowercase();
            let value = value.trim();
            match key.as_str() {
                "title" => m.title = non_empty(value),
                "status" => m.status = non_empty(value),
                "author" => m.author = non_empty(value),
                "tags" | "tag" => {
                    if value.is_empty() {
                        collecting_tags = true;
                    } else {
                        m.tags = parse_tag_list(value);
                    }
                }
                _ => {}
            }
        }
    }

    if m.title.is_none() {
        m.title = first_h1(content);
    }
    m
}

fn non_empty(s: &str) -> Option<String> {
    let s = s.trim().trim_matches(['"', '\'']).trim();
    (!s.is_empty()).then(|| s.to_owned())
}

/// Primer encabezado `# H1` del cuerpo, ignorando bloques de código.
fn first_h1(content: &str) -> Option<String> {
    outline(content)
        .into_iter()
        .find(|h| h.level == 1)
        .map(|h| h.text)
}

/// Árbol de encabezados ATX (`#`..`######`), saltando *frontmatter* y
/// bloques de código vallados (``` o ~~~).
pub fn outline(content: &str) -> Vec<Heading> {
    let (_, body_start) = split_frontmatter(content);
    let mut out = Vec::new();
    let mut fenced: Option<char> = None;
    for (i, raw) in content.lines().enumerate() {
        if i < body_start {
            continue;
        }
        let line = raw.trim_start();
        if let Some(fence) = line.chars().next().filter(|c| *c == '`' || *c == '~') {
            let run = line.chars().take_while(|c| *c == fence).count();
            if run >= 3 {
                match fenced {
                    Some(open) if open == fence => fenced = None,
                    None => fenced = Some(fence),
                    _ => {}
                }
                continue;
            }
        }
        if fenced.is_some() {
            continue;
        }
        let hashes = line.chars().take_while(|c| *c == '#').count();
        if (1..=6).contains(&hashes) {
            if let Some(rest) = line[hashes..].strip_prefix(' ') {
                let text = rest.trim().trim_end_matches('#').trim();
                if !text.is_empty() {
                    out.push(Heading {
                        level: hashes,
                        text: text.to_owned(),
                        line: i,
                    });
                }
            }
        }
    }
    out
}

// ---- escritura del frontmatter (round-trip) -----------------------------

/// Cambios a aplicar al *frontmatter*. `None` = no tocar esa clave.
/// Para `title`/`status`/`author`, `Some("")` elimina la clave. Para
/// `tags`, `Some(vec![])` elimina la clave y `Some(no vacío)` la fija.
#[derive(Debug, Default, Deserialize)]
pub struct MetaEdit {
    pub title: Option<String>,
    pub status: Option<String>,
    pub author: Option<String>,
    pub tags: Option<Vec<String>>,
}

/// Separa el documento en (líneas del frontmatter sin `---`, cuerpo exacto).
/// Si no abre con `---` … `---`, no hay frontmatter y el cuerpo es todo.
fn split_raw(content: &str) -> (Option<Vec<String>>, String) {
    let mut it = content.split_inclusive('\n');
    let Some(first) = it.next() else {
        return (None, String::new());
    };
    if first.trim_end() != "---" {
        return (None, content.to_owned());
    }
    let mut consumed = first.len();
    let mut front = Vec::new();
    for chunk in it {
        consumed += chunk.len();
        if chunk.trim_end() == "---" {
            return (Some(front), content[consumed..].to_owned());
        }
        front.push(chunk.trim_end_matches('\n').to_owned());
    }
    // Frontmatter sin cierre: trátalo como inexistente (no se corrompe).
    (None, content.to_owned())
}

fn render_tags(tags: &[String]) -> String {
    let inner = tags
        .iter()
        .map(|t| t.trim().trim_start_matches('#').trim())
        .filter(|t| !t.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
    format!("tags: [{inner}]")
}

/// Reescribe el *frontmatter* aplicando `edit`, **preservando las claves
/// desconocidas y el cuerpo intactos** (round-trip seguro). Crea el bloque
/// si no existía y hay algo que escribir; lo elimina si queda vacío.
pub fn write_meta(content: &str, edit: &MetaEdit) -> String {
    let (front, body) = split_raw(content);
    let mut out: Vec<String> = Vec::new();
    let (mut seen_title, mut seen_status, mut seen_author, mut seen_tags) =
        (false, false, false, false);

    let scalar = |key: &str, val: &Option<String>, out: &mut Vec<String>| {
        if let Some(v) = val {
            let v = v.trim();
            if !v.is_empty() {
                out.push(format!("{key}: {v}"));
            }
        }
    };

    if let Some(lines) = &front {
        let mut i = 0;
        while i < lines.len() {
            let line = &lines[i];
            i += 1;
            let trimmed = line.trim();
            let Some((k, _)) = trimmed.split_once(':') else {
                out.push(line.clone());
                continue;
            };
            match k.trim().to_ascii_lowercase().as_str() {
                "title" => {
                    seen_title = true;
                    match &edit.title {
                        None => out.push(line.clone()),
                        Some(_) => scalar("title", &edit.title, &mut out),
                    }
                }
                "status" => {
                    seen_status = true;
                    match &edit.status {
                        None => out.push(line.clone()),
                        Some(_) => scalar("status", &edit.status, &mut out),
                    }
                }
                "author" => {
                    seen_author = true;
                    match &edit.author {
                        None => out.push(line.clone()),
                        Some(_) => scalar("author", &edit.author, &mut out),
                    }
                }
                "tags" | "tag" => {
                    seen_tags = true;
                    // Si era lista en bloque (`tags:` + `- item`), sáltala.
                    let is_block = trimmed
                        .split_once(':')
                        .map(|(_, v)| v.trim().is_empty())
                        .unwrap_or(false);
                    let mut block: Vec<String> = Vec::new();
                    if is_block {
                        while i < lines.len() && lines[i].trim().starts_with('-') {
                            block.push(lines[i].clone());
                            i += 1;
                        }
                    }
                    match &edit.tags {
                        None => {
                            out.push(line.clone());
                            out.extend(block);
                        }
                        Some(ts) if !ts.is_empty() => out.push(render_tags(ts)),
                        Some(_) => {} // vaciar = eliminar la clave
                    }
                }
                _ => out.push(line.clone()),
            }
        }
    }

    if !seen_title {
        scalar("title", &edit.title, &mut out);
    }
    if !seen_status {
        scalar("status", &edit.status, &mut out);
    }
    if !seen_author {
        scalar("author", &edit.author, &mut out);
    }
    if !seen_tags {
        if let Some(ts) = &edit.tags {
            if !ts.is_empty() {
                out.push(render_tags(ts));
            }
        }
    }

    if out.is_empty() {
        return body;
    }
    format!("---\n{}\n---\n{}", out.join("\n"), body)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frontmatter_meta() {
        let src = "---\ntitle: API\nstatus: Borrador\nauthor: Jane\ntags: [api, v2]\n---\n# Cuerpo\n";
        let m = meta(src);
        assert_eq!(m.title.as_deref(), Some("API"));
        assert_eq!(m.status.as_deref(), Some("Borrador"));
        assert_eq!(m.author.as_deref(), Some("Jane"));
        assert_eq!(m.tags, vec!["api", "v2"]);
    }

    #[test]
    fn tags_as_yaml_list() {
        let src = "---\ntags:\n  - uno\n  - dos\n---\ncuerpo\n";
        assert_eq!(meta(src).tags, vec!["uno", "dos"]);
    }

    #[test]
    fn title_falls_back_to_first_h1() {
        let m = meta("texto\n# Primero\n## Segundo\n");
        assert_eq!(m.title.as_deref(), Some("Primero"));
    }

    #[test]
    fn outline_skips_frontmatter_and_code_fences() {
        let src = "---\ntitle: T\n---\n# Uno\n```\n# noheading\n```\n## Dos\n";
        let o = outline(src);
        assert_eq!(o.len(), 2);
        assert_eq!((o[0].level, o[0].text.as_str(), o[0].line), (1, "Uno", 3));
        assert_eq!((o[1].level, o[1].text.as_str()), (2, "Dos"));
    }

    #[test]
    fn no_frontmatter_is_fine() {
        let m = meta("solo texto sin nada\n");
        assert!(m.title.is_none() && m.tags.is_empty());
    }
}

#[cfg(test)]
mod write_tests {
    use super::*;

    fn edit(
        title: Option<&str>,
        status: Option<&str>,
        tags: Option<&[&str]>,
    ) -> MetaEdit {
        MetaEdit {
            title: title.map(str::to_owned),
            status: status.map(str::to_owned),
            author: None,
            tags: tags.map(|t| t.iter().map(|s| s.to_string()).collect()),
        }
    }

    #[test]
    fn updates_keeping_unknown_keys_and_body() {
        let src = "---\ntitle: Viejo\ncustom: xyz\n---\n# Cuerpo\ntexto\n";
        let out = write_meta(src, &edit(Some("Nuevo"), Some("Borrador"), None));
        assert!(out.contains("title: Nuevo"));
        assert!(out.contains("custom: xyz")); // clave desconocida intacta
        assert!(out.contains("status: Borrador")); // añadida
        assert!(out.ends_with("# Cuerpo\ntexto\n")); // cuerpo intacto
        // Round-trip: leer lo escrito devuelve lo esperado.
        let m = meta(&out);
        assert_eq!(m.title.as_deref(), Some("Nuevo"));
        assert_eq!(m.status.as_deref(), Some("Borrador"));
    }

    #[test]
    fn creates_frontmatter_when_absent() {
        let out = write_meta("# Solo cuerpo\n", &edit(Some("T"), None, Some(&["a", "b"])));
        assert!(out.starts_with("---\n"));
        assert!(out.contains("title: T"));
        assert!(out.contains("tags: [a, b]"));
        assert!(out.ends_with("# Solo cuerpo\n"));
        assert_eq!(meta(&out).tags, vec!["a", "b"]);
    }

    #[test]
    fn replaces_block_tag_list_and_can_remove() {
        let src = "---\ntags:\n  - uno\n  - dos\nauthor: Jane\n---\ncuerpo\n";
        let out = write_meta(src, &edit(None, None, Some(&["x"])));
        assert!(out.contains("tags: [x]"));
        assert!(!out.contains("- uno"));
        assert!(out.contains("author: Jane"));
        // Vaciar tags elimina la clave.
        let out2 = write_meta(src, &edit(None, None, Some(&[])));
        assert!(!out2.contains("tags"));
        assert!(out2.contains("author: Jane"));
    }

    #[test]
    fn empty_result_drops_frontmatter() {
        let src = "---\ntitle: X\n---\ncuerpo\n";
        let out = write_meta(src, &edit(Some(""), None, None));
        assert_eq!(out, "cuerpo\n");
    }
}
