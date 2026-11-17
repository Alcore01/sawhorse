import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
// Self-hosted fonts (the desktop app works offline): the mock's three families and weights.
import "@fontsource/barlow-condensed/500.css";
import "@fontsource/barlow-condensed/600.css";
import "@fontsource/ibm-plex-sans/400.css";
import "@fontsource/ibm-plex-sans/500.css";
import "@fontsource/ibm-plex-sans/600.css";
import "@fontsource/ibm-plex-mono/400.css";
import "@fontsource/ibm-plex-mono/500.css";
import "./styles/tokens.css";
import "./styles/app.css";
import "./styles/drawings.css";
import { App } from "./App";

// ?theme=dark or ?theme=light forces a theme (screenshots, demos); otherwise follow the OS.
try {
  const theme = new URLSearchParams(location.search).get("theme");
  if (theme === "dark" || theme === "light") document.documentElement.dataset.theme = theme;
} catch {
  /* no location in some hosts */
}

// sketchcraft E-34: installable on a tablet (manifest.webmanifest) and usable offline: the
// built web app registers its service worker; the dev server, the desktop app and the
// Android app (E-35, which carries its files) do not.
if (import.meta.env.PROD && "serviceWorker" in navigator && window.isSecureContext && !("__TAURI_INTERNALS__" in window) && !("Capacitor" in window)) {
  window.addEventListener("load", () => {
    navigator.serviceWorker.register("./sw.js").catch(() => {
      /* offline use only: the app works without it */
    });
  });
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
