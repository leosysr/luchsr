/**
 * Statusmodell — die Darstellungsschicht der sechs Zustände.
 *
 * Verbindliche Regel: Status wird NIE allein über Farbe kodiert. Jeder Zustand
 * trägt zusätzlich ein Symbol und ein Kürzel. Wer farbfehlsichtig ist oder auf
 * einen schlecht kalibrierten Monitor schaut, muss die Liste trotzdem lesen
 * können.
 *
 * Die Farbwerte selbst stehen ausschliesslich in src/styles/tokens.css. Hier
 * stehen nur Token- bzw. Klassennamen. Die Klassenstrings sind absichtlich
 * literal ausgeschrieben und nicht zusammengesetzt, weil Tailwind die Quellen
 * statisch scannt und dynamisch gebaute Klassennamen nicht findet.
 *
 * Die Zuordnung der CheckMK-Zustandsnummern auf diese Schlüssel kommt mit dem
 * API-Client (Slice 3) und gehört nicht hierher.
 */

import {
  Check,
  CircleHelp,
  ClockFading,
  OctagonX,
  ServerOff,
  TriangleAlert,
} from "lucide-react";
import type { LucideIcon } from "lucide-react";

export type StatusKey = "ok" | "warn" | "crit" | "unknown" | "down" | "stale";

export interface StatusMeta {
  readonly key: StatusKey;
  /** Kürzel für die dichte Tabelle. */
  readonly short: string;
  readonly icon: LucideIcon;
  /**
   * Sortierordnung, höher ist schlimmer: Host DOWN schlägt CRIT, CRIT schlägt
   * UNKNOWN.
   *
   * Die Begründung für diese Reihenfolge: ein ausgefallener Host macht jede
   * Aussage über seine Services wertlos, und „der Check funktioniert nicht"
   * (UNKNOWN) ist weniger dringend als „der Dienst ist kaputt" (CRIT).
   */
  readonly severity: number;
  /** Vordergrundfarbe als Tailwind-Klasse. */
  readonly fg: string;
  /** Flächenfarbe für Zeilenhintergrund und Chip. */
  readonly soft: string;
  /** Rahmenfarbe, gleiche Farbe wie der Vordergrund. */
  readonly ring: string;
}

export const STATUS: Readonly<Record<StatusKey, StatusMeta>> = {
  down: {
    key: "down",
    short: "DOWN",
    icon: ServerOff,
    severity: 50,
    fg: "text-state-down",
    soft: "bg-state-down-soft",
    ring: "border-state-down",
  },
  crit: {
    key: "crit",
    short: "CRIT",
    icon: OctagonX,
    severity: 40,
    fg: "text-state-crit",
    soft: "bg-state-crit-soft",
    ring: "border-state-crit",
  },
  unknown: {
    key: "unknown",
    short: "UNKN",
    icon: CircleHelp,
    severity: 30,
    fg: "text-state-unknown",
    soft: "bg-state-unknown-soft",
    ring: "border-state-unknown",
  },
  warn: {
    key: "warn",
    short: "WARN",
    icon: TriangleAlert,
    severity: 20,
    fg: "text-state-warn",
    soft: "bg-state-warn-soft",
    ring: "border-state-warn",
  },
  stale: {
    key: "stale",
    short: "STALE",
    icon: ClockFading,
    severity: 10,
    fg: "text-state-stale",
    soft: "bg-state-stale-soft",
    ring: "border-state-stale",
  },
  ok: {
    key: "ok",
    short: "OK",
    icon: Check,
    severity: 0,
    fg: "text-state-ok",
    soft: "bg-state-ok-soft",
    ring: "border-state-ok",
  },
} as const;

/** Alle Zustände, absteigend nach Schwere — die Standardsortierung der Liste. */
export const STATUS_BY_SEVERITY: readonly StatusMeta[] = Object.values(
  STATUS,
).sort((a, b) => b.severity - a.severity);
