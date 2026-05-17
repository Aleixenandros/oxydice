# Cómo crear una extensión (plugin/módulo) de Oxydice

> En Oxydice una «extensión» (también «plugin» o «módulo» en conversación)
> es **in-tree**: se compila dentro de `oxydice-core`. No hay carga dinámica
> de `.so`/`.dll` ni WASM (decisión por ABI/seguridad/empaquetado; ver
> `arquitectura.md`). Añadir una extensión = implementar un *trait* y
> registrarla en el `Registry`.

- **Dónde vive:** `crates/oxydice-core/src/ext/`
- **Contrato base:** todas implementan `Extension`
- **Tipos actuales (`ExtKind`):** `Theme`, `Sync`
- **Registro:** `ext::Registry::builtin()`

---

## 1. El contrato

```rust
pub trait Extension {
    fn id(&self) -> &'static str;   // id estable y persistible
    fn name(&self) -> &'static str; // nombre visible
    fn kind(&self) -> ExtKind;      // Theme | Sync (amplía el enum si añades otro)
}
```

Sobre ese contrato hay *traits* especializados:

- **`ThemeExtension`** — aporta un catálogo de paletas:
  `fn themes(&self) -> Vec<ThemeEntry>` (cada `ThemeEntry { id, name, palette }`).
- **`SyncProvider`** — backend de sincronización:
  `fn state(&self) -> SyncState` y `fn sync(&mut self) -> Result<(), String>`.
  El **transporte real** (red) se implementa aparte con el trait
  `sync::Remote` (ver `sync/backend.rs` para el ejemplo OpenDAL); el motor
  `sync::sync_space` lo consume. Una `SyncProvider` envuelve esa fontanería.

> Para una clase nueva (p. ej. un **visor de código** o un **exportador**)
> se añade una variante a `ExtKind`, un *trait* nuevo (`CodeViewer`,
> `Exporter`, …) y su `Vec<...>` en `Registry`. Sigue el mismo patrón que
> `ThemeExtension`/`SyncProvider`.

## 2. Pasos

1. **Crea el archivo** `crates/oxydice-core/src/ext/mi_extension.rs`
   (o parte de la plantilla [`plantilla-extension.rs`](./plantilla-extension.rs)).
2. **Declárala** en `ext/mod.rs`: `pub mod mi_extension;`.
3. **Implementa** `Extension` + el trait especializado de su `ExtKind`.
4. **Regístrala** en `Registry::builtin()` (empuja un `Box::new(...)` al
   `Vec` correspondiente: `themes`, `syncs`, …).
5. **La UI no se toca**: la pestaña *Extensiones* de Ajustes itera el
   `Registry` (`listing()`), así que aparece sola. Si tu clase necesita UI
   propia, añádela en `apps/oxydice/src/routes/+page.svelte` leyendo de un
   comando Tauri nuevo (no metas lógica en el frontend).
6. **Tests headless** en el propio archivo (`#[cfg(test)] mod tests`). Es el
   estándar del core: nada de UI para probar la lógica.
7. **i18n**: cualquier cadena visible nueva va a
   `apps/oxydice/src/lib/i18n.svelte.ts` (ES/EN/DE/PT) con su clave; no
   escribas texto fijo en los componentes.

## 3. Reglas de oro

- **El core es agnóstico de UI.** Nada de `tauri`/DOM en `oxydice-core`.
- **Sin pérdida de datos.** Operaciones sobre archivos: escritura atómica
  (`vault::write_atomic`) y, si hay remoto, reconciliación a 3 bandas.
- **Errores como `Result<_, String>`**, nunca `panic!` en rutas de datos.
- **`id` estable**: se persiste en `config.json`; no lo cambies a la ligera.
- **Calidad**: `cargo test` y `cargo clippy --workspace` sin warnings.

## 4. Exponerla por Tauri (si hace falta)

Si la extensión necesita acciones desde la UI, añade un `#[tauri::command]`
en `apps/oxydice/src-tauri/src/lib.rs` que **solo traduzca** argumentos y
llame al core, regístralo en `invoke_handler![…]`, y un *wrapper* tipado en
`apps/oxydice/src/lib/api.ts`. Mantén la lógica en el core.

Ver también: `arquitectura.md` §4–§5, `tareas.md` (backlog T17 visor de
código, T18 exportador multiformato).
