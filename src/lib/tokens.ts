/**
 * Zugriff auf Token-Werte aus JavaScript.
 *
 * Manches braucht den Wert als **Zahl**, nicht als Klasse: die virtualisierte
 * Liste muss wissen, wie hoch eine Zeile ist, um zu rechnen, wie viele davon
 * sichtbar sind. Eine `32` im Code wäre genau der Bruch, den die Token-Regel
 * verhindern soll — bei einer Änderung von `--row-height` liefe die
 * Virtualisierung aus dem Takt, ohne dass es auffällt.
 *
 * Deshalb wird der Wert zur Laufzeit aus dem berechneten Stil gelesen. Die
 * einzige Wahrheit bleibt `tokens.css`.
 */

/**
 * Liest ein Token als Pixelzahl.
 *
 * `fallback` greift nur, wenn das Token fehlt oder keine px-Angabe ist — etwa
 * beim ersten Rendern vor dem Anwenden des Stylesheets. Ein falscher Wert wäre
 * dort unschön, ein Absturz wäre schlimmer.
 */
export function tokenPx(name: string, fallback: number): number {
  if (typeof window === "undefined") return fallback;
  const raw = window
    .getComputedStyle(document.documentElement)
    .getPropertyValue(name)
    .trim();
  if (!raw.endsWith("px")) return fallback;
  const value = Number.parseFloat(raw);
  return Number.isFinite(value) && value > 0 ? value : fallback;
}

/** Höhe einer Tabellenzeile, aus `--row-height`. */
export function rowHeight(): number {
  return tokenPx("--row-height", 32);
}
