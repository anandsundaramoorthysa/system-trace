import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Vite config tuned for Tauri. The Tauri CLI sets the host/port the webview
// loads; we keep a fixed dev port and disable Vite's clearing of the screen so
// Rust (cargo) logs remain visible in the same terminal.
//
// https://tauri.app/v1/api/config (port and strictPort must match tauri.conf.json
// build.devUrl).
const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [react()],

  // Prevent Vite from obscuring Rust errors.
  clearScreen: false,

  server: {
    host: host || "localhost",
    port: 1420,
    strictPort: true,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    // Tauri reads the local filesystem; do not watch the Rust source tree.
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },

  // Tauri uses a fixed output for the bundled frontend.
  build: {
    target: "es2021",
    minify: "esbuild",
    sourcemap: false,
    outDir: "dist",
  },

  // Activity data never leaves the machine; no env prefixing beyond Tauri's own.
  envPrefix: ["VITE_", "TAURI_"],
});
