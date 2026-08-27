/**
 * Minimaler i18n-Zugriff.
 *
 * Der Auftrag verlangt "i18n-fähig anlegen, aber nur de ausliefern" — deshalb
 * kein i18n-Framework, sondern ein typisierter Wörterbuchzugriff. Der Typ von
 * `de` ist der Vertrag: eine weitere Sprache muss dieselben Schlüssel
 * bedienen, sonst meckert der Compiler.
 *
 * Wenn später wirklich mehrere Sprachen gebraucht werden, wird hier ein
 * aktives Wörterbuch umgeschaltet — die Aufrufstellen bleiben unverändert.
 */

import { de } from "./de";
import type { StatusKey } from "@/lib/status";

export type Dictionary = typeof de;
export type MessageKey = keyof Dictionary;

/** Nur Schlüssel, deren Wert ein einfacher String ist. */
type PlainKey = {
  [K in MessageKey]: Dictionary[K] extends string ? K : never;
}[MessageKey];

const active: Dictionary = de;

/** Übersetzt einen Schlüssel. */
export function t(key: PlainKey): string {
  return active[key];
}

/** Zugriff auf die parametrisierten Einträge, typsicher pro Schlüssel. */
export function tf<K extends MessageKey>(key: K): Dictionary[K] {
  return active[key];
}

/** Anzeigename eines Zustands. */
export function statusLabel(key: StatusKey): string {
  return t(`status.${key}` as PlainKey);
}
