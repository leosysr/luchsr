/**
 * Bildaufnahme für das README — ein Werkzeug, keine Anwendung.
 *
 * Gerendert wird die **echte** [`App`]-Komponente. Gefälscht ist einzig die
 * IPC-Schicht: `window.__TAURI_INTERNALS__.invoke` liefert vorbereitete
 * Antworten. Die Bilder zeigen damit die wirklichen Komponenten, Tokens,
 * Schriften und Abstände.
 *
 * ## Warum das hier liegt und nicht wegwerfbar ist
 *
 * Ein Screenshot im README ist eine **erzeugte Datei**. Für Icons und Klänge
 * gilt in diesem Projekt, dass der Erzeuger im Repository liegt — sonst hat
 * man Dateien, die niemand mehr auffrischen kann. Für Bilder gilt dasselbe,
 * und Bilder veralten schneller als Icons.
 *
 * Das widerspricht **nicht** D39, wo ein Token-Musterblatt gelöscht wurde. Das
 * Musterblatt baute Komponenten ein zweites Mal ein und lief deshalb
 * auseinander. Hier wird nichts nachgebaut: es wird `App` gerendert.
 *
 * Die Beispieldaten sind **getypt**. Ändert sich `Settings` oder `Problem`,
 * schlägt `npm run typecheck` hier fehl — die Attrappe kann also nicht
 * stillschweigend verrotten.
 *
 * ## Aufruf
 *
 * ```text
 * npm run dev
 * pwsh -File scripts/screenshots.ps1
 * ```
 *
 * Parameter: `?view=settings` öffnet die Einstellungen, `?scroll=N` rollt die
 * Ansicht um N Pixel — für Abschnitte, die nicht oben stehen.
 */

import { StrictMode } from "react";
import { createRoot } from "react-dom/client";

import App from "../App";
import "../styles/index.css";
import type {
  AboutInfo,
  BuiltinSoundInfo,
  LoadOutcome,
  Problem,
  Settings,
  StatusPayload,
} from "../lib/types";

/* -------------------------------------------------------------------------- */
/* Beispieldaten                                                              */
/* -------------------------------------------------------------------------- */

/** Minuten und Stunden zurück, als ISO-Zeitstempel. */
const vor = (minuten: number) => new Date(Date.now() - minuten * 60_000).toISOString();

/**
 * Erfundene Hosts und Meldungen.
 *
 * Bewusst erdacht: aus einer echten Umgebung dürfen keine Namen in ein
 * öffentliches README geraten. Die Meldungstexte sind der Form nach echt, damit
 * die Spaltenbreiten stimmen — eine Liste mit „foo/bar" zeigt nicht, ob die
 * Darstellung bei wirklichen Ausgaben trägt.
 */
