# Oxydice

Aplicación de notas en Markdown organizadas en carpetas (*espacios*).
Local-first: la fuente de verdad son tus archivos `.md` en disco. Núcleo en
**Rust** (`oxydice-core`), interfaz **Tauri 2 + Svelte 5 + CodeMirror 6**, un
solo frontend para escritorio (Linux/macOS/Windows) y, a futuro, móvil.

## Características

- **Espacios**: registra varias carpetas como espacios y cambia entre ellas;
  el espacio activo se recuerda entre sesiones.
- **Árbol de carpetas y notas** por espacio, con **menú contextual** (clic
  derecho): crear nota, crear carpeta, renombrar y eliminar (con confirmación).
- **Autoguardado**: las notas se escriben a disco solas mientras editas; sin
  botón de guardar (también `Ctrl+S` para forzarlo).
- **Editar / Vista**: editor Markdown (CodeMirror 6) con números de línea,
  resaltado y panel de **esquema**; o vista de lectura renderizada en el core
  (`pulldown-cmark`, HTML saneado) con **cabecera de metadatos** del
  *frontmatter* (modificación, estado, autor, etiquetas).
- **Búsqueda global**: texto en todas las notas del espacio, con ruta, línea
  y término resaltado; abrir un resultado salta a la línea.
- **Visor de código**: además de `.md`, abre y resalta archivos html, css,
  php, js, ts, rs, py, json, yaml… en solo lectura.
- **Exportación**: la nota a **HTML** autónomo (con el tema) y a **PDF**.
- **Idiomas**: ES/EN/DE/PT con detección del sistema (Ajustes › Apariencia).
- **Temas**: presets Claro, Oscuro, Sistema, Nord, Solarized y Dracula, más un
  tema **Personalizado**; la paleta se traduce a variables CSS en el core
  (única fuente de verdad de estilo). Exportar/importar el tema como `.json`.
- **Ajustes**: apariencia (tema, fuente y tamaño del editor, escala),
  sincronización con registro, copia de seguridad y "acerca de".

## Estructura

```text
oxydice/
├── crates/oxydice-core/   # lógica agnóstica de UI (doc, search, render,
│                          #   theme→CSS, vault, sync, extensiones); con tests
└── apps/oxydice/          # app Tauri 2
    ├── src-tauri/         # capa Rust: envuelve el core en comandos tipados
    └── src/               # frontend SvelteKit (SPA) + CodeMirror 6
```

## Compilar desde el código

Requiere [Rust](https://www.rust-lang.org/) estable y
[Node.js](https://nodejs.org/) ≥ 20.

```sh
git clone https://github.com/Aleixenandros/oxydice.git
cd oxydice/apps/oxydice
npm install
npm run tauri dev      # desarrollo
npm run tauri build    # binarios + instaladores para tu sistema
```

En Linux, Tauri necesita WebKitGTK y GTK:

```sh
sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev \
  build-essential curl wget file libssl-dev libayatana-appindicator3-dev
```

(En Fedora: `webkit2gtk4.1-devel gtk3-devel librsvg2-devel openssl-devel`.)

## Instalación

Descarga el paquete para tu sistema desde
[Releases](https://github.com/Aleixenandros/oxydice/releases). El bundler de
Tauri genera `.deb`/`.rpm`/AppImage (Linux), `.dmg` (macOS) e instalador
(Windows). En macOS, la app no está firmada: la primera vez ábrela con clic
derecho → Abrir.

## Uso

1. **Añadir espacio** (menú ☰) y elige una carpeta.
2. Crea notas/carpetas con **＋ Nota** / **＋ Carpeta** o con clic derecho.
3. Edita en el panel central: se **autoguarda**. Alterna **Editar / Vista** y
   abre el **esquema**.
4. **Buscar** en el rail para encontrar texto en todas las notas.
5. Elige el tema en la barra superior o en **Ajustes › Apariencia**.

## Estado y hoja de ruta

Migrado de egui a Tauri 2 (un solo frontend). Detalle en
[`memoria.md`](./memoria.md), [`arquitectura.md`](./arquitectura.md),
[`tareas.md`](./tareas.md) y [`CHANGELOG.md`](./CHANGELOG.md). Siguiente:
sincronización (OpenDAL: WebDAV/S3 con índice baseline y resolución de
conflictos por bifurcación).

## Licencia

[Apache-2.0](./LICENSE).
