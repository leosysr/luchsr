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
 */

import { useEffect, useState } from "react";
import { ExternalLink } from "lucide-react";

import { aboutInfo, openProjectPage } from "@/lib/api";
import { t } from "@/i18n";
import type { AboutInfo } from "@/lib/types";

export function Colophon() {
  const [info, setInfo] = useState<AboutInfo | null>(null);

  useEffect(() => {
    // Scheitert der Abruf, bleibt die Zeile weg. Eine Fehlermeldung über eine
    // Fusszeile wäre lauter als die Fusszeile selbst.
    aboutInfo()
      .then(setInfo)
      .catch(() => undefined);
  }, []);

  if (!info) return null;

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
      <p>
        <button
          type="button"
          onClick={() => {
            openProjectPage().catch(() => undefined);
          }}
          className="inline-flex items-center gap-1 rounded-xs text-accent underline decoration-dotted underline-offset-2 transition-colors duration-fast ease-out hover:text-accent-solid-hover focus:outline-none focus-visible:ring-input"
        >
          <ExternalLink size={13} aria-hidden />
          {info.projectUrl.replace(/^https:\/\//, "")}
        </button>
      </p>
    </div>
  );
}
