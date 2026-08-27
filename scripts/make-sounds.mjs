/**
 * DIE HINWEISTÖNE: Entwürfe, Synthese, WAV-Ausgabe.
 *
 * Erzeugt die eingebauten Klänge nach `src-tauri/sounds/`. Nach einer Änderung
 * an den Entwürfen unten:
 *
 *     node scripts/make-sounds.mjs
 *
 * # Warum erzeugt und nicht besorgt
 *
 * Dieselbe Begründung wie bei der Bildmarke: eine Datei, die man von irgendwo
 * herunterlädt, hat eine Herkunft und eine Lizenz, die man mitführen muss. Ein
 * Skript, das sie erzeugt, hat beides nicht — der Klang ist dann so eigen wie
 * der Quelltext, der ihn beschreibt. Für zwei bis vier Sinustöne ist das keine
 * Einschränkung.
 *
 * # Warum so kurz
 *
 * Die Klänge in `C:\Windows\Media` sind Melodien von einer halben bis ganzen
 * Sekunde. Wer zwanzig Meldungen am Tag bekommt, hört sie zwanzigmal — und
 * fängt an, sie abzuschalten. Ein Hinweis darf höchstens so lang sein, dass er
 * vorbei ist, bevor man ihn bewusst wahrnimmt: zwei bis vier Töne, unter
 * 350 ms. Das ist die ganze Absicht dieser Datei.
 *
 * # Warum Sinus
 *
 * Ein Rechteck- oder Sägezahnton hat Obertöne und klingt dadurch schrill —
 * über kleine Laptoplautsprecher besonders. Sinus mit weicher Hüllkurve klingt
 * bei gleicher Lautstärke deutlich freundlicher und trägt trotzdem.
 */

import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HIER = dirname(fileURLToPath(import.meta.url));
const ZIEL = join(HIER, "..", "src-tauri", "sounds");

/* -------------------------------------------------------------------------- */
/* Format                                                                     */
/* -------------------------------------------------------------------------- */

/**
 * 22050 Hz reicht: der höchste verwendete Ton liegt bei 1568 Hz, die
 * Nyquist-Grenze bei 11025 Hz. 44100 würde die Dateien verdoppeln, ohne dass
 * man einen Unterschied hört.
 */
const RATE = 22050;

/** 16 Bit PCM, mono. Das Einzige, was `PlaySoundW` zuverlässig frisst. */
const BITS = 16;
const KANAELE = 1;

/**
 * Aussteuerung. Bewusst nicht bis an die Grenze: ein Hinweiston soll unter der
 * Lautstärke des Systemklangs bleiben, damit er nicht erschreckt.
 */
const PEGEL = 0.34;

/* -------------------------------------------------------------------------- */
/* Synthese                                                                   */
/* -------------------------------------------------------------------------- */

/**
 * Hüllkurve eines einzelnen Tons.
 *
 * Ohne sie beginnt und endet der Ton an einer Sprungstelle, und das hört man
 * als Knacken — bei kurzen Tönen lauter als den Ton selbst. Der Anstieg ist
 * kurz (der Ton soll pünktlich sein), der Abfall länger (das klingt nach
 * Anschlag statt nach Abschneiden).
 */
function huellkurve(i, gesamt) {
  const anstieg = Math.min(0.006 * RATE, gesamt * 0.2);
  const abfall = Math.min(0.05 * RATE, gesamt * 0.6);
  if (i < anstieg) return i / anstieg;
  const restlich = gesamt - i;
  if (restlich < abfall) return restlich / abfall;
  return 1;
}

/** Ein Ton als Array von Werten in [-1, 1]. */
function ton(hz, ms, pegel = 1) {
  const n = Math.round((ms / 1000) * RATE);
  const werte = new Float64Array(n);
  for (let i = 0; i < n; i += 1) {
    const phase = (2 * Math.PI * hz * i) / RATE;
    werte[i] = Math.sin(phase) * huellkurve(i, n) * pegel;
  }
  return werte;
}

/** Stille, zum Trennen der Töne. */
function pause(ms) {
  return new Float64Array(Math.round((ms / 1000) * RATE));
}

/** Hängt Abschnitte aneinander. */
function folge(...teile) {
  const n = teile.reduce((s, t) => s + t.length, 0);
  const aus = new Float64Array(n);
  let k = 0;
  for (const teil of teile) {
    aus.set(teil, k);
    k += teil.length;
  }
  return aus;
}

