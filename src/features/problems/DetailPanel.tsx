/**
 * Detail-Panel zu einer Zeile.
 *
 * Zeigt, was in der Liste nicht hineinpasst: den **vollständigen**
 * `plugin_output` mit allen Zeilen, die Zeitstempel und den Bearbeitungsstand.
 *
 * Die Aktionsknöpfe erscheinen nur, wenn die Freigabe in den Einstellungen
 * gesetzt ist — und sind gesperrt, wenn sie nichts mehr bewirken würden.
 * Durchgesetzt wird die Freigabe im Backend (`actions/mod.rs`); ein
 * ausgeblendeter Knopf ist Anzeige, keine Sicherung.
 *
 * Alles Technische ist mit `selectable` versehen — Hostname, Service und
 * Ausgabe wandern regelmässig in ein Ticket, und ein Text, den man nicht
 * markieren kann, ist dort nutzlos. Die App sperrt die Auswahl global, damit
 * sich das Fenster wie eine Anwendung anfühlt und nicht wie eine Webseite.
 */

import { Check, ExternalLink, Wrench, X } from "lucide-react";

import { Badge, Button } from "@/components";
import { STATUS } from "@/lib/status";
import { statusLabel, t } from "@/i18n";
import type { Problem, WriteAction } from "@/lib/types";
import { durationSince, formatTimestamp } from "./duration";
import { statusKeyOf } from "./grouping";

interface DetailPanelProps {
  problem: Problem;
  now: number;
  onClose: () => void;
  onOpenInCheckmk: (problem: Problem) => void;
  /** Ob überhaupt eine gültige Server-URL vorliegt. */
  canOpenInCheckmk: boolean;
  onAction: (action: WriteAction, problem: Problem) => void;
  /**
   * Freigaben aus den Einstellungen. Sie steuern nur, ob der Knopf **erscheint**
   * — durchgesetzt wird die Freigabe im Backend, siehe `actions/mod.rs`.
   */
  canAcknowledge: boolean;
  canDowntime: boolean;
}

export function DetailPanel({
  problem,
  now,
  onClose,
  onOpenInCheckmk,
  canOpenInCheckmk,
  onAction,
  canAcknowledge,
  canDowntime,
}: DetailPanelProps) {
  const key = statusKeyOf(problem.state);
  const meta = STATUS[key];
  const Icon = meta.icon;

  return (
    <section
      aria-label={t("detail.region")}
      className="flex max-h-detail shrink-0 flex-col border-t border-line bg-card"
    >
      <header className="flex shrink-0 items-center justify-between gap-4 border-b border-line px-row-x py-3">
        <div className="flex min-w-0 items-center gap-cgap-md">
          <span
            className={`flex shrink-0 items-center gap-1 rounded-sm px-badge-x py-badge-y font-mono text-mono-xs font-semibold tracking-badge uppercase ${meta.soft} ${meta.fg}`}
          >
            <Icon size={12} aria-hidden />
            {meta.short}
          </span>
          <p className="selectable min-w-0 truncate font-mono text-mono-sm font-semibold text-body">
            {problem.host}
            {problem.service ? (
              <span className="text-muted"> · {problem.service}</span>
            ) : null}
          </p>
        </div>

        <div className="flex shrink-0 items-center gap-cgap-md">
          {/* Quittieren ist bei einem schon quittierten Problem sinnlos, und
              eine zweite Wartungszeit über eine laufende zu legen erzeugt in
              CheckMK einen zweiten Eintrag, den niemand wollte. Deshalb hier
              gesperrt und nicht nur unschön. */}
          {canAcknowledge ? (
            <Button
              size="sm"
              variant="secondary"
              iconLeft={Check}
              disabled={problem.acknowledged}
              title={problem.acknowledged ? t("detail.alreadyAcknowledged") : undefined}
              onClick={() => onAction("acknowledge", problem)}
            >
              {t("action.acknowledge")}
            </Button>
          ) : null}
          {canDowntime ? (
            <Button
              size="sm"
              variant="secondary"
              iconLeft={Wrench}
              disabled={problem.downtimeDepth > 0}
              title={problem.downtimeDepth > 0 ? t("detail.alreadyDowntime") : undefined}
              onClick={() => onAction("downtime", problem)}
            >
              {t("action.downtime")}
            </Button>
          ) : null}
          {canOpenInCheckmk ? (
            <Button
              size="sm"
              variant="ghost"
              iconLeft={ExternalLink}
              onClick={() => onOpenInCheckmk(problem)}
            >
              {t("action.openInCheckmk")}
            </Button>
          ) : null}
          <button
            type="button"
            aria-label={t("detail.close")}
            title={t("detail.close")}
            onClick={onClose}
            className="flex size-control-sm items-center justify-center rounded-md text-muted transition-colors duration-fast ease-out press hover:bg-sunken hover:text-body"
          >
            <X size={16} aria-hidden />
          </button>
        </div>
      </header>

      <div className="min-h-0 flex-1 overflow-y-auto px-row-x py-4">
        <dl className="mb-4 grid grid-cols-[auto_1fr] gap-x-5 gap-y-2">
          <Zeile label={t("detail.state")}>
            <span className={meta.fg}>{statusLabel(key)}</span>
            {/* Der Rohzustand steht daneben: down und unreachable teilen sich
                das Symbol, unterscheiden sich aber in der Ursache. */}
            <span className="ml-2 font-mono text-mono-xs text-faint">
              {problem.state}
            </span>
          </Zeile>

          <Zeile label={t("detail.since")}>
            {formatTimestamp(problem.lastStateChange)}
          </Zeile>

          <Zeile label={t("detail.duration")}>
            {durationSince(problem.lastStateChange, now)}
          </Zeile>

          {problem.acknowledged || problem.downtimeDepth > 0 || problem.flapping ? (
            <Zeile label={t("list.column.status")}>
              <span className="flex flex-wrap gap-1">
                {problem.acknowledged ? (
                  <Badge tone="allow">{t("detail.acknowledged")}</Badge>
                ) : null}
                {problem.downtimeDepth > 0 ? (
                  <Badge tone="neutral">
                    {t("detail.downtime")} ·{" "}
                    {t("detail.downtimeDepth").replace(
                      "{n}",
                      String(problem.downtimeDepth),
                    )}
                  </Badge>
                ) : null}
                {problem.flapping ? (
                  <Badge tone="block">{t("detail.flapping")}</Badge>
                ) : null}
              </span>
            </Zeile>
          ) : null}
        </dl>

        <p className="mb-2 font-mono text-mono-xs font-semibold tracking-kicker text-faint uppercase">
          {t("detail.output")}
        </p>
        {/* Der vollständige Text, Zeilenumbrüche erhalten. `pre-wrap` statt
            `pre`: eine lange Zeile soll umbrechen und nicht waagerecht
            scrollen, sonst liest man sie in einem 780-px-Fenster nie. */}
        <pre className="selectable overflow-x-auto rounded-sm border border-line bg-code-bg p-4 font-mono text-mono-xs leading-mono whitespace-pre-wrap text-code-text">
          {problem.output || "—"}
        </pre>
      </div>
    </section>
  );
}

function Zeile({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <div className="col-span-2 grid grid-cols-subgrid items-baseline">
      <dt className="font-mono text-mono-xs tracking-kicker text-faint uppercase">
        {label}
      </dt>
      <dd className="selectable font-mono text-mono-sm text-body">{children}</dd>
    </div>
  );
}
