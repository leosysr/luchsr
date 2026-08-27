/**
 * Wörterbuchhygiene.
 *
 * Zwei Richtungen, beide sind schon einmal auseinandergelaufen: ein Schlüssel
 * bleibt liegen, nachdem die Komponente gelöscht wurde (toter Text, der beim
 * Übersetzen Arbeit macht), oder ein Aufruf zeigt auf einen Schlüssel, den es
 * nicht gibt. Die zweite Richtung fängt normalerweise der Compiler ab —
 * ausser bei dynamisch gebauten Schlüsseln, und genau dort ist es gefährlich.
 *
 * Geprüft wird gegen den Quelltext, nicht gegen Laufzeitverhalten: der Test
 * liest die Dateien und sucht nach dem Schlüssel als Zeichenkette. Das ist
 * grob, aber für ein flaches Wörterbuch ausreichend und hat keine Blindstelle
 * durch Mocking.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { describe, expect, it } from "vitest";

import { de } from "./de";

/** Wurzel der Frontend-Quellen, relativ zu dieser Datei. */
const SRC = join(import.meta.dirname, "..");

/**
 * Das Wörterbuch hat einen zweiten Konsumenten: das Traymenü wird nativ
 * gebaut, und `src-tauri/src/i18n.rs` prüft seine Konstanten gegen diese
 * Datei. Die Schlüssel stehen also nirgends im Frontend — verwaist sind sie
 * trotzdem nicht.
 */
const RUST_QUELLE = join(SRC, "..", "src-tauri", "src", "i18n.rs");

/**
 * Schlüssel, die nicht wörtlich im Quelltext stehen, weil sie zur Laufzeit
 * zusammengesetzt werden. Jeder Eintrag braucht die Stelle, die ihn baut —
 * sonst wird diese Liste zur Ausrede.
 */
const DYNAMISCH: readonly { muster: RegExp; gebautIn: string }[] = [
  // i18n/index.ts: statusLabel() baut `status.${key}` aus StatusKey.
  { muster: /^status\.(ok|warn|crit|unknown|down|stale)$/, gebautIn: "i18n/index.ts" },
];

function quellDateien(dir: string): string[] {
  const treffer: string[] = [];
  for (const eintrag of readdirSync(dir, { withFileTypes: true })) {
    const pfad = join(dir, eintrag.name);
    if (eintrag.isDirectory()) {
      treffer.push(...quellDateien(pfad));
    } else if (/\.tsx?$/.test(eintrag.name) && !/\.test\.tsx?$/.test(eintrag.name)) {
      treffer.push(pfad);
    }
  }
  return treffer;
}

/** Alle Quellen ausser dem Wörterbuch selbst, als ein Text plus Herkunft. */
function quellen(): { pfad: string; text: string }[] {
  const dateien = quellDateien(SRC)
    .filter((pfad) => !pfad.endsWith(join("i18n", "de.ts")))
    .map((pfad) => ({ pfad: relative(SRC, pfad), text: readFileSync(pfad, "utf8") }));
  dateien.push({
    pfad: relative(SRC, RUST_QUELLE),
    text: readFileSync(RUST_QUELLE, "utf8"),
  });
  return dateien;
}

describe("Wörterbuch de", () => {
  const dateien = quellen();
  const schluessel = Object.keys(de);

  it("hat Einträge und findet die Quellen", () => {
    // Schlägt der Pfad fehl, wären alle folgenden Prüfungen still grün.
    expect(schluessel.length).toBeGreaterThan(50);
    expect(dateien.length).toBeGreaterThan(10);
    expect(dateien.map((d) => d.pfad)).toContain(join("i18n", "index.ts"));
  });

  it("enthält keinen Schlüssel, den niemand benutzt", () => {
    const verwaist = schluessel.filter((key) => {
      if (DYNAMISCH.some((d) => d.muster.test(key))) return false;
      return !dateien.some((d) => d.text.includes(`"${key}"`));
    });
    expect(verwaist).toEqual([]);
  });

  it("bedient jeden dynamisch gebauten Schlüssel", () => {
    // Gegenrichtung zur Ausnahmeliste: ein Muster darf nicht ins Leere zeigen.
    for (const { muster, gebautIn } of DYNAMISCH) {
      const bedient = schluessel.filter((key) => muster.test(key));
      expect(bedient.length, `${muster} (${gebautIn})`).toBeGreaterThan(0);
      const quelle = dateien.find((d) => d.pfad === join(...gebautIn.split("/")));
      expect(quelle, `${gebautIn} existiert nicht mehr`).toBeDefined();
    }
  });

  it("hat keine leeren Werte und keine offensichtlichen Platzhalter", () => {
    for (const [key, wert] of Object.entries(de)) {
      expect(typeof wert, key).toBe("string");
      expect(wert.trim().length, key).toBeGreaterThan(0);
      expect(wert, key).not.toMatch(/\bTODO\b|\bTBD\b|XXX/);
    }
  });

  it("benutzt typografische Zeichen statt ASCII-Ersatz", () => {
    // Der Export setzt echte Auslassungspunkte und Gedankenstriche. Drei
    // Punkte hintereinander sind im Fliesstext ein Fehler, nicht eine Variante.
    for (const [key, wert] of Object.entries(de)) {
      expect(wert, key).not.toContain("...");
      expect(wert, `${key}: Bindestrich als Gedankenstrich`).not.toMatch(/\s-\s/);
    }
  });
});
