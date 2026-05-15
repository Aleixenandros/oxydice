# Changelog

Todos los cambios notables de RustNotes se documentan en este archivo.

El formato sigue [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/)
y el proyecto se adhiere a [Versionado Semántico](https://semver.org/lang/es/).

## [Unreleased]

## [0.1.0] - 2026-05-15

### Added
- Versión mínima viable de la aplicación de escritorio (egui/eframe).
- Abrir una carpeta como *vault* y listar recursivamente las notas `.md`.
- Editor de texto Markdown con guardado a disco.
- Vista previa Markdown en vivo.
- Workflow de CI: build + `fmt` + `clippy` en Linux, macOS y Windows.
- Workflow de release: paquetes `.deb` y `.rpm` (Linux), `tar.gz` (macOS),
  ejecutable e instalable y versión portable `.zip` (Windows).

[Unreleased]: https://github.com/Aleixenandros/RustNotes/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Aleixenandros/RustNotes/releases/tag/v0.1.0
