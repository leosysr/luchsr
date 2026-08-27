/**
 * Erzeugt alle ausgelieferten Icon-Dateien aus der Bildmarke.
 *
 * Die Geometrie steht in `mark.mjs` — hier wird nur ausgegeben. Das Skript ist
 * idempotent: gleiche Eingabe, gleiche Bytes.
 *
 * Aufruf:  node scripts/make-icons.mjs
 *
 * ## Was entsteht
 *
 * `src/assets/icons/`      SVG für das Frontend, Farbe über currentColor
 * `src-tauri/icons/`       App-Icon in den Grössen, die tauri.conf.json listet,
 *                          plus icon.ico für die Exe-Ressourcen
 * `src-tauri/icons/tray/`  sechs Zustände in 16 und 32 px
 *
 * ## Warum zwei Grössen je Tray-Zustand
 *
 * Windows fragt im Infobereich 16 px bei 100 % Skalierung und 32 px bei 200 %.
 * Ein hochskaliertes 16er ist bei 200 % matschig, ein herunterskaliertes 32er
 * bei 100 % unscharf. Beide Grössen einzeln zu rendern kostet nichts und ist
 * die einzige Variante, die auf beiden Skalierungen scharf ist — zumal die
 * Augen erst ab 24 px überhaupt gezeichnet werden.
 */

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import {
  COLORS,
  TILE_RADIUS,
  TRAY_STATES,
  encodeIco,
  encodePng,
  markSvg,
  render,
  tileSvg,
} from "./mark.mjs";

const ROOT = join(dirname(fileURLToPath(import.meta.url)), "..");
const ICON_DIR = join(ROOT, "src-tauri", "icons");
const TRAY_DIR = join(ICON_DIR, "tray");
const ASSET_DIR = join(ROOT, "src", "assets", "icons");

mkdirSync(ICON_DIR, { recursive: true });
mkdirSync(TRAY_DIR, { recursive: true });
mkdirSync(ASSET_DIR, { recursive: true });

const written = [];
function write(path, data, note) {
  writeFileSync(path, data);
  written.push({ path: path.replace(ROOT + "\\", "").replace(ROOT + "/", ""), size: data.length, note });
}

/* -------------------------------------------------------------------- SVG -- */

write(join(ASSET_DIR, "luchsr-mark.svg"), Buffer.from(markSvg(), "utf8"), "currentColor");
write(
  join(ASSET_DIR, "luchsr-tile.svg"),
  Buffer.from(tileSvg(), "utf8"),
  "Mint-Kachel, Luchs in Ink",
);

/* --------------------------------------------------------------- App-Icon -- */

/** App-Icon: Luchs in Ink, ausgestanzt aus einer mintfarbenen Kachel. */
const appSpec = {
  color: COLORS.inkTief,
  tile: { color: COLORS.mintHell, radius: TILE_RADIUS },
};

const appCache = new Map();
function appPng(size) {
  if (!appCache.has(size)) appCache.set(size, encodePng(size, size, render(size, appSpec)));
  return appCache.get(size);
}

// Die Namen sind von tauri.conf.json vorgegeben.
for (const [name, size] of [
  ["32x32.png", 32],
  ["128x128.png", 128],
  ["128x128@2x.png", 256],
  ["icon.png", 512],
]) {
  write(join(ICON_DIR, name), appPng(size), `${size} px`);
}

// icon.ico wird von tauri-build in die Exe-Ressourcen eingebettet.
const ICO_SIZES = [16, 24, 32, 48, 64, 128, 256];
write(
  join(ICON_DIR, "icon.ico"),
  encodeIco(ICO_SIZES.map((size) => ({ size, png: appPng(size) }))),
  ICO_SIZES.join(", ") + " px",
);

/* ------------------------------------------------------------ Tray-Icons -- */

const TRAY_SIZES = [16, 32];

for (const state of TRAY_STATES) {
  for (const size of TRAY_SIZES) {
    const png = encodePng(
      size,
      size,
      render(size, {
        color: COLORS.inkTief,
        tile: { color: state.color, radius: TILE_RADIUS },
      }),
    );
    write(join(TRAY_DIR, `${state.key}-${size}.png`), png, state.note);
  }
}

/* ----------------------------------------------------------- Toast-Logos -- */

/**
 * Logos für die Windows-Benachrichtigungen.
 *
 * Windows zeigt `appLogoOverride` mit 48 px bei 100 % Skalierung und skaliert
 * bis 400 % hoch. 192 px ist damit die Grösse, ab der nichts mehr dazukommt.
 * Die Augen sind hier sichtbar — anders als beim 16-px-Tray-Icon, wo sie unter
 * 2 px lägen und matschen würden.
 *
 * `app-64.png` ist ein anderer Fall: es steht in der Registry unter `IconUri`
 * und erscheint klein in der Kopfzeile des Toasts, neben dem Namen der
 * sendenden Anwendung. Deshalb trägt es die Markenfarbe und keinen Zustand —
 * es beantwortet „wer meldet das", nicht „wie schlimm ist es".
 *
 * `disconnected` fehlt bewusst: nach einem Fehlversuch wird nicht gemeldet
 * (D62), es gäbe also nie einen Toast in diesem Zustand. Ein Logo dafür wären
 * eingebaute Bytes, die nie gelesen werden.
 */
const TOAST_DIR = join(ICON_DIR, "toast");
mkdirSync(TOAST_DIR, { recursive: true });

const TOAST_SIZE = 192;

for (const state of TRAY_STATES.filter((s) => s.key !== "disconnected")) {
  write(
    join(TOAST_DIR, `${state.key}-${TOAST_SIZE}.png`),
    encodePng(
      TOAST_SIZE,
      TOAST_SIZE,
      render(TOAST_SIZE, {
        color: COLORS.inkTief,
        tile: { color: state.color, radius: TILE_RADIUS },
      }),
    ),
    state.note,
  );
}

write(join(TOAST_DIR, "app-64.png"), appPng(64), "Kopfzeile des Toasts");

/* ------------------------------------------------------------- Ausgabe ---- */

const spalte = Math.max(...written.map((w) => w.path.length));
for (const item of written) {
  console.log(
    `${item.path.padEnd(spalte)}  ${String(item.size).padStart(7)} B  ${item.note}`,
  );
}
console.log(`\n${written.length} Dateien, ${written.reduce((n, w) => n + w.size, 0)} B insgesamt`);
