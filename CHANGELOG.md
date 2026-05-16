# Changelog

Todos los cambios notables de RustNotes se documentan en este archivo.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y el proyecto se adhiere a [Versionado Semántico](https://semver.org/lang/es/).

## [Unreleased]

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

[Unreleased]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.2...HEAD
[0.1.2]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.1...v0.1.2
[0.1.1]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/Aleixenandros/RustNotes/releases/tag/v0.1.0
