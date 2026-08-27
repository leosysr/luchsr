/**
 * Auffangnetz für Renderfehler.
 *
 * Ohne Boundary hängt React bei einem Fehler den ganzen Baum ab: das Popup ist
 * leer, weiss und stumm. Bei einer Anwendung im Infobereich gibt es keine
 * Konsole, in die man schauen könnte — der Benutzer sieht ein kaputtes Fenster
 * und hat nichts, was er melden könnte.
 *
 * Also beides: die Meldung auf dem Bildschirm, markierbar zum Kopieren, und
 * dieselbe Meldung ins Protokoll unter `%LOCALAPPDATA%\de.leosysr.luchsr\logs`
 * — dorthin, wo auch das Backend schreibt. Das ist dieselbe Begründung wie in
 * D32: Diagnose, die niemand sieht, ist toter Code.
 *
 * Klassenkomponente, weil `getDerivedStateFromError` und `componentDidCatch`
 * bis heute keine Entsprechung in Hooks haben.
 */

import { Component } from "react";
import type { ErrorInfo, ReactNode } from "react";
import { error as logError } from "@tauri-apps/plugin-log";

import { Button, Callout } from "@/components";
import { t } from "@/i18n";

interface Props {
  children: ReactNode;
}

interface State {
  fehler: Error | null;
  /** Komponentenpfad aus React. Sagt, *wo* es geknallt hat. */
  pfad: string | null;
}

export class ErrorBoundary extends Component<Props, State> {
  override state: State = { fehler: null, pfad: null };

  static getDerivedStateFromError(fehler: Error): Partial<State> {
    return { fehler };
  }

  override componentDidCatch(fehler: Error, info: ErrorInfo) {
    const pfad = info.componentStack ?? null;
    this.setState({ pfad });

    // Der Stapel gehört dazu: die reine Meldung eines Minified-Fehlers ist
    // ohne die Aufrufkette meist nicht zuzuordnen.
    const text = [
      `Renderfehler: ${fehler.message}`,
      fehler.stack ?? "",
      pfad ?? "",
    ]
      .filter((teil) => teil.length > 0)
      .join("\n");

    // Kein `await`: schlägt das Protokollieren fehl, darf das die Anzeige der
    // Meldung nicht verhindern — sie ist der wichtigere der beiden Wege.
    void logError(text).catch(() => undefined);
  }

  private neuLaden = () => {
    // Vollständiger Neuaufbau statt `setState({fehler: null})`. Der Zustand,
    // der zum Fehler geführt hat, steckt sonst noch in den Komponenten
    // darunter und der Fehler kommt sofort wieder.
    window.location.reload();
  };

  override render() {
    const { fehler, pfad } = this.state;
    if (!fehler) return this.props.children;

    return (
      <div className="min-h-0 flex-1 overflow-y-auto p-6">
        <Callout tone="crit" title={t("app.renderFailed")}>
          <p className="selectable">{fehler.message}</p>
          <p className="mt-2 text-sm text-muted">{t("app.renderFailedHint")}</p>
          {pfad ? (
            <details className="mt-3">
              <summary className="cursor-pointer text-sm text-muted">
                {t("action.details")}
              </summary>
              <pre className="selectable mt-2 overflow-x-auto rounded-sm bg-code-bg p-3 font-mono text-mono-xs text-code-text">
                {pfad.trim()}
              </pre>
            </details>
          ) : null}
          <div className="mt-4">
            <Button size="sm" variant="secondary" onClick={this.neuLaden}>
              {t("action.reload")}
            </Button>
          </div>
        </Callout>
      </div>
    );
  }
}
