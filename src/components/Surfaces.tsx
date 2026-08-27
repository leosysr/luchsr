/**
 * Flächen nach den Spezifikationen des Design-Exports.
 *
 *   Card     Padding 20, Radius 14, 1px --border-subtle, --shadow-card.
 *            Im Dunkelmodus fällt der Schatten weg — das regelt tokens.css,
 *            nicht diese Datei ("shadow on ink reads as mud").
 *   Callout  Padding 14 16, Radius 10, **vierseitiger** Rahmen plus weicher
 *            Ton. Das Muster mit nur farbigem linken Rand ist ausdrücklich
 *            nicht Teil dieses Designs.
 *   Badge    Padding 4 9, Radius 6, 11px Mono 600, Grossbuchstaben,
 *            Laufweite .06em.
 */

import type { ReactNode } from "react";
import { CircleCheck, CircleHelp, Info, OctagonX, TriangleAlert } from "lucide-react";
import type { LucideIcon } from "lucide-react";

/* ----------------------------------------------------------------- Card --- */

interface CardProps {
  children: ReactNode;
  /** Kicker über dem Titel: Mono, Grossbuchstaben, weit gesperrt. */
  kicker?: string;
  title?: string;
  /** Rechts oben, für Aktionen. */
  actions?: ReactNode;
}

export function Card({ children, kicker, title, actions }: CardProps) {
  return (
    <section className="rounded-lg border border-line bg-card p-card shadow-card">
      {kicker || title || actions ? (
        <header className="mb-5 flex items-start justify-between gap-4">
          <div className="flex flex-col gap-1">
            {kicker ? (
              <p className="font-mono text-mono-xs font-semibold tracking-kicker text-faint uppercase">
                {kicker}
              </p>
            ) : null}
            {title ? (
              <h2 className="text-h3 font-display font-extrabold tracking-heading text-body">
                {title}
              </h2>
            ) : null}
          </div>
          {actions ? <div className="flex shrink-0 gap-cgap-md">{actions}</div> : null}
        </header>
      ) : null}
      {children}
    </section>
  );
}

/* -------------------------------------------------------------- Callout --- */

export type CalloutTone = "info" | "ok" | "warn" | "crit" | "unknown";

const TONE: Record<CalloutTone, { border: string; bg: string; fg: string; icon: LucideIcon }> = {
  info: {
    border: "border-line-strong",
    bg: "bg-sunken",
    fg: "text-muted",
    icon: Info,
  },
  ok: {
    border: "border-state-ok",
    bg: "bg-state-ok-soft",
    fg: "text-state-ok",
    icon: CircleCheck,
  },
  warn: {
    border: "border-state-warn",
    bg: "bg-state-warn-soft",
    fg: "text-state-warn",
    icon: TriangleAlert,
  },
  crit: {
    border: "border-state-crit",
    bg: "bg-state-crit-soft",
    fg: "text-state-crit",
    icon: OctagonX,
  },
  unknown: {
    border: "border-state-unknown",
    bg: "bg-state-unknown-soft",
    fg: "text-state-unknown",
    icon: CircleHelp,
  },
};

interface CalloutProps {
  tone?: CalloutTone;
  title?: string;
  children?: ReactNode;
  /** Rechts unten, für Folgeaktionen. */
  actions?: ReactNode;
}

export function Callout({ tone = "info", title, children, actions }: CalloutProps) {
  const style = TONE[tone];
  const Icon = style.icon;

  return (
    <div
      className={`flex flex-col gap-3 rounded-md border px-callout-x py-callout-y ${style.border} ${style.bg}`}
    >
      <div className="flex items-start gap-cgap-md">
        <Icon size={18} className={`mt-1 shrink-0 ${style.fg}`} aria-hidden />
        <div className="flex min-w-0 flex-col gap-1">
          {title ? (
            <p className={`text-base font-bold ${style.fg}`}>{title}</p>
          ) : null}
          {children ? (
            <div className="text-sm leading-body text-body">{children}</div>
          ) : null}
        </div>
      </div>
      {actions ? <div className="flex justify-end gap-cgap-md">{actions}</div> : null}
    </div>
  );
}

/* ---------------------------------------------------------------- Badge --- */

export type BadgeTone = "allow" | "block" | "neutral" | "ink" | "solid";

const BADGE: Record<BadgeTone, string> = {
  allow: "bg-state-ok-soft text-state-ok",
  block: "bg-state-crit-soft text-state-crit",
  neutral: "bg-sunken text-muted",
  ink: "bg-ink text-code-text",
  solid: "bg-hot-solid text-on-hot",
};

interface BadgeProps {
  children: ReactNode;
  tone?: BadgeTone;
  icon?: LucideIcon;
}

export function Badge({ children, tone = "neutral", icon: Icon }: BadgeProps) {
  return (
    <span
      className={`inline-flex items-center gap-1 rounded-sm px-badge-x py-badge-y font-mono text-mono-xs font-semibold tracking-badge uppercase ${BADGE[tone]}`}
    >
      {Icon ? <Icon size={12} aria-hidden /> : null}
      {children}
    </span>
  );
}
