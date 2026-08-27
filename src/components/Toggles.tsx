/**
 * Schalter und Kontrollkästchen nach den Spezifikationen des Design-Exports.
 *
 *   Switch    Track 40×22, Radius Pill, Knopf 16 mit --shadow-hairline,
 *             Weg links 3 → 21, Dauer 180 ms
 *   Checkbox  20×20, Radius 4, angehakt --accent-solid gefüllt,
 *             Haken 14 px in Weiss
 *
 * Der Switch-Track ist die **einzige** Stelle in diesem Design, an der ein
 * Pill-Radius vorkommt.
 */

import { Check } from "lucide-react";

interface SwitchProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
  /** Pflicht, wenn kein sichtbares Label daneben steht. */
  label?: string;
  id?: string;
}

export function Switch({
  checked,
  onCheckedChange,
  disabled = false,
  label,
  id,
}: SwitchProps) {
  return (
    <button
      type="button"
      role="switch"
      id={id}
      aria-checked={checked}
      aria-label={label}
      disabled={disabled}
      onClick={() => onCheckedChange(!checked)}
      className={[
        "relative shrink-0 rounded-pill border transition-colors duration-base ease-out",
        "w-switch-w h-switch-h",
        "disabled:is-disabled disabled:pointer-events-none",
        checked
          ? "bg-accent-solid border-accent-solid"
          : "bg-sunken border-line-strong",
      ].join(" ")}
    >
      <span
        aria-hidden
        className="absolute top-1/2 size-knob -translate-y-1/2 rounded-pill bg-card shadow-hairline transition-[left] duration-base ease-out"
        style={{
          left: checked ? "var(--switch-knob-on)" : "var(--switch-knob-off)",
        }}
      />
    </button>
  );
}

interface CheckboxProps {
  checked: boolean;
  onCheckedChange: (checked: boolean) => void;
  disabled?: boolean;
  label: string;
  /**
   * Erläuterung unter dem Label, gleiche Form wie der Hinweis in `Field`.
   * Bleibt weg, wenn das Label für sich verständlich ist.
   */
  hint?: string | undefined;
  id?: string;
}

export function Checkbox({
  checked,
  onCheckedChange,
  disabled = false,
  label,
  hint,
  id,
}: CheckboxProps) {
  return (
    // Grid statt Flex: die Box liegt in derselben Zeile wie das Label und
    // zentriert sich in dessen Zeilenhöhe, der Hinweis sitzt darunter in der
    // Textspalte. So braucht die Ausrichtung keinen gemessenen Versatz.
    <label
      className={[
        "grid cursor-pointer grid-cols-[auto_1fr] items-center gap-x-cgap-md select-none",
        disabled ? "is-disabled pointer-events-none" : "",
      ].join(" ")}
    >
      <button
        type="button"
        role="checkbox"
        id={id}
        aria-checked={checked}
        disabled={disabled}
        onClick={() => onCheckedChange(!checked)}
        className={[
          "flex shrink-0 items-center justify-center rounded-xs border size-checkbox",
          "transition-colors duration-base ease-out",
          checked
            ? "bg-accent-solid border-accent-solid text-on-accent"
            : "bg-card border-line-strong text-transparent",
        ].join(" ")}
      >
        <Check size={14} strokeWidth={3} aria-hidden />
      </button>
      <span className="text-base text-body">{label}</span>
      {hint ? <p className="col-start-2 mt-1 text-sm text-muted">{hint}</p> : null}
    </label>
  );
}

interface SegmentedOption<T extends string> {
  value: T;
  label: string;
  icon?: React.ComponentType<{ size?: number; "aria-hidden"?: boolean }>;
}

interface SegmentedProps<T extends string> {
  value: T;
  options: readonly SegmentedOption<T>[];
  onValueChange: (value: T) => void;
  label: string;
}

/**
 * Segmentierte Auswahl für kurze, gleichrangige Optionen — Theme, Proxy-Modus.
 *
 * Kein eigenes Muster des Exports, sondern aus Ghost- und Primary-Button
 * zusammengesetzt: dieselben Farben, dieselben Dauern.
 */
export function Segmented<T extends string>({
  value,
  options,
  onValueChange,
  label,
}: SegmentedProps<T>) {
  return (
    <div
      role="group"
      aria-label={label}
      className="inline-flex gap-1 rounded-md border border-line bg-sunken p-1"
    >
      {options.map((option) => {
        const Icon = option.icon;
        const active = option.value === value;
        return (
          <button
            key={option.value}
            type="button"
            aria-pressed={active}
            onClick={() => onValueChange(option.value)}
            className={[
              "inline-flex items-center gap-cgap-md rounded-sm px-cpx-sm py-2 text-sm font-bold",
              "transition-colors duration-fast ease-out press",
              active
                ? "bg-accent-solid text-on-accent"
                : "text-muted hover:bg-accent-soft hover:text-accent",
            ].join(" ")}
          >
            {Icon ? <Icon size={16} aria-hidden /> : null}
            {option.label}
          </button>
        );
      })}
    </div>
  );
}
