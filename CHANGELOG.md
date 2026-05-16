# Changelog

Todos los cambios notables de RustNotes se documentan en este archivo.

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

[0.6.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.2.0...v0.3.0
[0.2.0]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.2...v0.2.0
[0.1.2]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Aleixenandros/RustNotes/releases/tag/v0.1.0
