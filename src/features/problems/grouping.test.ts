import { describe, expect, it } from "vitest";

import type { Problem, ProblemState } from "@/lib/types";
import type { StatusKey } from "@/lib/status";
import {
  buildRows,
  countByState,
  countHandled,
  isHandled,
  problemId,
  statusKeyOf,
} from "./grouping";

/* -------------------------------------------------------------------------- */
/* Hilfsmittel                                                                */
/* -------------------------------------------------------------------------- */

function problem(
  state: ProblemState,
  host: string,
  service: string | null,
  extra: Partial<Problem> = {},
): Problem {
  return {
    host,
    service,
    state,
    output: "",
    lastStateChange: null,
    acknowledged: false,
    downtimeDepth: 0,
    flapping: false,
    ...extra,
  };
}

const alle: ReadonlySet<StatusKey> = new Set();
const keineGruppen: ReadonlySet<string> = new Set();

function optionen(over: Partial<Parameters<typeof buildRows>[1]> = {}) {
  return {
    query: "",
    states: alle,
    includeHandled: false,
    expandedHosts: keineGruppen,
    ...over,
  };
}

/* -------------------------------------------------------------------------- */

describe("statusKeyOf", () => {
  it("bildet die Zustände ab", () => {
    expect(statusKeyOf("ok")).toBe("ok");
    expect(statusKeyOf("warn")).toBe("warn");
    expect(statusKeyOf("crit")).toBe("crit");
    expect(statusKeyOf("unknown")).toBe("unknown");
  });

  it("fasst down und unreachable zusammen", () => {
    expect(statusKeyOf("down")).toBe("down");
    expect(statusKeyOf("unreachable")).toBe("down");
  });
});

describe("isHandled", () => {
  it("erkennt quittiert und Wartungszeit", () => {
    expect(isHandled(problem("crit", "h", "s"))).toBe(false);
    expect(isHandled(problem("crit", "h", "s", { acknowledged: true }))).toBe(true);
    expect(isHandled(problem("crit", "h", "s", { downtimeDepth: 1 }))).toBe(true);
    expect(isHandled(problem("crit", "h", "s", { downtimeDepth: 3 }))).toBe(true);
  });
});

describe("problemId", () => {
  it("unterscheidet Host- und Serviceprobleme", () => {
    expect(problemId(problem("down", "h", null))).not.toBe(
      problemId(problem("crit", "h", "")),
    );
  });

  /// Ohne Längenpräfix wären das dieselben Schlüssel, und die virtualisierte
  /// Liste würde zwei Zeilen gegeneinander austauschen.
  it("kollidiert nicht, wenn ein Trennzeichen in den Daten steckt", () => {
    for (const trenner of ["|", ":", "|3:"]) {
      const a = problemId(problem("crit", "host", `a${trenner}b`));
      const b = problemId(problem("crit", `host${trenner}a`, "b"));
      expect(a).not.toBe(b);
    }
  });

  it("ist für dasselbe Problem stabil", () => {
    const p = problem("crit", "h", "s");
    expect(problemId(p)).toBe(problemId({ ...p, output: "anders" }));
  });
});

/* -------------------------------------------------------------------------- */
/* Filtern                                                                    */
/* -------------------------------------------------------------------------- */

describe("buildRows – Filter", () => {
  const liste = [
    problem("crit", "sql-01", "Disk"),
    problem("warn", "sql-01", "Memory", { acknowledged: true }),
    problem("warn", "web-01", "HTTP", { downtimeDepth: 2 }),
    problem("unknown", "print-01", "Agent"),
  ];

  it("blendet quittierte und Wartungszeiten standardmässig aus", () => {
    const rows = buildRows(liste, optionen());
    expect(rows.map((r) => r.problem.host)).toEqual(["sql-01", "print-01"]);
  });

  it("blendet sie per Umschalter ein", () => {
    const rows = buildRows(liste, optionen({ includeHandled: true }));
    expect(rows).toHaveLength(4);
  });

  /// Eine leere Menge heisst „alle", nicht „keine". Sonst wäre die Liste beim
  /// Öffnen leer, weil noch nichts angeklickt wurde.
  it("zeigt bei leerer Statusmenge alles", () => {
    expect(buildRows(liste, optionen({ states: new Set() }))).toHaveLength(2);
  });

  it("filtert nach Statusklassen", () => {
    const rows = buildRows(liste, optionen({ states: new Set<StatusKey>(["crit"]) }));
    expect(rows).toHaveLength(1);
    expect(rows[0]!.problem.state).toBe("crit");
  });

  it("filtert Freitext über Host und Service", () => {
    expect(buildRows(liste, optionen({ query: "sql" }))).toHaveLength(1);
    expect(buildRows(liste, optionen({ query: "Disk" }))).toHaveLength(1);
    expect(buildRows(liste, optionen({ query: "gibtsnicht" }))).toHaveLength(0);
  });

  it("ignoriert Gross- und Kleinschreibung sowie Randleerzeichen", () => {
    expect(buildRows(liste, optionen({ query: "  DISK  " }))).toHaveLength(1);
    expect(buildRows(liste, optionen({ query: "SQL-01" }))).toHaveLength(1);
  });

  it("kombiniert Text- und Statusfilter", () => {
    const rows = buildRows(
      liste,
      optionen({ query: "sql", states: new Set<StatusKey>(["warn"]), includeHandled: true }),
    );
    expect(rows).toHaveLength(1);
    expect(rows[0]!.problem.service).toBe("Memory");
  });
});

