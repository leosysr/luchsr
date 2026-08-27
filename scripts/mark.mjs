/**
 * Die Luchsr-Bildmarke: Geometrie, Rasterung, Kodierung.
 *
 * Geteiltes Modul. Die Geometrie steht hier **einmal**; `make-icons.mjs`
 * erzeugt daraus die ausgelieferten Dateien, `mark-studio.mjs` die
 * Vergleichsblätter. Zwei Kopien derselben Punkte wären der übliche Weg, wie
 * eine Bildmarke stillschweigend auseinanderläuft.
 *
 * Reines Node, keine Abhängigkeit. PNG über zlib, ICO als Vista-Format
 * (PNG-in-ICO), Rasterung über Punkt-in-Polygon mit Supersampling.
 *
 * ## Warum überhaupt ein eigener Rasterizer
 *
 * Weil ein Tray-Icon sich bei 16 px entscheidet und bei 16 px keine
 * Beschreibung hilft, sondern nur der Blick darauf. Der Rasterizer war das
 * Entwurfswerkzeug; dass er jetzt auch die Auslieferung erzeugt, ist ein
 * Nebeneffekt und erspart eine Abhängigkeit auf resvg oder sharp.
 */

import { deflateSync } from "node:zlib";

/* ========================================================================== */
/* Farben                                                                     */
/* ========================================================================== */

/**
 * Aus `src/styles/tokens.css`, hier für das Build-Skript dupliziert.
 *
 * Das ist die eine erlaubte Ausnahme von der Token-Regel: ein Skript, das
 * Binärdateien erzeugt, kann kein CSS lesen. Bei einer Palettenänderung muss
 * diese Tabelle mitgeführt werden — deshalb steht sie hier oben und nicht
 * verstreut im Code.
 */
export const COLORS = {
  inkTief: [0x0a, 0x10, 0x0d], // --st-ink-900
  ink: [0x0e, 0x15, 0x12], // --st-ink-800
  papier: [0xf5, 0xf7, 0xf3], // --st-paper
  mintHell: [0x3d, 0xdc, 0x97], // --st-mint-400
  mintTief: [0x0b, 0x7a, 0x41], // --st-mint-600
};

/**
 * Die sechs Tray-Zustände.
 *
 * Bewusst die Dunkelmodus-Werte aus tokens.css: die sind auf Kontrast gegen
 * Ink gewählt, und die Windows-Taskleiste ist im Regelfall dunkel. Die
 * Farbtöne liegen mindestens 16° auseinander — bei einer 16-px-Farbfläche
 * entscheidet allein der Farbton.
 */
export const TRAY_STATES = [
  { key: "ok", color: [0x3d, 0xdc, 0x97], note: "--state-ok, Mint" },
  { key: "warn", color: [0xff, 0xc2, 0x4d], note: "--state-warn, Amber" },
  { key: "crit", color: [0xff, 0x6a, 0x3d], note: "--state-crit, Zinnober" },
  { key: "down", color: [0xff, 0x2f, 0x5c], note: "--state-down, Karmin" },
  { key: "unknown", color: [0xff, 0x6e, 0xb5], note: "--state-unknown, Pink" },
  { key: "disconnected", color: [0x7e, 0x96, 0x89], note: "--state-stale, Slate" },
];

/* ========================================================================== */
/* Geometrie — im 0..100-Raum, y nach unten                                   */
/* ========================================================================== */

/**
 * Kopfsilhouette des Luchses.
 *
 * Drei Merkmale tragen die Erkennbarkeit, und nur diese drei überleben 16 px:
 *
 *   1. die langen Ohrpinsel — das eigentliche Luchsmerkmal
 *   2. der gezackte Backenbart, der TIEFER hängt als das Kinn
 *   3. das kurze, breite Gesicht
 *
 * Verworfen wurden: ein langes spitzes Kinn (las sich als Fuchs), ein einzelner
 * breiter Backenbart (las sich generisch), und eine Reduktion auf nur die Ohren
 * (las sich als Krone). Siehe Entscheidungslog D22 in CLAUDE.md.
 *
 * Reihenfolge: im Uhrzeigersinn, beginnend am linken Ohrpinsel.
 */
export const HEAD = [
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
  [50, 83], // Kinn
  [40, 78], // Einzug zum Kinn
  [20, 86], // linker Backenbart, unterer Zacken
  [24, 72], // Einzug
  [10, 66], // linker Backenbart, oberer Zacken
  [20, 52], // linke Wange
  [26, 39], // linkes Ohr, Ansatz aussen
];

