<script lang="ts">
  // Editor Markdown sobre CodeMirror 6. El resaltado de lenguajes anidados
  // lo hace Lezer en el cliente (decisión de arquitectura: Tree-sitter fuera
  // de v1). El aspecto deriva de las variables CSS del tema; ningún color
  // fijo aquí. La fuente de verdad del texto la lleva el padre (autoguardado).
  import { onMount } from "svelte";
  import { EditorState, Annotation } from "@codemirror/state";
  import {
    EditorView,
    keymap,
    lineNumbers,
    highlightActiveLine,
    highlightActiveLineGutter,
    drawSelection,
  } from "@codemirror/view";
  import {
    history,
    defaultKeymap,
    historyKeymap,
    indentWithTab,
  } from "@codemirror/commands";
  import {
    syntaxHighlighting,
    HighlightStyle,
    bracketMatching,
    LanguageDescription,
    type LanguageSupport,
  } from "@codemirror/language";
  import { markdown } from "@codemirror/lang-markdown";
  import { languages } from "@codemirror/language-data";
  import { tags as t } from "@lezer/highlight";

  interface Props {
    content: string;
    font: "Mono" | "Sans";
    fontSize: number;
    onChange: (value: string) => void;
    onSave: () => void;
    /** Solo lectura (visor de código, T17): sin edición ni autoguardado. */
    readOnly?: boolean;
    /** Nombre de archivo para detectar el lenguaje (visor de código). */
    filename?: string;
  }

  let {
    content,
    font,
    fontSize,
    onChange,
    onSave,
    readOnly = false,
    filename = "",
  }: Props = $props();

  // Lenguaje resuelto para el visor de código (carga perezosa de Lezer).
  let codeLang = $state<LanguageSupport | null>(null);

  let host: HTMLDivElement;
  let view: EditorView | undefined;

  // Marca los cambios programáticos para no confundirlos con edición real.
  const External = Annotation.define<boolean>();

  const family = $derived(
    font === "Mono"
      ? "'JetBrains Mono', ui-monospace, monospace"
      : "'Inter', ui-sans-serif, system-ui, sans-serif",
  );

  // Resaltado de sintaxis Markdown mapeado a variables del tema (guía §4.2).
  const highlightStyle = HighlightStyle.define([
    { tag: t.heading, color: "var(--text)", fontWeight: "700" },
    { tag: [t.strong], color: "var(--text)", fontWeight: "700" },
    { tag: [t.emphasis], fontStyle: "italic" },
    { tag: [t.link, t.url], color: "var(--accent)" },
    { tag: [t.monospace], color: "var(--accent)" },
    { tag: [t.quote], color: "var(--muted)", fontStyle: "italic" },
    { tag: [t.list], color: "var(--accent)" },
    { tag: [t.strikethrough], textDecoration: "line-through" },
    { tag: [t.comment], color: "var(--muted)" },
    { tag: [t.keyword], color: "var(--accent)" },
    { tag: [t.processingInstruction], color: "var(--muted)" },
  ]);

  function baseTheme() {
    return EditorView.theme({
      "&": {
        height: "100%",
        backgroundColor: "var(--bg)",
        color: "var(--text)",
        fontSize: `${fontSize}px`,
      },
      ".cm-scroller": {
        fontFamily: family,
        lineHeight: "1.6",
        overflow: "auto",
      },
      ".cm-content": { padding: "8px 16px 8px 14px", caretColor: "var(--accent)" },
      "&.cm-focused": { outline: "none" },
      ".cm-gutters": {
        backgroundColor: "var(--bg)",
        color: "var(--muted)",
        border: "none",
        borderRight: "1px solid var(--border)",
      },
      ".cm-lineNumbers .cm-gutterElement": {
        padding: "0 8px 0 14px",
        minWidth: "2ch",
      },
      ".cm-activeLine": { backgroundColor: "var(--faint)" },
      ".cm-activeLineGutter": {
        backgroundColor: "var(--faint)",
        color: "var(--text)",
      },
      "&.cm-focused .cm-selectionBackground, .cm-selectionBackground, ::selection":
        { backgroundColor: "var(--selection)" },
      ".cm-cursor, .cm-dropCursor": { borderLeftColor: "var(--accent)" },
    });
  }

  function languageExt() {
    // Visor de código (T17): lenguaje por extensión; si no se reconoce,
    // texto plano. Markdown en el caso normal de edición.
    if (readOnly) return codeLang ? [codeLang] : [];
    return [markdown({ codeLanguages: languages })];
  }

  function extensions() {
    return [
      lineNumbers(),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      drawSelection(),
      history(),
      bracketMatching(),
      ...languageExt(),
      syntaxHighlighting(highlightStyle),
      EditorView.lineWrapping,
      ...(readOnly
        ? [EditorState.readOnly.of(true), EditorView.editable.of(false)]
        : []),
      keymap.of([
        {
          key: "Mod-s",
          preventDefault: true,
          run: () => {
            onSave();
            return true;
          },
        },
        indentWithTab,
        ...defaultKeymap,
        ...historyKeymap,
      ]),
      baseTheme(),
      EditorView.updateListener.of((u) => {
        if (
          u.docChanged &&
          !u.transactions.some((tr) => tr.annotation(External))
        ) {
          onChange(u.state.doc.toString());
        }
      }),
    ];
  }

  // Resuelve el lenguaje del visor de código a partir del nombre de archivo.
  $effect(() => {
    if (!readOnly || !filename) {
      codeLang = null;
      return;
    }
    const desc = LanguageDescription.matchFilename(languages, filename);
    if (!desc) {
      codeLang = null;
      return;
    }
    let alive = true;
    desc.load().then((s) => {
      if (alive) codeLang = s;
    });
    return () => {
      alive = false;
    };
  });

  onMount(() => {
    view = new EditorView({
      parent: host,
      state: EditorState.create({ doc: content, extensions: extensions() }),
    });
    return () => view?.destroy();
  });

  // Reconstruye el estado al cambiar fuente/tamaño/lenguaje/modo (mantiene
  // el doc). Cubre el visor de código cuando el lenguaje termina de cargar.
  $effect(() => {
    void family;
    void fontSize;
    void readOnly;
    void codeLang;
    if (!view) return;
    const doc = view.state.doc.toString();
    view.setState(EditorState.create({ doc, extensions: extensions() }));
  });

  /** Sustituye el contenido sin emitir `onChange` (cambio de nota). */
  export function setContent(text: string) {
    if (!view) return;
    view.dispatch({
      changes: { from: 0, to: view.state.doc.length, insert: text },
      annotations: External.of(true),
    });
  }

  /** Lleva el cursor al inicio de `line` (base 0) y enfoca el editor. */
  export function gotoLine(line: number) {
    if (!view) return;
    const n = Math.min(line + 1, view.state.doc.lines);
    const pos = view.state.doc.line(Math.max(1, n)).from;
    view.dispatch({
      selection: { anchor: pos },
      scrollIntoView: true,
      annotations: External.of(true),
    });
    view.focus();
  }

  export function focus() {
    view?.focus();
  }
</script>

<div class="editor" bind:this={host}></div>

<style>
  .editor {
    height: 100%;
    min-height: 0;
    overflow: hidden;
  }
</style>
