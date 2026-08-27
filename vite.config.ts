/// <reference types="vitest/config" />
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { fileURLToPath, URL } from "node:url";

// Tauri setzt diese Variable, wenn von einem anderen Gerät aus entwickelt wird.
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react(), tailwindcss()],

  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },

  // Tauri kümmert sich um die Konsolenausgabe; Vite soll sie nicht wegwischen.
  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: host ?? false,
    // Bedingtes Spread statt `hmr: undefined` — exactOptionalPropertyTypes
    // erlaubt kein explizites undefined.
    ...(host ? { hmr: { protocol: "ws", host, port: 1421 } } : {}),
    watch: {
      // Rust-Quellen triggern den Tauri-Watcher, nicht Vite.
      // handover-design bleibt unangetastet und wird nicht beobachtet.
      ignored: ["**/src-tauri/**", "**/handover-design/**"],
    },
  },

  test: {
    // Nur die reinen Logikmodule. Komponenten werden nicht getestet — dafür
    // bräuchte es ein DOM und würde Darstellung prüfen, nicht Entscheidungen.
    include: ["src/**/*.test.ts"],
    environment: "node",
    // Die Zeitformatierung nutzt toLocaleString("de-DE"); ohne feste Zone
    // wären die Tests auf einem anders eingestellten Rechner instabil.
    // Geprüft wird deshalb die Form, nicht der Wert — siehe duration.test.ts.
    reporters: ["default"],
  },

  build: {
    // Zielplattform ist ausschliesslich Windows 11 mit WebView2 (Chromium).
    // Kein Bedarf an Legacy-Transpilierung.
    target: "chrome120",
    // minify bleibt beim Standard: Vite 8 minifiziert über Rolldown/oxc.
    // "esbuild" wäre ein Fehler — esbuild ist ab Vite 8 nicht mehr enthalten.
    sourcemap: false,
  },
});