/* -------------------------------------------------------------------------- */
/* WAV                                                                        */
/* -------------------------------------------------------------------------- */

/**
 * Schreibt einen RIFF/WAVE-Kopf und die Werte als 16-Bit-PCM.
 *
 * Handgeschrieben, weil das Format an dieser Stelle 44 Byte Kopf und dann
 * Zahlen ist — eine Bibliothek dafür wäre mehr Abhängigkeit als Ersparnis.
 */
function wav(werte) {
  const bytesJeWert = BITS / 8;
  const daten = Buffer.alloc(werte.length * bytesJeWert);
  for (let i = 0; i < werte.length; i += 1) {
    // Begrenzen, nicht überlaufen lassen: ein Überlauf klingt wie ein Knacken
    // an genau der lautesten Stelle.
    const v = Math.max(-1, Math.min(1, werte[i] * PEGEL));
    daten.writeInt16LE(Math.round(v * 32767), i * bytesJeWert);
  }

  const kopf = Buffer.alloc(44);
  kopf.write("RIFF", 0, "ascii");
  kopf.writeUInt32LE(36 + daten.length, 4);
  kopf.write("WAVE", 8, "ascii");
  kopf.write("fmt ", 12, "ascii");
  kopf.writeUInt32LE(16, 16); // Länge des fmt-Blocks
  kopf.writeUInt16LE(1, 20); // 1 = PCM, unkomprimiert
  kopf.writeUInt16LE(KANAELE, 22);
  kopf.writeUInt32LE(RATE, 24);
  kopf.writeUInt32LE((RATE * KANAELE * BITS) / 8, 28); // Byte pro Sekunde
  kopf.writeUInt16LE((KANAELE * BITS) / 8, 32); // Byte pro Rahmen
  kopf.writeUInt16LE(BITS, 34);
  kopf.write("data", 36, "ascii");
  kopf.writeUInt32LE(daten.length, 40);

  return Buffer.concat([kopf, daten]);
}

/* -------------------------------------------------------------------------- */
/* Die Entwürfe                                                               */
/* -------------------------------------------------------------------------- */

/**
 * Frequenzen aus der gleichstufigen Stimmung, weil aufeinanderfolgende Töne
 * sonst schief klingen. Namen statt Zahlen, damit die Entwürfe lesbar bleiben.
 */
const E5 = 659.26;
const G5 = 783.99;
const A5 = 880.0;
const B5 = 987.77;
const D6 = 1174.66;
const E6 = 1318.51;
const G6 = 1567.98;
const D5 = 587.33;
const Eb5 = 622.25;
const Fs5 = 739.99;

/**
 * Die eingebauten Klänge.
 *
 * `id` landet in der Konfiguration und darf sich **nicht** mehr ändern —
 * sonst zeigt eine gespeicherte Auswahl ins Leere. `label` ist die Anzeige im
 * Einstellungsdialog und darf sich ändern.
 *
 * Die Richtung der Tonfolge trägt die Bedeutung: aufwärts wirkt fragend bis
 * freundlich, abwärts abschliessend bis ernst. Deshalb steigen Hinweis und
 * Entwarnung, und Warnung wie Kritisch fallen.
 */
const KLAENGE = [
  {
    id: "hinweis",
    label: "Hinweis (zwei Töne, aufwärts)",
    bauen: () => folge(ton(A5, 70), pause(25), ton(D6, 110)),
  },
  {
    id: "warnung",
    label: "Warnung (zwei Töne, abwärts)",
    bauen: () => folge(ton(Fs5, 80), pause(30), ton(Eb5, 130)),
  },
  {
    id: "kritisch",
    label: "Kritisch (drei Töne, abwärts)",
    bauen: () => folge(ton(A5, 70), pause(25), ton(Fs5, 70), pause(25), ton(D5, 150)),
  },
  {
    id: "alarm",
    label: "Alarm (drei kurze, einer tief)",
    // Vier Töne sind die Obergrenze. Damit das Ganze trotzdem unter 350 ms
    // bleibt, sind die drei ersten kürzer als bei den anderen Entwürfen.
    bauen: () =>
      folge(
        ton(B5, 50),
        pause(25),
        ton(B5, 50),
        pause(25),
        ton(B5, 50),
        pause(30),
        ton(G5, 100),
      ),
  },
  {
    id: "entwarnung",
    label: "Entwarnung (zwei Töne, aufwärts)",
    // Leiser als die Problemtöne: eine gute Nachricht muss nicht laut sein.
    bauen: () => folge(ton(E5, 80, 0.8), pause(25), ton(B5, 150, 0.8)),
  },
  {
    id: "bestaetigung",
    label: "Bestätigung (zwei sehr kurze)",
    // Für eigene Aktionen: man hat geklickt und weiss, was kommt. Der Ton
    // bestätigt nur, dass es angekommen ist.
    bauen: () => folge(ton(E6, 45, 0.7), pause(20), ton(G6, 70, 0.7)),
  },
];

