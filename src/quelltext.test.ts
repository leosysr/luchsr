/**
 * Quelltexthygiene: keine Datei, die Werkzeuge für binär halten.
 *
 * Aufgekommen bei einem Audit. In `SoundPicker.tsx` stand ein Sonderwert mit
 * vorangestelltem **NUL-Byte** — als Trick, damit er mit keiner Klangkennung
 * kollidieren kann. Das Argument stimmte; die Folge war, dass Git und `grep`
 * die Datei als Binärdatei behandelten. Im Diff stand `Bin 3412 -> 4218 bytes`
 * statt der geänderten Zeilen, und `grep` meldete nur „Binary file matches".
 *
 * Eine Quelldatei, die man nicht diffen kann, ist eine Datei, in der eine
 * Änderung unbemerkt bleibt. Das ist teurer als der Trick wert war — und es ist
 * mechanisch prüfbar, also wird es geprüft.
 *
 * Geprüft wird gleich mit, dass jede Datei gültiges UTF-8 ist: eine kaputte
 * Sequenz macht aus „Wärmefühler" ein „WÃ¤rmefÃ¼hler", und das fällt sonst
 * erst auf, wenn es jemand liest.
 */

import { readFileSync, readdirSync } from "node:fs";
import { join, relative } from "node:path";

import { describe, expect, it } from "vitest";

/** Projektwurzel, relativ zu dieser Datei. */
const WURZEL = join(import.meta.dirname, "..");

/** Verzeichnisse, die nicht uns gehören oder erzeugt sind. */
const AUSGENOMMEN = new Set([
  "node_modules",
  "target",
  ".git",
  "dist",
  // Der Design-Export ist Referenz und wird nie verändert (D52).
  "handover-design",
]);

/** Endungen, die Text enthalten müssen. */
const TEXT = /\.(ts|tsx|rs|mjs|js|json|jsonc|css|html|md|ps1|yml|yaml|toml)$/;

function textdateien(verzeichnis: string, gesammelt: string[] = []): string[] {
  for (const eintrag of readdirSync(verzeichnis, { withFileTypes: true })) {
    if (AUSGENOMMEN.has(eintrag.name)) continue;
    const pfad = join(verzeichnis, eintrag.name);
    if (eintrag.isDirectory()) textdateien(pfad, gesammelt);
    else if (TEXT.test(eintrag.name)) gesammelt.push(pfad);
  }
  return gesammelt;
}

const DATEIEN = textdateien(WURZEL);

describe("Quelltexthygiene", () => {
  it("findet überhaupt Dateien", () => {
    // Ohne diese Zusicherung würde ein Fehler in der Sammelfunktion die
    // beiden Prüfungen unten stillschweigend zu Nichtprüfungen machen.
    expect(DATEIEN.length).toBeGreaterThan(50);
  });

  it("keine Textdatei enthält ein NUL-Byte", () => {
    const treffer = DATEIEN.filter((pfad) => readFileSync(pfad).includes(0)).map((pfad) =>
      relative(WURZEL, pfad),
    );
    expect(treffer, "NUL-Byte macht die Datei für Git und grep binär").toEqual([]);
  });

  it("jede Textdatei ist gültiges UTF-8", () => {
    // Node ersetzt ungültige Sequenzen durch U+FFFD. Steht das Zeichen in der
    // dekodierten Fassung, war die Datei nicht sauber kodiert.
    //
    // Aus dem Codepunkt gebaut und nicht hingeschrieben: sonst beanstandet der
    // Test seine eigene Quelle. Beim ersten Lauf ist genau das passiert — und
    // damit ist gleich belegt, dass er wirklich sucht und nicht nur besteht.
    const ERSATZZEICHEN = String.fromCodePoint(0xfffd);

    const treffer = DATEIEN.filter((pfad) =>
      readFileSync(pfad).toString("utf8").includes(ERSATZZEICHEN),
    ).map((pfad) => relative(WURZEL, pfad));
    expect(treffer, "kaputte UTF-8-Sequenz").toEqual([]);
  });
});
