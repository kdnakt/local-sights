import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// Tauri expects a fixed dev port and must not be hidden behind Vite's
// screen clearing. See https://v2.tauri.app/start/frontend/vite/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "localhost",
    watch: {
      ignored: ["**/src-tauri/**", "**/target/**"],
    },
  },
  build: {
    target: "es2022",
    outDir: "dist",
  },
});
