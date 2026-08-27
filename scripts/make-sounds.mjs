/**
 * DIE HINWEISTÖNE: Entwürfe, Synthese, WAV-Ausgabe, Selbstprüfung.
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
 * der Quelltext, der ihn beschreibt.
 *
 * # Warum so kurz
 *
 * Die Klänge in `C:\Windows\Media` sind Melodien von einer halben bis ganzen
 * Sekunde. Wer zwanzig Meldungen am Tag bekommt, hört sie zwanzigmal — und
 * fängt an, sie abzuschalten. Ein Hinweis darf höchstens so lang sein, dass er
 * vorbei ist, bevor man ihn bewusst wahrnimmt: zwei bis vier Töne, unter
 * [`MAX_MS`]. Das gilt für **jede** Familie; variiert wird die Klangfarbe, nicht
 * die Länge.
 *
 * # Die Familien
 *
 * | Familie   | Charakter                        | Wellenform                  |
 * |-----------|----------------------------------|-----------------------------|
 * | `sinus`   | weich, neutral                   | reiner Sinus                |
 * | `marimba` | holzig, perkussiv                | Grundton + 4. Teilton       |
 * | `glocke`  | metallisch, nachklingend         | inharmonische Teiltöne      |
 * | `blip`    | digital, knapp                   | bandbegrenzte Pulswelle     |
 * | `tropfen` | gleitende Tonhöhe                | Sinus mit Frequenzverlauf   |
 * | `akkord`  | mehrere Töne gleichzeitig        | Dreiklang                   |
 *
 * Die Kennung (`id`) landet in der Konfiguration und darf sich **nie** ändern —
 * eine gespeicherte Auswahl zeigt sonst ins Leere. Die Beschriftung darf.
 */

