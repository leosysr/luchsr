/**
 * Entwurfswerkstatt für die Luchsr-Bildmarke.
 *
 * Zweck: Kandidaten als Geometrie beschreiben, rastern und als Kontaktabzug
 * ausgeben — damit die Entwürfe *angesehen* werden können, statt sie zu erraten.
 * Ein Tray-Icon entscheidet sich bei 16 px, und bei 16 px hilft keine
 * Beschreibung, nur der Blick darauf.
 *
 * Reines Node, keine Abhängigkeit: PNG-Kodierung über zlib, Rasterung über
 * Punkt-in-Polygon und Abstand-zu-Polygon mit 4×4-Supersampling.
 *
 * Aufruf:  node scripts/mark-studio.mjs
 * Ausgabe: scratch/mark-sheet.png
 *
 * Diese Datei ist ein Entwurfswerkzeug, kein Produktionscode. Sie verschwindet,
 * sobald die Marke steht — zusammen mit make-icons.mjs.
 */

import { deflateSync } from "node:zlib";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const OUT_DIR = process.argv[2] ?? join(dirname(fileURLToPath(import.meta.url)), "..", "scratch");

/* ========================================================================== */
/* Farben — aus src/styles/tokens.css, hier für das Werkzeug dupliziert.       */
/* ========================================================================== */

const C = {
  mint400: [0x3d, 0xdc, 0x97],
  mint600: [0x0b, 0x7a, 0x41],
  ink900: [0x0a, 0x10, 0x0d],
  ink800: [0x0e, 0x15, 0x12],
  paper: [0xf5, 0xf7, 0xf3],
  white: [0xff, 0xff, 0xff],
  amber400: [0xff, 0xc2, 0x4d],
  red400: [0xff, 0x6a, 0x3d], // CRIT, Zinnober
  red500: [0xff, 0x2f, 0x5c], // DOWN, Karmin
  pink400: [0xff, 0x6e, 0xb5],
  fog500: [0x7e, 0x96, 0x89],
};

/* ========================================================================== */
/* Geometrie der Kandidaten, im 0..100-Raum, y nach unten.                    */
/* ========================================================================== */

/**
 * Kopfsilhouette des Luchses.
 *
 * Die drei Merkmale, an denen ein Luchs erkennbar ist und die bei 16 px noch
 * tragen: die langen Ohrpinsel, der breite Backenbart und der schmale Kinn.
 * Alles andere — Nase, Fellzeichnung, Schnurrhaare — fällt bei dieser Grösse
 * ohnehin weg und macht die Form nur unruhig.
 */
/** Variante 1: zwei Backenbart-Zacken je Seite, tiefer als das Kinn. */
const HEAD_ZACKEN = [
  [18, 3], // linker Ohrpinsel, Spitze
  [40, 31], // linkes Ohr, Innenkante unten
  [50, 35], // Stirnsenke
  [60, 31], // rechtes Ohr, Innenkante unten
  [82, 3], // rechter Ohrpinsel, Spitze
  [74, 39], // rechtes Ohr, Ansatz aussen
  [80, 52], // rechte Wange
  [90, 66], // rechter Backenbart, oberer Zacken
  [76, 72], // Einzug
  [80, 86], // rechter Backenbart, unterer Zacken
  [60, 78], // Einzug zum Kinn
  [50, 83], // Kinn, kurz
  [40, 78], // Einzug zum Kinn
  [20, 86], // linker Backenbart, unterer Zacken
  [24, 72], // Einzug
  [10, 66], // linker Backenbart, oberer Zacken
  [20, 52], // linke Wange
  [26, 39], // linkes Ohr, Ansatz aussen
];

/**
 * Variante 2: ein breiter, flacher Backenbart je Seite, der über die
 * Ohrenspannweite hinausreicht.
 *
 * Das ist die eigentlich luchstypische Proportion — breit und niedrig. Die
 * Unterkante wird damit ruhiger: nur noch ein sanftes W statt vier Zacken.
 */
const HEAD_BREIT = [
  [18, 3], // linker Ohrpinsel, Spitze
  [40, 31], // linkes Ohr, Innenkante unten
  [50, 35], // Stirnsenke
  [60, 31], // rechtes Ohr, Innenkante unten
  [82, 3], // rechter Ohrpinsel, Spitze
  [73, 40], // rechtes Ohr, Ansatz aussen
  [80, 56], // rechte Wange
  [96, 70], // rechter Backenbart — breiter als die Ohren
  [72, 76], // Einzug
  [50, 84], // Kinn
  [28, 76], // Einzug
  [4, 70], // linker Backenbart
  [20, 56], // linke Wange
  [27, 40], // linkes Ohr, Ansatz aussen
];

