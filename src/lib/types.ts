/**
 * Die Datentypen, die zwischen Frontend und Backend laufen.
 *
 * Spiegel von `src-tauri/src/config/schema.rs` und den Rückgabetypen in
 * `src-tauri/src/commands.rs`. Rust serialisiert nach camelCase, deshalb stehen
 * hier camelCase-Felder.
 *
 * **Es gibt bewusst keinen Typ für das Automation-Secret.** Es kommt nie vom
 * Backend zurück; es geht nur als nackter String in `secretSet` hinein. Ein
 * Feld dafür in diesen Typen wäre eine Einladung, es im Zustand zu halten.
 */

export type ProxyMode = "system" | "none" | "manual";

export type ProxyConfig =
  | { mode: "system" }
  | { mode: "none" }
  | { mode: "manual"; url: string };

export interface Connection {
  id: string;
  name: string;
  server: string;
  site: string;
  username: string;
  verifyTls: boolean;
  proxy: ProxyConfig;
}

export interface PollingSettings {
  intervalSeconds: number;
  timeoutSeconds: number;
}

export type NotificationLevel = "off" | "criticalOnly" | "allChanges";

export interface NotificationSettings {
  level: NotificationLevel;
  sounds: SoundSettings;
  /**
   * **Veraltet** (Schemaversion 1: ein Klang für alles). Das Backend überführt
   * das Feld beim Laden nach `sounds.critical` und leert es. Steht hier nur,
   * damit eine geladene alte Datei typkonform bleibt — nicht benutzen.
   */
  soundPath?: string | null;
}

export type ThemeMode = "system" | "light" | "dark";

export interface AppearanceSettings {
  theme: ThemeMode;
  language: string;
}

export interface BehaviourSettings {
  autostart: boolean;
  autostartInitialised: boolean;
  startMinimised: boolean;
  pinPopup: boolean;
  hideHandled: boolean;
}

export interface PermissionSettings {
  allowAcknowledge: boolean;
  allowDowntime: boolean;
  /**
   * Vorlagen für den Kommentar. Die Platzhalter setzt das Backend ein — das
   * Frontend holt den fertigen Text über `actionComment()` und baut die
   * Ersetzung nicht nach.
   */
  acknowledgeComment: string;
  downtimeComment: string;
}

/**
 * Woher der Klang für ein Ereignis kommt. Spiegelt `SoundChoice` in `schema.rs`.
 *
 * `builtin` verweist auf einen eingebauten Klang; die Liste holt
 * `builtinSounds()` aus dem Backend, weil die Klänge dort im Programm liegen.
 */
export type SoundChoice =
  | { kind: "none" }
  | { kind: "builtin"; id: string }
  | { kind: "file"; path: string };

/** Version und Projektadresse, für die Fusszeile. */
export interface AboutInfo {
  version: string;
  projectUrl: string;
}

/** Ein eingebauter Klang, wie das Backend ihn meldet. */
export interface BuiltinSoundInfo {
  id: string;
  label: string;
}

/**
 * Ein Klang je Ereignis. Jedes einzeln abschaltbar.
 *
 * Der Verbindungsfehler fehlt absichtlich: er wiederholt sich bei einem
 * Ausfall jede Minute, und ein Ton dazu wäre der Grund, alle abzuschalten.
 */
export interface SoundSettings {
  critical: SoundChoice;
  warning: SoundChoice;
  recovery: SoundChoice;
  acknowledged: SoundChoice;
  downtime: SoundChoice;
}

/** Welche Schreibaktion gemeint ist. Spiegelt `WriteAction` in `actions/mod.rs`. */
export type WriteAction = "acknowledge" | "downtime";

/**
 * Dauerauswahl für eine Wartungszeit. Spiegelt `DowntimeChoice` in
 * `commands.rs`; `custom` verlangt zusätzlich eine Minutenangabe.
 */
export type DowntimeChoice =
  | "minutes15"
  | "hour1"
  | "hours4"
  | "untilMorning"
  | "custom";

export interface Settings {
  schemaVersion: number;
  connections: Connection[];
  activeConnection: number;
  polling: PollingSettings;
  notifications: NotificationSettings;
  appearance: AppearanceSettings;
  behaviour: BehaviourSettings;
  permissions: PermissionSettings;
}