import { mkdirSync, writeFileSync, readdirSync, unlinkSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HIER = dirname(fileURLToPath(import.meta.url));
const ZIEL = join(HIER, "..", "src-tauri", "sounds");

/* -------------------------------------------------------------------------- */
/* Format                                                                     */
/* -------------------------------------------------------------------------- */

/**
 * 22050 Hz reicht: der höchste verwendete Grundton liegt bei rund 1570 Hz, die
 * Nyquist-Grenze bei 11025 Hz. Oberwellen werden bandbegrenzt erzeugt, damit
 * nichts darüber hinaus entsteht und als Aliasing zurückfaltet.
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

/** Obergrenze, die dieses Skript sich selbst setzt. Siehe Kopfkommentar. */
const MAX_MS = 350;

/* -------------------------------------------------------------------------- */
/* Hüllkurven                                                                 */
/* -------------------------------------------------------------------------- */

/**
 * Jede Hüllkurve endet garantiert bei null.
 *
 * Ohne das beginnt oder endet ein Ton an einer Sprungstelle, und das hört man
 * als Knacken — bei kurzen Tönen lauter als den Ton selbst. Besonders bei den
 * perkussiven Familien, deren exponentieller Abfall mathematisch nie ganz
 * null erreicht: die letzten Millisekunden werden zusätzlich ausgeblendet.
 */
const AUSBLENDE_MS = 8;

/** Weich: linearer An- und Abstieg. Für die Sinus-Familie. */
function weich(i, n) {
  const anstieg = Math.min(0.006 * RATE, n * 0.2);
  const abfall = Math.min(0.05 * RATE, n * 0.6);
  if (i < anstieg) return i / anstieg;
  const rest = n - i;
  if (rest < abfall) return rest / abfall;
  return 1;
}

/**
 * Perkussiv: sehr kurzer Anstieg, exponentieller Abfall.
 *
 * `tau` als Anteil der Gesamtlänge — so bleibt die Form gleich, egal wie lang
 * der Ton ist.
 */
function perkussiv(tau) {
  return (i, n) => {
    const anstieg = Math.min(0.0015 * RATE, n * 0.05);
    const hoch = i < anstieg ? i / anstieg : 1;
    return hoch * Math.exp(-i / (n * tau));
  };
}

/** Garantiert null am Ende, unabhängig von der Hüllkurve. */
function ausblenden(i, n) {
  const fade = Math.min(AUSBLENDE_MS * 0.001 * RATE, n * 0.3);
  const rest = n - i;
  return rest < fade ? rest / fade : 1;
}

/* -------------------------------------------------------------------------- */
/* Wellenformen                                                               */
/* -------------------------------------------------------------------------- */

/**
 * Eine Wellenform ist eine Funktion Phase → Wert, wobei die Phase in Radiant
 * läuft. Die Teiltöne werden **bandbegrenzt** summiert: nur was unter der
 * Nyquist-Grenze liegt, wird erzeugt. Sonst faltet es als Aliasing in den
 * Hörbereich zurück und klingt schmutzig.
 */
const NYQUIST_ANTEIL = 0.9;

function obertoenGrenze(hz) {
  return Math.floor(((RATE / 2) * NYQUIST_ANTEIL) / hz);
}

/** Reiner Sinus. */
const sinus = () => (phase) => Math.sin(phase);

/**
 * Marimba: Grundton plus vierter Teilton.
 *
 * Ein Marimbastab schwingt stark im vierfachen der Grundfrequenz — das ist der
 * Grund, warum er holzig und nicht flötig klingt.
 */
const marimba = (hz) => {
  const vier = obertoenGrenze(hz) >= 4;
  return (phase) => Math.sin(phase) + (vier ? 0.32 * Math.sin(4 * phase) : 0);
};

/**
 * Glocke: inharmonische Teiltöne.
 *
 * Die Verhältnisse sind der bekannten Näherung für Röhrenglocken entlehnt. Sie
 * sind bewusst **keine** ganzzahligen Vielfachen — genau das macht den
 * metallischen Charakter: die Teiltöne bilden keinen Akkord.
 */
const GLOCKE_TEILTOENE = [
  [1.0, 1.0],
  [2.76, 0.5],
  [5.4, 0.25],
  [8.93, 0.12],
];
const glocke = (hz) => {
  const erlaubt = GLOCKE_TEILTOENE.filter(([v]) => hz * v < (RATE / 2) * NYQUIST_ANTEIL);
  return (phase) => erlaubt.reduce((s, [v, a]) => s + a * Math.sin(v * phase), 0);
};

/**
 * Pulswelle, additiv aus ungeraden Teiltönen.
 *
 * Nicht als harter Rechteck: der hätte Oberwellen bis ins Unendliche, und alles
 * oberhalb der Nyquist-Grenze käme als Aliasing zurück. Additiv aufgebaut endet
 * das Spektrum genau dort, wo es enden soll.
 */
const puls = (hz) => {
  const max = obertoenGrenze(hz);
  const teiltoene = [];
  for (let n = 1; n <= max; n += 2) teiltoene.push(n);
  return (phase) => teiltoene.reduce((s, n) => s + Math.sin(n * phase) / n, 0) * 0.8;
};

/* -------------------------------------------------------------------------- */
/* Synthese                                                                   */
/* -------------------------------------------------------------------------- */

/**
 * Ein Ton.
 *
 * Die Phase wird **aufsummiert** und nicht aus `2π f t` berechnet. Bei einer
 * gleitenden Tonhöhe wäre die geschlossene Form falsch: sie erzeugt an jedem
 * Abtastpunkt einen Phasensprung, und den hört man als Rauschen. Aufsummiert
 * bleibt die Phase stetig.
 *
 * @param {object} o
 * @param {number} o.hz       Grundfrequenz
 * @param {number} [o.nachHz] Zielfrequenz für einen Gleitton
 * @param {number} o.ms       Länge
 * @param {number} [o.pegel]  relative Lautstärke
 * @param {(hz:number)=>(phase:number)=>number} [o.form] Wellenform
 * @param {(i:number,n:number)=>number} [o.huelle] Hüllkurve
 */
function ton({ hz, nachHz, ms, pegel = 1, form = sinus, huelle = weich }) {
  const n = Math.round((ms / 1000) * RATE);
  const welle = form(Math.max(hz, nachHz ?? hz));
  const werte = new Float64Array(n);
  let phase = 0;
  for (let i = 0; i < n; i += 1) {
    // Exponentiell gleiten, nicht linear: Tonhöhe wird logarithmisch
    // wahrgenommen, ein linearer Verlauf klingt am Ende zu langsam.
    const t = n > 1 ? i / (n - 1) : 0;
    const f = nachHz === undefined ? hz : hz * Math.pow(nachHz / hz, t);
    werte[i] = welle(phase) * huelle(i, n) * ausblenden(i, n) * pegel;
    phase += (2 * Math.PI * f) / RATE;
  }
  return werte;
}

/** Mehrere Töne gleichzeitig, auf die Summe der Pegel normiert. */
function akkord({ hzListe, ms, pegel = 1, form = sinus, huelle = weich }) {
  const stimmen = hzListe.map((hz) => ton({ hz, ms, form, huelle }));
  const n = stimmen[0].length;
  const aus = new Float64Array(n);
  for (let i = 0; i < n; i += 1) {
    let s = 0;
    for (const stimme of stimmen) s += stimme[i];
    aus[i] = (s / hzListe.length) * pegel;
  }
  return aus;
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
 *
 * Normiert vorher auf die Spitze: die Familien haben von Natur aus verschiedene
 * Amplituden (ein Dreiklang summiert sich, eine Pulswelle hat mehr Energie als
 * ein Sinus). Ohne Normierung wäre die Auswahl im Dialog ein Lautstärkeritt.
 */
function wav(werte) {
  let spitze = 0;
  for (const v of werte) spitze = Math.max(spitze, Math.abs(v));
  const norm = spitze > 0 ? PEGEL / spitze : 0;

  const bytesJeWert = BITS / 8;
  const daten = Buffer.alloc(werte.length * bytesJeWert);
  for (let i = 0; i < werte.length; i += 1) {
    const v = Math.max(-1, Math.min(1, werte[i] * norm));
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
/* Tonvorrat                                                                  */
/* -------------------------------------------------------------------------- */

/**
 * Frequenzen aus der gleichstufigen Stimmung, weil aufeinanderfolgende Töne
 * sonst schief klingen. Namen statt Zahlen, damit die Entwürfe lesbar bleiben.
 */
const C5 = 523.25;
const D5 = 587.33;
const Eb5 = 622.25;
const E5 = 659.26;
const F5 = 698.46;
const Fs5 = 739.99;
const G5 = 783.99;
const A5 = 880.0;
const Bb5 = 932.33;
const B5 = 987.77;
const C6 = 1046.5;
const D6 = 1174.66;
const E6 = 1318.51;
const G6 = 1567.98;

/* -------------------------------------------------------------------------- */
/* Die Entwürfe                                                               */
/* -------------------------------------------------------------------------- */

/**
 * Die Richtung der Tonfolge trägt die Bedeutung: aufwärts wirkt fragend bis
 * freundlich, abwärts abschliessend bis ernst. Deshalb steigen die
 * Hinweis- und Entwarnungsklänge, und die Warn- und Kritischklänge fallen —
 * in jeder Familie gleich, damit die Bedeutung erkennbar bleibt, egal welche
 * Klangfarbe gewählt ist.
 *
 * Die ersten sechs Kennungen sind die der ersten Fassung und bleiben
 * unverändert: eine gespeicherte Auswahl muss weiter gelten.
 */
const KLAENGE = [
  /* ------------------------------------------------------------ Sinus ---- */
  {
    id: "hinweis",
    label: "Sinus · Hinweis (zwei Töne, aufwärts)",
    bauen: () => folge(ton({ hz: A5, ms: 70 }), pause(25), ton({ hz: D6, ms: 110 })),
  },
  {
    id: "warnung",
    label: "Sinus · Warnung (zwei Töne, abwärts)",
    bauen: () => folge(ton({ hz: Fs5, ms: 80 }), pause(30), ton({ hz: Eb5, ms: 130 })),
  },
  {
    id: "kritisch",
    label: "Sinus · Kritisch (drei Töne, abwärts)",
    bauen: () =>
      folge(
        ton({ hz: A5, ms: 70 }),
        pause(25),
        ton({ hz: Fs5, ms: 70 }),
        pause(25),
        ton({ hz: D5, ms: 150 }),
      ),
  },
  {
    id: "alarm",
    label: "Sinus · Alarm (drei kurze, einer tief)",
    bauen: () =>
      folge(
        ton({ hz: B5, ms: 50 }),
        pause(25),
        ton({ hz: B5, ms: 50 }),
        pause(25),
        ton({ hz: B5, ms: 50 }),
        pause(30),
        ton({ hz: G5, ms: 100 }),
      ),
  },
  {
    id: "entwarnung",
    label: "Sinus · Entwarnung (zwei Töne, aufwärts)",
    bauen: () =>
      folge(ton({ hz: E5, ms: 80, pegel: 0.8 }), pause(25), ton({ hz: B5, ms: 150, pegel: 0.8 })),
  },
  {
    id: "bestaetigung",
    label: "Sinus · Bestätigung (zwei sehr kurze)",
    bauen: () =>
      folge(ton({ hz: E6, ms: 45, pegel: 0.7 }), pause(20), ton({ hz: G6, ms: 70, pegel: 0.7 })),
  },

  /* ---------------------------------------------------------- Marimba ---- */
  {
    id: "marimba-hinweis",
    label: "Marimba · Hinweis (zwei Töne, aufwärts)",
    bauen: () =>
      folge(
        ton({ hz: A5, ms: 90, form: marimba, huelle: perkussiv(0.22) }),
        pause(15),
        ton({ hz: D6, ms: 150, form: marimba, huelle: perkussiv(0.26) }),
      ),
  },
  {
    id: "marimba-warnung",
    label: "Marimba · Warnung (zwei Töne, abwärts)",
    bauen: () =>
      folge(
        ton({ hz: G5, ms: 90, form: marimba, huelle: perkussiv(0.22) }),
        pause(20),
        ton({ hz: Eb5, ms: 170, form: marimba, huelle: perkussiv(0.28) }),
      ),
  },
  {
    id: "marimba-kritisch",
    label: "Marimba · Kritisch (drei Töne, abwärts)",
    bauen: () =>
      folge(
        ton({ hz: A5, ms: 80, form: marimba, huelle: perkussiv(0.2) }),
        pause(15),
        ton({ hz: F5, ms: 80, form: marimba, huelle: perkussiv(0.2) }),
        pause(15),
        ton({ hz: C5, ms: 150, form: marimba, huelle: perkussiv(0.3) }),
      ),
  },
  {
    id: "marimba-anschlag",
    label: "Marimba · Anschlag (ein Ton)",
    bauen: () => ton({ hz: C6, ms: 200, form: marimba, huelle: perkussiv(0.24) }),
  },

  /* ----------------------------------------------------------- Glocke ---- */
  {
    id: "glocke-hinweis",
    label: "Glocke · Hinweis (zwei Töne, aufwärts)",
    bauen: () =>
      folge(
        ton({ hz: E5, ms: 100, form: glocke, huelle: perkussiv(0.3) }),
        pause(10),
        ton({ hz: A5, ms: 200, form: glocke, huelle: perkussiv(0.32) }),
      ),
  },
  {
    id: "glocke-warnung",
    label: "Glocke · Warnung (zwei Töne, abwärts)",
    bauen: () =>
      folge(
        ton({ hz: A5, ms: 100, form: glocke, huelle: perkussiv(0.3) }),
        pause(15),
        ton({ hz: F5, ms: 210, form: glocke, huelle: perkussiv(0.34) }),
      ),
  },
  {
    id: "glocke-kritisch",
    label: "Glocke · Kritisch (drei Töne, abwärts)",
    bauen: () =>
      folge(
        ton({ hz: Bb5, ms: 80, form: glocke, huelle: perkussiv(0.24) }),
        pause(10),
        ton({ hz: G5, ms: 80, form: glocke, huelle: perkussiv(0.24) }),
        pause(10),
        ton({ hz: D5, ms: 160, form: glocke, huelle: perkussiv(0.3) }),
      ),
  },
  {
    id: "glocke-einzeln",
    label: "Glocke · Einzelschlag (ein Ton)",
    bauen: () => ton({ hz: A5, ms: 300, form: glocke, huelle: perkussiv(0.3) }),
  },

  /* ------------------------------------------------------------- Blip ---- */
  {
    id: "blip-hinweis",
    label: "Blip · Hinweis (zwei Töne, aufwärts)",
    bauen: () =>
      folge(
        ton({ hz: A5, ms: 45, form: puls, huelle: perkussiv(0.4) }),
        pause(20),
        ton({ hz: E6, ms: 70, form: puls, huelle: perkussiv(0.4) }),
      ),
  },
  {
    id: "blip-warnung",
    label: "Blip · Warnung (zwei Töne, abwärts)",
    bauen: () =>
      folge(
        ton({ hz: G5, ms: 50, form: puls, huelle: perkussiv(0.4) }),
        pause(25),
        ton({ hz: D5, ms: 90, form: puls, huelle: perkussiv(0.45) }),
      ),
  },
  {
    id: "blip-kritisch",
    label: "Blip · Kritisch (drei Töne, abwärts)",
    bauen: () =>
      folge(
        ton({ hz: B5, ms: 45, form: puls, huelle: perkussiv(0.35) }),
        pause(20),
        ton({ hz: G5, ms: 45, form: puls, huelle: perkussiv(0.35) }),
        pause(20),
        ton({ hz: D5, ms: 110, form: puls, huelle: perkussiv(0.45) }),
      ),
  },
  {
    id: "blip-doppel",
    label: "Blip · Doppelklick (zwei gleiche)",
    bauen: () =>
      folge(
        ton({ hz: C6, ms: 40, form: puls, huelle: perkussiv(0.35) }),
        pause(35),
        ton({ hz: C6, ms: 40, form: puls, huelle: perkussiv(0.35) }),
      ),
  },

  /* ---------------------------------------------------------- Tropfen ---- */
  {
    id: "tropfen-auf",
    label: "Tropfen · aufwärts (gleitend)",
    bauen: () => ton({ hz: E5, nachHz: C6, ms: 170, huelle: perkussiv(0.35) }),
  },
  {
    id: "tropfen-ab",
    label: "Tropfen · abwärts (gleitend)",
    bauen: () => ton({ hz: C6, nachHz: E5, ms: 190, huelle: perkussiv(0.35) }),
  },
  {
    id: "tropfen-doppel",
    label: "Tropfen · zwei gleitende",
    bauen: () =>
      folge(
        ton({ hz: A5, nachHz: E6, ms: 110, huelle: perkussiv(0.3) }),
        pause(25),
        ton({ hz: E5, nachHz: B5, ms: 150, huelle: perkussiv(0.34) }),
      ),
  },

  /* ----------------------------------------------------------- Akkord ---- */
  {
    id: "akkord-hell",
    label: "Akkord · hell (Dur-Dreiklang)",
    bauen: () => akkord({ hzListe: [C5, E5, G5], ms: 260, huelle: perkussiv(0.32) }),
  },
  {
    id: "akkord-dunkel",
    label: "Akkord · dunkel (Moll-Dreiklang)",
    bauen: () => akkord({ hzListe: [C5, Eb5, G5], ms: 270, huelle: perkussiv(0.32) }),
  },
  {
    id: "akkord-warnung",
    label: "Akkord · Warnung (Dreiklang, dann tief)",
    bauen: () =>
      folge(
        akkord({ hzListe: [D5, F5, A5], ms: 120, huelle: perkussiv(0.28) }),
        pause(20),
        ton({ hz: D5, ms: 160, huelle: perkussiv(0.3) }),
      ),
  },
];

/* -------------------------------------------------------------------------- */
/* Selbstprüfung                                                              */
/* -------------------------------------------------------------------------- */

/**
 * Liest eine erzeugte Datei zurück und prüft sie.
 *
 * Nicht Zierde: ein falsch geschriebener Kopf ergibt eine Datei, die Windows
 * stillschweigend nicht spielt — derselbe Fehler ohne Fehlermeldung, gegen den
 * auch die Formatprüfung im Einstellungsdialog steht.
 *
 * Geprüft wird zusätzlich auf **Knacken**, und zwar an den Rändern: beginnt
 * oder endet die Datei nicht bei nahezu null, springt die Membran beim Anfangen
 * bzw. Aufhören, und das hört man deutlicher als den Ton selbst.
 *
 * Bewusst **nicht** geprüft wird die Steilheit innerhalb der Welle. Ein erster
 * Versuch tat das und beanstandete acht Klänge — zu Unrecht: eine Pulswelle hat
 * von Natur aus steile Flanken, das ist ihr Timbre. Die Prüfung hätte die
 * `blip`-Familie unmöglich gemacht, obwohl an ihr nichts falsch ist. Ein Test,
 * der die falsche Eigenschaft misst, verbietet richtige Entwürfe.
 */
const MAX_RAND = 400;

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

  let spitze = 0;
  for (let i = 44; i < datei.length; i += 2) {
    spitze = Math.max(spitze, Math.abs(datei.readInt16LE(i)));
  }
  const erste = Math.abs(datei.readInt16LE(44));
  const letzte = Math.abs(datei.readInt16LE(datei.length - 2));
  const rand = Math.max(erste, letzte);

  if (spitze >= 32767) fehler.push("übersteuert");
  if (spitze < 1000) fehler.push("praktisch stumm");
  if (rand > MAX_RAND) fehler.push(`Rand ${rand} — knackt beim Ein- oder Ausschwingen`);

  return { fehler, spitze, rand };
}

/* -------------------------------------------------------------------------- */
/* Ausgabe                                                                    */
/* -------------------------------------------------------------------------- */

// Doppelte Kennungen ergäben eine überschriebene Datei und einen Eintrag im
// Dialog, der auf einen anderen Klang zeigt als er behauptet.
const kennungen = KLAENGE.map((k) => k.id);
const doppelt = kennungen.filter((id, i) => kennungen.indexOf(id) !== i);
if (doppelt.length) {
  console.error(`Doppelte Kennung: ${[...new Set(doppelt)].join(", ")}`);
  process.exit(1);
}

mkdirSync(ZIEL, { recursive: true });

// Verwaiste Dateien entfernen: eine gelöschte Kennung liesse sonst eine WAV
// liegen, die `include_bytes!` nicht mehr einbindet — und die niemand vermisst,
// bis sie im Paket auffällt.
const erwartet = new Set(kennungen.map((id) => `${id}.wav`));
for (const name of readdirSync(ZIEL)) {
  if (name.endsWith(".wav") && !erwartet.has(name)) {
    unlinkSync(join(ZIEL, name));
    console.log(`entfernt (verwaist): ${name}`);
  }
}

let gesamt = 0;
let beanstandet = 0;
let familie = "";
for (const klang of KLAENGE) {
  const werte = klang.bauen();
  const datei = wav(werte);
  writeFileSync(join(ZIEL, `${klang.id}.wav`), datei);

  const ms = Math.round((werte.length / RATE) * 1000);
  const { fehler, spitze, rand } = nachpruefen(datei, werte.length);
  if (ms > MAX_MS) fehler.push(`länger als ${MAX_MS} ms`);

  gesamt += datei.length;
  if (fehler.length) beanstandet += 1;

  const f = klang.label.split(" · ")[0];
  if (f !== familie) {
    familie = f;
    console.log(`\n${familie}`);
  }
  const marke = fehler.length ? `  ← ${fehler.join(", ")}` : "";
  console.log(
    `  ${klang.id.padEnd(20)} ${String(ms).padStart(4)} ms ${String(datei.length).padStart(6)} B  Spitze ${String(spitze).padStart(5)}  Rand ${String(rand).padStart(4)}${marke}`,
  );
}

console.log(
  `\n${KLAENGE.length} Klänge, ${(gesamt / 1024).toFixed(0)} KB nach ${ZIEL}`,
);
if (beanstandet > 0) {
  console.error(`\n${beanstandet} Klang/Klänge beanstandet — siehe oben.`);
  process.exit(1);
}
console.log(
  "\nDie Tabelle BUILTIN in src-tauri/src/notify/sound.rs muss dieselben\n" +
    "Kennungen führen — der Dateiname wird dort aus der Kennung gebildet.",
);
