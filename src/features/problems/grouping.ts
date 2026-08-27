/**
 * Filtern, Gruppieren und Abflachen der Problemliste.
 *
 * Reine Funktionen über den Abzug. Die Liste muss bei 80 gleichzeitigen
 * Problemen brauchbar bleiben, und ob sie das tut, entscheidet sich hier —
 * nicht in der Darstellung. Deshalb steht die Logik getrennt und getestet.
 */

import type { Problem, ProblemState } from "@/lib/types";
import type { StatusKey } from "@/lib/status";

/** Eine Zeile der abgeflachten Liste. */
export interface ListRow {
  /** Stabiler Schlüssel. Muss über Aktualisierungen hinweg gleich bleiben,
   *  sonst verliert die virtualisierte Liste ihre Scrollposition. */
  id: string;
  problem: Problem;
  /** Zahl der eingeklappten Services. Nur bei Host-Zeilen grösser als 0. */
  collapsedCount: number;
  /** Eingerückt, weil unter einem ausgefallenen Host. */
  nested: boolean;
  /** Ob diese Host-Gruppe aufgeklappt ist. */
  expanded: boolean;
}

export interface ListOptions {
  /** Freitext über Host und Service. Leer heisst: kein Textfilter. */
  query: string;
  /** Sichtbare Statusklassen. Eine leere Menge heisst **alle**, nicht keine. */
  states: ReadonlySet<StatusKey>;
  /** Quittierte und Wartungszeiten mit anzeigen. */
  includeHandled: boolean;
  /** Aufgeklappte Host-Gruppen. */
  expandedHosts: ReadonlySet<string>;
}

/** Zähler je Statusklasse, für die Beschriftung der Filter-Umschalter. */
export type StateCounts = Record<StatusKey, number>;

/* -------------------------------------------------------------------------- */

/**
 * Anzeigeschlüssel eines Problems.
 *
 * `unreachable` fällt mit `down` zusammen — für den Benutzer ist beides „der
 * Host ist weg". Dieselbe Abbildung wie `ProblemState::status_key` in Rust.
 */
export function statusKeyOf(state: ProblemState): StatusKey {
  switch (state) {
    case "ok":
      return "ok";
    case "warn":
      return "warn";
    case "crit":
      return "crit";
    case "unknown":
      return "unknown";
    case "down":
    case "unreachable":
      return "down";
  }
}

/** Quittiert oder in Wartungszeit — standardmässig ausgeblendet. */
export function isHandled(problem: Problem): boolean {
  return problem.acknowledged || problem.downtimeDepth > 0;
}

/** Ob das Problem den Host selbst betrifft. */
export function isHostProblem(problem: Problem): boolean {
  return problem.service === null;
}

/**
 * Stabiler Schlüssel.
 *
 * Längenpräfigiert statt mit Trennzeichen verkettet — dieselbe Überlegung wie
 * bei `Problem::notification_key` in Rust: `("host", "a|b")` und
 * `("host|a", "b")` dürfen nicht denselben Schlüssel ergeben, sonst tauscht die
 * virtualisierte Liste zwei Zeilen gegeneinander aus.
 */
export function problemId(problem: Problem): string {
  const service = problem.service ?? "";
  const kind = problem.service === null ? "H" : "S";
  return `${kind}|${problem.host.length}:${problem.host}|${service.length}:${service}`;
}

/** Ob Host oder Service den Suchtext enthalten. */
function matchesQuery(problem: Problem, needle: string): boolean {
  return (
    problem.host.toLowerCase().includes(needle) ||
    (problem.service ?? "").toLowerCase().includes(needle)
  );
}

/* -------------------------------------------------------------------------- */

