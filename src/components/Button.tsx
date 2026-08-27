/**
 * Button nach den Spezifikationen des Design-Exports.
 *
 *   Höhen    34 / 40 / 48        (sm / md / lg)
 *   Padding  0 12 / 0 16 / 0 22
 *   Gap      6 / 8 / 10
 *   Icon     16 / 18 / 20
 *   Radius   --radius-md (10px), Gewicht 700
 *
 * Varianten laut Export:
 *   primary    grün gefüllt — die normale Aktion
 *   hot        pink gefüllt — höchstens eine pro Ansicht
 *   secondary  weiss mit 1px --border-strong
 *   ghost      transparent, im Hover --surface-accent-soft
 *
 * Keine Pill-Buttons in diesem Design. Press verschiebt 1px nach unten und
 * skaliert nie — dafür sorgt die `press`-Utility aus utilities.css.
 */

import type { ButtonHTMLAttributes, ReactNode } from "react";
import type { LucideIcon } from "lucide-react";

export type ButtonVariant = "primary" | "hot" | "secondary" | "ghost";
export type ButtonSize = "sm" | "md" | "lg";

interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "className"> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  iconLeft?: LucideIcon;
  iconRight?: LucideIcon;
  /** Auf die Breite des Containers strecken. */
  full?: boolean;
  /** Zeigt einen Ladezustand und sperrt die Schaltfläche. */
  busy?: boolean;
  children?: ReactNode;
}

const VARIANT: Record<ButtonVariant, string> = {
  primary: "bg-accent-solid text-on-accent hover:bg-accent-solid-hover",
  hot: "bg-hot-solid text-on-hot hover:bg-hot-solid-hover",
  secondary:
    "bg-card text-body border border-line-strong hover:bg-sunken",
  ghost: "bg-transparent text-muted hover:bg-accent-soft hover:text-accent",
};

const SIZE: Record<ButtonSize, string> = {
  sm: "h-control-sm px-cpx-sm gap-cgap-sm text-sm",
  md: "h-control-md px-cpx-md gap-cgap-md text-base",
  lg: "h-control-lg px-cpx-lg gap-cgap-lg text-lg",
};

const ICON_PX: Record<ButtonSize, number> = { sm: 16, md: 18, lg: 20 };

export function Button({
  variant = "secondary",
  size = "md",
  iconLeft: IconLeft,
  iconRight: IconRight,
  full = false,
  busy = false,
  disabled,
  children,
  ...rest
}: ButtonProps) {
  const iconSize = ICON_PX[size];
  const locked = disabled || busy;

  return (
    <button
      type="button"
      disabled={locked}
      aria-busy={busy || undefined}
      className={[
        "inline-flex shrink-0 items-center justify-center rounded-md font-bold whitespace-nowrap",
        "transition-colors duration-fast ease-out press",
        "disabled:is-disabled disabled:pointer-events-none",
        VARIANT[variant],
        SIZE[size],
        full ? "w-full" : "",
      ].join(" ")}
      {...rest}
    >
      {IconLeft ? <IconLeft size={iconSize} aria-hidden /> : null}
      {children}
      {IconRight ? <IconRight size={iconSize} aria-hidden /> : null}
    </button>
  );
}

interface IconButtonProps extends Omit<ButtonProps, "children" | "iconLeft" | "iconRight"> {
  icon: LucideIcon;
  /** Pflicht: wird aria-label und Tooltip. */
  label: string;
}

/** Quadratische Schaltfläche mit nur einem Symbol. */
export function IconButton({
  icon: Icon,
  label,
  variant = "ghost",
  size = "md",
  disabled,
  busy = false,
  ...rest
}: IconButtonProps) {
  const iconSize = ICON_PX[size];
  const box = { sm: "size-control-sm", md: "size-control-md", lg: "size-control-lg" }[size];

  return (
    <button
      type="button"
      aria-label={label}
      title={label}
      disabled={disabled || busy}
      aria-busy={busy || undefined}
      className={[
        "inline-flex shrink-0 items-center justify-center rounded-md",
        "transition-colors duration-fast ease-out press",
        "disabled:is-disabled disabled:pointer-events-none",
        VARIANT[variant],
        box,
      ].join(" ")}
      {...rest}
    >
      <Icon size={iconSize} aria-hidden />
    </button>
  );
}
