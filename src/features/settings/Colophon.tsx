/**
 * Kleingedrucktes am Ende der Einstellungen: Version, Lizenz, Herkunft.
 *
 * Der Hinweis auf die KI-Beteiligung steht hier, weil es der Ort ist, an dem
 * man nach so etwas sucht — und weil er nicht in den Weg gehört. Ein Banner
 * über der Problemliste wäre falsch: wer ein Monitoringwerkzeug öffnet, will
 * Probleme sehen, nicht lesen, wie das Werkzeug entstanden ist.
 *
 * Die Version kommt aus dem Backend (`about_info`), nicht aus einer Konstante
 * hier: sie steht ausschliesslich in `tauri.conf.json`, und sie an zwei Stellen
 * zu pflegen hiesse, sie beim nächsten Release an einer davon falsch zu haben.
 *
 * ## Warum der Update-Check hier sitzt und ein Knopf ist
 *
 * Hier, weil die Version daneben steht — „welche habe ich" und „gibt es eine
 * neuere" sind dieselbe Frage. Und als Knopf, nicht als Automatik: die
 * GitHub-API erlaubt unangemeldet 60 Anfragen je Stunde und IP, und hinter
 * einem Firmen-NAT teilen sich das alle Rechner. Ausführliche Begründung im
 * Kopf von `src-tauri/src/update/mod.rs`.
 */

import { useEffect, useState } from "react";
import { ExternalLink, RefreshCw } from "lucide-react";

import { aboutInfo, asCommandError, checkForUpdate, openProjectPage, openReleasePage } from "@/lib/api";
import { t } from "@/i18n";
import type { AboutInfo, UpdateReport } from "@/lib/types";

/** Was der Check gerade macht. */
type CheckState =
  | { phase: "idle" }
  | { phase: "running" }
  | { phase: "done"; report: UpdateReport }
  | { phase: "failed"; message: string };

export function Colophon() {
  const [info, setInfo] = useState<AboutInfo | null>(null);
  const [check, setCheck] = useState<CheckState>({ phase: "idle" });
  /**
   * Fehlschlag beim Öffnen des Browsers.
   *
   * Eigener Zustand, weil er nichts mit dem Update-Check zu tun hat. Und
   * überhaupt einer: vorher stand hier `.catch(() => undefined)`, und ein Link,
   * der nichts tut, ist genau der Fehler, der beim Schliessknopf Wochen
   * unentdeckt geblieben ist (D101). Die Meldung aus dem Backend nennt auch die
   * Adresse — dann kann man sie notfalls selbst öffnen.
   */
  const [linkFehler, setLinkFehler] = useState<string | null>(null);

  useEffect(() => {
    // Scheitert der Abruf, bleibt die Zeile weg. Eine Fehlermeldung über eine
    // Fusszeile wäre lauter als die Fusszeile selbst.
    aboutInfo()
      .then(setInfo)
      .catch(() => undefined);
  }, []);

  if (!info) return null;

  const laeuft = check.phase === "running";

  return (
    <div className="flex flex-col gap-1 border-t border-line pt-4 text-sm text-faint">
      <p>
        {t("app.name")}{" "}
        <span className="font-mono text-mono-xs">{info.version}</span>
        {" · "}
        {t("about.author")}
        {" · "}
        {t("about.license")}
      </p>
      <p>{t("about.ai")}</p>

      <p className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <button
          type="button"
          disabled={laeuft}
          onClick={() => {
            setCheck({ phase: "running" });
            checkForUpdate()
              .then((report) => setCheck({ phase: "done", report }))
              .catch((raw: unknown) =>
                // Der Fehlschlag wird gezeigt, nicht verschluckt: die Meldung
                // aus dem Backend nennt die Ursache — Proxy, Anfragelimit,
                // oder eine Antwort, die nicht von GitHub kam.
                setCheck({ phase: "failed", message: asCommandError(raw).message }),
              );
          }}
          className="inline-flex items-center gap-1 rounded-xs text-accent underline decoration-dotted underline-offset-2 transition-colors duration-fast ease-out hover:text-accent-solid-hover focus:outline-none focus-visible:ring-input disabled:is-disabled disabled:pointer-events-none"
        >
          <RefreshCw size={13} aria-hidden className={laeuft ? "animate-spin" : undefined} />
          {laeuft ? t("update.checking") : t("update.check")}
        </button>

        {check.phase === "done" ? (
          <Ergebnis report={check.report} onError={setLinkFehler} />
        ) : null}
        {check.phase === "failed" ? (
          <span className="text-state-warn selectable">{check.message}</span>
        ) : null}
      </p>

      <p>
        <button
          type="button"
          onClick={() => {
            setLinkFehler(null);
            openProjectPage().catch((raw: unknown) =>
              setLinkFehler(asCommandError(raw).message),
            );
          }}
          className="inline-flex items-center gap-1 rounded-xs text-accent underline decoration-dotted underline-offset-2 transition-colors duration-fast ease-out hover:text-accent-solid-hover focus:outline-none focus-visible:ring-input"
        >
          <ExternalLink size={13} aria-hidden />
          {info.projectUrl.replace(/^https:\/\//, "")}
        </button>
      </p>

      {linkFehler ? <p className="text-state-warn selectable">{linkFehler}</p> : null}
    </div>
  );
}

/**
 * Das Ergebnis in einem Satz.
 *
 * `ahead` bekommt einen eigenen Fall und wird nicht als „aktuell" gemeldet.
 * Bei einem Entwicklungsbau ist der Unterschied gerade der interessante — und
 * „aktuell" wäre dort schlicht falsch.
 */
function Ergebnis({
  report,
  onError,
}: {
  report: UpdateReport;
  onError: (message: string) => void;
}) {
  if (report.verdict === "upToDate") {
    return <span>{t("update.upToDate")}</span>;
  }

  const text = report.verdict === "updateAvailable" ? t("update.available") : t("update.ahead");

  return (
    <span className="flex flex-wrap items-center gap-x-2">
      <span>
        {text} <span className="font-mono text-mono-xs">{report.latest}</span>
      </span>
      <button
        type="button"
        onClick={() => {
          // Auch hier gezeigt statt verschluckt. Das Backend lehnt eine Adresse
          // ab, die nicht unter den Releases dieses Repositorys liegt — bliebe
          // die Ablehnung stumm, sähe es wie ein kaputter Knopf aus statt wie
          // die Sicherung, die sie ist.
          openReleasePage(report.releaseUrl).catch((raw: unknown) =>
            onError(asCommandError(raw).message),
          );
        }}
        className="inline-flex items-center gap-1 rounded-xs text-accent underline decoration-dotted underline-offset-2 transition-colors duration-fast ease-out hover:text-accent-solid-hover focus:outline-none focus-visible:ring-input"
      >
        <ExternalLink size={13} aria-hidden />
        {t("update.openRelease")}
      </button>
      {report.verdict === "updateAvailable" ? (
        <span className="w-full">{t("update.hint")}</span>
      ) : null}
    </span>
  );
}
