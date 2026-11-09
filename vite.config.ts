import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  build: {
    rollupOptions: {
      // One page per Tauri window. Paths match `app.windows[].url` in tauri.conf.json.
      input: {
        main: new URL("./index.html", import.meta.url).pathname,
        orb: new URL("./src/windows/orb/index.html", import.meta.url).pathname,
      },
    },
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: "ws", host, port: 1421 } : undefined,
    watch: { ignored: ["**/src-tauri/**"] },
  },
});
