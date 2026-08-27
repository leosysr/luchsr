import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "./App";
import "./styles/index.css";

/**
 * Das Kontextmenü von WebView2 abschalten.
 *
 * Ohne das zeigt ein Rechtsklick Chromes Menü: „Zurück", „Aktualisieren",
 * „Speichern unter", „Drucken", „Untersuchen". In einer Desktop-Anwendung ist
 * das falsch — es verrät, dass ein Browser darunter steckt, und die Einträge
 * tun teils Unsinn (ein Popup „drucken", das Fenster „zurück" navigieren).
 *
 * Nur im Auslieferungsbau: in der Entwicklung ist „Untersuchen" das einzige
 * Werkzeug, um im WebView etwas nachzusehen.
 *
 * Text markieren und mit Strg+C kopieren bleibt möglich; nur der Rechtsklick
 * darauf entfällt.
 */
if (!import.meta.env.DEV) {
  window.addEventListener("contextmenu", (event) => event.preventDefault());
}

const container = document.getElementById("root");
if (!container) {
  throw new Error("#root fehlt in index.html");
}

createRoot(container).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