/** Nur Ohren und Braue — die abstrakteste Reduktion. */
const EARS_ONLY = [
  [16, 5],
  [38, 40],
  [50, 44],
  [62, 40],
  [84, 5],
  [76, 46],
  [78, 58],
  [22, 58],
  [24, 46],
];

/**
 * Augen. Kleiner und schräg gestellt — die runden, grossen Augen des ersten
 * Entwurfs lasen sich bei 128 px wie eine Eulenmaske.
 */
const EYES = [
  { cx: 38, cy: 50, rx: 6, ry: 3.6, rot: -18 },
  { cx: 62, cy: 50, rx: 6, ry: 3.6, rot: 18 },
];

/* ========================================================================== */
/* Rasterung                                                                  */
/* ========================================================================== */

function insidePolygon(x, y, poly) {
  let inside = false;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    const [xi, yi] = poly[i];
    const [xj, yj] = poly[j];
    if (yi > y !== yj > y && x < ((xj - xi) * (y - yi)) / (yj - yi) + xi) {
      inside = !inside;
    }
  }
  return inside;
}

function distanceToSegment(px, py, ax, ay, bx, by) {
  const dx = bx - ax;
  const dy = by - ay;
  const lengthSquared = dx * dx + dy * dy;
  let t = lengthSquared === 0 ? 0 : ((px - ax) * dx + (py - ay) * dy) / lengthSquared;
  t = Math.max(0, Math.min(1, t));
  const cx = ax + t * dx;
  const cy = ay + t * dy;
  return Math.hypot(px - cx, py - cy);
}

function distanceToPolygon(x, y, poly) {
  let best = Infinity;
  for (let i = 0, j = poly.length - 1; i < poly.length; j = i++) {
    best = Math.min(
      best,
      distanceToSegment(x, y, poly[i][0], poly[i][1], poly[j][0], poly[j][1]),
    );
  }
  return best;
}

function insideEllipse(x, y, e) {
  // Punkt in das Koordinatensystem der Ellipse drehen, dann prüfen.
  const angle = ((e.rot ?? 0) * Math.PI) / 180;
  const px = x - e.cx;
  const py = y - e.cy;
  const rx = px * Math.cos(-angle) - py * Math.sin(-angle);
  const ry = px * Math.sin(-angle) + py * Math.cos(-angle);
  const dx = rx / e.rx;
  const dy = ry / e.ry;
  return dx * dx + dy * dy <= 1;
}

function insideRoundedRect(x, y, size, radius) {
  const r = radius;
  const inX = x >= 0 && x <= size;
  const inY = y >= 0 && y <= size;
  if (!inX || !inY) return false;
  const cx = Math.min(Math.max(x, r), size - r);
  const cy = Math.min(Math.max(y, r), size - r);
  return Math.hypot(x - cx, y - cy) <= r;
}

/**
 * Rendert eine Marke in einen RGBA-Puffer.
 *
 * `spec` beschreibt, was gezeichnet wird. Supersampling 4×4 gegen Treppen.
 */
function render(size, spec) {
  const pixels = Buffer.alloc(size * size * 4);
  const SS = 4;
  const step = 1 / SS;
  const scale = 100 / size; // Bildraum -> Entwurfsraum

  for (let py = 0; py < size; py++) {
    for (let px = 0; px < size; px++) {
      let tile = 0;
      let glyph = 0;
      const total = SS * SS;

      for (let sy = 0; sy < SS; sy++) {
        for (let sx = 0; sx < SS; sx++) {
          const ux = (px + (sx + 0.5) * step) * scale;
          const uy = (py + (sy + 0.5) * step) * scale;

          if (spec.tile) {
            if (!insideRoundedRect(ux, uy, 100, spec.tile.radius)) continue;
            tile++;
          } else {
            tile++;
          }
          if (spec.hit(ux, uy, size)) glyph++;
        }
      }

      const offset = (py * size + px) * 4;
      const tileAlpha = tile / total;
      const glyphRatio = tile === 0 ? 0 : glyph / tile;

      const base = spec.tile ? spec.tile.color : null;
      const fg = spec.color;

      if (base) {
        // Kachel mit ausgestanzter Marke.
        for (let c = 0; c < 3; c++) {
          pixels[offset + c] = Math.round(base[c] * (1 - glyphRatio) + fg[c] * glyphRatio);
        }
        pixels[offset + 3] = Math.round(tileAlpha * 255);
      } else {
        // Freistehende Marke.
        for (let c = 0; c < 3; c++) pixels[offset + c] = fg[c];
        pixels[offset + 3] = Math.round((glyph / total) * 255);
      }
    }
  }
  return pixels;
}

