<script lang="ts">
  let { children } = $props();
</script>

{@render children()}

<style>
  /* Fuentes empotradas (guía de estilo §3): Inter para la UI, JetBrains
     Mono para el editor. Servidas desde `static/fonts` (CSP font-src 'self'). */
  @font-face {
    font-family: "Inter";
    src: url("/fonts/Inter-Regular.ttf") format("truetype");
    font-weight: 400;
    font-display: swap;
  }
  @font-face {
    font-family: "Inter";
    src: url("/fonts/Inter-SemiBold.ttf") format("truetype");
    font-weight: 600;
    font-display: swap;
  }
  @font-face {
    font-family: "JetBrains Mono";
    src: url("/fonts/JetBrainsMono-Regular.ttf") format("truetype");
    font-weight: 400;
    font-display: swap;
  }

  /* Valores de reserva hasta que `theme.ts` fija las variables del tema. */
  :global(:root) {
    --accent: #ce412b;
    --accent-contrast: #ffffff;
    --bg: #131313;
    --surface: #1e1e1e;
    --text: #e0e0e0;
    --muted: #a0a0a0;
    --border: #393939;
    --hover: #2a2a2a;
    --faint: #1f1f1f;
    --inactive: #262626;
    --code-bg: #1f1f1f;
    --selection: rgba(206, 65, 43, 0.31);
    --shadow: rgba(0, 0, 0, 0.38);
    --danger: #c94a4a;
    --danger-contrast: #ffffff;
    --success: #4caf5c;
    color-scheme: dark;
  }

  :global(*) {
    box-sizing: border-box;
  }
  :global(html),
  :global(body) {
    margin: 0;
    height: 100%;
    overflow: hidden;
  }
  :global(body) {
    font-family: "Inter", ui-sans-serif, system-ui, sans-serif;
    font-size: 14px;
    line-height: 1.5;
    color: var(--text);
    background: var(--surface);
    -webkit-font-smoothing: antialiased;
  }
  :global(button) {
    font-family: inherit;
  }
  :global(::-webkit-scrollbar) {
    width: 10px;
    height: 10px;
  }
  :global(::-webkit-scrollbar-thumb) {
    background: var(--border);
    border-radius: 4px;
  }
  :global(::-webkit-scrollbar-track) {
    background: transparent;
  }
</style>
