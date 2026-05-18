# Changelog

Todos los cambios notables de Oxydice (antes RustNotes) se documentan en
este archivo.

## [0.8.0] - 2026-05-18

### Added

- **Pestañas (una por nota)**: barra de pestañas en el área central; abrir
  una nota crea su pestaña (si ya está abierta, salta a ella). Punto de
  «sin guardar», cierre con ✕ o botón central del ratón; conmutar conserva
  las ediciones sin guardar de cada pestaña. **Se restauran entre sesiones**
  (`config.open_tabs` / `config.active_tab`); renombrar o eliminar (incluido
  carpetas) reescribe o cierra las pestañas afectadas.
- **Vista dividida**: tercer modo del documento (Editar · Dividir · Vista)
  con el editor y la vista previa en vivo lado a lado.
- **Columna izquierda contraíble**: el botón ▤ (o `Ctrl/⌘ + B`) colapsa el
  rail de iconos junto con la barra lateral; botón flotante para reabrir.
- **Editar metadatos desde el menú de la nota**: clic derecho sobre una
  nota `.md` → «Editar metadatos» (abre la nota si no lo está). Se quitó el
  botón de la cabecera del documento.
- **Módulos activables/desactivables (T21)**: cada módulo/extensión (temas,
  sync, visor de código, exportador) se puede activar o desactivar desde
  Ajustes › Extensiones; el estado se persiste (`config.disabled_ext`) y se
  respeta (visor desactivado → el árbol solo lista `.md`; exportador o temas
  desactivados → no se ofrecen).
- **Ajustes por secciones (T22)**: navegación por secciones (Apariencia,
  Sincronización, Copia de seguridad, Extensiones, Acerca de), una a la vez.
- **Acerca de (T23)**: autor «Alejandro Soriano»; enlace «Reportar
  incidencia»; botón «Comprobar actualizaciones» que solo comprueba la
  release más reciente en GitHub y, si hay una nueva, abre la web (sin
  auto-updater).
- **Menú de formato en el editor (T24)**: clic derecho dentro del editor →
  negrita, cursiva, tachado, código, H1/H2/H3, lista, cita, enlace, sobre la
  selección. Suprime el menú nativo del webview dentro del editor.

- **Menú nativo del webview eliminado (T19)**: el clic derecho ya no muestra
  «Inspeccionar»/devtools; solo aparecen los menús propios.
- **Exportar en el menú de la nota (T20)**: se quitaron los botones de
  exportación de la cabecera; ahora el clic derecho sobre una nota `.md`
  ofrece «Exportar ▸» con submenú **HTML · PDF · Markdown** (junto a
  Renombrar/Eliminar). Funciona también sobre notas no abiertas.
- **Exportar a Markdown (T26)**: nuevo formato `.md` del exportador.
- **Fuente del sistema en el editor (T25)**: campo para usar cualquier
  familia instalada; vacío = fuente integrada (Mono/Sans).

### Changed

- **Rediseño minimalista estilo Obsidian (D2)**: tarjetas planas (sin
  sombra), marca sobria sin caja de acento, navegación y selección con
  realce tenue (no relleno fuerte), metadatos del documento como línea
  ligera sin cajas, modales más discretos. Colores siempre desde el tema.
- **Iconos monocromáticos**: la lupa de búsqueda (emoji a color) pasa a un
  glifo de texto `⌕`; `font-variant-emoji: text` fuerza la presentación
  monocromática del resto. Todos heredan el color del tema.
- Limpieza de CSS muerto que dejó el rediseño D2.

### Fixed

- El texto ya se puede **seleccionar** (el webview lo impedía); el cromo
  (botones/navegación) no es seleccionable.
- **Clic derecho en el área vacía del explorador** ahora abre «Nueva nota /
  Nueva carpeta» en la raíz del espacio; antes el menú contextual solo
  existía sobre las filas del árbol.

## [0.7.0] - 2026-05-17

### Changed

- **Renombrado a Oxydice** y **migración de egui a Tauri 2 + Svelte 5 +
  CodeMirror 6**. Workspace Cargo: `crates/oxydice-core` (lógica agnóstica
  de UI, con tests) + `apps/oxydice` (capa Tauri + frontend SvelteKit SPA).
  El frontend reproduce fielmente la UI egui anterior.
- **Render Markdown en el core** (`pulldown-cmark`), HTML **saneado** con
  `ammonia`: única fuente de verdad de la vista previa, sin XSS.
- **Tema → variables CSS** resueltas en el core; el frontend solo las fija
  en `:root`. Ningún color codificado en la UI.