/* ========================================================================== */
/* Kandidaten                                                                 */
/* ========================================================================== */

/**
 * Ob die Augen bei dieser Grösse gezeichnet werden.
 *
 * Unter 24 px wären sie unter 2 px gross und würden nur matschen. Ein Icon,
 * das bei 16 px anders aufgebaut ist als bei 32 px, ist kein Fehler sondern
 * genau die Anpassung, die Windows-Tray-Icons brauchen.
 */
const eyesVisible = (size) => size >= 24;

const head = (poly) => (x, y, size) => {
  if (!insidePolygon(x, y, poly)) return false;
  // Augen als Aussparung, erst ab 24 px. Sie haben sich als Verbesserung
  // erwiesen: ohne sie ist die Fläche richtungslos.
  if (eyesVisible(size) && EYES.some((e) => insideEllipse(x, y, e))) return false;
  return true;
};

function candidates(color) {
  return [
    { name: "Zacken — freistehend", color, hit: head(HEAD_ZACKEN) },
    {
      name: "Zacken — Kachel",
      color: C.ink900,
      tile: { color, radius: 26 },
      hit: head(HEAD_ZACKEN),
    },
    { name: "Breit — freistehend", color, hit: head(HEAD_BREIT) },
    {
      name: "Breit — Kachel",
      color: C.ink900,
      tile: { color, radius: 26 },
      hit: head(HEAD_BREIT),
    },
  ];
}

/* ========================================================================== */
/* PNG                                                                        */
/* ========================================================================== */

const CRC_TABLE = (() => {
  const table = new Int32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1;
    table[n] = c;
  }
  return table;
})();

function crc32(buf) {
  let c = 0xffffffff;
  for (const byte of buf) c = CRC_TABLE[(c ^ byte) & 0xff] ^ (c >>> 8);
  return (c ^ 0xffffffff) >>> 0;
}

function chunk(type, data) {
  const length = Buffer.alloc(4);
  length.writeUInt32BE(data.length, 0);
  const body = Buffer.concat([Buffer.from(type, "ascii"), data]);
  const crc = Buffer.alloc(4);
  crc.writeUInt32BE(crc32(body), 0);
  return Buffer.concat([length, body, crc]);
}

