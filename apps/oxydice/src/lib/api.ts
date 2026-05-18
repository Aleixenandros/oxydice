// Capa API tipada: envoltura fina sobre los comandos de `oxydice-core`
// expuestos por la capa Tauri. Toda la lógica vive en Rust; aquí solo se
// traducen tipos. Los nombres de campo siguen el `serde` del core (snake
// case) para que `Config`/`Palette` hagan round-trip sin perder claves.

import { invoke } from "@tauri-apps/api/core";

export type Rgb = [number, number, number];

export interface Palette {
  dark: boolean;
  accent: Rgb;
  bg: Rgb;
  surface: Rgb;
  text: Rgb;
  muted: Rgb;
  border: Rgb;
}

export type EditorFont = "Mono" | "Sans";

/** Ajustes del remoto de sync **sin secretos** (van al llavero del SO). */
export interface RemoteSettings {
  kind: string; // "none" | "webdav" | "s3"
  endpoint: string;
  root: string;
  username: string;
  bucket: string;
  region: string;
  access_key: string;
  auto: boolean;
  interval_secs: number;
}

export interface Config {
  spaces: string[];
  selected: number | null;
  theme: string;
  custom_theme: Palette;
  sync: string;
  remote: RemoteSettings;
  editor_font: EditorFont;
  editor_font_size: number;
  /** Familia del sistema para el editor (T25); "" = usar Mono/Sans. */
  editor_font_family: string;
  ui_scale: number;
  /** Idioma de la UI: "" = seguir al sistema; o es/en/de/pt. */
  lang: string;
  backup_dir: string | null;
  backup_on_save: boolean;
  /** Ids de módulos/extensiones desactivados (T21). */
  disabled_ext: string[];
  /** Rutas de notas con pestaña abierta (se restauran al arrancar). */
  open_tabs: string[];
  /** Índice de la pestaña activa dentro de `open_tabs`. */
  active_tab: number;
}

/** Cambios de frontmatter (T6). `null` = no tocar; "" en escalares borra. */
export interface MetaEdit {
  title: string | null;
  status: string | null;
  author: string | null;
  tags: string[] | null;
}

/** Estado de sync (espejo de `oxydice_core::ext::sync::SyncState`). */
export type SyncState =
  | "Disabled"
  | "Synced"
  | "Syncing"
  | { Error: string };

export interface SyncReport {
  uploaded: number;
  downloaded: number;
  deleted_local: number;
  deleted_remote: number;
  conflicts: string[];
}

export interface Entry {
  path: string;
  name: string;
  is_dir: boolean;
}

export interface NoteData {
  content: string;
  mtime_secs: number | null;
}

export interface DocMeta {
  title: string | null;
  tags: string[];
  status: string | null;
  author: string | null;
}

export interface Heading {
  level: number;
  text: string;
  line: number;
}

export interface Hit {
  path: string;
  line: number;
  text: string;
  /** Rango `[inicio, fin)` del término dentro de `text`, en bytes UTF-8. */
  at: [number, number];
}

export interface Results {
  hits: Hit[];
  files: number;
  truncated: boolean;
}

export interface ThemeEntry {
  id: string;
  name: string;
  palette: Palette;
}

export interface ExtRow {
  kind: string;
  id: string;
  name: string;
  detail: string;
  enabled: boolean;
}

export interface UpdateInfo {
  current: string;
  latest: string;
  newer: boolean;
  url: string;
}

export interface ResolvedTheme {
  palette: Palette;
  css: Record<string, string>;
  name: string;
}

// ---- configuración ------------------------------------------------------

export const loadConfig = () => invoke<Config>("load_config");
export const saveConfig = (config: Config) =>
  invoke<void>("save_config", { config });

// ---- vault (sistema de archivos) ----------------------------------------

export const listDir = (path: string) =>
  invoke<Entry[]>("list_dir", { path });
export const readNote = (path: string) =>
  invoke<NoteData>("read_note", { path });
export const writeNote = (path: string, content: string) =>
  invoke<number | null>("write_note", { path, content });
export const createNote = (parent: string, name: string) =>
  invoke<string>("create_note", { parent, name });
export const createFolder = (parent: string, name: string) =>
  invoke<string>("create_folder", { parent, name });
export const renamePath = (old: string, name: string) =>
  invoke<string>("rename_path", { old, name });
