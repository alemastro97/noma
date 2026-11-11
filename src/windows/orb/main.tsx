import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../../styles/global.css";
import { initTheme } from "../../theme/theme";
import { initLocale } from "../../i18n/locale";
import { LocaleRoot } from "../../i18n/LocaleRoot";
import { OrbWindow } from "./OrbWindow";

initTheme();

async function boot() {
  // Plain-browser development (no Tauri shell): use the in-memory mock backend.
  if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
    const { installMockBackend } = await import("../../dev/mockBackend");
    installMockBackend();
  }
}

void boot().then(() => {
  initLocale();
  createRoot(document.getElementById("root") as HTMLElement).render(
    <StrictMode>
      <LocaleRoot>
        <OrbWindow />
      </LocaleRoot>
    </StrictMode>,
  );
});