/**
 * Augen als Aussparung. Schräg gestellt und klein.
 *
 * Grosse runde Augen lasen sich als Eulenmaske. Diese Grösse gibt der Fläche
 * eine Blickrichtung, ohne das Gesicht zu erzählen.
 */
export const EYES = [
  { cx: 38, cy: 50, rx: 6, ry: 3.6, rot: -18 },
  { cx: 62, cy: 50, rx: 6, ry: 3.6, rot: 18 },
];

/** Eckenradius der Kachel — entspricht --radius-avatar (26 %) aus dem Export. */
export const TILE_RADIUS = 26;

/**
 * Ab welcher Grösse die Augen gezeichnet werden.
 *
 * Darunter wären sie unter 2 px und würden nur matschen. Dass ein Icon bei
 * 16 px anders aufgebaut ist als bei 32 px, ist kein Fehler, sondern genau die
 * Anpassung, die Windows-Tray-Icons brauchen.
 */
export const eyesVisible = (size) => size >= 24;

/* ========================================================================== */
/* Rasterung                                                                  */
/* ========================================================================== */

export function insidePolygon(x, y, poly) {
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

export function insideEllipse(x, y, e) {
  const angle = ((e.rot ?? 0) * Math.PI) / 180;
  const px = x - e.cx;
  const py = y - e.cy;
  const rx = px * Math.cos(-angle) - py * Math.sin(-angle);
  const ry = px * Math.sin(-angle) + py * Math.cos(-angle);
  return (rx / e.rx) ** 2 + (ry / e.ry) ** 2 <= 1;
}

export function insideRoundedRect(x, y, size, radius) {
  if (x < 0 || x > size || y < 0 || y > size) return false;
  const cx = Math.min(Math.max(x, radius), size - radius);
  const cy = Math.min(Math.max(y, radius), size - radius);
  return Math.hypot(x - cx, y - cy) <= radius;
}

/** Ob ein Punkt im Luchs liegt. Augen sind Aussparungen. */
export function insideMark(x, y, size) {
  if (!insidePolygon(x, y, HEAD)) return false;
  if (eyesVisible(size) && EYES.some((e) => insideEllipse(x, y, e))) return false;
  return true;
}

/**
 * Rendert eine Marke in einen RGBA-Puffer.
 *
 * `spec.tile` gesetzt  → gefüllte Kachel, Luchs in `spec.color` ausgestanzt
 * `spec.tile` leer     → freistehende Marke in `spec.color`, transparent aussen
 *
 * Supersampling: 4× reicht bis 128 px, darüber wird es sichtbar sparsam.
 */
export function render(size, spec) {
  const pixels = Buffer.alloc(size * size * 4);
  const samples = size <= 32 ? 6 : 4;
  const step = 1 / samples;
  const scale = 100 / size;
  const total = samples * samples;
  const hit = spec.hit ?? insideMark;

  for (let py = 0; py < size; py++) {
    for (let px = 0; px < size; px++) {
      let tile = 0;
      let glyph = 0;

      for (let sy = 0; sy < samples; sy++) {
        for (let sx = 0; sx < samples; sx++) {
          const ux = (px + (sx + 0.5) * step) * scale;
          const uy = (py + (sy + 0.5) * step) * scale;
          if (spec.tile) {
            if (!insideRoundedRect(ux, uy, 100, spec.tile.radius ?? TILE_RADIUS)) continue;
            tile++;
          } else {
            tile++;
          }
          if (hit(ux, uy, size)) glyph++;
        }
      }

      const at = (py * size + px) * 4;
      if (spec.tile) {
        const ratio = tile === 0 ? 0 : glyph / tile;
        for (let c = 0; c < 3; c++) {
          pixels[at + c] = Math.round(
            spec.tile.color[c] * (1 - ratio) + spec.color[c] * ratio,
          );
        }
        pixels[at + 3] = Math.round((tile / total) * 255);
      } else {
        for (let c = 0; c < 3; c++) pixels[at + c] = spec.color[c];
        pixels[at + 3] = Math.round((glyph / total) * 255);
      }
    }
  }
  return pixels;
}

/* ========================================================================== */
/* PNG und ICO                                                                */
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

export function encodePng(width, height, rgba) {
  const ihdr = Buffer.alloc(13);
  ihdr.writeUInt32BE(width, 0);
  ihdr.writeUInt32BE(height, 4);
  ihdr[8] = 8; // Bittiefe
  ihdr[9] = 6; // RGBA
  const raw = Buffer.alloc(height * (width * 4 + 1));
  for (let y = 0; y < height; y++) {
    const at = y * (width * 4 + 1);
    raw[at] = 0; // Filterbyte
    rgba.copy(raw, at + 1, y * width * 4, (y + 1) * width * 4);
  }
  return Buffer.concat([
    Buffer.from([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]),
    chunk("IHDR", ihdr),
    chunk("IDAT", deflateSync(raw, { level: 9 })),
    chunk("IEND", Buffer.alloc(0)),
  ]);
}

/** Vista-ICO: die Einträge sind vollständige PNGs. */
export function encodeIco(entries) {
  const header = Buffer.alloc(6);
  header.writeUInt16LE(1, 2); // Typ 1 = Icon
  header.writeUInt16LE(entries.length, 4);

  const directory = Buffer.alloc(16 * entries.length);
  let offset = header.length + directory.length;

  entries.forEach((entry, index) => {
    const at = index * 16;
    // 0 bedeutet 256 — ein Byte kann 256 nicht darstellen.
    directory[at] = entry.size >= 256 ? 0 : entry.size;
    directory[at + 1] = entry.size >= 256 ? 0 : entry.size;
    directory.writeUInt16LE(1, at + 4); // Farbebenen
    directory.writeUInt16LE(32, at + 6); // Bit pro Pixel
    directory.writeUInt32LE(entry.png.length, at + 8);
    directory.writeUInt32LE(offset, at + 12);
    offset += entry.png.length;
  });

  return Buffer.concat([header, directory, ...entries.map((e) => e.png)]);
}

/* ========================================================================== */
/* SVG                                                                        */
/* ========================================================================== */

const round = (n) => Number(n.toFixed(3));

function polygonPath(poly) {
  return (
    poly.map(([x, y], i) => `${i === 0 ? "M" : "L"}${round(x)} ${round(y)}`).join("") + "Z"
  );
}

/**
 * Gedrehte Ellipse als Pfad, aus zwei Halbbögen.
 *
 * SVG hat kein gedrehtes `<ellipse>`; ein `transform` pro Auge würde das
 * Zusammenfassen zu einem Pfad mit `fill-rule="evenodd"` verhindern — und
 * genau das braucht es, damit die Augen als Aussparung wirken statt als
 * überlagerte Form.
 */
function ellipsePath(e) {
  const angle = ((e.rot ?? 0) * Math.PI) / 180;
  const cos = Math.cos(angle);
  const sin = Math.sin(angle);
  const ax = e.cx + e.rx * cos;
  const ay = e.cy + e.rx * sin;
  const bx = e.cx - e.rx * cos;
  const by = e.cy - e.rx * sin;
  const rot = round(e.rot ?? 0);
  return (
    `M${round(ax)} ${round(ay)}` +
    `A${round(e.rx)} ${round(e.ry)} ${rot} 0 1 ${round(bx)} ${round(by)}` +
    `A${round(e.rx)} ${round(e.ry)} ${rot} 0 1 ${round(ax)} ${round(ay)}Z`
  );
}

/** Kopf plus Augen als ein Pfad. Aussparung über `fill-rule="evenodd"`. */
export function markPath() {
  return polygonPath(HEAD) + EYES.map(ellipsePath).join("");
}

const hex = (rgb) => "#" + rgb.map((c) => c.toString(16).padStart(2, "0")).join("");

/**
 * Freistehende Marke.
 *
 * `fill="currentColor"` — die Farbe kommt aus dem CSS und damit aus den Tokens.
 * Die SVG-Datei enthält bewusst keinen Farbwert.
 */
export function markSvg() {
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" role="img" aria-label="Luchsr">
  <title>Luchsr</title>
  <path fill="currentColor" fill-rule="evenodd" d="${markPath()}"/>
</svg>
`;
}

/** Marke auf gefüllter Kachel. Für Stellen, an denen die Marke Fläche braucht. */
export function tileSvg(tileColor = COLORS.mintHell, markColor = COLORS.inkTief) {
  const r = TILE_RADIUS;
  return `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" role="img" aria-label="Luchsr">
  <title>Luchsr</title>
  <rect width="100" height="100" rx="${r}" ry="${r}" fill="${hex(tileColor)}"/>
  <path fill="${hex(markColor)}" fill-rule="evenodd" d="${markPath()}"/>
</svg>
`;
}
