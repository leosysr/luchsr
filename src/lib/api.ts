/**
 * Typisierte Hülle um die Tauri-Befehle.
 *
 * Ein Ort für alle `invoke`-Aufrufe. Komponenten rufen Funktionen auf, keine
 * Zeichenketten — ein umbenannter Befehl wird damit zum Compilerfehler und
 * nicht zu einem Laufzeitfehler in einem Dialog, den niemand testet.
 *
 * **Das Automation-Secret geht nur hinein.** Es gibt keine Funktion, die es
 * ausliest, weil es im Backend keinen Befehl dafür gibt. `secretExists` gibt
 * einen Wahrheitswert zurück, mehr nicht.
 */

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";

import type {
  BuiltinSoundInfo,
  CommandError,
  Connection,
  ConnectionReport,
  DowntimeChoice,
  LoadOutcome,
  Settings,
  SoundChoice,
  StatusPayload,
  ValidationIssue,
  WriteAction,
} from "./types";
import { EVENT_SHOW_SETTINGS, EVENT_STATUS } from "./types";

/**
 * Erkennt die Fehlerform des Backends.
 *
 * Tauri wirft, was der Befehl als `Err` zurückgibt. Bei unseren Befehlen ist
 * das immer ein [`CommandError`] — aber wenn die Brücke selbst scheitert
 * (Befehl unbekannt, Serialisierung kaputt), kommt ein nackter String. Beides
 * muss hier ankommen, sonst zeigt der Dialog „[object Object]".
 */
export function asCommandError(error: unknown): CommandError {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message: unknown }).message === "string"
  ) {
    const candidate = error as Partial<CommandError> & { message: string };
    return {
      message: candidate.message,
      isTlsProblem: candidate.isTlsProblem ?? false,
      retryable: candidate.retryable ?? false,
      fields: candidate.fields ?? [],
      details: candidate.details ?? [],
    };
  }
  return {
    message:
      typeof error === "string"
        ? error
        : `Unerwarteter Fehler: ${String(error)}`,
    isTlsProblem: false,
    retryable: false,
    fields: [],
    details: [],
  };
}

/* -------------------------------------------------------------------------- */
/* Einstellungen                                                              */
/* -------------------------------------------------------------------------- */

/** Lädt die Einstellungen von der Platte, inklusive Herkunft und Hinweisen. */
export function settingsLoad(): Promise<LoadOutcome> {
  return invoke<LoadOutcome>("settings_load");
}

/** Die zwischengespeicherten Einstellungen, ohne Plattenzugriff. */
export function settingsCurrent(): Promise<Settings> {
  return invoke<Settings>("settings_current");
}

/**
 * Speichert die Einstellungen und gibt sie zurück, wie sie auf der Platte
 * gelandet sind — geklemmte Werte inklusive.
 */
export function settingsSave(settings: Settings): Promise<Settings> {
  return invoke<Settings>("settings_save", { settings });
}

/** Prüft, ohne zu speichern. Für die laufende Anzeige im Dialog. */
export function settingsValidate(settings: Settings): Promise<ValidationIssue[]> {
  return invoke<ValidationIssue[]>("settings_validate", { settings });
}

/* -------------------------------------------------------------------------- */
/* Automation-Secret                                                          */
/* -------------------------------------------------------------------------- */

/** Ob der Windows Credential Manager nutzbar ist. */
export function credentialStoreAvailable(): Promise<void> {
  return invoke<void>("credential_store_available");
}

/** Legt das Secret ab. Ein leerer Wert löscht den Eintrag. */
export function secretSet(username: string, secret: string): Promise<void> {
  return invoke<void>("secret_set", { username, secret });
}

/** Ob ein Secret gespeichert ist. Nur ja oder nein. */
export function secretExists(username: string): Promise<boolean> {
  return invoke<boolean>("secret_exists", { username });
}

/** Löscht das Secret. „War schon weg" ist kein Fehler. */
export function secretDelete(username: string): Promise<void> {
  return invoke<void>("secret_delete", { username });
}

/* -------------------------------------------------------------------------- */
/* Verbindungstest                                                            */
/* -------------------------------------------------------------------------- */

/**
 * Prüft eine Verbindung.
 *
 * `secret` darf ein noch nicht gespeicherter Wert aus dem Dialog sein — sonst
 * müsste man erst speichern, um testen zu können. Fehlt er, nimmt das Backend
 * den gespeicherten.
 */
export function connectionTest(
  connection: Connection,
  secret: string | null,
  timeoutSeconds: number,
): Promise<ConnectionReport> {
  return invoke<ConnectionReport>("connection_test", {
    connection,
    secret,
    timeoutSeconds,
  });
}

/* -------------------------------------------------------------------------- */
/* Abrufzustand                                                               */
/* -------------------------------------------------------------------------- */

/**
 * Der zuletzt bekannte Zustand.
 *
 * Beim Öffnen des Fensters nötig: war es versteckt, hat es das letzte Ereignis
 * nicht mitbekommen.
 */
export function statusCurrent(): Promise<StatusPayload> {
  return invoke<StatusPayload>("status_current");
}

