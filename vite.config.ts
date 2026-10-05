import { defineConfig } from "vite";
import { resolve } from "node:path";

// Pages: the manager window, the desktop tank window and the quick dock.
export default defineConfig({
  clearScreen: false,
  // Rust build outputs are watched by Tauri, not Vite. Windows locks live EXEs/PDBs.
  server: { port: 1420, strictPort: true, watch: { ignored: ["**/src-tauri/target/**"] } },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    rollupOptions: {
      input: {
        main: resolve(import.meta.dirname, "index.html"),
        tank: resolve(import.meta.dirname, "tank.html"),
        hud: resolve(import.meta.dirname, "hud.html"),
      },
    },
  },
});
