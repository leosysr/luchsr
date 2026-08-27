/**
 * Filterzeile: Freitext, Statusklassen, Bearbeitete, Export.
 *
 * Alles in **einer** Zeile. Der Umschalter für „quittierte und Wartung
 * anzeigen" war zuerst ein Kontrollkästchen mit vollem Text in einer zweiten
 * Zeile — bei einem Popup von 600 px Höhe kostet das Platz, den die Liste
 * braucht, und optisch wog die Nebensache schwerer als die Statusfilter. Jetzt
 * ist es ein Umschalter in derselben Grösse wie die Statusknöpfe, mit
 * Augensymbol und Trefferzahl; der ausgeschriebene Text steht im Tooltip.
 *
 * Die Statusumschalter tragen ihre Anzahl. Eine leere Auswahl heisst **alle** —
 * beim Öffnen ist noch nichts angeklickt, und eine dann leere Liste wäre
 * verwirrend.
 */

import { Download, Eye, EyeOff, Search, X } from "lucide-react";

import { STATUS_BY_SEVERITY } from "@/lib/status";
import type { StatusKey } from "@/lib/status";
import { t } from "@/i18n";
import type { StateCounts } from "./grouping";

interface FilterBarProps {
  query: string;
  onQueryChange: (value: string) => void;
  states: ReadonlySet<StatusKey>;
  onToggleState: (key: StatusKey) => void;
  counts: StateCounts;
  includeHandled: boolean;
  onIncludeHandledChange: (value: boolean) => void;
  handledCount: number;
  onExport: () => void;
  /** Läuft ein Export? Solange bleibt der Knopf gesperrt. */
  exporting: boolean;
  /** Ohne Abzug gibt es nichts zu exportieren. */
  canExport: boolean;
}

/**
 * Welche Klassen als Umschalter erscheinen.
 *
 * `ok` und `stale` fehlen: der Abzug enthält nur Probleme, also nie `ok`, und
 * `stale` ist kein CheckMK-Zustand. Ein Umschalter, der immer 0 zeigt, ist nur
 * Fläche.
 */
const FILTERBAR: readonly StatusKey[] = ["down", "crit", "unknown", "warn"];

/** Gemeinsame Form aller Knöpfe der Zeile — Höhe, Radius, Schrift, Druck. */
const KNOPF = [
  "inline-flex h-control-sm items-center gap-1 rounded-md border px-cpx-sm",
  "font-mono text-mono-xs font-semibold tracking-badge uppercase",
  "transition-colors duration-fast ease-out press",
].join(" ");

/** Ruhezustand: mitlaufend, aber nicht laut. */
const RUHIG = "border-line bg-card text-muted hover:bg-sunken";

export function FilterBar({
  query,
  onQueryChange,
  states,
  onToggleState,
  counts,
  includeHandled,
  onIncludeHandledChange,
  handledCount,
  onExport,
  exporting,
  canExport,
}: FilterBarProps) {
  const HandledIcon = includeHandled ? Eye : EyeOff;

  return (
    <div className="flex items-center gap-cgap-md border-b border-line bg-page px-row-x py-3">
      <div className="relative min-w-0 flex-1">
        <Search
          size={16}
          aria-hidden
          className="pointer-events-none absolute inset-y-0 left-cpx-sm my-auto text-faint"
        />
        <input
          type="search"
          value={query}
          placeholder={t("list.filterPlaceholder")}
          aria-label={t("list.filterPlaceholder")}
          onChange={(event) => onQueryChange(event.target.value)}
          className={[
            "h-control-sm w-full rounded-md border border-line-strong bg-card",
            "pl-9 pr-cpx-sm font-mono text-mono-sm text-body",
            "transition-colors duration-fast ease-out placeholder:text-faint",
            "focus:outline-none focus-visible:border-accent-solid focus-visible:ring-input",
            // Das native Löschkreuz von WebView2 passt nicht ins Design.
            "[&::-webkit-search-cancel-button]:hidden",
          ].join(" ")}
        />
        {query ? (
          <button
            type="button"
            aria-label={t("list.clearFilter")}
            title={t("list.clearFilter")}
            onClick={() => onQueryChange("")}
            className="absolute inset-y-0 right-1 my-auto flex size-6 items-center justify-center rounded-xs text-muted transition-colors duration-fast ease-out hover:bg-sunken hover:text-body"
          >
            <X size={14} aria-hidden />
          </button>
        ) : null}
      </div>

      <div
        role="group"
        aria-label={t("list.filterStates")}
        className="flex shrink-0 gap-1"
      >
        {FILTERBAR.map((key) => {
          const meta = STATUS_BY_SEVERITY.find((s) => s.key === key)!;
          const Icon = meta.icon;
          const active = states.has(key);
          const count = counts[key];
          return (
            <button
              key={key}
              type="button"
              aria-pressed={active}
              onClick={() => onToggleState(key)}
              title={`${meta.short} — ${count}`}
              className={[
                KNOPF,
                // Ohne Treffer optisch zurücknehmen, aber anklickbar lassen:
                // die Zahl ist selbst eine Auskunft.
                count === 0 && !active ? "handled" : "",
                active ? `${meta.soft} ${meta.fg} ${meta.ring}` : RUHIG,
              ].join(" ")}
            >
              <Icon size={12} aria-hidden />
              {count}
            </button>
          );
        })}
      </div>

      {/* Eigene Achse, deshalb abgesetzt und nicht in der Statusgruppe. */}
      <div className="flex shrink-0 items-center gap-1 border-l border-line pl-cgap-md">
        <button
          type="button"
          aria-pressed={includeHandled}
          onClick={() => onIncludeHandledChange(!includeHandled)}
          title={t("list.showHandled")}
          aria-label={t("list.showHandled")}
          className={[
            KNOPF,
            handledCount === 0 && !includeHandled ? "handled" : "",
            includeHandled ? "border-accent-solid bg-accent-soft text-accent" : RUHIG,
          ].join(" ")}
        >
          <HandledIcon size={12} aria-hidden />
          {handledCount}
        </button>

        <button
          type="button"
          onClick={onExport}
          disabled={exporting || !canExport}
          title={t("action.exportCsv")}
          aria-label={t("action.exportCsv")}
          className={[
            KNOPF,
            RUHIG,
            exporting || !canExport ? "is-disabled pointer-events-none" : "",
          ].join(" ")}
        >
          <Download size={12} aria-hidden />
        </button>
      </div>
    </div>
  );
}
