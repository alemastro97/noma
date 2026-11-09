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
        overlay: new URL("./src/windows/overlay/index.html", import.meta.url).pathname,
        capture: new URL("./src/windows/capture/index.html", import.meta.url).pathname,
        panel: new URL("./src/windows/panel/index.html", import.meta.url).pathname,
        toasts: new URL("./src/windows/toasts/index.html", import.meta.url).pathname,
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
