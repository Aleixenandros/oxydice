# RustNotes

Aplicación de escritorio nativa, escrita en Rust, para crear y leer notas en
Markdown organizadas en una carpeta (*vault*). Rápida, de un solo binario y
multiplataforma (Linux, macOS y Windows).

## Características

- **Espacios**: registra varias carpetas como espacios y cambia entre ellas
  desde la barra lateral; el espacio activo se recuerda entre sesiones.
- **Árbol de carpetas y notas** por espacio, con **menú contextual** (clic
  derecho): crear nota, crear carpeta, renombrar y eliminar (con
  confirmación), tanto en carpetas como en notas.
- **Autoguardado**: las notas se escriben a disco solas mientras editas;
  sin botón de guardar (también `Ctrl+S` para forzarlo).
- **Editar / Vista**: editor Markdown con números de línea y panel de
  **esquema** (encabezados), o vista de lectura renderizada con **cabecera
  de metadatos** (modificación, estado, autor, etiquetas del *frontmatter*).
- **Búsqueda global**: busca texto en todas las notas del espacio, con
  ruta, línea y término resaltado; abrir un resultado salta a la línea.
- **Temas**: presets Claro, Oscuro, Sistema, Nord, Solarized y Dracula, más
  un tema **Personalizado** con editor de colores; **exportar/importar** el
  tema como archivo `.json`.
- **Ajustes**: apariencia (tema, fuente y tamaño del editor, escala),
  sincronización con registro de actividad, copia de seguridad y "acerca de".
- Interfaz moderna y minimalista, guiada por la paleta del tema.
- Los datos son archivos de texto plano: siempre legibles fuera de la app.

## Instalación

Descarga el paquete para tu sistema desde la página de
[Releases](https://github.com/Aleixenandros/RustNotes/releases):

Los nombres incluyen la versión (la del tag), p. ej. `0.6.0`:

| Sistema | Archivo |
|---|---|
| Linux (Debian/Ubuntu) | `rustnotes-<versión>-amd64.deb` |
| Linux (Fedora/RHEL) | `rustnotes-<versión>-x86_64.rpm` |
| Linux (portable, cualquier distro) | `rustnotes-<versión>-x86_64.AppImage` |
| macOS | `rustnotes-<versión>-macos.dmg` |
| Windows (instalador) | `rustnotes-<versión>-windows-setup.exe` |
| Windows (portable) | `rustnotes-<versión>-windows-portable.exe` |

Los paquetes `.deb` y `.rpm` añaden RustNotes al menú de aplicaciones.
El **AppImage** no requiere instalación: dale permiso de ejecución y ábrelo
(`chmod +x rustnotes-*.AppImage && ./rustnotes-*.AppImage`).
En **Windows** tienes dos opciones: el **instalador** (`-windows-setup.exe`)
crea accesos directos y desinstalador; el **portable**
(`-windows-portable.exe`) se ejecuta directamente sin instalar.
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
2. Crea notas y carpetas con **＋ Nota** / **＋ Carpeta**, o con **clic
   derecho** sobre una carpeta o nota del árbol (crear, renombrar, eliminar).
3. Selecciona una nota en el árbol y edítala en el panel central: se
   **autoguarda** sola. Alterna **Editar / Vista** y abre el **esquema**.
4. Usa **Buscar** en el rail para encontrar texto en todas las notas.
5. Elige el tema desde la barra superior o en **Ajustes › Apariencia**,
   donde además puedes personalizar los colores y exportar/importar el tema.

## Próximos pasos

- Enlaces internos entre notas y backlinks.
- Caché del árbol y *watcher* de disco.
- Sincronización con la nube (varios proveedores).
- Exportación a PDF, HTML, DOCX y LaTeX.

## Licencia

MIT.