/* -------------------------------------------------------------------------- */
/* Gruppieren                                                                 */
/* -------------------------------------------------------------------------- */

describe("buildRows – Gruppierung", () => {
  /** Ein ausgefallener Host mit vier betroffenen Services. */
  const ausfall = [
    problem("down", "esxi-03", null),
    problem("crit", "esxi-03", "Multipath"),
    problem("crit", "esxi-03", "Datastore"),
    problem("warn", "esxi-03", "Memory"),
    problem("warn", "esxi-03", "NTP"),
    problem("crit", "sql-01", "Disk"),
  ];

  /// Der Kern des Auftrags: nicht vierzig rote Zeilen für einen Rechner.
  it("fasst die Services eines ausgefallenen Hosts zu einer Zeile zusammen", () => {
    const rows = buildRows(ausfall, optionen());

    expect(rows).toHaveLength(2);
    expect(rows[0]!.problem.host).toBe("esxi-03");
    expect(rows[0]!.problem.service).toBeNull();
    expect(rows[0]!.collapsedCount).toBe(4);
    expect(rows[0]!.expanded).toBe(false);

    // Der unbeteiligte Host bleibt eine eigene Zeile.
    expect(rows[1]!.problem.host).toBe("sql-01");
    expect(rows[1]!.collapsedCount).toBe(0);
  });

  it("klappt eine Gruppe auf und rückt die Services ein", () => {
    const rows = buildRows(
      ausfall,
      optionen({ expandedHosts: new Set(["esxi-03"]) }),
    );

    expect(rows).toHaveLength(6);
    expect(rows[0]!.expanded).toBe(true);
    expect(rows[0]!.nested).toBe(false);

    const services = rows.slice(1, 5);
    expect(services.every((r) => r.nested)).toBe(true);
    expect(services.map((r) => r.problem.service)).toEqual([
      "Multipath",
      "Datastore",
      "Memory",
      "NTP",
    ]);

    // Danach geht es auf oberster Ebene weiter.
    expect(rows[5]!.problem.host).toBe("sql-01");
    expect(rows[5]!.nested).toBe(false);
  });

  it("erhält die Reihenfolge des Abzugs", () => {
    const rows = buildRows(
      ausfall,
      optionen({ expandedHosts: new Set(["esxi-03"]) }),
    );
    // Die Eingabe war nach Schwere sortiert; die Ausgabe darf das nicht
    // durcheinanderbringen.
    expect(rows.map((r) => r.problem.service ?? "<host>")).toEqual([
      "<host>",
      "Multipath",
      "Datastore",
      "Memory",
      "NTP",
      "Disk",
    ]);
  });

  it("gruppiert auch bei unreachable", () => {
    const rows = buildRows(
      [problem("unreachable", "vm-77", null), problem("crit", "vm-77", "Disk")],
      optionen(),
    );
    expect(rows).toHaveLength(1);
    expect(rows[0]!.collapsedCount).toBe(1);
  });

  /// Ein Host-Eintrag mit UNKNOWN ist kein Ausfall und bildet keine Gruppe.
  it("gruppiert nicht bei einem Host mit UNKNOWN", () => {
    const rows = buildRows(
      [problem("unknown", "print-01", null), problem("crit", "print-01", "Agent")],
      optionen(),
    );
    expect(rows).toHaveLength(2);
    expect(rows.every((r) => r.collapsedCount === 0)).toBe(true);
  });

  it("zeigt einen ausgefallenen Host ohne Services als schlichte Zeile", () => {
    const rows = buildRows([problem("down", "einsam", null)], optionen());
    expect(rows).toHaveLength(1);
    expect(rows[0]!.collapsedCount).toBe(0);
  });

  /// Wer sucht, will die Treffer sehen — nicht eine Gruppe, in der der
  /// gesuchte Service versteckt ist.
  it("gruppiert bei einem Textfilter nicht", () => {
    const rows = buildRows(ausfall, optionen({ query: "esxi" }));
    expect(rows).toHaveLength(5);
    expect(rows.every((r) => r.collapsedCount === 0)).toBe(true);
    expect(rows.every((r) => !r.nested)).toBe(true);
  });

  it("findet einen Service, der sonst eingeklappt wäre", () => {
    const rows = buildRows(ausfall, optionen({ query: "Datastore" }));
    expect(rows).toHaveLength(1);
    expect(rows[0]!.problem.service).toBe("Datastore");
  });

  /// Der Statusfilter darf die Gruppierung nicht abschalten.
  it("gruppiert bei einem Statusfilter weiter", () => {
    const rows = buildRows(
      ausfall,
      optionen({ states: new Set<StatusKey>(["down", "crit"]) }),
    );
    expect(rows[0]!.problem.host).toBe("esxi-03");
    // Nur die beiden CRIT-Services zählen jetzt zur Gruppe.
    expect(rows[0]!.collapsedCount).toBe(2);
  });

  /// Fällt der Host-Eintrag selbst aus dem Filter, gibt es keinen Anker mehr —
  /// dann müssen die Services einzeln erscheinen, nicht verschwinden.
  it("zeigt Services einzeln, wenn der Host-Eintrag weggefiltert ist", () => {
    const rows = buildRows(ausfall, optionen({ states: new Set<StatusKey>(["crit"]) }));
    expect(rows.map((r) => r.problem.service)).toEqual([
      "Multipath",
      "Datastore",
      "Disk",
    ]);
    expect(rows.every((r) => !r.nested)).toBe(true);
  });

  it("liefert stabile, eindeutige Schlüssel", () => {
    const rows = buildRows(
      ausfall,
      optionen({ expandedHosts: new Set(["esxi-03"]) }),
    );
    const ids = rows.map((r) => r.id);
    expect(new Set(ids).size).toBe(ids.length);
  });

  it("verkraftet eine leere Liste", () => {
    expect(buildRows([], optionen())).toEqual([]);
  });
});

