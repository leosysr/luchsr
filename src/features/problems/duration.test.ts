import { describe, expect, it } from "vitest";

import {
  KEINE_DAUER,
  durationSince,
  firstLine,
  formatDuration,
  formatTimestamp,
} from "./duration";

const s = 1000;
const m = 60 * s;
const h = 60 * m;
const d = 24 * h;

describe("formatDuration", () => {
  it("zeigt unter einem Tag HH:MM:SS", () => {
    expect(formatDuration(0)).toBe("00:00:00");
    expect(formatDuration(1 * s)).toBe("00:00:01");
    expect(formatDuration(1 * m + 2 * s)).toBe("00:01:02");
    expect(formatDuration(1 * h + 22 * m + 47 * s)).toBe("01:22:47");
    expect(formatDuration(23 * h + 59 * m + 59 * s)).toBe("23:59:59");
  });

  it("wechselt ab einem Tag auf Nd HH:MM", () => {
    expect(formatDuration(1 * d)).toBe("1d 00:00");
    expect(formatDuration(1 * d + 4 * h + 12 * m)).toBe("1d 04:12");
    expect(formatDuration(7 * d + 23 * m)).toBe("7d 00:23");
    expect(formatDuration(142 * d + 19 * h)).toBe("142d 19:00");
  });

  /// Die Spaltenbreite hängt daran: bei Stunden zweistellig, bei Tagen ohne
  /// Sekunden. Sonst springt das Format mitten in der Liste.
  it("wechselt genau an der Tagesgrenze", () => {
    expect(formatDuration(1 * d - 1)).toBe("23:59:59");
    expect(formatDuration(1 * d)).toBe("1d 00:00");
  });

  it("füllt Stunden, Minuten und Sekunden zweistellig", () => {
    expect(formatDuration(9 * h + 8 * m + 7 * s)).toBe("09:08:07");
    expect(formatDuration(1 * d + 9 * h + 8 * m)).toBe("1d 09:08");
  });

  /// Eine negative Dauer heisst: der Zeitstempel liegt in der Zukunft. Das
  /// kommt bei auseinanderlaufenden Uhren vor und darf nicht als „-00:00:01"
  /// erscheinen.
  it("weist Unsinn ab statt ihn anzuzeigen", () => {
    expect(formatDuration(-1)).toBe(KEINE_DAUER);
    expect(formatDuration(-5 * h)).toBe(KEINE_DAUER);
    expect(formatDuration(Number.NaN)).toBe(KEINE_DAUER);
    expect(formatDuration(Number.POSITIVE_INFINITY)).toBe(KEINE_DAUER);
  });
});

describe("durationSince", () => {
  const jetzt = Date.parse("2026-08-24T12:00:00Z");

  it("rechnet gegen den übergebenen Zeitpunkt", () => {
    expect(durationSince("2026-08-24T10:37:13Z", jetzt)).toBe("01:22:47");
    expect(durationSince("2026-08-17T11:37:00Z", jetzt)).toBe("7d 00:23");
  });

  /// Der wichtigste Fall: CheckMK liefert `0` für „noch nie gewechselt", das
  /// Backend macht daraus `null`. Als `00:00:00` wäre das eine Lüge.
  it("zeigt bei fehlendem Zeitstempel keinen Nullwert", () => {
    expect(durationSince(null, jetzt)).toBe(KEINE_DAUER);
  });

  it("verkraftet einen unlesbaren Zeitstempel", () => {
    expect(durationSince("kein datum", jetzt)).toBe(KEINE_DAUER);
    expect(durationSince("", jetzt)).toBe(KEINE_DAUER);
  });

  it("verkraftet einen Zeitstempel aus der Zukunft", () => {
    expect(durationSince("2026-08-24T13:00:00Z", jetzt)).toBe(KEINE_DAUER);
  });
});

describe("formatTimestamp", () => {
  it("gibt Datum und Uhrzeit aus", () => {
    const text = formatTimestamp("2026-08-24T10:37:13Z");
    // Die Ortszeit hängt von der Zeitzone des Rechners ab; geprüft wird die
    // Form, nicht der Wert.
    expect(text).toMatch(/^\d{2}\.\d{2}\.\d{4}, \d{2}:\d{2}:\d{2}$/);
  });

  it("zeigt bei fehlendem Zeitstempel keinen Nullwert", () => {
    expect(formatTimestamp(null)).toBe(KEINE_DAUER);
    expect(formatTimestamp("unlesbar")).toBe(KEINE_DAUER);
  });
});

describe("firstLine", () => {
  it("nimmt die erste Zeile und trimmt sie", () => {
    expect(firstLine("  CRIT - erste Zeile  \nzweite\ndritte")).toBe(
      "CRIT - erste Zeile",
    );
    expect(firstLine("einzeilig")).toBe("einzeilig");
    expect(firstLine("")).toBe("");
  });

  /// Gekürzt wird per CSS, nicht hier — sonst hängt die Kürzung an einer
  /// geratenen Zeichenzahl statt an der Spaltenbreite.
  it("kürzt nicht auf eine Zeichenzahl", () => {
    const lang = "x".repeat(500);
    expect(firstLine(lang)).toHaveLength(500);
  });
});
