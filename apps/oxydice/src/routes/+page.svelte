<script lang="ts">
  // Oxydice — escritorio (Tauri 2 + Svelte 5 + CodeMirror 6). Reproduce
  // fielmente la app egui anterior: barra superior, barra lateral con rail
  // de navegación y árbol, explorador con cabecera de documento + esquema +
  // editor o vista de lectura, búsqueda global y Ajustes. Toda la lógica
  // vive en `oxydice-core`; aquí solo estado de UI y orquestación.
  import { onMount, tick } from "svelte";
  import { open as openDialog, save as saveDialog } from "@tauri-apps/plugin-dialog";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { listen } from "@tauri-apps/api/event";
  import Tree from "$lib/Tree.svelte";
  import Editor from "$lib/Editor.svelte";
  import { refreshTheme, onSystemSchemeChange } from "$lib/theme";
  import * as api from "$lib/api";
  import { SYSTEM_ID, CUSTOM_ID } from "$lib/api";
  import {
    t,
    setLang,
    getLang,
    detectLang,
    LANGS,
    type Lang,
  } from "$lib/i18n.svelte";
  import type {
    Config,
    DocMeta,
    Heading,
    Results,
    ThemeEntry,
    ExtRow,
    Rgb,
    SyncState,
  } from "$lib/api";

  const AUTOSAVE_MS = 600;
  const SEARCH_DEBOUNCE_MS = 220;
  const VERSION = "0.8.0";

  type View = "explorer" | "search" | "settings";
  type DocMode = "edit" | "read" | "split";
  type LogLevel = "info" | "warn" | "error";

  let config = $state<Config | null>(null);
  let themeName = $state("");
  let themeCatalog = $state<ThemeEntry[]>([]);
  let extRows = $state<ExtRow[]>([]);

  let view = $state<View>("explorer");
  let docMode = $state<DocMode>("edit");
  let showSidebar = $state(true);
  let showOutline = $state(true);

  // Pestañas: una por nota abierta. `tabs` es la verdad (ruta + contenido +
  // sucio + mtime); `current/buffer/dirty/mtime` son el espejo vivo de la
  // pestaña activa para no reescribir toda la lógica de documento.
  interface Tab {
    path: string;
    content: string;
    dirty: boolean;
    mtime: number | null;
  }
  let tabs = $state<Tab[]>([]);
  let activeIdx = $state(-1);

  let current = $state<string | null>(null);
  let buffer = $state("");
  let dirty = $state(false);
  let mtime = $state<number | null>(null);
  let status = $state("");
  let log = $state<{ ts: string; level: LogLevel; msg: string }[]>([]);

  let meta = $state<DocMeta>({ title: null, tags: [], status: null, author: null });
  let headings = $state<Heading[]>([]);
  let previewHtml = $state("");
  // Exportación a PDF (T18): vista de impresión fuera del layout normal.
  let printing = $state(false);
  let printHtml = $state("");

  let query = $state("");
  let results = $state<Results>({ hits: [], files: 0, truncated: false });

  // T6: filtro por etiqueta en el explorador y editor de metadatos.
  let tagFilter = $state("");
  let tagHits = $state<string[]>([]);
  let metaDlg = $state<{
    title: string;
    status: string;
    author: string;
    tags: string;
  } | null>(null);

  let dialog = $state<
    | { kind: "note" | "folder" | "rename"; parent: string; target?: string; name: string }
    | null
  >(null);
  let pendingDelete = $state<string | null>(null);
  let menu = $state<{
    x: number;
    y: number;
    path: string;
    isDir: boolean;
    /** Menú abierto sobre el área vacía del explorador (raíz del espacio):
        solo «nueva nota/carpeta», sin renombrar/borrar/exportar. */
    root?: boolean;
  } | null>(null);

  let reloadKey = $state(0);
  let pendingGoto = $state<number | null>(null);
  let editorRef = $state<Editor | undefined>(undefined);
  let loadedPath = $state<string | null>(null);
  let autosaveTimer: ReturnType<typeof setTimeout> | undefined;
  let searchTimer: ReturnType<typeof setTimeout> | undefined;
  let showMenu = $state(false);
  // T24: menú de formato del editor. T22: sección activa de Ajustes.
  let fmtMenu = $state<{ x: number; y: number } | null>(null);
  let settingsSection = $state<
    "appearance" | "sync" | "backup" | "extensions" | "about"
  >("appearance");

  /** Un clic en cualquier parte cierra menús contextuales y desplegables. */
  function closeMenus() {
    menu = null;
    showMenu = false;
    fmtMenu = null;
  }

  const selectedSpace = $derived(
    config && config.selected != null
      ? (config.spaces[config.selected] ?? null)
      : null,
  );
  // T21: módulos desactivados → puertas para exportador y temas.
  const exporterOn = $derived(
    !(config?.disabled_ext ?? []).includes("exporter"),
  );
  const themesOn = $derived(
    !(config?.disabled_ext ?? []).includes("core-themes"),
  );

  // T21: activar/desactivar un módulo y refrescar lo afectado.
  async function toggleExt(id: string, enabled: boolean) {
    if (!config) return;
    const set = new Set(config.disabled_ext);
    if (enabled) set.delete(id);
    else set.add(id);
    config.disabled_ext = [...set];
    await persist();
    extRows = await api.extensionsListing();
    reloadKey++; // el árbol refleja el visor de código on/off
  }

  // T23: reportar incidencia y comprobar actualizaciones.
  async function reportIssue() {
    await openUrl("https://github.com/Aleixenandros/oxydice/issues");
  }
  async function doCheckUpdate() {
    try {
      const u = await api.checkUpdate();
      if (u.newer) {
        logMsg("info", t("st.updateAvailable", u.latest));
        await openUrl(u.url);
      } else {
        logMsg("info", t("st.upToDate", u.current));
      }
    } catch (e) {
      logMsg("error", t("st.updateErr", String(e)));
    }
  }
  // Visor de código (T17): cualquier archivo abierto que no sea `.md`.
  const isCode = $derived(current != null && !/\.md$/i.test(current));
  const editing = $derived(
    view === "explorer" &&
      current != null &&
      (docMode === "edit" || docMode === "split" || isCode),
  );

  function logMsg(level: LogLevel, msg: string) {
    const d = new Date();
    const ts = [d.getUTCHours(), d.getUTCMinutes(), d.getUTCSeconds()]
      .map((n) => String(n).padStart(2, "0"))
      .join(":");
    log = [...log, { ts, level, msg }].slice(-80);
    status = msg;
  }

  async function persist() {
    if (config) await api.saveConfig($state.snapshot(config));
  }

  async function applyTheme() {
    if (!config) return;
    const r = await refreshTheme($state.snapshot(config));
    themeName = r.name;
  }

  // ---- ciclo de vida ----------------------------------------------------

  onMount(() => {
    let dispose = () => {};
    let unlistenSync = () => {};
    (async () => {
      config = await api.loadConfig();
      setLang((config.lang as Lang) || detectLang());
      await applyTheme();
      themeCatalog = await api.themeCatalog();
      extRows = await api.extensionsListing();
      syncState = await api.syncGetState();
      const un = await listen<SyncState>("sync:state", (e) => {
        syncState = e.payload;
      });
      unlistenSync = un;
      // Restaura las pestañas de la sesión anterior; las notas que ya no
      // existan en disco se ignoran sin error.
      const restored: Tab[] = [];
      for (const p of config.open_tabs ?? []) {
        try {
          const n = await api.readNote(p);
          restored.push({ path: p, content: n.content, dirty: false, mtime: n.mtime_secs });
        } catch {
          /* nota ausente: se omite */
        }
      }
      if (restored.length) {
        tabs = restored;
        activeIdx = Math.min(config.active_tab ?? 0, restored.length - 1);
        const tb = restored[activeIdx];
        current = tb.path;
        buffer = tb.content;
        dirty = false;
        mtime = tb.mtime;
        loadedPath = null;
      }
      logMsg("info", t("app.started"));
    })();
    dispose = onSystemSchemeChange(() => void applyTheme());

    const onKey = (e: KeyboardEvent) => {
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        void flush(false);
      }
      // Contrae/expande la columna izquierda (rail + barra lateral).
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "b") {
        e.preventDefault();
        showSidebar = !showSidebar;
      }
    };
    window.addEventListener("keydown", onKey);
    const onClose = () => void flush(true);
    window.addEventListener("beforeunload", onClose);
    // T19: quita el menú nativo del webview (incl. «Inspeccionar») en toda
    // la app. Los menús propios (árbol, editor) abren igual: solo se anula
    // la acción por defecto, no la propagación.
    const onCtx = (e: MouseEvent) => e.preventDefault();
    window.addEventListener("contextmenu", onCtx);

    return () => {
      dispose();
      unlistenSync();
      window.removeEventListener("keydown", onKey);
      window.removeEventListener("beforeunload", onClose);
      window.removeEventListener("contextmenu", onCtx);
    };
  });

  // Recalcula metadatos/esquema al cambiar el texto.
  $effect(() => {
    const content = buffer;
    void current;
    api.docMeta(content).then((m) => (meta = m)).catch(() => {});
    api.outline(content).then((h) => (headings = h)).catch(() => {});
  });

  // Render de la vista de lectura (única fuente de verdad: el core).
  $effect(() => {
    if (
      view === "explorer" &&
      current != null &&
      (docMode === "read" || docMode === "split")
    ) {
      api.renderMarkdown(buffer).then((h) => (previewHtml = h)).catch(() => {});
    }
  });

  // Búsqueda con rebote.
  $effect(() => {
    const q = query;
    const root = selectedSpace;
    clearTimeout(searchTimer);
    if (!root || q.trim() === "") {
      results = { hits: [], files: 0, truncated: false };
      return;
    }
    searchTimer = setTimeout(() => {
      api.searchSpace(root, q).then((r) => (results = r)).catch(() => {});
    }, SEARCH_DEBOUNCE_MS);
  });

  // Sincroniza el documento con el editor montado y aplica saltos pendientes.
  $effect(() => {
    if (!editorRef || !editing) return;
    if (loadedPath !== current) {
      editorRef.setContent(buffer);
      loadedPath = current;
    }
    if (pendingGoto != null) {
      const line = pendingGoto;
      pendingGoto = null;
      tick().then(() => editorRef?.gotoLine(line));
    }
  });

  // ---- documento y pestañas ---------------------------------------------

  /** Vuelca el estado vivo (buffer/dirty/mtime) en la pestaña activa para
      que conmutar entre pestañas no pierda ediciones sin guardar. */
  function syncActiveTab() {
    if (activeIdx >= 0 && tabs[activeIdx]) {
      tabs[activeIdx].content = buffer;
      tabs[activeIdx].dirty = dirty;
      tabs[activeIdx].mtime = mtime;
    }
  }

  /** Persiste las pestañas abiertas (rutas + activa) en la config. */
  async function persistTabs() {
    if (!config) return;
    config.open_tabs = tabs.map((tb) => tb.path);
    config.active_tab = activeIdx < 0 ? 0 : activeIdx;
    await persist();
  }

  async function flush(quiet: boolean) {
    clearTimeout(autosaveTimer);
    if (!dirty || current == null) return;
    try {
      mtime = await api.writeNote(current, buffer);
      dirty = false;
      if (activeIdx >= 0 && tabs[activeIdx]) {
        tabs[activeIdx].dirty = false;
        tabs[activeIdx].mtime = mtime;
        tabs[activeIdx].content = buffer;
      }
      if (quiet) status = t("st.saved");
      else logMsg("info", t("st.saved"));
      if (config?.backup_on_save) await runBackup(true);
    } catch (e) {
      logMsg("error", t("st.saveErr", String(e)));
    }
  }

  function scheduleAutosave() {
    clearTimeout(autosaveTimer);
    autosaveTimer = setTimeout(() => void flush(true), AUTOSAVE_MS);
  }

  function onEdit(value: string) {
    buffer = value;
    dirty = true;
    if (activeIdx >= 0 && tabs[activeIdx]) tabs[activeIdx].dirty = true;
    status = t("st.unsaved");
    scheduleAutosave();
  }

  /** Activa la pestaña `i` (guarda antes la saliente). */
  async function selectTab(i: number) {
    if (i < 0 || i >= tabs.length) return;
    view = "explorer";
    if (i === activeIdx) return;
    await flush(true);
    syncActiveTab();
    activeIdx = i;
    const tb = tabs[i];
    current = tb.path;
    buffer = tb.content;
    dirty = tb.dirty;
    mtime = tb.mtime;
    loadedPath = null;
    void persistTabs();
  }

  /** Cierra la pestaña `i` y activa una vecina (o vacía la vista). */
  async function closeTab(i: number) {
    if (i < 0 || i >= tabs.length) return;
    const wasActive = i === activeIdx;
    if (wasActive) await flush(true);
    const next = tabs.filter((_, k) => k !== i);
    if (next.length === 0) {
      tabs = [];
      activeIdx = -1;
      current = null;
      buffer = "";
      dirty = false;
      mtime = null;
      loadedPath = null;
      void persistTabs();
      return;
    }
    let ni = activeIdx;
    if (i < activeIdx) ni = activeIdx - 1;
    else if (wasActive) ni = Math.min(i, next.length - 1);
    tabs = next;
    if (wasActive) {
      activeIdx = -1; // fuerza la recarga en selectTab
      await selectTab(ni);
    } else {
      activeIdx = ni;
      void persistTabs();
    }
  }

  async function openNote(path: string) {
    view = "explorer";
    const ex = tabs.findIndex((tb) => tb.path === path);
    if (ex >= 0) {
      await selectTab(ex);
      return;
    }
    await flush(true);
    try {
      const note = await api.readNote(path);
      syncActiveTab();
      tabs = [...tabs, { path, content: note.content, dirty: false, mtime: note.mtime_secs }];
      activeIdx = tabs.length - 1;
      current = path;
      buffer = note.content;
      dirty = false;
      mtime = note.mtime_secs;
      loadedPath = null;
      status = t("st.opened", api.baseName(path));
      void persistTabs();
    } catch (e) {
      logMsg("error", t("st.readErr", String(e)));
    }
  }

  function gotoHeading(h: Heading) {
    docMode = "edit";
    if (editorRef) editorRef.gotoLine(h.line);
    else pendingGoto = h.line;
  }

  async function openHit(path: string, line: number) {
    await openNote(path);
    docMode = "edit";
    pendingGoto = line;
  }

  async function switchView(v: View) {
    if (v !== "explorer") await flush(true);
    view = v;
  }

  // ---- espacios y diálogos nativos --------------------------------------

  async function addSpace() {
    const dir = await openDialog({ directory: true, multiple: false });
    if (typeof dir !== "string" || !config) return;
    let idx = config.spaces.indexOf(dir);
    if (idx < 0) {
      config.spaces = [...config.spaces, dir];
      idx = config.spaces.length - 1;
    }
    config.selected = idx;
    clearOpen();
    await persist();
    logMsg("info", t("st.spaceAdded", api.baseName(dir)));
  }

  // Cierra todo (cambio/eliminación de espacio). El llamante persiste la
  // config justo después, así que aquí solo se vacían los campos.
  function clearOpen() {
    void flush(true);
    tabs = [];
    activeIdx = -1;
    current = null;
    buffer = "";
    dirty = false;
    mtime = null;
    loadedPath = null;
    if (config) {
      config.open_tabs = [];
      config.active_tab = 0;
    }
  }

  async function removeCurrentSpace() {
    if (!config || config.selected == null) return;
    config.spaces = config.spaces.filter((_, i) => i !== config!.selected);
    config.selected = config.spaces.length ? 0 : null;
    clearOpen();
    await persist();
  }

  async function selectSpace(i: number) {
    if (!config) return;
    config.selected = i;
    clearOpen();
    await persist();
  }

  // ---- crear / renombrar / borrar ---------------------------------------

  function openDlg(
    kind: "note" | "folder" | "rename",
    parent: string,
    target?: string,
  ) {
    dialog = {
      kind,
      parent,
      target,
      name: kind === "rename" && target ? api.baseName(target) : "",
    };
  }

  async function submitDialog() {
    if (!dialog) return;
    const name = dialog.name.trim();
    if (!name) return;
    const d = dialog;
    dialog = null;
    try {
      if (d.kind === "folder") {
        await api.createFolder(d.parent, name);
        logMsg("info", t("st.folderCreated", name));
      } else if (d.kind === "note") {
        const path = await api.createNote(d.parent, name);
        reloadKey++;
        await openNote(path);
        return;
      } else if (d.kind === "rename" && d.target) {
        const np = await api.renamePath(d.target, name);
        const tgt = d.target;
        const remap = (p: string) =>
          p === tgt ? np : p.startsWith(tgt) ? np + p.slice(tgt.length) : p;
        tabs = tabs.map((tb) => ({ ...tb, path: remap(tb.path) }));
        if (current) current = remap(current);
        void persistTabs();
        logMsg("info", t("st.renamed", api.baseName(np)));
      }
      reloadKey++;
    } catch (e) {
      logMsg("warn", String(e));
    }
  }

  async function confirmDelete() {
    const path = pendingDelete;
    pendingDelete = null;
    if (!path) return;
    try {
      await api.deletePath(path);
      const hit = (p: string) => p === path || p.startsWith(path);
      if (tabs.some((tb) => hit(tb.path))) {
        const activePath = activeIdx >= 0 ? tabs[activeIdx].path : null;
        const survive = tabs.filter((tb) => !hit(tb.path));
        if (survive.length === 0) {
          tabs = [];
          activeIdx = -1;
          current = null;
          buffer = "";
          dirty = false;
          mtime = null;
          loadedPath = null;
          void persistTabs();
        } else {
          tabs = survive;
          const keep =
            activePath && !hit(activePath)
              ? survive.findIndex((tb) => tb.path === activePath)
              : 0;
          activeIdx = -1; // fuerza la recarga
          await selectTab(Math.max(0, keep));
        }
      }
      reloadKey++;
      logMsg("info", t("st.deleted"));
    } catch (e) {
      logMsg("error", t("st.deleteErr", String(e)));
    }
  }

  function onContextMenu(e: MouseEvent, path: string, isDir: boolean) {
    menu = { x: e.clientX, y: e.clientY, path, isDir };
  }

  // Clic derecho en el área vacía del explorador: menú raíz del espacio
  // (nueva nota/carpeta). Si el clic cae sobre una fila del árbol, su
  // propio manejador ya abrió el menú; aquí no hacemos nada.
  function onSpaceMenu(e: MouseEvent) {
    if ((e.target as HTMLElement).closest("button")) return;
    if (!selectedSpace) return;
    e.preventDefault();
    menu = { x: e.clientX, y: e.clientY, path: selectedSpace, isDir: true, root: true };
  }

  // ---- temas ------------------------------------------------------------

  async function setTheme(id: string) {
    if (!config || config.theme === id) return;
    config.theme = id;
    await persist();
    await applyTheme();
  }

  type ColorRole = "accent" | "bg" | "surface" | "text" | "muted" | "border";

  async function setCustomColor(role: ColorRole, rgb: Rgb) {
    if (!config) return;
    config.custom_theme[role] = rgb;
    config.theme = CUSTOM_ID;
    await persist();
    await applyTheme();
  }

  async function setCustomDark(dark: boolean) {
    if (!config) return;
    config.custom_theme.dark = dark;
    config.theme = CUSTOM_ID;
    await persist();
    await applyTheme();
  }

  function hexToRgb(hex: string): Rgb {
    const n = parseInt(hex.slice(1), 16);
    return [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  }
  function rgbToHex([r, g, b]: Rgb): string {
    return "#" + [r, g, b].map((v) => v.toString(16).padStart(2, "0")).join("");
  }

  async function startFromCurrent() {
    if (!config) return;
    const r = await api.resolveTheme(
      config.theme,
      false,
      $state.snapshot(config.custom_theme),
    );
    config.custom_theme = r.palette;
    config.theme = CUSTOM_ID;
    await persist();
    await applyTheme();
  }

  // T18/T20/T26: exportación de una nota (por ruta, desde el menú de la
  // nota). Lee el contenido para poder exportar también notas no abiertas.
  async function noteContent(path: string): Promise<string> {
    if (path === current) return buffer;
    return (await api.readNote(path)).content;
  }

  async function exportNoteHtml(path: string) {
    if (!config) return;
    const out = await saveDialog({
      defaultPath: `${api.stem(path)}.html`,
      filters: [{ name: "HTML", extensions: ["html"] }],
    });
    if (typeof out !== "string") return;
    try {
      const md = await noteContent(path);
      const r = await api.resolveTheme(
        config.theme,
        false,
        $state.snapshot(config.custom_theme),
      );
      await api.exportHtml(out, md, api.stem(path), r.palette);
      logMsg("info", t("st.htmlExported", out));
    } catch (e) {
      logMsg("error", t("st.exportErr", String(e)));
    }
  }

  async function exportNoteMd(path: string) {
    const out = await saveDialog({
      defaultPath: `${api.stem(path)}.md`,
      filters: [{ name: "Markdown", extensions: ["md"] }],
    });
    if (typeof out !== "string") return;
    try {
      await api.exportMd(out, await noteContent(path));
      logMsg("info", t("st.htmlExported", out));
    } catch (e) {
      logMsg("error", t("st.exportErr", String(e)));
    }
  }

  // PDF vía «imprimir» del webview sobre una vista de solo impresión (mismo
  // HTML saneado + tema; sin dependencias). El usuario elige «Guardar como
  // PDF» en el diálogo del sistema.
  async function exportNotePdf(path: string) {
    try {
      printHtml = await api.renderMarkdown(await noteContent(path));
      printing = true;
      await tick();
      window.print();
      printing = false;
    } catch (e) {
      logMsg("error", t("st.exportErr", String(e)));
    }
  }

  async function doExportTheme() {
    if (!config) return;
    const path = await saveDialog({
      defaultPath: `oxydice-${themeName.toLowerCase()}.json`,
      filters: [{ name: t("th.jsonFilter"), extensions: ["json"] }],
    });
    if (typeof path !== "string") return;
    const r = await api.resolveTheme(
      config.theme,
      false,
      $state.snapshot(config.custom_theme),
    );
    try {
      await api.exportTheme(path, r.palette);
      logMsg("info", t("st.themeExported", path));
    } catch (e) {
      logMsg("error", t("st.exportErr", String(e)));
    }
  }

  async function doImportTheme() {
    const path = await openDialog({
      multiple: false,
      filters: [{ name: t("th.jsonFilter"), extensions: ["json"] }],
    });
    if (typeof path !== "string" || !config) return;
    try {
      config.custom_theme = await api.importTheme(path);
      config.theme = CUSTOM_ID;
      await persist();
      await applyTheme();
      logMsg("info", t("st.themeImported"));
    } catch {
      logMsg("error", t("st.themeInvalid"));
    }
  }

  // ---- copias de seguridad ---------------------------------------------

  async function pickBackupDir() {
    const dir = await openDialog({ directory: true, multiple: false });
    if (typeof dir !== "string" || !config) return;
    config.backup_dir = dir;
    await persist();
  }

  async function runBackup(quiet: boolean) {
    if (!config?.backup_dir || !selectedSpace) {
      if (!quiet) logMsg("warn", t("st.backupNeedsDir"));
      return;
    }
    try {
      const out = await api.backupNow(selectedSpace, config.backup_dir);
      logMsg("info", t("st.backupCreated", out));
    } catch (e) {
      logMsg("error", t("st.backupErr", String(e)));
    }
  }

  // Credencial pendiente de guardar en el llavero (nunca se persiste en el
  // JSON de config; va al keychain del SO vía la capa shell).
  let secret = $state("");

  async function saveRemote() {
    if (config) await persist();
  }

  async function applySecret() {
    if (!config || config.remote.kind === "none" || !secret) return;
    try {
      await api.syncSetSecret(config.remote.kind, secret);
      secret = "";
      logMsg("info", t("st.credSaved"));
    } catch (e) {
      logMsg("error", t("st.keyringErr", String(e)));
    }
  }

  async function runSync() {
    try {
      const r = await api.syncRun();
      const moved = r.deleted_local + r.deleted_remote;
      logMsg(
        "info",
        t("st.syncSummary", r.uploaded, r.downloaded, moved) +
          (r.conflicts.length ? ` · ${r.conflicts.length}` : ""),
      );
      for (const c of r.conflicts) logMsg("warn", t("st.syncConflict", c));
      reloadKey++;
    } catch (e) {
      logMsg("error", t("st.syncErr", String(e)));
    }
  }

  async function setUiLang(l: Lang) {
    if (!config) return;
    config.lang = l;
    setLang(l);
    await persist();
  }

  // T6: filtro por etiqueta en el explorador.
  async function applyTagFilter() {
    const tag = tagFilter.trim();
    if (!selectedSpace || !tag) {
      tagHits = [];
      return;
    }
    try {
      tagHits = await api.notesWithTag(selectedSpace, tag);
    } catch {
      tagHits = [];
    }
  }

  // T6: editor de metadatos del frontmatter (round-trip en el core). Se
  // abre desde el menú contextual de la nota; si no es la nota abierta, la
  // abre primero para que el guardado actúe sobre `current`.
  async function openMetaFor(path: string) {
    if (path !== current) await openNote(path);
    const m = await api.docMeta(buffer);
    metaDlg = {
      title: m.title ?? "",
      status: m.status ?? "",
      author: m.author ?? "",
      tags: m.tags.join(", "),
    };
  }

  async function submitMetaDlg() {
    if (!metaDlg || !current) return;
    const d = metaDlg;
    metaDlg = null;
    try {
      mtime = await api.writeMeta(current, {
        title: d.title.trim(),
        status: d.status.trim(),
        author: d.author.trim(),
        tags: d.tags.split(",").map((s) => s.trim()).filter(Boolean),
      });
      const note = await api.readNote(current);
      buffer = note.content;
      dirty = false;
      syncActiveTab(); // la pestaña activa refleja el frontmatter nuevo
      loadedPath = null; // fuerza recargar el editor con el frontmatter nuevo
      logMsg("info", t("st.metaSaved"));
    } catch (e) {
      logMsg("error", t("st.saveErr", String(e)));
    }
  }

  const customRoles: { key: ColorRole; k: string }[] = [
    { key: "accent", k: "col.accent" },
    { key: "bg", k: "col.bg" },
    { key: "surface", k: "col.surface" },
    { key: "text", k: "col.text" },
    { key: "muted", k: "col.muted" },
    { key: "border", k: "col.border" },
  ];

  let syncState = $state<SyncState>("Disabled");
  const syncErr = $derived(
    typeof syncState === "object" ? syncState.Error : null,
  );
  const syncGlyph = $derived(
    syncState === "Synced"
      ? "✔"
      : syncState === "Syncing"
        ? "↻"
        : syncErr != null
          ? "!"
          : "○",
  );
  const syncTone = $derived(
    syncState === "Synced"
      ? "ok"
      : syncState === "Syncing"
        ? "busy"
        : syncErr != null
          ? "err"
          : "",
  );
  const syncTitle = $derived(
    syncState === "Synced"
      ? t("sync.synced")
      : syncState === "Syncing"
        ? t("sync.syncing")
        : syncErr != null
          ? t("sync.error", syncErr)
          : t("sync.none"),
  );
</script>

<svelte:window onclick={closeMenus} />

<div class="app">
  <!-- ===== rail de iconos (disposición estilo Obsidian) ===== -->
  <!-- Contraíble: el botón ▤ (o Ctrl/⌘+B) colapsa rail + barra lateral. -->
  {#if showSidebar}
    <nav class="rail">
      <button class="rail-btn" title={t("tb.toggleSidebar")}
        onclick={() => (showSidebar = false)} aria-label={t("tb.sidebar")}>▤</button>
      <div class="rail-mid">
        <button class="rail-btn" class:on={view === "explorer"}
          title={t("nav.explorer")} onclick={() => switchView("explorer")}>🗀</button>
        <button class="rail-btn" class:on={view === "search"}
          title={t("nav.search")} onclick={() => switchView("search")}>⌕</button>
      </div>
      <div class="rail-bottom">
        <button class="rail-btn" class:on={view === "settings"}
          title={t("nav.settings")} onclick={() => switchView("settings")}>⚙</button>
      </div>
    </nav>
  {:else}
    <button class="reveal" title={t("tb.toggleSidebar")}
      onclick={() => (showSidebar = true)} aria-label={t("tb.sidebar")}>▤</button>
  {/if}

  <!-- ===== barra lateral (explorador) ===== -->
  {#if showSidebar}
    <aside class="sidebar">
      <div class="side-tools">
        {#if config && config.spaces.length}
          <select class="select wide" value={config.selected ?? -1}
            onchange={(e) => selectSpace(+(e.currentTarget as HTMLSelectElement).value)}>
            {#each config.spaces as sp, i (sp)}
              <option value={i}>{api.baseName(sp)}</option>
            {/each}
          </select>
        {/if}
        {#if selectedSpace}
          <button class="icon-btn" title={t("sb.newNote")}
            onclick={() => openDlg("note", selectedSpace!)} aria-label={t("sb.newNote")}>✑</button>
          <button class="icon-btn" title={t("sb.newFolder")}
            onclick={() => openDlg("folder", selectedSpace!)} aria-label={t("sb.newFolder")}>🗀</button>
        {/if}
        <div class="menu-wrap">
          <button class="icon-btn" aria-label={t("tb.menu")}
            onclick={(e) => { e.stopPropagation(); showMenu = !showMenu; }}>☰</button>
          {#if showMenu}
            <div class="popup" role="menu">
              <button onclick={() => { showMenu = false; void addSpace(); }}>{t("tb.addSpace")}</button>
              <button disabled={config?.selected == null}
                onclick={() => { showMenu = false; void removeCurrentSpace(); }}>
                {t("tb.removeSpace")}
              </button>
            </div>
          {/if}
        </div>
      </div>

      {#if view !== "explorer"}
        <p class="hint side-hint">{t("sb.selectExplorer")}</p>
      {:else if !config}
        <p class="hint side-hint">{t("sb.loading")}</p>
      {:else if selectedSpace}
        <input class="text-in" placeholder={t("sb.filterTag")}
          bind:value={tagFilter}
          onkeydown={(e) => { if (e.key === "Enter") applyTagFilter(); }}
          oninput={() => { if (!tagFilter.trim()) tagHits = []; }} />
        <!-- svelte-ignore a11y_no_static_element_interactions -->
        <div class="tree-scroll" oncontextmenu={onSpaceMenu}>
          {#if tagFilter.trim() && tagHits.length > 0}
            <p class="hint small">{t("sb.tagResults", tagHits.length, tagFilter.trim().replace(/^#/, ""))}</p>
            {#each tagHits as p (p)}
              <button class="tag-hit" class:active={current === p}
                onclick={() => openNote(p)}>{api.stem(p)}</button>
            {/each}
          {:else}
            <Tree dir={selectedSpace} currentPath={current} {reloadKey}
              onOpen={openNote} {onContextMenu} />
          {/if}
        </div>
      {:else}
        <div class="empty-side">
          <p>{t("sb.noSpaces")}</p>
          <button class="btn" onclick={addSpace}>{t("sb.addSpace")}</button>
        </div>
      {/if}

      <div class="side-bottom">
        <span class="app-name">Oxydice <small>v{VERSION}</small></span>
        <span class="spacer"></span>
        <button class="sync {syncTone}" title={syncTitle}
          onclick={runSync} aria-label={t("tb.syncNow")}>{syncGlyph}</button>
        <button class="icon-btn" title={t("nav.settings")}
          onclick={() => switchView("settings")} aria-label={t("nav.settings")}>⚙</button>
      </div>
    </aside>
  {/if}

  <!-- ===== área central ===== -->
  <main class="center" class:canvas={editing}>
    <header class="main-head">
      <span class="crumb">
        {#if selectedSpace && current}
          {api.baseName(selectedSpace)} &nbsp;›&nbsp; {api.relativeTo(selectedSpace, current)}
        {:else if selectedSpace}
          {api.baseName(selectedSpace)}
        {:else}
          {t("tb.noSpace")}
        {/if}
      </span>
      <div class="spacer"></div>
      {#if view === "explorer" && current && !isCode}
        <button class="icon-btn" title={t("tb.outline")}
          onclick={() => (showOutline = !showOutline)}
          aria-label={t("dh.outline")}>{showOutline ? "◧" : "▢"}</button>
        <div class="segmented">
          <button class:on={docMode === "edit"} onclick={() => (docMode = "edit")}>{t("common.edit")}</button>
          <button class:on={docMode === "split"} onclick={() => (docMode = "split")}>{t("common.split")}</button>
          <button class:on={docMode === "read"} onclick={() => (docMode = "read")}>{t("common.view")}</button>
        </div>
      {/if}
      <span class="status">{dirty ? t("st.unsaved") : status}</span>
    </header>
    {#if view === "explorer" && tabs.length > 0}
      <div class="tabs" role="tablist">
        {#each tabs as tb, i (tb.path)}
          <div class="tab" class:on={i === activeIdx}>
            <button class="tab-label" role="tab" aria-selected={i === activeIdx}
              title={tb.path} onclick={() => selectTab(i)}
              onauxclick={(e) => { if (e.button === 1) closeTab(i); }}>
              <span class="tab-dot" class:dirty={tb.dirty}>●</span>
              <span class="tab-name">{api.stem(tb.path)}</span>
            </button>
            <button class="tab-x" aria-label={t("tab.close")} title={t("tab.close")}
              onclick={(e) => { e.stopPropagation(); closeTab(i); }}>✕</button>
          </div>
        {/each}
      </div>
    {/if}
    <div class="center-body">
      {#if view === "search"}
        <div class="pad">
          <h1>{t("se.title")}</h1>
          <input class="search-input" placeholder={t("se.placeholder")}
            bind:value={query} />
          {#if !selectedSpace}
            <p class="hint">{t("se.needSpace")}</p>
          {:else if query.trim() === ""}
            <p class="hint">{t("se.type")}</p>
          {:else}
            <p class="hint small">
              {t("se.summary", results.hits.length, results.files, results.truncated ? t("se.truncated") : "")}
            </p>
            <div class="results">
              {#each results.hits as hit (hit.path + ":" + hit.line)}
                {@const parts = api.splitByBytes(hit.text, hit.at[0], hit.at[1])}
                <div class="card hit">
                  <div class="hit-head">
                    <button class="link" onclick={() => openHit(hit.path, hit.line)}>
                      {api.baseName(hit.path)}
                    </button>
                    <span class="hint small">{api.relativeTo(selectedSpace, hit.path)}</span>
                  </div>
                  <div class="hit-body">
                    <span class="lineno">{String(hit.line + 1).padStart(4)}</span>
                    <span class="snippet">{parts[0]}<mark>{parts[1]}</mark>{parts[2]}</span>
                  </div>
                </div>
              {/each}
            </div>
          {/if}
        </div>
      {:else if view === "settings"}
        <div class="pad scroll">
          <h1>{t("set.title")}</h1>
          {#if config}
            <nav class="set-nav">
              {#each [["appearance", "set.appearance"], ["sync", "set.sync"], ["backup", "set.backup"], ["extensions", "set.extensions"], ["about", "set.about"]] as [s, key] (s)}
                <button class:on={settingsSection === s}
                  onclick={() => (settingsSection = s as typeof settingsSection)}>{t(key)}</button>
              {/each}
            </nav>

            {#if settingsSection === "appearance"}
            <!-- Apariencia -->
            <div class="card">
              <div class="section-label">{t("set.appearance")}</div>
              <div class="field-label">{t("set.lang")}</div>
              <select class="select" value={getLang()}
                onchange={(e) => setUiLang((e.currentTarget as HTMLSelectElement).value as Lang)}>
                {#each LANGS as l (l.id)}
                  <option value={l.id}>{l.name}</option>
                {/each}
              </select>

              <div class="field-label">{t("set.theme")}</div>
              <div class="chips">
                <button class="chip-btn" class:on={config.theme === SYSTEM_ID}
                  onclick={() => setTheme(SYSTEM_ID)}>{t("common.system")}</button>
                {#if themesOn}
                  {#each themeCatalog as th (th.id)}
                    <button class="chip-btn" class:on={config.theme === th.id}
                      onclick={() => setTheme(th.id)}>{th.name}</button>
                  {/each}
                {/if}
                <button class="chip-btn" class:on={config.theme === CUSTOM_ID}
                  onclick={() => setTheme(CUSTOM_ID)}>{t("common.custom")}</button>
              </div>

              <div class="field-label">{t("set.editorFont")}</div>
              <select class="select" value={config.editor_font}
                disabled={!!config.editor_font_family.trim()}
                onchange={async (e) => { config!.editor_font = (e.currentTarget as HTMLSelectElement).value as "Mono" | "Sans"; await persist(); }}>
                <option value="Mono">JetBrains Mono</option>
                <option value="Sans">Inter</option>
              </select>

              <div class="field-label">{t("set.systemFont")}</div>
              <input class="text-in" placeholder={t("set.systemFontPh")}
                value={config.editor_font_family}
                onchange={async (e) => { config!.editor_font_family = (e.currentTarget as HTMLInputElement).value; await persist(); }} />

              <div class="field-label">{t("set.fontSize", config.editor_font_size)}</div>
              <input type="range" min="11" max="22" step="1"
                value={config.editor_font_size}
                oninput={async (e) => { config!.editor_font_size = +(e.currentTarget as HTMLInputElement).value; await persist(); }} />

              <div class="field-label">{t("set.uiScale", config.ui_scale.toFixed(2))}</div>
              <input type="range" min="0.8" max="1.6" step="0.05"
                value={config.ui_scale}
                oninput={async (e) => { config!.ui_scale = +(e.currentTarget as HTMLInputElement).value; await persist(); await applyTheme(); }} />

              <hr />
              <div class="section-label">{t("set.customTheme")}</div>
              <p class="hint small">{t("set.customHint")}</p>
              <div class="row-btns">
                <button class="btn" onclick={startFromCurrent}>{t("set.fromCurrent")}</button>
                <button class="btn" onclick={doExportTheme}>{t("set.export")}</button>
                <button class="btn" onclick={doImportTheme}>{t("set.import")}</button>
              </div>
              {#each customRoles as r (r.key)}
                <div class="color-row">
                  <input type="color"
                    value={rgbToHex(config.custom_theme[r.key] as Rgb)}
                    oninput={(e) => setCustomColor(r.key, hexToRgb((e.currentTarget as HTMLInputElement).value))} />
                  <span>{t(r.k)}</span>
                </div>
              {/each}
              <label class="check">
                <input type="checkbox" checked={config.custom_theme.dark}
                  onchange={(e) => setCustomDark((e.currentTarget as HTMLInputElement).checked)} />
                {t("set.darkBase")}
              </label>
            </div>

            {/if}

            {#if settingsSection === "sync"}
            <!-- Sincronización -->
            <div class="card">
              <div class="section-label">{t("set.sync")}</div>
              <p class="hint small">{t("set.syncHint", syncTitle)}</p>

              <div class="field-label">{t("set.provider")}</div>
              <select class="select" value={config.remote.kind}
                onchange={async (e) => { config!.remote.kind = (e.currentTarget as HTMLSelectElement).value; await persist(); }}>
                <option value="none">{t("set.provNone")}</option>
                <option value="webdav">WebDAV</option>
                <option value="s3">{t("set.provS3")}</option>
              </select>

              {#if config.remote.kind !== "none"}
                <div class="field-label">{t("set.endpoint")}</div>
                <input class="text-in" placeholder="https://…"
                  bind:value={config.remote.endpoint} />
                <div class="field-label">{t("set.remoteRoot")}</div>
                <input class="text-in" placeholder="oxydice/mi-espacio"
                  bind:value={config.remote.root} />

                {#if config.remote.kind === "webdav"}
                  <div class="field-label">{t("set.user")}</div>
                  <input class="text-in" bind:value={config.remote.username} />
                {:else}
                  <div class="field-label">{t("set.bucket")}</div>
                  <input class="text-in" bind:value={config.remote.bucket} />
                  <div class="field-label">{t("set.region")}</div>
                  <input class="text-in" bind:value={config.remote.region} />
                  <div class="field-label">{t("set.accessKey")}</div>
                  <input class="text-in" bind:value={config.remote.access_key} />
                {/if}

                <div class="field-label">
                  {config.remote.kind === "webdav" ? t("set.password") : t("set.secretKey")}
                  <span class="hint small">{t("set.secretHint")}</span>
                </div>
                <div class="row-btns">
                  <input class="text-in grow" type="password"
                    placeholder="••••••••" bind:value={secret} />
                  <button class="btn" disabled={!secret} onclick={applySecret}>
                    {t("set.saveCred")}
                  </button>
                </div>

                <label class="check">
                  <input type="checkbox" checked={config.remote.auto}
                    onchange={async (e) => { config!.remote.auto = (e.currentTarget as HTMLInputElement).checked; await persist(); }} />
                  {t("set.autoSync")}
                </label>
                <div class="field-label">{t("set.interval")}</div>
                <input class="text-in" type="number" min="60"
                  value={config.remote.interval_secs}
                  onchange={async (e) => { config!.remote.interval_secs = +(e.currentTarget as HTMLInputElement).value; await persist(); }} />
              {/if}

              <div class="row-btns">
                <button class="btn" onclick={saveRemote}>{t("set.saveSettings")}</button>
                <button class="btn primary"
                  disabled={config.remote.kind === "none"} onclick={runSync}>
                  {t("tb.syncNow")}
                </button>
              </div>

              <div class="section-label">{t("set.registry")}</div>
              <div class="logbox">
                {#each log as e (e.ts + e.msg)}
                  <div class="logline {e.level}">[{e.ts}] {e.msg}</div>
                {/each}
              </div>
            </div>

            {/if}

            {#if settingsSection === "backup"}
            <!-- Copia de seguridad -->
            <div class="card">
              <div class="section-label">{t("set.backup")}</div>
              <p class="hint">{t("set.backupFolder", config.backup_dir ?? t("set.notSet"))}</p>
              <div class="row-btns">
                <button class="btn" onclick={pickBackupDir}>{t("set.pickFolder")}</button>
                {#if config.backup_dir}
                  <button class="btn" onclick={async () => { config!.backup_dir = null; await persist(); }}>{t("set.remove")}</button>
                {/if}
              </div>
              <label class="check">
                <input type="checkbox" checked={config.backup_on_save}
                  onchange={async (e) => { config!.backup_on_save = (e.currentTarget as HTMLInputElement).checked; await persist(); }} />
                {t("set.backupOnSave")}
              </label>
              <button class="btn" disabled={!config.backup_dir || !selectedSpace}
                onclick={() => runBackup(false)}>{t("set.backupNow")}</button>
            </div>

            {/if}

            {#if settingsSection === "extensions"}
            <!-- Extensiones -->
            <div class="card">
              <div class="section-label">{t("set.extensions")}</div>
              <p class="hint small">{t("set.extHint")}</p>
              {#each extRows as r (r.kind + r.id)}
                <div class="ext-row">
                  <label class="switch" title={r.id}>
                    <input type="checkbox" checked={r.enabled}
                      onchange={(e) => toggleExt(r.id, (e.currentTarget as HTMLInputElement).checked)} />
                  </label>
                  <span class="hint small">[{r.kind}]</span>
                  <span>{r.name}</span>
                  <span class="spacer"></span>
                  <span class="hint small">{r.detail}</span>
                </div>
              {/each}
            </div>

            {/if}

            {#if settingsSection === "about"}
            <!-- Acerca de -->
            <div class="card">
              <div class="section-label">{t("set.about")}</div>
              <p><strong>Oxydice v{VERSION}</strong></p>
              <p>{t("set.author")}</p>
              <p>{t("set.license")}</p>
              <div class="row-btns">
                <button class="btn" onclick={reportIssue}>{t("set.reportIssue")}</button>
                <button class="btn" onclick={doCheckUpdate}>{t("set.checkUpdate")}</button>
                <button class="btn" onclick={() => openUrl("https://github.com/Aleixenandros/oxydice")}>{t("set.repo")}</button>
              </div>
            </div>
            {/if}
          {/if}
        </div>
      {:else if !selectedSpace}
        <div class="empty">{t("empty.addSpace")}</div>
      {:else if !current}
        <div class="empty">{t("empty.pickNote")}</div>
      {:else if isCode}
        <!-- visor de código (T17): solo lectura, resaltado por extensión -->
        <div class="doc-header">
          <div class="doc-title-row">
            <h1>{api.baseName(current)}</h1>
            <span class="hint small">{t("dh.readonly")}</span>
          </div>
        </div>
        {#if config}
          <div class="edit-area">
            <div class="editor-wrap">
              <Editor bind:this={editorRef} content={buffer}
                font={config.editor_font} fontSize={config.editor_font_size}
                fontFamily={config.editor_font_family}
                onChange={() => {}} onSave={() => {}}
                readOnly={true} filename={api.baseName(current)} />
            </div>
          </div>
        {/if}
      {:else}
        <!-- cabecera de documento -->
        <div class="doc-header">
          <div class="doc-title-row">
            <h1>{meta.title ?? api.stem(current)}</h1>
          </div>
          <div class="meta-cards">
            <div class="meta-card">
              <span class="meta-k">{t("dh.modified")}</span>
              <span class="meta-v">{api.relTime(mtime)}</span>
            </div>
            <div class="meta-card">
              <span class="meta-k">{t("dh.status")}</span>
              <span class="meta-v">{meta.status ?? (dirty ? t("dh.unsaved") : t("dh.saved"))}</span>
            </div>
            {#if meta.author}
              <div class="meta-card">
                <span class="meta-k">{t("dh.authorK")}</span>
                <span class="meta-v">{meta.author}</span>
              </div>
            {/if}
          </div>
          {#if meta.tags.length}
            <div class="tags">
              <span class="hint">#</span>
              {#each meta.tags as tag (tag)}<span class="chip">#{tag}</span>{/each}
            </div>
          {/if}
        </div>

        {#if docMode === "read"}
          <div class="preview scroll">
            <!-- HTML saneado en el core con `ammonia` (sin scripts/handlers). -->
            <div class="md">{@html previewHtml}</div>
          </div>
        {:else}
          <div class="edit-area">
            {#if showOutline}
              <div class="outline">
                <div class="section-label">{t("dh.outline")}</div>
                <div class="outline-scroll">
                  {#if headings.length === 0}
                    <p class="hint small">{t("dh.noHeadings")}</p>
                  {:else}
                    {#each headings as h (h.line)}
                      <button class="outline-item"
                        style="padding-left:{(h.level - 1) * 12 + 4}px"
                        class:weak={h.level > 2}
                        onclick={() => gotoHeading(h)}>{h.text}</button>
                    {/each}
                  {/if}
                </div>
              </div>
            {/if}
            {#if config}
              <div class="editor-wrap">
                <Editor bind:this={editorRef} content={buffer}
                  font={config.editor_font} fontSize={config.editor_font_size}
                  fontFamily={config.editor_font_family}
                  onChange={onEdit} onSave={() => flush(false)}
                  onContextMenu={(x, y) => (fmtMenu = { x, y })} />
              </div>
            {/if}
            {#if docMode === "split"}
              <!-- Vista dividida: editor a la izquierda, vista previa a la
                   derecha (HTML saneado en el core con `ammonia`). -->
              <div class="preview split-pane scroll">
                <div class="md">{@html previewHtml}</div>
              </div>
            {/if}
          </div>
        {/if}
      {/if}
    </div>
    </main>
</div>

<!-- ===== menú contextual ===== -->
{#if menu}
  <div class="ctx" style="left:{menu.x}px; top:{menu.y}px" role="menu">
    {#if menu.isDir}
      <button onclick={() => { openDlg("note", menu!.path); menu = null; }}>{t("ctx.newNote")}</button>
      <button onclick={() => { openDlg("folder", menu!.path); menu = null; }}>{t("ctx.newFolder")}</button>
      {#if !menu.root}<hr />{/if}
    {/if}
    {#if !menu.root}
      <button onclick={() => { openDlg("rename", "", menu!.path); menu = null; }}>{t("ctx.rename")}</button>
      {#if !menu.isDir && /\.md$/i.test(menu.path)}
        <button onclick={() => { openMetaFor(menu!.path); menu = null; }}>{t("ctx.editMeta")}</button>
      {/if}
      {#if !menu.isDir && exporterOn && /\.md$/i.test(menu.path)}
        <div class="ctx-sub">
          <button class="ctx-parent">{t("ctx.export")} <span class="caret">▸</span></button>
          <div class="flyout">
            <button onclick={() => { exportNoteHtml(menu!.path); closeMenus(); }}>HTML</button>
            <button onclick={() => { exportNotePdf(menu!.path); closeMenus(); }}>PDF</button>
            <button onclick={() => { exportNoteMd(menu!.path); closeMenus(); }}>Markdown</button>
          </div>
        </div>
      {/if}
      <hr />
      <button class="danger" onclick={() => { pendingDelete = menu!.path; menu = null; }}>{t("ctx.delete")}</button>
    {/if}
  </div>
{/if}

<!-- ===== menú de formato del editor (T24) ===== -->
{#if fmtMenu}
  <div class="ctx" style="left:{fmtMenu.x}px; top:{fmtMenu.y}px" role="menu">
    {#each [["bold", "fmt.bold"], ["italic", "fmt.italic"], ["strike", "fmt.strike"], ["code", "fmt.code"]] as [k, key] (k)}
      <button onclick={() => { editorRef?.format(k); fmtMenu = null; }}>{t(key)}</button>
    {/each}
    <hr />
    {#each [["h1", "fmt.h1"], ["h2", "fmt.h2"], ["h3", "fmt.h3"], ["ul", "fmt.ul"], ["quote", "fmt.quote"], ["link", "fmt.link"]] as [k, key] (k)}
      <button onclick={() => { editorRef?.format(k); fmtMenu = null; }}>{t(key)}</button>
    {/each}
  </div>
{/if}

<!-- ===== diálogo crear/renombrar ===== -->
{#if dialog}
  <div class="overlay" role="presentation" onclick={() => (dialog = null)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1"
      onclick={(e) => e.stopPropagation()}>
      <h2>{dialog.kind === "rename" ? t("dlg.rename") : dialog.kind === "folder" ? t("dlg.newFolder") : t("dlg.newNote")}</h2>
      <!-- svelte-ignore a11y_autofocus -->
      <input autofocus placeholder={t("dlg.name")} bind:value={dialog.name}
        onkeydown={(e) => { if (e.key === "Enter") submitDialog(); if (e.key === "Escape") dialog = null; }} />
      <div class="row-btns">
        <button class="btn primary" onclick={submitDialog}>
          {dialog.kind === "rename" ? t("common.rename") : t("common.create")}
        </button>
        <button class="btn" onclick={() => (dialog = null)}>{t("common.cancel")}</button>
      </div>
    </div>
  </div>
{/if}

<!-- ===== confirmar borrado ===== -->
{#if pendingDelete}
  <div class="overlay" role="presentation" onclick={() => (pendingDelete = null)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1"
      onclick={(e) => e.stopPropagation()}>
      <h2>{t("del.title")}</h2>
      <p>{t("del.q", pendingDelete.endsWith(".md") ? t("del.note") : t("del.folder"))}</p>
      <p class="hint small">{api.baseName(pendingDelete)}</p>
      <div class="row-btns">
        <button class="btn danger" onclick={confirmDelete}>{t("del.title")}</button>
        <button class="btn" onclick={() => (pendingDelete = null)}>{t("common.cancel")}</button>
      </div>
    </div>
  </div>
{/if}

<!-- ===== editor de metadatos (T6) ===== -->
{#if metaDlg}
  <div class="overlay" role="presentation" onclick={() => (metaDlg = null)}>
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_interactive_supports_focus -->
    <div class="modal" role="dialog" aria-modal="true" tabindex="-1"
      onclick={(e) => e.stopPropagation()}>
      <h2>{t("mh.title")}</h2>
      <div class="field-label">{t("mh.fTitle")}</div>
      <input class="text-in" bind:value={metaDlg.title} />
      <div class="field-label">{t("mh.fStatus")}</div>
      <input class="text-in" bind:value={metaDlg.status} />
      <div class="field-label">{t("mh.fAuthor")}</div>
      <input class="text-in" bind:value={metaDlg.author} />
      <div class="field-label">{t("mh.fTags")}</div>
      <input class="text-in" bind:value={metaDlg.tags} />
      <div class="row-btns">
        <button class="btn primary" onclick={submitMetaDlg}>{t("common.save")}</button>
        <button class="btn" onclick={() => (metaDlg = null)}>{t("common.cancel")}</button>
      </div>
    </div>
  </div>
{/if}

<!-- ===== vista de impresión (T18 → PDF) ===== -->
{#if printing}
  <div class="print-root">
    <div class="md">{@html printHtml}</div>
  </div>
{/if}

<style>
  .app {
    display: flex;
    flex-direction: row;
    height: 100vh;
    background: var(--surface);
    color: var(--text);
    overflow: hidden;
  }
  h1 {
    font-size: 1.55rem; font-weight: 600; letter-spacing: -0.01em;
    line-height: 1.25; margin: 0 0 0.2rem;
  }
  h2 { font-size: 1.1rem; font-weight: 600; margin: 0 0 0.6rem; }
  hr { border: none; border-top: 1px solid var(--border); margin: 16px 0; }

  /* Foco accesible coherente (D1): un solo anillo de acento para todo lo
     interactivo. Solo con teclado (`:focus-visible`), nunca al click. */
  :where(button, input, select):focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: 1px;
    border-radius: 4px;
  }
  /* Respeta «reduce motion»: sin transiciones si el SO lo pide. */
  @media (prefers-reduced-motion: reduce) {
    * { transition: none !important; }
  }

  /* ===== rail de iconos (disposición estilo Obsidian) ===== */
  .rail {
    width: 44px; flex-shrink: 0;
    display: flex; flex-direction: column; align-items: center;
    gap: 4px; padding: 8px 0;
    background: var(--surface);
    border-right: 1px solid var(--border);
  }
  .rail-mid { display: flex; flex-direction: column; gap: 4px; margin-top: 8px; }
  .rail-bottom { margin-top: auto; }
  .rail-btn {
    width: 32px; height: 32px; border: none; background: none;
    color: var(--muted); border-radius: 6px; cursor: pointer;
    font-size: 16px; display: flex; align-items: center;
    justify-content: center;
    transition: background-color 0.12s ease, color 0.12s ease;
  }
  .rail-btn:hover { background: var(--hover); color: var(--text); }
  .rail-btn.on { color: var(--accent); background: var(--faint); }

  /* Iconos siempre monocromáticos: fuerza la presentación de texto (nunca
     emoji a color); heredan `color` del tema. */
  .rail-btn, .icon-btn, .sync, .reveal, .tab-x {
    font-variant-emoji: text;
  }

  /* Botón flotante para reabrir la columna izquierda cuando está contraída. */
  .reveal {
    position: fixed; top: 8px; left: 8px; z-index: 90;
    width: 30px; height: 30px; border: 1px solid var(--border);
    background: var(--surface); color: var(--muted); border-radius: 6px;
    cursor: pointer; font-size: 15px; display: flex; align-items: center;
    justify-content: center; opacity: 0.7;
    transition: opacity 0.12s ease, color 0.12s ease;
  }
  .reveal:hover { opacity: 1; color: var(--text); }

  /* cabecera mínima del documento (título/breadcrumb + controles) */
  .main-head {
    position: relative;
    display: flex; align-items: center; gap: 6px;
    height: 40px; padding: 0 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .crumb {
    position: absolute; left: 50%; transform: translateX(-50%);
    color: var(--muted); font-size: 0.82rem;
    max-width: 60%; overflow: hidden; text-overflow: ellipsis;
    white-space: nowrap; pointer-events: none;
  }
  .center-body { flex: 1; min-height: 0; display: flex; flex-direction: column; }
  .spacer { flex: 1; }

  /* Barra de pestañas (una por nota abierta), estilo Obsidian: plana,
     activa con realce tenue + acento, punto de «sin guardar». */
  .tabs {
    display: flex; flex-shrink: 0; overflow-x: auto;
    border-bottom: 1px solid var(--border); background: var(--surface);
  }
  .tab {
    display: flex; align-items: center; flex-shrink: 0;
    max-width: 200px; border-right: 1px solid var(--border);
    color: var(--muted);
    transition: background-color 0.12s ease, color 0.12s ease;
  }
  .tab:hover { background: var(--hover); }
  .tab.on { background: var(--faint); color: var(--accent); }
  .tab-label {
    display: flex; align-items: center; gap: 6px; min-width: 0;
    background: none; border: none; color: inherit; font: inherit;
    padding: 7px 4px 7px 12px; cursor: pointer;
  }
  .tab-name {
    overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
    font-size: 0.85rem;
  }
  .tab-dot {
    font-size: 0.6rem; line-height: 1; color: var(--muted);
    opacity: 0; flex-shrink: 0;
  }
  .tab-dot.dirty { opacity: 1; color: var(--accent); }
  .tab-x {
    background: none; border: none; color: var(--muted); cursor: pointer;
    font-size: 0.75rem; padding: 4px 8px; border-radius: 4px;
    opacity: 0.5; transition: opacity 0.12s ease, background-color 0.12s ease;
  }
  .tab-x:hover { opacity: 1; background: var(--hover); }
  .status { color: var(--muted); font-size: 0.78rem; min-width: 4ch; }
  .sync {
    background: none; border: none; cursor: pointer; font: inherit;
    color: var(--muted); width: 26px; height: 26px; border-radius: 4px;
  }
  .sync:hover { background: var(--hover); }
  .sync.ok { color: var(--success); }
  .sync.busy { color: var(--accent); }
  .sync.err { color: var(--danger); }
  .text-in {
    width: 100%; padding: 7px 10px; margin: 2px 0;
    background: var(--bg); color: var(--text);
    border: 1px solid var(--border); border-radius: 4px; font: inherit;
    transition: border-color 0.12s ease;
  }
  .text-in:hover { border-color: var(--muted); }
  .text-in:focus { border-color: var(--accent); }
  .text-in.grow { flex: 1; width: auto; }
  .menu-wrap { position: relative; }
  .popup {
    position: absolute; top: 110%; left: 0; z-index: 50;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 4px; box-shadow: 0 8px 24px var(--shadow); min-width: 200px;
  }
  .popup button {
    display: block; width: 100%; text-align: left; padding: 8px 12px;
    background: none; border: none; color: var(--text); cursor: pointer;
  }
  .popup button:hover:not(:disabled) { background: var(--hover); }
  .popup button:disabled { color: var(--muted); cursor: default; }

  .icon-btn {
    background: none; border: none; color: var(--muted);
    width: 32px; height: 28px; border-radius: 4px; cursor: pointer;
    font-size: 16px; display: inline-flex; align-items: center;
    justify-content: center; transition: background-color 0.12s ease,
      color 0.12s ease;
  }
  .icon-btn:hover { background: var(--hover); color: var(--text); }
  .icon-btn:active { background: var(--inactive); }

  .select {
    background: var(--bg); color: var(--text);
    border: 1px solid var(--border); border-radius: 4px; padding: 4px 6px;
    font: inherit; cursor: pointer; transition: border-color 0.12s ease;
  }
  .select:hover { border-color: var(--muted); }
  .select.wide { width: 100%; }

  .segmented {
    display: flex; border: 1px solid var(--border);
    border-radius: 4px; padding: 2px; gap: 2px;
  }
  .segmented button {
    background: none; border: none; color: var(--muted);
    padding: 3px 10px; border-radius: 3px; cursor: pointer; font: inherit;
    transition: background-color 0.12s ease, color 0.12s ease;
  }
  .segmented button:hover { color: var(--text); }
  .segmented button.on { background: var(--accent); color: var(--accent-contrast); }

  .body { display: flex; flex: 1; min-height: 0; }

  /* barra lateral */
  .sidebar {
    width: 266px; flex-shrink: 0; background: var(--surface);
    border-right: 1px solid var(--border); padding: 10px;
    display: flex; flex-direction: column; min-height: 0;
  }
  .row-btns { display: flex; gap: 6px; flex-wrap: wrap; margin: 8px 0; }
  .tree-scroll { overflow: auto; flex: 1; min-height: 0; }
  .empty-side { text-align: center; color: var(--muted); margin-top: 20px; }
  .empty-side p { margin-bottom: 8px; }

  .section-label {
    font-size: 0.7rem; font-weight: 700; letter-spacing: 0.08em;
    color: var(--muted); margin: 14px 0 8px; text-transform: uppercase;
    opacity: 0.9;
  }
  .card > .section-label:first-child { margin-top: 0; }
  .hint { color: var(--muted); }
  .hint.small { font-size: 0.78rem; }
  .field-label { display: block; margin: 12px 0 4px; font-size: 0.85rem; }

  .btn {
    background: var(--bg); color: var(--text); border: 1px solid var(--border);
    border-radius: 4px; padding: 6px 12px; cursor: pointer; font: inherit;
    transition: border-color 0.12s ease, background-color 0.12s ease,
      transform 0.06s ease;
  }
  .btn:hover:not(:disabled) {
    border-color: var(--accent); background: var(--hover);
  }
  .btn:active:not(:disabled) { transform: translateY(1px); }
  .btn:disabled { color: var(--muted); opacity: 0.55; cursor: default; }
  .btn.primary { background: var(--accent); color: var(--accent-contrast); border-color: var(--accent); }
  .btn.primary:hover:not(:disabled) { background: var(--accent); filter: brightness(1.08); }
  .btn.danger { background: var(--danger); color: var(--danger-contrast); border-color: var(--danger); }
  .btn.danger:hover:not(:disabled) { background: var(--danger); filter: brightness(1.08); }

  .center { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--surface); }
  .center.canvas { background: var(--bg); }
  .pad { padding: 22px 28px; min-height: 0; }
  .scroll { overflow: auto; flex: 1; }
  .empty {
    flex: 1; display: flex; align-items: center; justify-content: center;
    color: var(--muted); font-size: 1.1rem; font-weight: 500;
    letter-spacing: -0.005em; padding: 2rem; text-align: center;
  }

  /* cabecera de documento */
  /* D2: cabecera mínima; metadatos como línea tenue, sin cajas. */
  .doc-header { padding: 20px 28px 14px; border-bottom: 1px solid var(--border); }
  .meta-cards {
    display: flex; gap: 18px; margin-top: 8px; flex-wrap: wrap;
    font-size: 0.8rem;
  }
  .meta-card { display: flex; gap: 6px; align-items: baseline; }
  .meta-k {
    font-size: 0.7rem; color: var(--muted); letter-spacing: 0.04em;
    text-transform: uppercase;
  }
  .meta-v { color: var(--muted); }
  .tags { display: flex; flex-wrap: wrap; gap: 6px; align-items: center; margin-top: 10px; }
  .chip {
    border: 1px solid var(--border); border-radius: 4px;
    padding: 2px 8px; font-size: 0.78rem;
  }

  .edit-area { flex: 1; display: flex; min-height: 0; }
  .outline {
    width: 220px; flex-shrink: 0; border-right: 1px solid var(--border);
    padding: 8px; display: flex; flex-direction: column; min-height: 0;
  }
  .outline-scroll { overflow: auto; min-height: 0; }
  .outline-item {
    display: block; width: 100%; text-align: left; background: none;
    border: none; color: var(--text); cursor: pointer; padding: 4px 6px;
    border-radius: 4px; font: inherit;
  }
  .outline-item {
    transition: background-color 0.12s ease;
  }
  .outline-item:hover { background: var(--hover); }
  .outline-item.weak { color: var(--muted); }
  .editor-wrap { flex: 1; min-width: 0; min-height: 0; }

  .doc-title-row {
    display: flex; align-items: center; justify-content: space-between;
    gap: 12px;
  }
  .tag-hit {
    display: block; width: 100%; text-align: left; background: none;
    border: none; color: var(--text); cursor: pointer; padding: 6px 8px;
    border-radius: 4px; font: inherit;
  }
  .tag-hit:hover { background: var(--hover); }
  .tag-hit.active { background: var(--faint); color: var(--accent); }

  .preview { padding: 8px 28px 24px; }
  /* Vista dividida: panel de previa a la derecha del editor, mismo ancho. */
  .split-pane {
    flex: 1; min-width: 0; border-left: 1px solid var(--border);
    background: var(--surface);
  }

  /* búsqueda */
  .search-input {
    width: 100%; padding: 10px 12px; margin: 14px 0 12px;
    background: var(--bg); color: var(--text);
    border: 1px solid var(--border); border-radius: 4px; font: inherit;
    transition: border-color 0.12s ease;
  }
  .search-input:hover { border-color: var(--muted); }
  .search-input:focus { border-color: var(--accent); }
  .results { display: flex; flex-direction: column; gap: 8px; }
  /* D2: tarjetas planas (sin sombra), separación por espacio. */
  .card {
    background: var(--bg); border: 1px solid var(--border);
    border-radius: 6px; padding: 18px; margin-bottom: 16px;
  }
  .hit { padding: 12px; margin: 0; }
  .hit-head { display: flex; gap: 8px; align-items: baseline; }
  .hit-body { display: flex; gap: 8px; margin-top: 4px; }
  .lineno { color: var(--muted); font-family: "JetBrains Mono", monospace; }
  .snippet { font-family: "JetBrains Mono", monospace; font-size: 0.85rem; }
  mark { background: var(--accent); color: var(--accent-contrast); }
  .link {
    background: none; border: none; color: var(--accent);
    cursor: pointer; font: inherit; font-weight: 600; padding: 0;
    text-decoration: none;
  }
  .link:hover { text-decoration: underline; }

  /* ajustes */
  .chips, .row-btns { display: flex; flex-wrap: wrap; gap: 6px; }
  .chips { margin: 4px 0 8px; }
  .chip-btn {
    background: var(--bg); color: var(--text); border: 1px solid var(--border);
    border-radius: 4px; padding: 5px 10px; cursor: pointer; font: inherit;
    transition: border-color 0.12s ease, background-color 0.12s ease;
  }
  .chip-btn:hover:not(.on) { border-color: var(--muted); background: var(--hover); }
  .chip-btn.on { background: var(--accent); color: var(--accent-contrast); border-color: var(--accent); }
  .color-row { display: flex; align-items: center; gap: 8px; margin: 6px 0; }
  .color-row input[type="color"] {
    width: 32px; height: 24px; padding: 0; border: 1px solid var(--border);
    background: none; border-radius: 3px;
  }
  .check { display: flex; align-items: center; gap: 6px; margin: 10px 0; }
  .logbox {
    background: var(--bg); border: 1px solid var(--border);
    border-radius: 4px; padding: 10px; max-height: 150px; overflow: auto;
  }
  .logline { font-family: "JetBrains Mono", monospace; font-size: 0.75rem; color: var(--muted); }
  .logline.warn { color: var(--accent); }
  .logline.error { color: var(--danger); }
  .ext-row { display: flex; align-items: center; gap: 8px; padding: 6px 0; }
  .switch { display: inline-flex; }
  .switch input { accent-color: var(--accent); cursor: pointer; }

  /* T22: navegación por secciones de Ajustes (sobria, tipo Obsidian). */
  .set-nav {
    display: flex; flex-wrap: wrap; gap: 4px;
    margin: 10px 0 16px; border-bottom: 1px solid var(--border);
    padding-bottom: 8px;
  }
  .set-nav button {
    background: none; border: none; color: var(--muted); cursor: pointer;
    font: inherit; padding: 6px 10px; border-radius: 4px;
    transition: background-color 0.12s ease, color 0.12s ease;
  }
  .set-nav button:hover { background: var(--hover); color: var(--text); }
  .set-nav button.on { color: var(--accent); font-weight: 600; }

  /* menú contextual */
  .ctx {
    position: fixed; z-index: 100; background: var(--surface);
    border: 1px solid var(--border); border-radius: 4px;
    box-shadow: 0 8px 24px var(--shadow); min-width: 160px; padding: 4px;
  }
  .ctx button {
    display: block; width: 100%; text-align: left; padding: 7px 10px;
    background: none; border: none; color: var(--text); cursor: pointer;
    border-radius: 3px; font: inherit;
  }
  .ctx button:hover { background: var(--hover); }
  .ctx button.danger { color: var(--danger); }
  .ctx hr { margin: 4px 0; }

  /* Submenú «Exportar»: se despliega a la derecha al pasar el ratón. */
  .ctx-sub { position: relative; }
  .ctx-sub > .ctx-parent {
    display: flex; align-items: center; justify-content: space-between;
    gap: 8px;
  }
  .ctx-sub > .ctx-parent .caret { color: var(--muted); }
  .ctx-sub .flyout {
    display: none;
    position: absolute; left: 100%; top: -5px;
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 4px; box-shadow: 0 8px 24px var(--shadow);
    min-width: 140px; padding: 4px;
  }
  /* Puente invisible para no perder el hover al pasar al desplegable. */
  .ctx-sub::after {
    content: ""; position: absolute; left: 100%; top: 0;
    width: 8px; height: 100%;
  }
  .ctx-sub:hover .flyout { display: block; }

  /* modales */
  .overlay {
    position: fixed; inset: 0; background: rgba(0, 0, 0, 0.4);
    display: flex; align-items: center; justify-content: center; z-index: 200;
  }
  .modal {
    background: var(--surface); border: 1px solid var(--border);
    border-radius: 8px; padding: 22px; min-width: 340px; max-width: 90vw;
    box-shadow: 0 8px 28px var(--shadow);
  }
  .modal input {
    width: 100%; padding: 8px 10px; margin-bottom: 12px;
    background: var(--bg); color: var(--text);
    border: 1px solid var(--border); border-radius: 4px; font: inherit;
    transition: border-color 0.12s ease;
  }
  .modal input:focus { border-color: var(--accent); }
  .overlay { backdrop-filter: blur(1.5px); }

  /* vista de lectura (Markdown renderizado en el core) */
  .md :global(h1),
  .md :global(h2),
  .md :global(h3) { margin: 1.2em 0 0.5em; }
  .md :global(p) { margin: 0.6em 0; line-height: 1.7; }
  .md :global(a) { color: var(--accent); }
  .md :global(code) {
    background: var(--code-bg); padding: 0.15em 0.35em;
    border-radius: 3px; font-family: "JetBrains Mono", monospace; font-size: 0.9em;
  }
  .md :global(pre) {
    background: var(--code-bg); padding: 12px 14px;
    border-radius: 4px; overflow: auto;
  }
  .md :global(pre code) { background: none; padding: 0; }
  .md :global(blockquote) {
    border-left: 3px solid var(--accent); margin: 0.8em 0;
    padding-left: 12px; color: var(--muted);
  }
  .md :global(table) { border-collapse: collapse; margin: 0.8em 0; }
  .md :global(th),
  .md :global(td) { border: 1px solid var(--border); padding: 6px 10px; }
  .md :global(img) { max-width: 100%; }
  .md :global(hr) { border: none; border-top: 1px solid var(--border); }

  /* Vista de impresión (T18 → PDF): solo en pantalla está oculta; al
     imprimir se oculta la app y se muestra solo el documento. */
  .print-root { display: none; }
  @media print {
    :global(.app) { display: none !important; }
    .print-root {
      display: block;
      max-width: 46rem;
      margin: 0 auto;
      padding: 1rem;
      color: #000;
      background: #fff;
    }
  }
</style>