/* -------------------------------------------------------------------------- */
/* Zähler                                                                     */
/* -------------------------------------------------------------------------- */

describe("countByState", () => {
  const liste = [
    problem("down", "a", null),
    problem("unreachable", "b", null),
    problem("crit", "c", "s"),
    problem("crit", "d", "s", { acknowledged: true }),
    problem("warn", "e", "s"),
    problem("unknown", "f", "s"),
  ];

  it("zählt je Klasse und fasst down mit unreachable zusammen", () => {
    const counts = countByState(liste, false);
    expect(counts.down).toBe(2);
    expect(counts.crit).toBe(1);
    expect(counts.warn).toBe(1);
    expect(counts.unknown).toBe(1);
    expect(counts.ok).toBe(0);
  });

  it("zählt mit eingeblendeten Bearbeiteten mehr", () => {
    expect(countByState(liste, true).crit).toBe(2);
  });

  it("zählt bearbeitete Probleme getrennt", () => {
    expect(countHandled(liste)).toBe(1);
    expect(countHandled([])).toBe(0);
  });
});

/* -------------------------------------------------------------------------- */
/* Grössenordnung                                                             */
/* -------------------------------------------------------------------------- */

describe("buildRows – 80 Probleme", () => {
  /// Der Auftrag nennt 80 gleichzeitige Probleme als Messlatte. Geprüft wird
  /// hier nicht die Geschwindigkeit der Darstellung, sondern dass die
  /// Gruppierung bei dieser Menge das Richtige tut.
  it("bleibt bei achtzig Problemen richtig", () => {
    const viele: Problem[] = [];
    // Vier ausgefallene Hosts mit je 15 Services.
    for (let h = 0; h < 4; h++) {
      viele.push(problem("down", `esxi-0${h}`, null));
      for (let s = 0; s < 15; s++) {
        viele.push(problem("crit", `esxi-0${h}`, `Service ${s}`));
      }
    }
    // Dazu 16 einzelne Probleme.
    for (let i = 0; i < 16; i++) {
      viele.push(problem("warn", `host-${i}`, "Memory"));
    }
    expect(viele).toHaveLength(80);

    const eingeklappt = buildRows(viele, optionen());
    expect(eingeklappt).toHaveLength(20);
    expect(eingeklappt.slice(0, 4).every((r) => r.collapsedCount === 15)).toBe(true);

    const einesOffen = buildRows(
      viele,
      optionen({ expandedHosts: new Set(["esxi-02"]) }),
    );
    expect(einesOffen).toHaveLength(35);
  });
});
