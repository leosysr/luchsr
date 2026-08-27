/**
 * Die Problemliste, virtualisiert.
 *
 * ## Warum virtualisiert
 *
 * Der Auftrag nennt 80 gleichzeitige Probleme als Messlatte. Aufgeklappte
 * Host-Gruppen können daraus deutlich mehr Zeilen machen. 300 DOM-Zeilen mit je
 * fünf Spalten sind in einem WebView spürbar; gerendert werden deshalb nur die
 * sichtbaren.
 *
 * Die Zeilenhöhe kommt über [`rowHeight`] aus `--row-height` — eine Zahl im
 * Code würde bei einer Änderung des Tokens die Rechnung aus dem Takt bringen,
 * ohne dass es auffällt.
 */

import { useEffect, useRef } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import { ChevronDown, ChevronRight } from "lucide-react";

import { STATUS } from "@/lib/status";
import { t } from "@/i18n";
import { rowHeight } from "@/lib/tokens";
import { durationSince, firstLine } from "./duration";
import { isHandled, statusKeyOf } from "./grouping";
import type { ListRow } from "./grouping";

interface ProblemListProps {
  rows: readonly ListRow[];
  /** Zeitbezug für die Dauerspalte. Wird von aussen gesetzt, damit alle
   *  Zeilen dieselbe Bezugszeit nutzen und nicht jede ihre eigene. */
  now: number;
  selectedId: string | null;
  onSelect: (row: ListRow) => void;
  onToggleHost: (host: string) => void;
}

