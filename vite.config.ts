import { defineConfig } from "vite";
import { resolve } from "node:path";

// Two pages: the manager window and the desktop tank window.
export default defineConfig({
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
  build: {
    target: "es2021",
    rollupOptions: {
      input: {
        main: resolve(import.meta.dirname, "index.html"),
        tank: resolve(import.meta.dirname, "tank.html"),
        hud: resolve(import.meta.dirname, "hud.html"),
        hunt: resolve(import.meta.dirname, "hunt.html"),
      },
    },
  },
});
