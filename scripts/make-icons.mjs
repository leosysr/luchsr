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
 * Die Kopfzeile des Toasts trägt bewusst KEIN Symbol: ein grünes Markensymbol
 * neben einer roten Zustandsfläche liest sich widersprüchlich, weil Grün in
 * diesem Programm „OK" heisst. Es der Farbe folgen zu lassen ginge nicht — das
 * Symbol kommt aus einer Datei für die ganze Anwendung, und das Info-Center
 * zeichnet alte Toasts daraus neu. Siehe src/notify/toast.rs.
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


/* --------------------------------------------------------------- Banner --- */

/**
 * Kopfbild für das README.
 *
 * Reine Bildmarke ohne Wortmarke: die Schriften des Projekts sind Manrope und
 * IBM Plex Mono, und ein Rasterizer, der keine Schrift zeichnen kann, würde
 * daraus zwangsläufig eine andere Schrift machen. Die Überschrift des README
 * ist der Wortmarke näher als ein selbstgemalter Schriftzug.
 *
 * Der Streifen unten trägt die sechs Zustandsfarben in der Reihenfolge des
 * Tray-Icons. Er ist nicht Dekoration: er sagt, worum es in diesem Programm
 * geht, bevor die erste Zeile gelesen ist.
 *
 * 1280 px breit, weil GitHub das Bild auf die Spaltenbreite skaliert — kleiner
 * würde auf einem hochauflösenden Bildschirm unscharf.
 */
const BANNER_DIR = join(ROOT, "docs", "bilder");
mkdirSync(BANNER_DIR, { recursive: true });

const BANNER = { breite: 1280, hoehe: 264, marke: 168, streifen: 10 };

/** Legt ein quadratisches RGBA-Bild in eine grössere Fläche, alpha-gemischt. */
function einsetzen(ziel, zielBreite, quelle, groesse, ox, oy) {
  for (let y = 0; y < groesse; y++) {
    for (let x = 0; x < groesse; x++) {
      const q = (y * groesse + x) * 4;
      const a = quelle[q + 3] / 255;
      if (a === 0) continue;
      const z = ((oy + y) * zielBreite + (ox + x)) * 4;
      for (let k = 0; k < 3; k++) {
        ziel[z + k] = Math.round(quelle[q + k] * a + ziel[z + k] * (1 - a));
      }
      ziel[z + 3] = 255;
    }
  }
}

{
  const { breite, hoehe, marke, streifen } = BANNER;
  const blatt = Buffer.alloc(breite * hoehe * 4);

  // Grundfläche in Ink.
  for (let i = 0; i < breite * hoehe; i++) {
    blatt[i * 4] = COLORS.inkTief[0];
    blatt[i * 4 + 1] = COLORS.inkTief[1];
    blatt[i * 4 + 2] = COLORS.inkTief[2];
    blatt[i * 4 + 3] = 255;
  }

  // Die Marke in Mint, ohne Kachel — auf Ink braucht sie keine.
  const nutzhoehe = hoehe - streifen;
  einsetzen(
    blatt,
    breite,
    render(marke, { color: COLORS.mintHell }),
    marke,
    Math.round((breite - marke) / 2),
    Math.round((nutzhoehe - marke) / 2),
  );

  // Zustandsstreifen am unteren Rand, gleich breite Abschnitte. Der letzte
  // reicht bis zum Rand, damit kein Pixel Ink stehenbleibt.
  TRAY_STATES.forEach((state, i) => {
    const von = Math.round((breite * i) / TRAY_STATES.length);
    const bis =
      i === TRAY_STATES.length - 1
        ? breite
        : Math.round((breite * (i + 1)) / TRAY_STATES.length);
    for (let y = nutzhoehe; y < hoehe; y++) {
      for (let x = von; x < bis; x++) {
        const z = (y * breite + x) * 4;
        blatt[z] = state.color[0];
        blatt[z + 1] = state.color[1];
        blatt[z + 2] = state.color[2];
        blatt[z + 3] = 255;
      }
    }
  });

  write(
    join(BANNER_DIR, "banner.png"),
    encodePng(breite, hoehe, blatt),
    `${breite}×${hoehe}, Marke auf Ink + Zustandsstreifen`,
  );
}

/* ------------------------------------------------------------- Ausgabe ---- */

const spalte = Math.max(...written.map((w) => w.path.length));
for (const item of written) {
  console.log(
    `${item.path.padEnd(spalte)}  ${String(item.size).padStart(7)} B  ${item.note}`,
  );
}
console.log(`\n${written.length} Dateien, ${written.reduce((n, w) => n + w.size, 0)} B insgesamt`);
