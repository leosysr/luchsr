/**
 * Die Problemansicht: Filterzeile, Liste, Detail-Panel.
 *
 * Hält den Anzeigezustand — Suchtext, Statusfilter, aufgeklappte Gruppen,
 * ausgewählte Zeile. Nichts davon geht in die Konfiguration: ein Filter ist
 * eine Momentaufnahme, und ihn zu speichern hiesse, beim nächsten Öffnen eine
 * gefilterte Liste zu sehen, ohne zu wissen warum.
 *
 * Einzige Ausnahme ist „Bearbeitete einblenden": das steht als `hideHandled`
 * in den Einstellungen und ist damit eine Vorliebe, keine Momentaufnahme. Der
 * Umschalter hier wirkt nur für diese Sitzung.
 */

import { useCallback, useEffect, useMemo, useState } from "react";

import { Callout } from "@/components";
import { asCommandError, exportCsv, openInCheckmk } from "@/lib/api";
import type { StatusKey } from "@/lib/status";
import { t } from "@/i18n";
import type { Problem, Settings, StatusPayload, WriteAction } from "@/lib/types";
import { ActionDialog } from "./ActionDialog";
import { DetailPanel } from "./DetailPanel";
import { FilterBar } from "./FilterBar";
import { ProblemList } from "./ProblemList";
import { buildRows, countByState, countHandled, problemId } from "./grouping";
import type { ListRow } from "./grouping";

interface ProblemsViewProps {
  status: StatusPayload | null;
  settings: Settings;
}

/**
 * Wie oft die Dauerspalte neu gerechnet wird.
 *
 * Eine Sekunde, weil die Spalte Sekunden zeigt. Dank Virtualisierung betrifft
 * das nur die sichtbaren Zeilen; die Gruppierung selbst wird nicht neu
 * berechnet, sie hängt nicht an der Zeit.
 */
const TICK_MS = 1000;

