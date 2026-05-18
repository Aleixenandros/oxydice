<script lang="ts">
  // Árbol del explorador: carpetas primero y orden alfabético (el orden lo
  // fija `vault::entries` en el core). Carga perezosa al expandir; el disco
  // es la verdad, así que `reloadKey` fuerza re-listar tras crear/renombrar/
  // borrar. Recursivo: cada carpeta expandida monta otro `Tree`.
  import Self from "./Tree.svelte";
  import { listDir, type Entry } from "./api";

  interface Props {
    dir: string;
    currentPath: string | null;
    reloadKey: number;
    onOpen: (path: string) => void;
    onContextMenu: (e: MouseEvent, path: string, isDir: boolean) => void;
  }

  let { dir, currentPath, reloadKey, onOpen, onContextMenu }: Props = $props();

  let entries = $state<Entry[]>([]);
  let expanded = $state<Set<string>>(new Set());

  $effect(() => {
    void reloadKey;
    const d = dir;
    listDir(d)
      .then((e) => {
        if (d === dir) entries = e;
      })
      .catch(() => {
        entries = [];
      });
  });

  function toggle(path: string) {
    const next = new Set(expanded);
    next.has(path) ? next.delete(path) : next.add(path);
    expanded = next;
  }
</script>

<ul class="tree">
  {#each entries as e (e.path)}
    <li>
      {#if e.is_dir}
        <button
          type="button"
          class="row folder"
          onclick={() => toggle(e.path)}
          oncontextmenu={(ev) => {
            ev.preventDefault();
            onContextMenu(ev, e.path, true);
          }}
        >
          <span class="caret" class:open={expanded.has(e.path)}>▸</span>
          <span class="label">{e.name}</span>
        </button>
        {#if expanded.has(e.path)}
          <div class="children">
            <Self
              dir={e.path}
              {currentPath}
              {reloadKey}
              {onOpen}
              {onContextMenu}
            />
          </div>
        {/if}
      {:else}
        <button
          type="button"
          class="row note"
          class:active={currentPath === e.path}
          onclick={() => onOpen(e.path)}
          oncontextmenu={(ev) => {
            ev.preventDefault();
            onContextMenu(ev, e.path, false);
          }}
        >
          <span class="label">{e.name}</span>
        </button>
      {/if}
    </li>
  {/each}
</ul>

<style>
  .tree {
    list-style: none;
    margin: 0;
    padding: 0;
  }
  .children {
    margin-left: 12px;
    border-left: 1px solid var(--border);
    padding-left: 4px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 5px 8px;
    border: none;
    border-radius: 4px;
    background: transparent;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
    min-height: 27px;
    transition: background-color 0.12s ease, color 0.12s ease;
  }
  .row:hover {
    background: var(--hover);
  }
  .row:focus-visible {
    outline: 2px solid var(--accent);
    outline-offset: -2px;
  }
  .row.active {
    background: var(--faint);
    color: var(--accent);
  }
  .caret {
    display: inline-block;
    width: 0.8em;
    color: var(--muted);
    transition: transform 0.12s ease;
  }
  .caret.open {
    transform: rotate(90deg);
  }
  .label {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