/**
 * Baut die abgeflachte Liste.
 *
 * ## Reihenfolge
 *
 * `problems` kommt aus dem Abzug bereits sortiert (Status absteigend, dann
 * Dauer absteigend). Diese Funktion **erhält** die Reihenfolge und sortiert
 * nicht neu — die Sortierung ist im Backend definiert und dort getestet.
 *
 * ## Gruppierung
 *
 * Ist ein Host ausgefallen, erscheint **eine** Zeile für den Host und dessen
 * Services darunter zusammengefasst. Der Auftrag begründet das selbst: nicht
 * vierzig rote Zeilen für einen einzigen ausgefallenen Rechner.
 *
 * **Bei einem Textfilter entfällt die Gruppierung.** Wer sucht, will genau die
 * Treffer sehen und nicht eine Gruppe, in der der gesuchte Service versteckt
 * ist. Statusfilter lassen die Gruppierung dagegen bestehen — dort blättert man
 * weiter durch eine Übersicht.
 */
export function buildRows(
  problems: readonly Problem[],
  options: ListOptions,
): ListRow[] {
  const { query, states, includeHandled, expandedHosts } = options;

  let visible = problems.filter((p) => includeHandled || !isHandled(p));
  if (states.size > 0) {
    visible = visible.filter((p) => states.has(statusKeyOf(p.state)));
  }

  const needle = query.trim().toLowerCase();
  if (needle.length > 0) {
    return visible
      .filter((p) => matchesQuery(p, needle))
      .map((problem) => ({
        id: problemId(problem),
        problem,
        collapsedCount: 0,
        nested: false,
        expanded: false,
      }));
  }

  // Hosts, die selbst ausgefallen sind und deshalb eine Gruppe bilden.
  const failedHosts = new Set(
    visible
      .filter(
        (p) =>
          isHostProblem(p) && (p.state === "down" || p.state === "unreachable"),
      )
      .map((p) => p.host),
  );

  // Services dieser Hosts vorindizieren, damit die Reihenfolge des Abzugs
  // erhalten bleibt.
  const grouped = new Map<string, Problem[]>();
  for (const problem of visible) {
    if (!isHostProblem(problem) && failedHosts.has(problem.host)) {
      const list = grouped.get(problem.host);
      if (list) list.push(problem);
      else grouped.set(problem.host, [problem]);
    }
  }

  const rows: ListRow[] = [];
  for (const problem of visible) {
    if (isHostProblem(problem) && failedHosts.has(problem.host)) {
      const services = grouped.get(problem.host) ?? [];
      const expanded = expandedHosts.has(problem.host);
      rows.push({
        id: problemId(problem),
        problem,
        collapsedCount: services.length,
        nested: false,
        expanded,
      });
      if (expanded) {
        for (const service of services) {
          rows.push({
            id: problemId(service),
            problem: service,
            collapsedCount: 0,
            nested: true,
            expanded: false,
          });
        }
      }
      continue;
    }

    // Services eines ausgefallenen Hosts stecken in der Gruppe.
    if (!isHostProblem(problem) && failedHosts.has(problem.host)) continue;

    rows.push({
      id: problemId(problem),
      problem,
      collapsedCount: 0,
      nested: false,
      expanded: false,
    });
  }

  return rows;
}

/**
 * Zählt je Statusklasse.
 *
 * Grundlage sind **nur** der Bearbeitungsfilter, nicht die Statusfilter selbst:
 * ein Umschalter, dessen Zahl auf 0 fällt, sobald man ihn abwählt, wäre nicht
 * mehr wiederzufinden.
 */
export function countByState(
  problems: readonly Problem[],
  includeHandled: boolean,
): StateCounts {
  const counts: StateCounts = {
    ok: 0,
    warn: 0,
    crit: 0,
    unknown: 0,
    down: 0,
    stale: 0,
  };
  for (const problem of problems) {
    if (!includeHandled && isHandled(problem)) continue;
    counts[statusKeyOf(problem.state)] += 1;
  }
  return counts;
}

/** Zahl der bearbeiteten Probleme — für die Beschriftung des Umschalters. */
export function countHandled(problems: readonly Problem[]): number {
  return problems.filter(isHandled).length;
}
