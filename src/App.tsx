/**
 * Anwendungsrahmen des Popup-Fensters.
 *
 * Das Fenster ist rahmenlos, erscheint nicht in der Taskleiste und wird vom
 * Tray-Icon geöffnet. Dekoration bringt [`PopupChrome`] mit — ohne die gäbe es
 * keinen Schliessknopf und keine Ziehfläche.
 *
 * Drei Zustände:
 *
 *   needsSetup  → Ersteinrichtung, nur der Verbindungsteil
 *   problems    → die Problemliste (Standard)
 *   settings    → der Einstellungsdialog
 */

import { useCallback, useEffect, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

import { Callout } from "@/components";
import { ProblemsView } from "@/features/problems/ProblemsView";
import { SettingsView } from "@/features/settings/SettingsView";
import { ErrorBoundary } from "@/features/shell/ErrorBoundary";
import { PopupChrome } from "@/features/shell/PopupChrome";
import {
  asCommandError,
  onShowSettings,
  onStatus,
  refreshNow,
  setPinPopup,
  settingsLoad,
  statusCurrent,
} from "@/lib/api";
import { applyTheme } from "@/lib/theme";
import { t } from "@/i18n";
import type { CommandError, LoadOutcome, Settings, StatusPayload } from "@/lib/types";

type View = "problems" | "settings";

export default function App() {
  const [outcome, setOutcome] = useState<LoadOutcome | null>(null);
  const [status, setStatus] = useState<StatusPayload | null>(null);
  const [error, setError] = useState<CommandError | null>(null);
  const [view, setView] = useState<View>("problems");
  const [refreshing, setRefreshing] = useState(false);

  /* ------------------------------------------------------ Einstellungen -- */

  useEffect(() => {
    settingsLoad()
      .then((loaded) => {
        setOutcome(loaded);
        applyTheme(loaded.settings.appearance.theme);
      })
      .catch((raw: unknown) => setError(asCommandError(raw)));
  }, []);

  /* ------------------------------------------------------------ Zustand -- */

  useEffect(() => {
    // Beim Öffnen den aktuellen Stand holen: war das Fenster versteckt, hat es
    // das letzte Ereignis nicht mitbekommen.
    statusCurrent()
      .then(setStatus)
      .catch(() => {
        // Kein Grund für eine Fehlermeldung — das Ereignis kommt gleich.
      });

    const abmelden: Array<() => void> = [];
    let verworfen = false;
    const merken = (off: () => void) => {
      if (verworfen) off();
      else abmelden.push(off);
    };

    void onStatus(setStatus).then(merken);
    void onShowSettings(() => setView("settings")).then(merken);

    return () => {
      verworfen = true;
      for (const off of abmelden) off();
    };
  }, []);

  /* ---------------------------------------------------------- Aktionen -- */

  const handleRefresh = useCallback(() => {
    setRefreshing(true);
    refreshNow()
      .catch((raw: unknown) => setError(asCommandError(raw)))
      // Der Abruf läuft im Backend weiter; die Sperre ist nur gegen
      // Doppelklicks. Das Ergebnis kommt als Ereignis.
      .finally(() => window.setTimeout(() => setRefreshing(false), 600));
  }, []);

  const handleClose = useCallback(() => {
    // Verstecken, nicht schliessen: die Anwendung lebt im Infobereich weiter.
    // `close()` würde das Fenster zerstören und den Tray-Klick wirkungslos
    // machen.
    void getCurrentWindow().hide();
  }, []);

  const pinned = outcome?.settings.behaviour.pinPopup ?? false;

  const handleTogglePin = useCallback(() => {
    if (!outcome) return;
    const next = !outcome.settings.behaviour.pinPopup;
    setOutcome({
      ...outcome,
      settings: {
        ...outcome.settings,
        behaviour: { ...outcome.settings.behaviour, pinPopup: next },
      },
    });
    setPinPopup(next).catch((raw: unknown) => setError(asCommandError(raw)));
  }, [outcome]);

  /* -------------------------------------------------------- Darstellung -- */

  if (error) {
    return (
      <Shell>
        <PopupChrome
          status={status}
          pinned={pinned}
          onTogglePin={handleTogglePin}
          onRefresh={handleRefresh}
          refreshing={refreshing}
          onOpenSettings={() => setView("settings")}
          onClose={handleClose}
          showingSettings={false}
        />
        <div className="p-6">
          <Callout tone="crit" title={t("app.startFailed")}>
            <p className="selectable">{error.message}</p>
          </Callout>
        </div>
      </Shell>
    );
  }

  if (!outcome) {
    // Kein Ladeindikator: das Laden ist ein Dateizugriff und dauert
    // Millisekunden. Ein aufblitzender Spinner wäre unruhiger als nichts.
    return <Shell />;
  }

  // Die Ersteinrichtung bekommt eine Ziehfläche und einen Schliessknopf, aber
  // keine Reiter — niemand soll in einer Liste landen, bevor eine Verbindung
  // steht.
  if (outcome.needsSetup) {
    return (
      <Shell>
        <PopupChrome
          status={status}
          pinned={pinned}
          onTogglePin={handleTogglePin}
          onRefresh={handleRefresh}
          refreshing={refreshing}
          onOpenSettings={() => undefined}
          onClose={handleClose}
          showingSettings
        />
        <ErrorBoundary>
          <div className="min-h-0 flex-1 overflow-y-auto">
            <SettingsView
              mode="setup"
              initial={outcome}
              onSaved={(settings: Settings) =>
                setOutcome({ ...outcome, settings, needsSetup: false })
              }
            />
          </div>
        </ErrorBoundary>
      </Shell>
    );
  }

  return (
    <Shell>
      <PopupChrome
        status={status}
        pinned={pinned}
        onTogglePin={handleTogglePin}
        onRefresh={handleRefresh}
        refreshing={refreshing}
        onOpenSettings={() =>
          setView((current) => (current === "settings" ? "problems" : "settings"))
        }
        onClose={handleClose}
        showingSettings={view === "settings"}
      />

      {/* Die Boundary sitzt unter der Titelzeile, nicht darüber: stürzt die
          Liste ab, müssen Schliessknopf und Ziehfläche weiter funktionieren —
          sonst bleibt ein rahmenloses Fenster stehen, das man nicht loswird.
          `key` setzt sie beim Wechsel zurück, damit ein Fehler in der Liste
          nicht auch die Einstellungen blockiert. */}
      <ErrorBoundary key={view}>
        {view === "problems" ? (
          <ProblemsView status={status} settings={outcome.settings} />
        ) : (
          <div className="min-h-0 flex-1 overflow-y-auto">
            <SettingsView
              mode="settings"
              initial={outcome}
              onSaved={(settings: Settings) => setOutcome({ ...outcome, settings })}
            />
          </div>
        )}
      </ErrorBoundary>
    </Shell>
  );
}

/**
 * Aussenrahmen.
 *
 * `h-full` plus `flex-col`: die Liste soll den verbleibenden Platz füllen und
 * innen scrollen, nicht das ganze Fenster. Ein Rand markiert die Kante — ohne
 * Fensterdekoration gibt es sonst keine.
 */
function Shell({ children }: { children?: React.ReactNode }) {
  return (
    <div className="flex h-full flex-col overflow-hidden border border-line bg-page">
      {children}
    </div>
  );
}
