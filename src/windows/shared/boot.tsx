import { StrictMode, type ReactElement } from "react";
import { createRoot } from "react-dom/client";
import "../../styles/global.css";
import { initTheme } from "../../theme/theme";
import { initLocale } from "../../i18n/locale";
import { LocaleRoot } from "../../i18n/LocaleRoot";
import "./floating.css";

/** Mounts a floating panel: theme, transparent page, React root. */
export function boot(element: ReactElement): void {
  initTheme();
  document.documentElement.classList.add("noma-floating");
  void (async () => {
    // Plain-browser development: in-memory backend, dark backdrop to see the glass.
    if (import.meta.env.DEV && !("__TAURI_INTERNALS__" in window)) {
      const { installMockBackend } = await import("../../dev/mockBackend");
      installMockBackend();
      document.body.style.setProperty(
        "background",
        "radial-gradient(700px 520px at 30% 70%, rgba(56,40,120,.6), transparent 70%), #0B0B10",
        "important",
      );
    }
    initLocale();
    createRoot(document.getElementById("root") as HTMLElement).render(
      <StrictMode>
        <LocaleRoot>{element}</LocaleRoot>
      </StrictMode>,
    );
  })();
}
