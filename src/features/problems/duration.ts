/**
 * Dauer seit dem Statuswechsel, formatiert für die dichte Tabelle.
 *
 * Reine Funktionen, damit die Formate prüfbar sind — bei einer Tabelle mit 80
 * Zeilen fällt eine falsche Spaltenbreite oder ein springendes Format sofort
 * auf, und beides hängt allein an diesen Regeln.
 */

const SEKUNDE = 1000;
const MINUTE = 60 * SEKUNDE;
const STUNDE = 60 * MINUTE;
const TAG = 24 * STUNDE;

/** Was in der Spalte steht, wenn CheckMK keinen Statuswechsel kennt. */
export const KEINE_DAUER = "—";

const zwei = (n: number) => String(Math.floor(n)).padStart(2, "0");

/**
 * Formatiert eine Dauer in Millisekunden.
 *
 * Zwei Formate, und der Wechsel ist Absicht:
 *
 *   unter einem Tag   `HH:MM:SS`   — Sekunden zählen, wenn etwas gerade passiert
 *   ab einem Tag      `Nd HH:MM`   — Sekunden sind dann Rauschen, die Spalte
 *                                    bleibt aber gleich breit
 *
 * Ohne den Wechsel stünde nach einer Woche `168:23:11` in der Spalte, und die
 * Zahl sagt weniger als `7d 00:23`.
 */
export function formatDuration(millis: number): string {
  if (!Number.isFinite(millis) || millis < 0) return KEINE_DAUER;

  if (millis >= TAG) {
    const tage = Math.floor(millis / TAG);
    const rest = millis % TAG;
    return `${tage}d ${zwei(rest / STUNDE)}:${zwei((rest % STUNDE) / MINUTE)}`;
  }

  const stunden = Math.floor(millis / STUNDE);
  const minuten = Math.floor((millis % STUNDE) / MINUTE);
  const sekunden = Math.floor((millis % MINUTE) / SEKUNDE);
  return `${zwei(stunden)}:${zwei(minuten)}:${zwei(sekunden)}`;
}

/**
 * Dauer zwischen einem ISO-Zeitstempel und `now`.
 *
 * `null` heisst „CheckMK kennt keinen Statuswechsel" — das ist nicht dasselbe
 * wie „vor 0 Sekunden" und darf nicht als `00:00:00` erscheinen.
 */
export function durationSince(iso: string | null, now: number): string {
  if (iso === null) return KEINE_DAUER;
  const then = Date.parse(iso);
  if (Number.isNaN(then)) return KEINE_DAUER;
  return formatDuration(now - then);
}

/**
 * Vollständiger Zeitstempel in Ortszeit, für das Detail-Panel.
 *
 * Das Backend liefert UTC; der Benutzer denkt in Ortszeit, und beim Vergleich
 * mit einem Ticket oder einer Logzeile zählt genau die.
 */
export function formatTimestamp(iso: string | null): string {
  if (iso === null) return KEINE_DAUER;
  const date = new Date(iso);
  if (Number.isNaN(date.getTime())) return KEINE_DAUER;
  return date.toLocaleString("de-DE", {
    year: "numeric",
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
}

/**
 * `plugin_output` auf eine Zeile bringen.
 *
 * CheckMK liefert oft mehrzeilige Ausgaben. In der Liste ist Platz für eine
 * Zeile; das Detail-Panel zeigt den vollen Text. Gekürzt wird **nicht** hier,
 * sondern per CSS — sonst hängt die Kürzung an einer geratenen Zeichenzahl
 * statt an der tatsächlichen Spaltenbreite.
 */
export function firstLine(output: string): string {
  const zeile = output.split("\n", 1)[0] ?? "";
  return zeile.trim();
}
