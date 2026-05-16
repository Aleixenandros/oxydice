//! Análisis del documento: *frontmatter*, metadatos y esquema de encabezados.
//!
//! Todo se deriva del contenido Markdown en memoria (más la fecha del
//! archivo en disco); no hay índice ni estado persistido. Es barato y se
//! recalcula bajo demanda al abrir o editar una nota.

/// Metadatos mostrados en la cabecera de la vista de lectura.
#[derive(Debug, Default, Clone)]
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
#[derive(Debug, Clone)]
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
