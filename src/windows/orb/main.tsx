import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import "../../styles/global.css";
import { initTheme } from "../../theme/theme";
import { OrbWindow } from "./OrbWindow";

initTheme();

createRoot(document.getElementById("root") as HTMLElement).render(
  <StrictMode>
    <OrbWindow />
  </StrictMode>,
);