export function ProblemsView({ status, settings }: ProblemsViewProps) {
  const [query, setQuery] = useState("");
  const [states, setStates] = useState<ReadonlySet<StatusKey>>(new Set());
  const [includeHandled, setIncludeHandled] = useState(
    !settings.behaviour.hideHandled,
  );
  const [expandedHosts, setExpandedHosts] = useState<ReadonlySet<string>>(new Set());
  const [selectedId, setSelectedId] = useState<string | null>(null);
  /** Offener Aktionsdialog. `null` heisst: keiner. */
  const [pending, setPending] = useState<{ action: WriteAction; problem: Problem } | null>(
    null,
  );
  /** Beschriftung der letzten erfolgreichen Aktion, für die Meldung. */
  const [actionDone, setActionDone] = useState<string | null>(null);
  const [exporting, setExporting] = useState(false);
  /** Pfad der letzten geschriebenen Datei. `null` heisst: nichts zu melden. */
  const [exportPath, setExportPath] = useState<string | null>(null);
  const [now, setNow] = useState(() => Date.now());
  const [actionError, setActionError] = useState<string | null>(null);

  // Ändert der Benutzer die Vorliebe in den Einstellungen, folgt die Ansicht.
  useEffect(() => {
    setIncludeHandled(!settings.behaviour.hideHandled);
  }, [settings.behaviour.hideHandled]);

  useEffect(() => {
    const timer = window.setInterval(() => setNow(Date.now()), TICK_MS);
    return () => window.clearInterval(timer);
  }, []);

  const problems = status?.snapshot?.problems ?? [];

  const rows = useMemo(
    () => buildRows(problems, { query, states, includeHandled, expandedHosts }),
    [problems, query, states, includeHandled, expandedHosts],
  );

  const counts = useMemo(
    () => countByState(problems, includeHandled),
    [problems, includeHandled],
  );
  const handledCount = useMemo(() => countHandled(problems), [problems]);

  /**
   * Das ausgewählte Problem wird über die Zeilenliste aufgelöst, nicht als
   * Objekt gehalten. Nach einem Abruf sind alle Objekte neu; ein gehaltenes
   * würde veraltete Werte zeigen, während die Liste daneben aktuelle hat.
   */
  const selected: Problem | null = useMemo(() => {
    if (selectedId === null) return null;
    return problems.find((p) => problemId(p) === selectedId) ?? null;
  }, [problems, selectedId]);

  const toggleState = useCallback((key: StatusKey) => {
    setStates((current) => {
      const next = new Set(current);
      if (next.has(key)) next.delete(key);
      else next.add(key);
      return next;
    });
  }, []);

  const toggleHost = useCallback((host: string) => {
    setExpandedHosts((current) => {
      const next = new Set(current);
      if (next.has(host)) next.delete(host);
      else next.add(host);
      return next;
    });
  }, []);

  const select = useCallback((row: ListRow) => {
    // Ein zweiter Klick auf dieselbe Zeile schliesst das Panel wieder.
    setSelectedId((current) => (current === row.id ? null : row.id));
  }, []);

  const handleOpen = useCallback((problem: Problem) => {
    setActionError(null);
    openInCheckmk(problem.host, problem.service).catch((raw: unknown) => {
      setActionError(asCommandError(raw).message);
    });
  }, []);

  const handleAction = useCallback((action: WriteAction, problem: Problem) => {
    setActionError(null);
    setExportPath(null);
    setPending({ action, problem });
  }, []);

  /**
   * Nach Erfolg: Dialog zu, Erfolgsmeldung stehen lassen.
   *
   * Die Liste aktualisiert sich nicht hier — das Backend hat einen sofortigen
   * Abruf ausgelöst, und dessen Ergebnis kommt als Ereignis. Den Zustand
   * vorwegzunehmen hiesse, „quittiert" zu zeigen, bevor CheckMK es bestätigt
   * hat.
   */
  const handleActionDone = useCallback((label: string) => {
    setPending(null);
    setActionDone(label);
  }, []);

  const handleExport = useCallback(() => {
    setActionError(null);
    setExportPath(null);
    setExporting(true);
    exportCsv()
      // `null` heisst: der Speicherdialog wurde abgebrochen. Das ist kein
      // Fehler und bekommt deshalb auch keine Meldung.
      .then((pfad) => setExportPath(pfad))
      .catch((raw: unknown) => setActionError(asCommandError(raw).message))
      .finally(() => setExporting(false));
  }, []);

  const configured = status?.configured ?? false;

  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      <FilterBar
        query={query}
        onQueryChange={setQuery}
        states={states}
        onToggleState={toggleState}
        counts={counts}
        includeHandled={includeHandled}
        onIncludeHandledChange={setIncludeHandled}
        handledCount={handledCount}
        onExport={handleExport}
        exporting={exporting}
        canExport={problems.length > 0}
      />

      {status?.error ? (
        <div className="shrink-0 px-row-x pt-3">
          <Callout tone={configured ? "warn" : "info"} title={t("status.lastError")}>
            <p className="selectable">{status.error}</p>
            {status.failures > 1 ? (
              <p className="mt-1 text-sm text-muted">
                {t("status.consecutiveFailures")}:{" "}
                <span className="font-mono text-mono-sm">{status.failures}</span>
              </p>
            ) : null}
            {problems.length > 0 ? (
              <p className="mt-1 text-sm text-muted">{t("status.showingStale")}</p>
            ) : null}
          </Callout>
        </div>
      ) : null}

      {actionError ? (
        <div className="shrink-0 px-row-x pt-3">
          <Callout tone="crit" title={t("action.failed")}>
            <p className="selectable">{actionError}</p>
          </Callout>
        </div>
      ) : null}

      {exportPath ? (
        <div className="shrink-0 px-row-x pt-3">
          {/* Der Pfad ist die eigentliche Auskunft: der Speicherdialog lässt
              jeden Ort zu, und „gespeichert" allein hilft nicht beim Finden.
              Mono, weil es ein technischer Wert ist. */}
          <Callout tone="ok" title={t("export.done")}>
            <p className="selectable font-mono text-mono-sm break-all">{exportPath}</p>
          </Callout>
        </div>
      ) : null}

      {actionDone ? (
        <div className="shrink-0 px-row-x pt-3">
          <Callout tone="ok" title={actionDone}>
            <p className="text-sm text-muted">{t("action.doneHint")}</p>
          </Callout>
        </div>
      ) : null}

      <ProblemList
        rows={rows}
        now={now}
        selectedId={selectedId}
        onSelect={select}
        onToggleHost={toggleHost}
      />

      {selected ? (
        <DetailPanel
          problem={selected}
          now={now}
          onAction={handleAction}
          canAcknowledge={settings.permissions.allowAcknowledge}
          canDowntime={settings.permissions.allowDowntime}
          onClose={() => setSelectedId(null)}
          onOpenInCheckmk={handleOpen}
          canOpenInCheckmk={configured}
        />
      ) : null}

      {/* `key` sorgt dafür, dass ein Wechsel von Quittieren auf Wartungszeit
          einen frischen Dialog ergibt und nicht den alten Kommentar behält. */}
      {pending ? (
        <ActionDialog
          key={`${pending.action}:${problemId(pending.problem)}`}
          action={pending.action}
          problem={pending.problem}
          onClose={() => setPending(null)}
          onDone={handleActionDone}
        />
      ) : null}
    </div>
  );
}