/* -------------------------------------------------------------------------- */
/* Ausgabe                                                                    */
/* -------------------------------------------------------------------------- */

/** Obergrenze, die dieses Skript sich selbst setzt. Siehe Kopfkommentar. */
const MAX_MS = 350;

/**
 * Liest eine erzeugte Datei zurück und prüft sie.
 *
 * Nicht Zierde: ein falsch geschriebener Kopf ergibt eine Datei, die Windows
 * stillschweigend nicht spielt — derselbe Fehler ohne Fehlermeldung, gegen den
 * schon die Formatprüfung im Dialog steht. Geprüft wird gegen die Werte, die
 * hier oben festgelegt sind, nicht gegen erneut berechnete.
 */
function nachpruefen(datei, erwarteteWerte) {
  const fehler = [];
  if (datei.toString("ascii", 0, 4) !== "RIFF") fehler.push("kein RIFF");
  if (datei.toString("ascii", 8, 12) !== "WAVE") fehler.push("kein WAVE");
  if (datei.readUInt32LE(4) !== datei.length - 8) fehler.push("RIFF-Länge falsch");
  if (datei.readUInt16LE(20) !== 1) fehler.push("nicht PCM");
  if (datei.readUInt16LE(22) !== KANAELE) fehler.push("Kanalzahl falsch");
  if (datei.readUInt32LE(24) !== RATE) fehler.push("Abtastrate falsch");
  if (datei.readUInt16LE(34) !== BITS) fehler.push("Bittiefe falsch");

  const datenLaenge = datei.readUInt32LE(40);
  if (datenLaenge !== datei.length - 44) fehler.push("data-Länge falsch");
  if (datenLaenge / (BITS / 8) !== erwarteteWerte) fehler.push("Wertezahl falsch");

  // Spitzenwert: darf nicht an der Grenze kleben, sonst wurde begrenzt und es
  // knackt an der lautesten Stelle.
  let spitze = 0;
  for (let i = 44; i < datei.length; i += 2) {
    spitze = Math.max(spitze, Math.abs(datei.readInt16LE(i)));
  }
  if (spitze >= 32767) fehler.push("übersteuert");
  if (spitze < 1000) fehler.push("praktisch stumm");

  return { fehler, spitze };
}

mkdirSync(ZIEL, { recursive: true });

let gesamt = 0;
let beanstandet = 0;
for (const klang of KLAENGE) {
  const werte = klang.bauen();
  const datei = wav(werte);
  const pfad = join(ZIEL, `${klang.id}.wav`);
  writeFileSync(pfad, datei);

  const ms = Math.round((werte.length / RATE) * 1000);
  const { fehler, spitze } = nachpruefen(datei, werte.length);
  if (ms > MAX_MS) fehler.push(`länger als ${MAX_MS} ms`);

  gesamt += datei.length;
  const marke = fehler.length ? `  ← ${fehler.join(", ")}` : "";
  if (fehler.length) beanstandet += 1;
  console.log(
    `${klang.id.padEnd(14)} ${String(ms).padStart(4)} ms  ${String(datei.length).padStart(6)} Byte  Spitze ${String(spitze).padStart(5)}${marke}`,
  );
}
console.log(`\n${KLAENGE.length} Klänge, ${gesamt} Byte nach ${ZIEL}`);
if (beanstandet > 0) {
  console.error(`\n${beanstandet} Klang/Klänge beanstandet — siehe oben.`);
  process.exit(1);
}

// Die Kennungen müssen zur Tabelle in src-tauri/src/notify/sound.rs passen.
// Ein Skript kann kein Rust lesen, deshalb die Erinnerung im Klartext.
console.log("\nBei einer neuen Kennung: BUILTIN in src-tauri/src/notify/sound.rs mitführen.");
