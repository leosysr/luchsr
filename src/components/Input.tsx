/**
 * Eingabefeld nach den Spezifikationen des Design-Exports.
 *
 *   Höhe 40, Padding 0 12, Radius 10, 1px --border-strong
 *   Fokus  Rahmen --accent-solid + --ring-input-focus (3px weich)
 *   Fehler Rahmen --hot-solid
 *
 * `mono` schaltet auf IBM Plex Mono — für alles Technische: URLs, Hostnamen,
 * Pfade, Zahlen. Das ist die Regel des Exports, nicht Geschmack.
 */

import type { InputHTMLAttributes } from "react";

interface InputProps
  extends Omit<InputHTMLAttributes<HTMLInputElement>, "className" | "size"> {
  /** Technische Werte in Mono setzen. */
  mono?: boolean;
  invalid?: boolean;
}

export function Input({ mono = false, invalid = false, ...rest }: InputProps) {
  return (
    <input
      aria-invalid={invalid || undefined}
      className={[
        "h-control-md w-full rounded-md border bg-card px-cpx-sm text-base text-body",
        "transition-colors duration-fast ease-out",
        "placeholder:text-faint",
        "focus:outline-none focus-visible:ring-input",
        "disabled:is-disabled",
        mono ? "font-mono text-mono" : "",
        invalid
          ? "border-hot-solid focus-visible:border-hot-solid"
          : "border-line-strong focus-visible:border-accent-solid",
      ].join(" ")}
      {...rest}
    />
  );
}

interface NumberInputProps extends Omit<InputProps, "type" | "value" | "onChange"> {
  value: number;
  onValueChange: (value: number) => void;
  min?: number;
  max?: number;
  /** Einheit rechts im Feld, etwa „s". */
  unit?: string;
}

/**
 * Zahlenfeld mit Einheit.
 *
 * Klemmt **nicht** beim Tippen: wer „6" eintippt, um auf „60" zu kommen, darf
 * nicht sofort auf 15 hochgesetzt werden. Geklemmt wird beim Verlassen des
 * Feldes — und zusätzlich im Backend, siehe `Settings::repair`.
 */
export function NumberInput({
  value,
  onValueChange,
  min,
  max,
  unit,
  invalid = false,
  ...rest
}: NumberInputProps) {
  return (
    <div className="relative">
      <input
        type="number"
        inputMode="numeric"
        value={value}
        min={min}
        max={max}
        aria-invalid={invalid || undefined}
        onChange={(event) => {
          const parsed = Number.parseInt(event.target.value, 10);
          onValueChange(Number.isNaN(parsed) ? 0 : parsed);
        }}
        onBlur={() => {
          let geklemmt = value;
          if (min !== undefined) geklemmt = Math.max(min, geklemmt);
          if (max !== undefined) geklemmt = Math.min(max, geklemmt);
          if (geklemmt !== value) onValueChange(geklemmt);
        }}
        className={[
          "h-control-md w-full rounded-md border bg-card px-cpx-sm font-mono text-mono text-body",
          "transition-colors duration-fast ease-out",
          "focus:outline-none focus-visible:ring-input",
          "disabled:is-disabled",
          unit ? "pr-10" : "",
          invalid
            ? "border-hot-solid focus-visible:border-hot-solid"
            : "border-line-strong focus-visible:border-accent-solid",
        ].join(" ")}
        {...rest}
      />
      {unit ? (
        <span
          className="pointer-events-none absolute inset-y-0 right-cpx-sm flex items-center font-mono text-mono-sm text-faint"
          aria-hidden
        >
          {unit}
        </span>
      ) : null}
    </div>
  );
}
