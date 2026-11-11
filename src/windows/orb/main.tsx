import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../../styles/global.css";
import { initTheme } from "../../theme/theme";
import { initLocale } from "../../i18n/locale";
import { LocaleRoot } from "../../i18n/LocaleRoot";
import { OrbWindow } from "./OrbWindow";

initTheme();
initLocale();

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <LocaleRoot>
      <OrbWindow />
    </LocaleRoot>
  </StrictMode>,
);