- **Licencia cambiada de MIT a Apache-2.0** (`LICENSE`, manifiestos, UI).
- Editor sobre **CodeMirror 6** (Markdown + lenguajes anidados Lezer),
  números de línea, autoguardado y salto a línea desde esquema/búsqueda.
- Release con **`tauri-action`** (matriz Linux/macOS/Windows); el bundler
  de Tauri genera `.deb`/`.rpm`/AppImage/`.dmg`/instalador. Se elimina el
  empaquetado manual de la era egui.
- Tooling al día: Vite 8, vite-plugin-svelte 7, TypeScript 6.

### Added

- **Sincronización (motor + transporte)**: índice *sidecar*
  `.oxydice/sync.json` (hash base + versión remota), reconciliación a
  **3 bandas** (base/local/remoto), **conflicto = bifurcación
  determinista** (sin pérdida de datos), **escrituras atómicas**
  (temp+rename). Backend **OpenDAL** para **WebDAV y S3** con claves del
  usuario; secretos en el **llavero del SO** (`keyring`). Comandos Tauri,
  estado `SyncState` en la barra superior y *worker* (intervalo + foco).
- **Internacionalización (T1)**: ES/EN/DE/PT, detección del idioma del SO,
  selector en Ajustes › Apariencia, preferencia persistida; toda la UI con
  cadenas externalizadas (cambio de idioma en caliente).
- **Frontmatter de escritura (T6)**: editor de metadatos con *round-trip*
  que preserva claves desconocidas y cuerpo; **filtro por etiqueta** en el
  explorador.
- **Visor de código (T17)**: además de `.md`, abre y muestra resaltados
  (solo lectura, lenguaje por extensión vía Lezer) archivos html/css/php/
  js/ts/rs/py/json/yaml/toml… El explorador los lista.
- **Exportador (T18)**: exportar la nota a **HTML autónomo** (cuerpo
  saneado + tema embebido, lo construye el core) y a **PDF** vía impresión
  del *webview* (sin dependencias).
- **Pasada de diseño (D1)**: foco accesible (`:focus-visible` de acento),
  transiciones sutiles, estados hover/active/disabled, jerarquía
  tipográfica, ritmo de espaciado, profundidad de tarjetas, soporte de
  *prefers-reduced-motion*. Todo derivado de las variables del tema.
- `docs/`: guía de creación de extensiones (`extensiones.md`) + plantilla
  (`plantilla-extension.rs`).

## [0.6.0] - 2026-05-16

### Added

- **Autoguardado**: las notas se escriben a disco solas tras dejar de
  escribir; ya no hay botón «Guardar». Se vacía lo pendiente al cambiar de
  nota, al cerrar y con `Ctrl+S`.
- **Búsqueda global** (icono en el rail): busca texto en todas las notas del
  espacio, con ruta, número de línea, fragmento y término resaltado; abrir
  un resultado salta a la línea.
- **Vista de lectura con metadatos**: cabecera con título, fecha de
  modificación, estado, autor y *chips* de etiquetas leídos del
  *frontmatter* YAML; alternancia **Editar / Vista**.
- **Esquema (outline)**: panel con el árbol de encabezados del documento;
  al pulsar uno, el cursor salta a esa línea.
- **Ajustes** rediseñados como página completa con tarjetas: apariencia
  (tema, **fuente del editor**, **tamaño de fuente**, escala), proveedores
  de sincronización con **registro de actividad**, copia de seguridad,
  extensiones y «acerca de».
- **Medianil de números de línea** en el editor, alineado al ajuste real.

### Changed

- Rediseño guiado por los mockups: **rail de navegación** (Explorador /
  Buscar / Ajustes) con marca, cabeceras de documento, tarjetas, *chips* y
  control segmentado, manteniendo la paleta del tema.

### Fixed

- El tema **«Sistema»** ya no se confunde con oscuro: si el SO no declara su
  preferencia (habitual en Linux/X11) se cae a **claro**, no a oscuro.
- Menús contextuales del árbol (clic derecho) revisados y consistentes en
  carpetas y notas.

## [0.5.0] - 2026-05-16

### Added

- **Sistema de extensiones** *in-tree* (traits compilados en el binario):
  los temas y la sincronización son extensiones. Nueva pestaña
  **Extensiones** en Preferencias e indicador de estado de sincronización
  en la barra superior. La sincronización es de momento interfaz + stub
  (los backends reales —Git, S3, Drive— quedan como backlog).