export const deletePath = (path: string) =>
  invoke<void>("delete_path", { path });
export const backupNow = (space: string, dest: string) =>
  invoke<string>("backup_now", { space, dest });

// ---- documento ----------------------------------------------------------

export const renderMarkdown = (markdown: string) =>
  invoke<string>("render_markdown", { markdown });
export const docMeta = (content: string) =>
  invoke<DocMeta>("doc_meta", { content });
export const outline = (content: string) =>
  invoke<Heading[]>("outline", { content });
export const writeMeta = (path: string, edit: MetaEdit) =>
  invoke<number | null>("write_meta", { path, edit });
export const notesWithTag = (root: string, tag: string) =>
  invoke<string[]>("notes_with_tag", { root, tag });

// ---- búsqueda -----------------------------------------------------------

export const searchSpace = (root: string, query: string) =>
  invoke<Results>("search_space", { root, query });

// ---- temas y extensiones ------------------------------------------------

export const themeCatalog = () =>
  invoke<ThemeEntry[]>("theme_catalog");
export const resolveTheme = (
  id: string,
  systemDark: boolean,
  custom: Palette,
) => invoke<ResolvedTheme>("resolve_theme", { id, systemDark, custom });
export const extensionsListing = () =>
  invoke<ExtRow[]>("extensions_listing");
export const checkUpdate = () => invoke<UpdateInfo>("check_update");
export const syncNow = (id: string) =>
  invoke<void>("sync_now", { id });

// ---- sincronización real (motor + OpenDAL) ------------------------------

export const syncRun = () => invoke<SyncReport>("sync_run");
export const syncGetState = () => invoke<SyncState>("sync_get_state");
export const syncSetSecret = (kind: string, secret: string) =>
  invoke<void>("sync_set_secret", { kind, secret });
export const syncClearSecret = (kind: string) =>
  invoke<void>("sync_clear_secret", { kind });
export const exportHtml = (
  path: string,
  markdown: string,
  title: string,
  palette: Palette,
) => invoke<void>("export_html", { path, markdown, title, palette });
export const exportMd = (path: string, markdown: string) =>
  invoke<void>("export_md", { path, markdown });
export const exportTheme = (path: string, palette: Palette) =>
  invoke<void>("export_theme", { path, palette });
export const importTheme = (path: string) =>
  invoke<Palette>("import_theme", { path });

// ---- ids especiales de tema (espejo de `oxydice_core::theme`) -----------

export const SYSTEM_ID = "System";
export const CUSTOM_ID = "Custom";

/** Nombre del archivo (último componente) de una ruta absoluta. */
export function baseName(path: string): string {
  const parts = path.replace(/[/\\]+$/, "").split(/[/\\]/);
  return parts[parts.length - 1] ?? path;
}

/** Nombre de nota sin la extensión `.md`. */
export function stem(path: string): string {
  return baseName(path).replace(/\.md$/i, "");
}

/** Ruta relativa de `path` respecto de `root` (con `/` como separador). */
export function relativeTo(root: string, path: string): string {
  const r = root.replace(/[/\\]+$/, "");
  if (path.startsWith(r)) {
    return path.slice(r.length).replace(/^[/\\]+/, "").replace(/\\/g, "/");
  }
  return path.replace(/\\/g, "/");
}

/** Tiempo relativo legible a partir de un epoch en segundos. */
export function relTime(secs: number | null): string {
  if (secs == null) return "—";
  const d = Math.floor(Date.now() / 1000) - secs;
  if (d < 0) return "ahora";
  if (d < 60) return "hace un momento";
  if (d < 3600) return `hace ${Math.floor(d / 60)} min`;
  if (d < 86400) return `hace ${Math.floor(d / 3600)} h`;
  return `hace ${Math.floor(d / 86400)} d`;
}

/**
 * Parte `text` en `[antes, término, después]` usando offsets de **bytes
 * UTF-8** (los que devuelve el core), no índices UTF-16 de JS.
 */
export function splitByBytes(
  text: string,
  start: number,
  end: number,
): [string, string, string] {
  if (!(start < end)) return [text, "", ""];
  const bytes = new TextEncoder().encode(text);
  if (end > bytes.length) return [text, "", ""];
  const dec = new TextDecoder();
  return [
    dec.decode(bytes.slice(0, start)),
    dec.decode(bytes.slice(start, end)),
    dec.decode(bytes.slice(end)),
  ];
}