/* -------------------------------------------------------------------------- */
/* Prüfung                                                                    */
/* -------------------------------------------------------------------------- */

export type IssueSeverity = "error" | "warning";

export interface ValidationIssue {
  /** Feldpfad, etwa `connection.server`. */
  field: string;
  message: string;
  severity: IssueSeverity;
}

/* -------------------------------------------------------------------------- */
/* Laden                                                                      */
/* -------------------------------------------------------------------------- */

export type SettingsSource = "userConfig" | "machineDefaults" | "firstRun";

export interface LoadOutcome {
  settings: Settings;
  source: SettingsSource;
  /** Nicht-fatale Vorfälle beim Laden, etwa eine beschädigte Datei. */
  notices: string[];
  /** Ob der Ersteinrichtungs-Assistent gezeigt werden soll. */
  needsSetup: boolean;
}

/* -------------------------------------------------------------------------- */
/* Verbindungstest                                                            */
/* -------------------------------------------------------------------------- */

export interface ConnectionReport {
  httpStatus: number;
  checkmkVersion: string | null;
  edition: string | null;
  editionLabel: string | null;
  site: string | null;
  elapsedMs: number;
  tlsVerificationDisabled: boolean;
}

/* -------------------------------------------------------------------------- */
/* Abzug                                                                      */
/* -------------------------------------------------------------------------- */

/**
 * Zustand eines Problems, wie das Backend ihn serialisiert.
 *
 * `unreachable` gibt es nur bei Hosts und wird in der Anzeige wie `down`
 * behandelt — siehe `ProblemState::status_key` in model.rs.
 */
export type ProblemState =
  | "ok"
  | "warn"
  | "crit"
  | "unknown"
  | "down"
  | "unreachable";

export interface Problem {
  host: string;
  /** `null` bedeutet: das Problem betrifft den Host selbst. */
  service: string | null;
  state: ProblemState;
  output: string;
  /** ISO-8601, `null` wenn CheckMK noch keinen Statuswechsel kennt. */
  lastStateChange: string | null;
  acknowledged: boolean;
  downtimeDepth: number;
  flapping: boolean;
}

export interface Snapshot {
  /** Absteigend nach Schwere vorsortiert. */
  problems: Problem[];
  fetchedAt: string;
}

/** Der Zustand, den die Abrufschleife meldet. */
export interface StatusPayload {
  snapshot: Snapshot | null;
  /** Meldung des letzten fehlgeschlagenen Abrufs, deutsch und konkret. */
  error: string | null;
  /** Kürzel des Tray-Zustands, etwa `CRIT` oder `GETRENNT`. */
  trayState: string;
  tooltip: string;
  /** Aufeinanderfolgende Fehlversuche. `0` heisst: letzter Abruf war gut. */
  failures: number;
  /** Ob überhaupt eine Verbindung eingerichtet ist. */
  configured: boolean;
}

/** Ereignisnamen des Backends. Müssen zu den Konstanten in Rust passen. */
export const EVENT_STATUS = "luchsr://status";
export const EVENT_SHOW_SETTINGS = "luchsr://show-settings";

/* -------------------------------------------------------------------------- */
/* Fehler                                                                     */
/* -------------------------------------------------------------------------- */

export interface CommandError {
  message: string;
  /** Zertifikatsproblem — das UI zeigt dann den Hinweis auf den Zertifikatspeicher. */
  isTlsProblem: boolean;
  retryable: boolean;
  /** Betroffene Feldpfade, wenn die Prüfung fehlschlug. */
  fields: string[];
  /** Technische Fehlerkette zum Aufklappen. */
  details: string[];
}

/** Grenzen aus `schema.rs`. Hier gespiegelt für die Eingabefelder. */
export const INTERVAL_MIN_SECONDS = 15;
export const INTERVAL_MAX_SECONDS = 600;
export const TIMEOUT_MIN_SECONDS = 2;
export const TIMEOUT_MAX_SECONDS = 120;

/** Ergebnis eines Update-Checks, wie das Backend es meldet. */
export interface UpdateReport {
  verdict: "updateAvailable" | "upToDate" | "ahead";
  /** Die laufende Fassung. */
  current: string;
  /** Die veröffentlichte Fassung. */
  latest: string;
  /** Die Seite des Releases — dort liegen MSI und Prüfsummen. */
  releaseUrl: string;
}