export function ProblemList({
  rows,
  now,
  selectedId,
  onSelect,
  onToggleHost,
}: ProblemListProps) {
  const scrollRef = useRef<HTMLDivElement>(null);

  const virtualizer = useVirtualizer({
    count: rows.length,
    getScrollElement: () => scrollRef.current,
    estimateSize: () => rowHeight(),
    // Ein kleiner Vorlauf verhindert weisse Streifen beim schnellen Scrollen.
    overscan: 8,
    getItemKey: (index) => rows[index]?.id ?? index,
  });

  // Wird die Liste kürzer — etwa durch einen Filter — muss die Messung neu
  // laufen, sonst bleibt der Scrollbereich zu hoch.
  useEffect(() => {
    virtualizer.measure();
  }, [rows.length, virtualizer]);

  if (rows.length === 0) {
    return (
      <div className="flex flex-1 items-center justify-center p-8">
        <p className="text-base text-muted">{t("list.empty")}</p>
      </div>
    );
  }

  const items = virtualizer.getVirtualItems();

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <ListHeader />
      <div ref={scrollRef} className="min-h-0 flex-1 overflow-y-auto">
        <div
          className="relative w-full"
          style={{ height: `${virtualizer.getTotalSize()}px` }}
        >
          {items.map((item) => {
            const row = rows[item.index];
            if (!row) return null;
            return (
              <div
                key={item.key}
                className="absolute inset-x-0 top-0"
                style={{ transform: `translateY(${item.start}px)` }}
              >
                <ProblemRow
                  row={row}
                  now={now}
                  selected={row.id === selectedId}
                  onSelect={onSelect}
                  onToggleHost={onToggleHost}
                />
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}

/* -------------------------------------------------------------------------- */

/**
 * Spaltenbreiten.
 *
 * An einer Stelle, weil Kopfzeile und Datenzeilen sonst auseinanderlaufen —
 * der klassische Fehler bei einer Tabelle aus Flexboxen. Ein CSS-Grid wäre
 * eleganter, verträgt sich aber schlecht mit der Virtualisierung, die jede
 * Zeile absolut positioniert.
 */
const SPALTE = {
  chevron: "w-5 shrink-0",
  status: "w-16 shrink-0",
  host: "w-32 shrink-0",
  service: "w-44 shrink-0",
  dauer: "w-20 shrink-0 text-right",
  ausgabe: "min-w-0 flex-1",
} as const;

function ListHeader() {
  return (
    <div className="flex h-row shrink-0 items-center gap-row-gap border-b border-line bg-sunken px-row-x font-mono text-mono-xs font-semibold tracking-kicker text-faint uppercase">
      <span className={SPALTE.chevron} aria-hidden />
      <span className={SPALTE.status}>{t("list.column.status")}</span>
      <span className={SPALTE.host}>{t("list.column.host")}</span>
      <span className={SPALTE.service}>{t("list.column.service")}</span>
      <span className={SPALTE.dauer}>{t("list.column.duration")}</span>
      <span className={SPALTE.ausgabe}>{t("list.column.output")}</span>
    </div>
  );
}

interface ProblemRowProps {
  row: ListRow;
  now: number;
  selected: boolean;
  onSelect: (row: ListRow) => void;
  onToggleHost: (host: string) => void;
}

function ProblemRow({ row, now, selected, onSelect, onToggleHost }: ProblemRowProps) {
  const { problem, collapsedCount, nested, expanded } = row;
  const meta = STATUS[statusKeyOf(problem.state)];
  const Icon = meta.icon;
  const gruppe = collapsedCount > 0;
  const Chevron = expanded ? ChevronDown : ChevronRight;

  return (
    <div
      role="row"
      tabIndex={0}
      aria-selected={selected}
      onClick={() => onSelect(row)}
      onKeyDown={(event) => {
        if (event.key === "Enter" || event.key === " ") {
          event.preventDefault();
          onSelect(row);
        }
        // Pfeiltasten klappen eine Gruppe auf und zu, wie im Explorer.
        if (gruppe && event.key === "ArrowRight" && !expanded) {
          event.preventDefault();
          onToggleHost(problem.host);
        }
        if (gruppe && event.key === "ArrowLeft" && expanded) {
          event.preventDefault();
          onToggleHost(problem.host);
        }
      }}
      className={[
        "flex h-row cursor-default items-center gap-row-gap border-b border-line px-row-x",
        "transition-colors duration-fast ease-out",
        selected ? meta.soft : "hover:bg-sunken",
        // Bearbeitete Zustände treten optisch zurück, wie im Auftrag verlangt.
        isHandled(problem) ? "handled" : "",
      ].join(" ")}
    >
      <span className={SPALTE.chevron}>
        {gruppe ? (
          <button
            type="button"
            aria-label={expanded ? t("list.collapse") : t("list.expand")}
            aria-expanded={expanded}
            onClick={(event) => {
              // Sonst öffnet der Klick zusätzlich das Detail-Panel.
              event.stopPropagation();
              onToggleHost(problem.host);
            }}
            className="flex size-5 items-center justify-center rounded-xs text-muted transition-colors duration-fast ease-out hover:bg-card hover:text-body"
          >
            <Chevron size={14} aria-hidden />
          </button>
        ) : null}
      </span>

      <span className={`flex items-center gap-1 ${SPALTE.status} ${meta.fg}`}>
        <Icon size={14} aria-hidden />
        <span className="font-mono text-mono-xs font-semibold">{meta.short}</span>
      </span>

      <span className={`truncate font-mono text-mono-sm text-body ${SPALTE.host}`}>
        {problem.host}
      </span>

      <span
        className={`truncate font-mono text-mono-sm text-body ${SPALTE.service}`}
        // Einrückung nur für Zeilen unter einem ausgefallenen Host.
        style={nested ? { paddingLeft: "var(--space-4)" } : undefined}
      >
        {problem.service ?? (
          <span className="text-muted">
            {gruppe
              ? t("list.groupedServices").replace("{n}", String(collapsedCount))
              : "—"}
          </span>
        )}
      </span>

      <span className={`font-mono text-mono-xs text-muted ${SPALTE.dauer}`}>
        {durationSince(problem.lastStateChange, now)}
      </span>

      <span className={`truncate font-mono text-mono-xs text-muted ${SPALTE.ausgabe}`}>
        {problem.flapping ? (
          <span className="mr-1 font-semibold text-state-warn">
            {t("list.flapping")}
          </span>
        ) : null}
        {firstLine(problem.output)}
      </span>
    </div>
  );
}