const PROBLEME: Problem[] = [
  {
    host: "srv-db01",
    service: "PostgreSQL Replikation",
    state: "crit",
    output: "CRITICAL - Replikation hinkt 412 s nach (Grenze 120 s)",
    lastStateChange: vor(27),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  {
    host: "srv-db01",
    service: "Filesystem /var/lib/pgsql",
    state: "warn",
    output: "WARNING - 87.4% belegt (Grenze 85.0%), 41.2 GB frei",
    lastStateChange: vor(180),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  // Host selbst ausgefallen: seine beiden Dienste klappen darunter zusammen.
  {
    host: "sw-core-02",
    service: null,
    state: "down",
    output: "CRITICAL - 10.0.0.24: rta nicht verfügbar, 100% Paketverlust",
    lastStateChange: vor(9),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  {
    host: "sw-core-02",
    service: "Interface Uplink",
    state: "crit",
    output: "CRITICAL - Verbindung unterbrochen",
    lastStateChange: vor(9),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  {
    host: "sw-core-02",
    service: "Temperatur",
    state: "warn",
    output: "WARNING - Keine Daten seit 9 Minuten",
    lastStateChange: vor(9),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  // Flatternd — zeigt die Kennzeichnung in der Ausgabespalte.
  {
    host: "srv-app03",
    service: "Interface Gi0/1",
    state: "warn",
    output: "WARNING - Eingangsfehler 143/s",
    lastStateChange: vor(52),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: true,
  },
  {
    host: "srv-app03",
    service: "Zertifikat HTTPS",
    state: "unknown",
    output: "UNKNOWN - Check-Plugin lieferte keine Ausgabe",
    lastStateChange: vor(26 * 60),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  {
    host: "srv-mail01",
    service: "Warteschlange",
    state: "crit",
    output: "CRITICAL - 1284 Nachrichten in der Warteschlange (Grenze 500)",
    lastStateChange: vor(71),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  {
    host: "srv-web01",
    service: "Apache Prozesse",
    state: "crit",
    output: "CRITICAL - 0 laufende Prozesse (erwartet mindestens 4)",
    lastStateChange: vor(14),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  {
    host: "srv-web01",
    service: "Zertifikat HTTPS",
    state: "warn",
    output: "WARNING - Läuft in 12 Tagen ab (Grenze 30 Tage)",
    lastStateChange: vor(4 * 24 * 60),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  {
    host: "nas-backup",
    service: "RAID-Verbund",
    state: "crit",
    output: "CRITICAL - Platte 3 ausgefallen, Verbund läuft entartet",
    lastStateChange: vor(6 * 60),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  {
    host: "nas-backup",
    service: "SMART sdc",
    state: "unknown",
    output: "UNKNOWN - Gerät antwortet nicht auf SMART-Abfragen",
    lastStateChange: vor(6 * 60),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  {
    host: "srv-dc01",
    service: "Zeitabweichung",
    state: "warn",
    output: "WARNING - Abweichung 1.84 s zur Referenz",
    lastStateChange: vor(38),
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
  },
  // Quittiert: wird ausgeblendet und erscheint nur im Zähler des
  // Augen-Knopfes. Das ist die Aussage des Bildes an dieser Stelle.
  {
    host: "srv-file02",
    service: "Backup gestern",
    state: "crit",
    output: "CRITICAL - Letzter erfolgreicher Lauf vor 51 Stunden",
    lastStateChange: vor(5 * 60),
    acknowledged: true,
    downtimeDepth: 0,
    flapping: false,
  },
];

/**
 * Der Tooltip muss zu den Daten passen.
 *
 * Im Betrieb rechnet ihn das Backend aus; hier steht er von Hand. Ein Bild,
 * dessen Kopfzeile der Liste widerspricht, sieht wie ein Fehler aus — beim
 * ersten Versuch stand dort „2 WARN" bei drei Warnungen.
 *
 * Gezählt werden die **nicht bearbeiteten** Probleme, wie im Tray.
 */
const TOOLTIP = "Luchsr — 1 DOWN, 5 CRIT, 5 WARN, 2 UNKNOWN";

const STATUS: StatusPayload = {
  snapshot: { problems: PROBLEME, fetchedAt: new Date().toISOString() },
  error: null,
  trayState: "DOWN",
  tooltip: TOOLTIP,
  failures: 0,
  configured: true,
};

/**
 * Eine eingerichtete Konfiguration.
 *
 * Aufgebaut auf den Vorgaben, damit neue Felder nicht vergessen werden — der
 * Typ erzwingt sie ohnehin, aber so bleiben auch die Vorgabewerte sichtbar.
 */
const SETTINGS: Settings = {
  schemaVersion: 2,
  connections: [
    {
      id: "default",
      name: "Rechenzentrum",
      server: "https://checkmk.example.intern",
      site: "haupt",
      username: "automation",
      verifyTls: true,
      proxy: { mode: "system" },
    },
  ],
  activeConnection: 0,
  polling: { intervalSeconds: 60, timeoutSeconds: 10 },
  notifications: {
    level: "criticalOnly",
    sounds: {
      critical: { kind: "builtin", id: "kritisch" },
      warning: { kind: "none" },
      recovery: { kind: "none" },
      acknowledged: { kind: "none" },
      downtime: { kind: "none" },
    },
  },
  appearance: { theme: "system", language: "de" },
  behaviour: {
    autostart: true,
    autostartInitialised: true,
    startMinimised: true,
    pinPopup: false,
    hideHandled: true,
  },
  permissions: {
    allowAcknowledge: true,
    allowDowntime: true,
    acknowledgeComment: "{service} auf {host} — bekannt, wird bearbeitet ({user})",
    downtimeComment: "{service} auf {host} — geplante Wartung ({user})",
  },
};

const OUTCOME: LoadOutcome = {
  settings: SETTINGS,
  source: "userConfig",
  notices: [],
  needsSetup: false,
};

const ABOUT: AboutInfo = {
  version: "1.2.0",
  projectUrl: "https://github.com/leosysr/luchsr",
};

/**
 * Die Klangliste.
 *
 * Nur zwei Einträge: das Auswahlfeld zeigt ohnehin den gewählten, und die
 * vollständige Liste aus dem Backend hier zu wiederholen wäre eine zweite
 * Wahrheit über 24 Kennungen.
 */
const SOUNDS: BuiltinSoundInfo[] = [
  { id: "kritisch", label: "Sinus · Kritisch (drei Töne, abwärts)" },
  { id: "warnung", label: "Sinus · Warnung (zwei Töne, abwärts)" },
];

/* -------------------------------------------------------------------------- */
/* Gefälschte IPC-Schicht                                                     */
/* -------------------------------------------------------------------------- */

const ANTWORTEN: Record<string, unknown> = {
  settings_load: OUTCOME,
  settings_current: SETTINGS,
  settings_validate: [],
  status_current: STATUS,
  builtin_sounds: SOUNDS,
  about_info: ABOUT,
  secret_exists: true,
  credential_store_available: null,
  action_comment: "",
  refresh_now: null,
  set_pin_popup: null,
  hide_popup: null,
  play_sound: null,
  // Die Ereignis-API: `listen` gibt eine Kennung zurück, `unlisten` nimmt sie.
  // Ohne die beiden scheitert die Anmeldung in App.tsx — seit dem Audit wird
  // dieser Fehlschlag protokolliert statt verschluckt, was hier hilft.
  listen: 1,
  unlisten: null,
};

interface TauriInternals {
  invoke: (befehl: string) => Promise<unknown>;
  transformCallback: (cb: unknown) => unknown;
  unregisterCallback: () => void;
}

(window as unknown as { __TAURI_INTERNALS__: TauriInternals }).__TAURI_INTERNALS__ = {
  invoke: (befehl: string) => {
    const name = String(befehl).replace(/^plugin:[^|]*\|/, "");
    if (name in ANTWORTEN) return Promise.resolve(ANTWORTEN[name]);
    if (name.startsWith("log")) return Promise.resolve(null);
    // Auffallen statt still etwas Falsches liefern: ein neuer Befehl im
    // Frontend soll hier eine Warnung erzeugen, nicht ein leeres Bild.
    console.warn("Attrappe kennt den Befehl nicht:", name);
    return Promise.resolve(null);
  },
  transformCallback: (cb: unknown) => cb,
  unregisterCallback: () => undefined,
};

/* -------------------------------------------------------------------------- */
/* Ansicht wählen                                                             */
/* -------------------------------------------------------------------------- */

const PARAMETER = new URLSearchParams(location.search);

/** Rollt die Ansicht, für Abschnitte die nicht oben stehen. */
function rollen() {
  const n = Number(PARAMETER.get("scroll") ?? 0);
  if (!n) return;
  const flaeche = document.querySelector("[class*=overflow-y-auto]");
  if (flaeche) flaeche.scrollTop = n;
}

if (PARAMETER.get("view") === "settings") {
  // Über den Knopf in der Titelzeile, nicht über internen Zustand: so nimmt
  // das Bild denselben Weg wie ein Benutzer.
  window.setTimeout(() => {
    document.querySelectorAll("button").forEach((b) => {
      if (b.getAttribute("aria-label") === "Einstellungen") b.click();
    });
    window.setTimeout(rollen, 300);
  }, 300);
}

const wurzel = document.getElementById("root");
if (!wurzel) throw new Error("#root fehlt in mockup.html");

createRoot(wurzel).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