/** Fordert einen sofortigen Abruf an. Bricht einen laufenden ab. */
export function refreshNow(): Promise<void> {
  return invoke<void>("refresh_now");
}

/**
 * Hört auf Zustandsmeldungen der Abrufschleife.
 *
 * Gibt die Abmeldefunktion zurück — die muss im Aufräumschritt des Effekts
 * gerufen werden, sonst sammeln sich bei jedem Neuaufbau der Komponente
 * weitere Zuhörer an.
 */
export function onStatus(
  handler: (status: StatusPayload) => void,
): Promise<UnlistenFn> {
  return listen<StatusPayload>(EVENT_STATUS, (event) => handler(event.payload));
}

/** Hört darauf, dass das Tray-Menü die Einstellungen öffnen will. */
export function onShowSettings(handler: () => void): Promise<UnlistenFn> {
  return listen(EVENT_SHOW_SETTINGS, () => handler());
}

/* -------------------------------------------------------------------------- */
/* Aktionen                                                                   */
/* -------------------------------------------------------------------------- */

/**
 * Öffnet die passende Ansicht in der CheckMK-Weboberfläche.
 *
 * `service === null` heisst: die Host-Ansicht. Die URL baut das Backend, damit
 * die Kodierung an einer Stelle liegt und dort getestet ist.
 */
export function openInCheckmk(
  host: string,
  service: string | null,
): Promise<void> {
  return invoke<void>("open_in_checkmk", { host, service });
}

/**
 * Schreibt die vollständige Problemliste des aktuellen Abzugs als CSV.
 *
 * Der Speicherdialog gehört zum Befehl: Dialog und Schreiben in einem Schritt
 * bedeuten einen Weg, auf dem der Pfad nie durch das Frontend läuft. Die
 * Rückgabe ist der geschriebene Pfad, oder `null` bei Abbruch — ein Abbruch ist
 * kein Fehler.
 *
 * Exportiert wird der **ganze** Abzug, unabhängig von den Filtern der Liste.
 */
export function exportCsv(): Promise<string | null> {
  return invoke<string | null>("export_csv");
}

/* -------------------------------------------------------------------------- */
/* Klänge                                                                     */
/* -------------------------------------------------------------------------- */

/**
 * Die eingebauten Klänge für die Auswahlfelder.
 *
 * Kommt aus dem Backend, weil die Klänge dort ins Programm eingebaut sind. Die
 * Liste hier nachzuschreiben wären zwei Wahrheiten, und eine Kennung, die
 * auseinanderläuft, ergibt eine Auswahl, die stumm bleibt.
 */
export function builtinSounds(): Promise<BuiltinSoundInfo[]> {
  return invoke<BuiltinSoundInfo[]>("builtin_sounds");
}

/**
 * Spielt eine Auswahl zum Vorhören.
 *
 * Nimmt die Auswahl aus dem Dialog, nicht aus den gespeicherten Einstellungen —
 * sonst liesse sich nur Gespeichertes probieren, und man müsste speichern, um
 * zu hören, was man gewählt hat.
 */
export function playSound(choice: SoundChoice): Promise<void> {
  return invoke<void>("play_sound", { choice });
}

/* -------------------------------------------------------------------------- */
/* Schreibaktionen                                                            */
/* -------------------------------------------------------------------------- */

/**
 * Holt den vorbelegten Kommentar für den Aktionsdialog.
 *
 * Die Vorlage steht in den Einstellungen, die Platzhalterersetzung im Backend.
 * Sie hier nachzubauen wären zwei Wahrheiten für denselben Text.
 */
export function actionComment(
  action: WriteAction,
  host: string,
  service: string | null,
): Promise<string> {
  return invoke<string>("action_comment", { action, host, service });
}

/**
 * Quittiert ein Problem.
 *
 * Ob es erlaubt ist, prüft das **Backend** — der ausgeblendete Knopf ist nur
 * die Anzeige derselben Einstellung, nicht ihre Durchsetzung.
 */
export function acknowledge(
  host: string,
  service: string | null,
  comment: string,
): Promise<void> {
  return invoke<void>("acknowledge", { host, service, comment });
}

/**
 * Setzt eine Wartungszeit. Beginn ist immer jetzt.
 *
 * `minutes` wird nur bei `duration === "custom"` ausgewertet.
 */
export function setDowntime(
  host: string,
  service: string | null,
  comment: string,
  duration: DowntimeChoice,
  minutes: number | null,
): Promise<void> {
  return invoke<void>("set_downtime", { host, service, comment, duration, minutes });
}

/**
 * Merkt die Anheftung des Fensters.
 *
 * Eigener Befehl statt `settingsSave`: das Anheften soll keinen Abruf auslösen
 * und keine Prüfung der ganzen Konfiguration durchlaufen. Es ist ein Umschalter
 * in der Titelzeile, kein Gang durch den Einstellungsdialog.
 */
export function setPinPopup(pinned: boolean): Promise<void> {
  return invoke<void>("set_pin_popup", { pinned });
}
