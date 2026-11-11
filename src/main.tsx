import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./app/App";
import { initTheme } from "./theme/theme";
import { initLocale } from "./i18n/locale";
import { LocaleRoot } from "./i18n/LocaleRoot";
import "./styles/global.css";

initTheme();

async function boot() {
  // Plain-browser development (no Tauri shell): use the in-memory mock backend.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMockBackend } = await import("./dev/mockBackend");
    installMockBackend();
  }
}

void boot().then(() => {
  initLocale();
  createRoot(document.getElementById("root") as HTMLElement).render(
    <StrictMode>
      <LocaleRoot>
        <App />
      </LocaleRoot>
    </StrictMode>,
  );
});
