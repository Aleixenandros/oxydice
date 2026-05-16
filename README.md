# RustNotes

Aplicación de escritorio nativa, escrita en Rust, para crear y leer notas en
Markdown organizadas en una carpeta (*vault*). Rápida, de un solo binario y
multiplataforma (Linux, macOS y Windows).

> Estado: versión inicial (0.1.0). Funcionalidad mínima viable.

## Características

- Abrir cualquier carpeta como *vault* y listar sus notas `.md` (recursivo).
- Editor de texto Markdown con guardado a disco.
- Vista previa de Markdown en vivo junto al editor.
- Los datos son archivos de texto plano: siempre legibles fuera de la app.

## Instalación

Descarga el paquete para tu sistema desde la página de
[Releases](https://github.com/Aleixenandros/RustNotes/releases):

Los nombres incluyen la versión (la del tag), p. ej. `0.1.0`:

| Sistema | Archivo |
|---|---|
| Linux (Debian/Ubuntu) | `rustnotes-<versión>-amd64.deb` |
| Linux (Fedora/RHEL) | `rustnotes-<versión>-x86_64.rpm` |
| Linux (portable, cualquier distro) | `rustnotes-<versión>-x86_64.AppImage` |
| macOS | `rustnotes-<versión>-macos.dmg` |
| Windows (portable) | `rustnotes-<versión>-windows-portable.exe` |

Los paquetes `.deb` y `.rpm` añaden RustNotes al menú de aplicaciones.
El **AppImage** no requiere instalación: dale permiso de ejecución y ábrelo
(`chmod +x rustnotes-*.AppImage && ./rustnotes-*.AppImage`).
El **.exe** de Windows es portable: ejecútalo directamente, sin instalar.
En **macOS**, abre el `.dmg` y arrastra `RustNotes` a Aplicaciones. La primera
vez ábrelo con clic derecho → Abrir (la app no está firmada con Apple).

## Compilar desde el código

Requiere [Rust](https://www.rust-lang.org/) estable.

```sh
git clone https://github.com/Aleixenandros/RustNotes.git
cd RustNotes
cargo build --release
./target/release/rustnotes
```

En Linux son necesarias algunas librerías del sistema:

```sh
sudo apt-get install -y \
  libgtk-3-dev libxcb-render0-dev libxcb-shape0-dev \
  libxcb-xfixes0-dev libxkbcommon-dev libssl-dev
```

## Uso

1. Pulsa **📂 Abrir vault** y elige una carpeta con notas `.md`.
2. Selecciona una nota en la lista de la izquierda.
3. Edita en el panel central; la vista previa se actualiza a la derecha.
4. Pulsa **💾 Guardar** para escribir los cambios a disco.

## Próximos pasos

- Enlaces internos entre notas y backlinks.
- Búsqueda por texto completo.
- Sincronización con la nube (varios proveedores).
- Exportación a PDF, HTML, DOCX y LaTeX.

## Licencia

MIT.