function encodePng(width, height, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8;
  ihdr[9] = 6;
  const raw = Buffer.alloc(height * (width * 4 + 1));
  for (let y = 0; y < height; y++) {
    const at = y * (width * 4 + 1);
    raw[at] = 0;
    rgba.copy(raw, at + 1, y * width * 4, (y + 1) * width * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/* ========================================================================== */
/* Kontaktabzug                                                               */
/* ========================================================================== */

/** Zeichnet einen RGBA-Block mit Alpha über den Hintergrund der Leinwand. */
function blit(canvas, canvasWidth, sprite, spriteSize, atX, atY) {
  for (let y = 0; y < spriteSize; y++) {
    for (let x = 0; x < spriteSize; x++) {
      const src = (y * spriteSize + x) * 4;
      const alpha = sprite[src + 3] / 255;
      if (alpha === 0) continue;
      const dst = ((atY + y) * canvasWidth + (atX + x)) * 4;
      for (let c = 0; c < 3; c++) {
        canvas[dst + c] = Math.round(canvas[dst + c] * (1 - alpha) + sprite[src + c] * alpha);
      }
      canvas[dst + 3] = 255;
    }
  }
}

function fill(canvas, width, color, fromY, toY) {
  for (let y = fromY; y < toY; y++) {
    for (let x = 0; x < width; x++) {
      const at = (y * width + x) * 4;
      canvas[at] = color[0];
      canvas[at + 1] = color[1];
      canvas[at + 2] = color[2];
      canvas[at + 3] = 255;
    }
  }
}

const SIZES = [16, 20, 24, 32, 48, 128];
const CELL = 150;

function contactSheet() {
  const cols = SIZES.length;
  const list = candidates(C.mint400);
  const rows = list.length * 2; // hell und dunkel
  const width = cols * CELL;
  const height = rows * CELL;
  const canvas = Buffer.alloc(width * height * 4);

  // Obere Hälfte helle Taskleiste, untere Hälfte dunkle.
  fill(canvas, width, C.paper, 0, (height / 2) | 0);
  fill(canvas, width, C.ink800, (height / 2) | 0, height);

  list.forEach((spec, rowIndex) => {
    SIZES.forEach((size, colIndex) => {
      // Auf hellem Grund braucht die freistehende Marke die dunkle Variante,
      // sonst verschwindet Mint auf Papier.
      const hellSpec = spec.tile
        ? spec
        : { ...spec, color: C.mint600 };
      const dunkelSpec = spec.tile ? spec : { ...spec, color: C.mint400 };

      const hell = render(size, hellSpec);
      const dunkel = render(size, dunkelSpec);

      const x = colIndex * CELL + ((CELL - size) >> 1);
      blit(canvas, width, hell, size, x, rowIndex * CELL + ((CELL - size) >> 1));
      blit(
        canvas,
        width,
        dunkel,
        size,
        x,
        (list.length + rowIndex) * CELL + ((CELL - size) >> 1),
      );
    });
  });

  return { png: encodePng(width, height, canvas), width, height, list };
}

/* ========================================================================== */

/* ========================================================================== */
/* Zustandsabzug — die sechs Tray-Zustände                                    */
/* ========================================================================== */

/**
 * Die sechs Zustände in den hellen Tokenwerten.
 *
 * Bewusst die Dunkelmodus-Varianten aus tokens.css: die sind auf Kontrast
 * gegen Ink gewählt, und die Windows-Taskleiste ist im Regelfall dunkel.
 */
const STATES = [
  { key: "ok", color: C.mint400 },
  { key: "warn", color: C.amber400 },
  { key: "crit", color: C.red400 },
  { key: "down", color: C.red500 },
  { key: "unknown", color: C.pink400 },
  { key: "getrennt", color: C.fog500 },
];

const TRAY_SIZES = [16, 20, 24, 32, 64];

function stateSheet() {
  const cols = TRAY_SIZES.length;
  // Zwei Behandlungen je Zustand: gefüllte Kachel und invertiert.
  const rows = STATES.length * 2;
  const cell = 90;
  const width = cols * cell;
  const height = rows * cell;
  const canvas = Buffer.alloc(width * height * 4);
  fill(canvas, width, C.ink800, 0, height);

  STATES.forEach((state, stateIndex) => {
    TRAY_SIZES.forEach((size, colIndex) => {
      // Behandlung 1: Kachel in der Zustandsfarbe, Luchs ausgestanzt.
      const gefuellt = {
        color: C.ink900,
        tile: { color: state.color, radius: 26 },
        hit: head(HEAD_ZACKEN),
      };
      // Behandlung 2: dunkle Kachel, Luchs in der Zustandsfarbe.
      const invertiert = {
        color: state.color,
        tile: { color: C.ink900, radius: 26 },
        hit: head(HEAD_ZACKEN),
      };

      const x = colIndex * cell + ((cell - size) >> 1);
      blit(
        canvas,
        width,
        render(size, gefuellt),
        size,
        x,
        stateIndex * cell + ((cell - size) >> 1),
      );
      blit(
        canvas,
        width,
        render(size, invertiert),
        size,
        x,
        (STATES.length + stateIndex) * cell + ((cell - size) >> 1),
      );
    });
  });

  return { png: encodePng(width, height, canvas), width, height };
}

/* ========================================================================== */

mkdirSync(OUT_DIR, { recursive: true });

const sheet = contactSheet();
writeFileSync(join(OUT_DIR, "mark-sheet.png"), sheet.png);
console.log(`Kontaktabzug: ${sheet.width}x${sheet.height}`);
console.log(`Spalten (Groessen): ${SIZES.join(", ")} px`);
console.log("Zeilen 1-4 auf Papier, Zeilen 5-8 auf Ink:");
sheet.list.forEach((c, i) => console.log(`  Zeile ${i + 1} / ${i + 5}: ${c.name}`));

/**
 * CRIT und DOWN sind bei 16 px als zwei Rottöne nicht zu trennen. Drei
 * Auswege, direkt neben CRIT gestellt:
 *
 *   1 wie bisher — runde Kachel, dunkleres Rot
 *   2 eckige Kachel — Unterschied über die FORM, trägt auch bei 16 px
 *   3 invertiert — dunkle Kachel, roter Luchs
 */
function downOptions() {
  const sizes = [16, 20, 24, 32, 64];
  const variants = [
    {
      name: "CRIT (Referenz)",
      color: C.ink900,
      tile: { color: C.red400, radius: 26 },
      hit: head(HEAD_ZACKEN),
    },
    {
      name: "DOWN 1 — runde Kachel",
      color: C.ink900,
      tile: { color: C.red500, radius: 26 },
      hit: head(HEAD_ZACKEN),
    },
    {
      name: "DOWN 2 — eckige Kachel",
      color: C.ink900,
      tile: { color: C.red500, radius: 4 },
      hit: head(HEAD_ZACKEN),
    },
    {
      name: "DOWN 3 — invertiert",
      color: C.red500,
      tile: { color: C.ink900, radius: 26 },
      hit: head(HEAD_ZACKEN),
    },
  ];

  const cell = 90;
  const width = sizes.length * cell;
  const height = variants.length * cell;
  const canvas = Buffer.alloc(width * height * 4);
  fill(canvas, width, C.ink800, 0, height);

  variants.forEach((spec, row) => {
    sizes.forEach((size, col) => {
      blit(
        canvas,
        width,
        render(size, spec),
        size,
        col * cell + ((cell - size) >> 1),
        row * cell + ((cell - size) >> 1),
      );
    });
  });

  return { png: encodePng(width, height, canvas), variants, sizes };
}

const states = stateSheet();
writeFileSync(join(OUT_DIR, "state-sheet.png"), states.png);
console.log(`\nZustandsabzug: ${states.width}x${states.height}, auf dunkler Taskleiste`);
console.log(`Spalten (Groessen): ${TRAY_SIZES.join(", ")} px`);
console.log("Zeilen 1-6  gefuellte Kachel, Luchs ausgestanzt:");
STATES.forEach((s, i) => console.log(`  Zeile ${i + 1}: ${s.key}`));
console.log("Zeilen 7-12 dunkle Kachel, Luchs in der Zustandsfarbe:");
STATES.forEach((s, i) => console.log(`  Zeile ${i + 7}: ${s.key}`));

const down = downOptions();
writeFileSync(join(OUT_DIR, "down-options.png"), down.png);
console.log(`\nDOWN-Varianten: Spalten ${down.sizes.join(", ")} px`);
down.variants.forEach((v, i) => console.log(`  Zeile ${i + 1}: ${v.name}`));

/* ========================================================================== */
/* Vorschlag — das fertige System auf einem Blatt                             */
/* ========================================================================== */

/**
 * Der Vorschlag, der zur Abnahme geht.
 *
 * Silhouette: HEAD_ZACKEN. Tray: gefüllte Kachel in der Zustandsfarbe, Luchs
 * ausgestanzt. DOWN bekommt eine ECKIGE Kachel — der Formunterschied trägt bei
 * 16 px, wo der Unterschied zwischen zwei Rottönen nicht mehr trägt.
 */
const PROPOSAL = [
  { key: "OK", color: C.mint400, radius: 26 },
  { key: "WARN", color: C.amber400, radius: 26 },
  { key: "CRIT", color: C.red400, radius: 26 },
  { key: "DOWN", color: C.red500, radius: 26 },
  { key: "UNKNOWN", color: C.pink400, radius: 26 },
  { key: "GETRENNT", color: C.fog500, radius: 26 },
];

function proposalSheet() {
  const traySizes = [16, 20, 24, 32];
  const cell = 100;
  const cols = PROPOSAL.length;
  const bigRow = 200;
  const width = cols * cell;
  const height = traySizes.length * cell + bigRow;
  const canvas = Buffer.alloc(width * height * 4);
  fill(canvas, width, C.ink800, 0, traySizes.length * cell);
  fill(canvas, width, C.paper, traySizes.length * cell, height);

  // Tray-Zustände, eine Zeile je Grösse.
  traySizes.forEach((size, row) => {
    PROPOSAL.forEach((state, col) => {
      const spec = {
        color: C.ink900,
        tile: { color: state.color, radius: state.radius },
        hit: head(HEAD_ZACKEN),
      };
      blit(
        canvas,
        width,
        render(size, spec),
        size,
        col * cell + ((cell - size) >> 1),
        row * cell + ((cell - size) >> 1),
      );
    });
  });

  // Unten auf Papier: App-Icon und freistehende Marke, gross.
  const baseY = traySizes.length * cell + 20;
  const big = 160;
  blit(
    canvas,
    width,
    render(big, {
      color: C.ink900,
      tile: { color: C.mint400, radius: 26 },
      hit: head(HEAD_ZACKEN),
    }),
    big,
    40,
    baseY,
  );
  blit(
    canvas,
    width,
    render(big, { color: C.mint600, hit: head(HEAD_ZACKEN) }),
    big,
    240,
    baseY,
  );
  blit(
    canvas,
    width,
    render(big, {
      color: C.mint400,
      tile: { color: C.ink900, radius: 26 },
      hit: head(HEAD_ZACKEN),
    }),
    big,
    420,
    baseY,
  );

  return { png: encodePng(width, height, canvas), traySizes };
}

const proposal = proposalSheet();
writeFileSync(join(OUT_DIR, "vorschlag.png"), proposal.png);
console.log(`\nVorschlag: Tray-Zeilen ${proposal.traySizes.join(", ")} px auf Ink,`);
console.log(`  Spalten: ${PROPOSAL.map((p) => p.key).join(", ")}`);
console.log("  unten auf Papier: App-Icon (Mint-Kachel), freistehend, Ink-Kachel");

/* ========================================================================== */
/* Farbpaare für CRIT und DOWN                                                */
/* ========================================================================== */

/**
 * CRIT und DOWN sollen sich über den Farbton trennen, nicht über die Form.
 *
 * Der Spielraum ist eng: WARN liegt bei Farbton ~40° (Amber), UNKNOWN bei ~330°
 * (Magenta). Dazwischen bleibt für zwei unterscheidbare Rottöne nur der Bereich
 * von etwa 350° bis 20°. Beide müssen ausserdem hell genug für eine dunkle
 * Taskleiste sein — Helligkeit als Unterscheidung fällt damit weg.
 *
 * Jede Zeile zeigt WARN, CRIT, DOWN, UNKNOWN nebeneinander: die Paare müssen
 * sich nicht nur voneinander trennen, sondern auch von ihren Nachbarn.
 */
const PAIRS = [
  { name: "1 — Zinnober / Reinrot", crit: [0xff, 0x6a, 0x3d], down: [0xff, 0x44, 0x38] },
  { name: "2 — Orangerot / Kirsch", crit: [0xff, 0x7a, 0x45], down: [0xf5, 0x20, 0x3f] },
  { name: "3 — Zinnober / Karmin", crit: [0xff, 0x5c, 0x47], down: [0xe0, 0x10, 0x40] },
  { name: "4 — Reinrot / Weinrot", crit: [0xff, 0x44, 0x38], down: [0xb8, 0x1e, 0x3c] },
];

function pairSheet() {
  const sizes = [16, 32];
  const cell = 70;
  const cols = 4 * sizes.length;
  const width = cols * cell;
  const height = PAIRS.length * cell;
  const canvas = Buffer.alloc(width * height * 4);
  fill(canvas, width, C.ink800, 0, height);

  PAIRS.forEach((pair, row) => {
    const reihe = [C.amber400, pair.crit, pair.down, C.pink400];
    sizes.forEach((size, sizeIndex) => {
      reihe.forEach((color, i) => {
        const spec = {
          color: C.ink900,
          tile: { color, radius: 26 },
          hit: head(HEAD_ZACKEN),
        };
        const col = sizeIndex * 4 + i;
        blit(
          canvas,
          width,
          render(size, spec),
          size,
          col * cell + ((cell - size) >> 1),
          row * cell + ((cell - size) >> 1),
        );
      });
    });
  });

  return { png: encodePng(width, height, canvas) };
}

const pairs = pairSheet();
writeFileSync(join(OUT_DIR, "farbpaare.png"), pairs.png);
console.log("\nFarbpaare: je Zeile WARN, CRIT, DOWN, UNKNOWN — links 16 px, rechts 32 px");
PAIRS.forEach((p, i) => console.log(`  Zeile ${i + 1}: ${p.name}`));
