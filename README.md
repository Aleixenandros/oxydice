# RustNotes

Aplicación de escritorio nativa, escrita en Rust, para crear y leer notas en
Markdown organizadas en una carpeta (*vault*). Rápida, de un solo binario y
multiplataforma (Linux, macOS y Windows).

## Características

- **Espacios**: registra varias carpetas como espacios y cambia entre ellas
  desde la barra lateral; el espacio activo se recuerda entre sesiones.
- **Árbol de carpetas y notas** por espacio: crear notas y carpetas (también
  con clic derecho sobre una carpeta).
- Editor Markdown con guardado a disco y **vista previa en vivo**.
- **Temas**: claro, oscuro y sistema; la preferencia se guarda.
- **Preferencias** (⚙): apariencia y escala, copia de seguridad y "acerca de".
- Interfaz limpia y moderna al estilo Obsidian.
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

1. Pulsa **＋ Añadir espacio** y elige una carpeta: será un espacio.
2. Crea notas y carpetas con **＋ Nota** / **＋ Carpeta** (o clic derecho
   sobre una carpeta del árbol).
3. Selecciona una nota en el árbol y edítala en el panel central; la vista
   previa se actualiza a la derecha.
4. Pulsa **Guardar** para escribir los cambios a disco.
5. Cambia el tema (claro / oscuro / sistema) desde la barra superior.

## Próximos pasos

- Enlaces internos entre notas y backlinks.
- Búsqueda por texto completo.
- Sincronización con la nube (varios proveedores).
- Exportación a PDF, HTML, DOCX y LaTeX.

## Licencia

MIT.
