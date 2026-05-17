// Aplicación del tema: el core resuelve la paleta y devuelve un mapa de
// *custom properties* CSS; aquí solo se fijan en `:root`. Ningún color
// vive en el frontend — es la única fuente de verdad de `oxydice-core`.

import { resolveTheme, type Config, type ResolvedTheme } from "./api";

/** `true` si el SO pide esquema oscuro (cae a `false` si no se detecta). */
export function systemDark(): boolean {
  return (
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-color-scheme: dark)").matches
  );
}

/** Fija las variables CSS del tema resuelto en `:root`. */
export function applyTheme(t: ResolvedTheme): void {
  const root = document.documentElement;
  for (const [k, v] of Object.entries(t.css)) {
    root.style.setProperty(k, v);
  }
  // `color-scheme` nativo: barras de scroll y controles del webview.
  const scheme = t.css["--color-scheme"] ?? (t.palette.dark ? "dark" : "light");
  root.style.colorScheme = scheme;
}

/**
 * Resuelve y aplica el tema de `config` (atendiendo a `prefers-color-scheme`
 * para «Sistema») y traduce `ui_scale` al `zoom` del webview, como hacía el
 * `zoom_factor` de egui. Devuelve el tema resuelto (para mostrar su nombre).
 */
export async function refreshTheme(config: Config): Promise<ResolvedTheme> {
  const resolved = await resolveTheme(
    config.theme,
    systemDark(),
    config.custom_theme,
  );
  applyTheme(resolved);
  // `zoom` lo soportan WebKitGTK / WKWebView / WebView2 (los webviews Tauri).
  (document.documentElement.style as CSSStyleDeclaration & {
    zoom?: string;
  }).zoom = String(config.ui_scale || 1);
  return resolved;
}

/** Suscribe `cb` a cambios del esquema del SO; devuelve el limpiador. */
export function onSystemSchemeChange(cb: () => void): () => void {
  const mq = window.matchMedia("(prefers-color-scheme: dark)");
  mq.addEventListener("change", cb);
  return () => mq.removeEventListener("change", cb);
}