- **Fuentes** Inter (interfaz) y JetBrains Mono (editor) empotradas en el
  binario, bajo licencia SIL Open Font License 1.1.
- **Instalador de Windows** (`rustnotes-<versión>-windows-setup.exe`,
  Inno Setup) además del ejecutable portable.

### Changed

- Guía de estilo aplicada: acento **naranja Rust `#ce412b`**, presets
  Claro/Oscuro a los colores exactos de la guía, esquinas de 4px,
  jerarquía tipográfica y bordes sutiles en inputs y botones secundarios.
- El tema activo y el de sincronización se guardan como identificadores;
  las configuraciones anteriores siguen siendo compatibles.

## [0.4.0] - 2026-05-16

### Added

- **Temas**: presets curados Nord, Solarized y Dracula, además de
  Claro/Oscuro/Sistema, y un tema **Personalizado** con editor de colores
  (acento, fondo del editor, superficie, texto, texto atenuado y bordes).
- **Exportar / importar tema** como archivo `.json`.
- **Menú contextual** (clic derecho) en el árbol, sobre carpetas y notas:
  crear nota, crear carpeta, **renombrar** y **eliminar** (con confirmación).

### Changed

- Rediseño visual moderno y minimalista guiado por la paleta del tema:
  tipografía y espaciado refinados, superficies planas, bordes sutiles,
  acento de color, barra superior con iconos sin marco y lienzo de edición
  a sangre.
- `directories` actualizada a la v6; el resto de dependencias en su última
  versión.
- Workflow de release: `actions/upload-artifact` y `download-artifact` a v5
  (Node 24 nativo); eliminado el forzado manual de Node.

## [0.3.0] - 2026-05-16

### Added

- **Preferencias** accesibles desde la barra superior (⚙), con secciones:
  - **Apariencia**: tema (claro/oscuro/sistema) y escala de la interfaz.
  - **Copia de seguridad**: carpeta destino, copia manual y copia automática
    tras cada guardado (omite archivos ocultos).
  - **Acerca de**: versión, autor, licencia y enlace al repositorio.

## [0.2.0] - 2026-05-16

### Added

- **Espacios**: registra varias carpetas como espacios y cámbialos desde un
  selector en la barra lateral; el espacio activo se recuerda entre sesiones.
- **Árbol de carpetas y notas** por espacio, con crear nota y crear carpeta
  (también con clic derecho sobre una carpeta).
- **Temas** claro, oscuro y sistema; la preferencia se guarda.
- Barra superior con breadcrumb (espacio / nota) y barra lateral ocultable.

### Changed

- Interfaz rediseñada al estilo Obsidian (barra lateral + editor + vista
  previa), con un aspecto limpio y moderno.

### Removed

- Workflow de integración continua (`ci.yml`): la automatización se ejecuta
  únicamente al publicar una versión (tag `v*`).

## [0.1.2] - 2026-05-16

### Changed

- macOS se distribuye como `.dmg` con un bundle `RustNotes.app` (Info.plist
  e icono), en lugar de un binario suelto en `.tar.gz`.

## [0.1.1] - 2026-05-16

### Added

- Paquete **AppImage** para Linux (portable, cualquier distribución).
- Entrada de menú (`.desktop`) e icono en los paquetes `.deb` y `.rpm`:
  RustNotes aparece en el menú de aplicaciones tras instalarlo.

### Changed

- Los archivos publicados incluyen la versión (tomada del tag) en el nombre.
- La versión portable de Windows es ahora un `.exe` directo (antes un `.zip`).

### Removed

- Ya no se publica el binario Linux suelto sin extensión (lo sustituye el
  AppImage).

## [0.1.0] - 2026-05-15

### Added

- Versión mínima viable de la aplicación de escritorio (egui/eframe).
- Abrir una carpeta como *vault* y listar recursivamente las notas `.md`.
- Editor de texto Markdown con guardado a disco.
- Vista previa Markdown en vivo.
- README con características, instalación, compilación y uso.
- Workflow de CI: build + `fmt` + `clippy` en Linux, macOS y Windows
  (jobs nombrados por plataforma).
- Workflow de release: paquetes `.deb` y `.rpm` (Linux), `tar.gz` (macOS),
  ejecutable e instalable y versión portable `.zip` (Windows).
- Las actions de CI se ejecutan en Node 24 (Node 20 quedó obsoleto).

[0.8.0]: https://github.com/Aleixenandros/oxydice/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/Aleixenandros/oxydice/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Aleixenandros/RustNotes/releases/tag/v0.1.0
