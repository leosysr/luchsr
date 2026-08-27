/**
 * Auswahlfeld. Gleiche Maße wie [`Input`]: Höhe 40, Padding 0 12, Radius 10.
 *
 * Nutzt das native `<select>`. Unter Windows öffnet WebView2 damit die
 * Systemliste — die verhält sich bei Tastaturbedienung und Bildschirmlesern
 * richtig, ohne dass wir das nachbauen. Ein eigenes Aufklappmenü wäre hier
 * Aufwand ohne Gewinn.
 */

import type { SelectHTMLAttributes } from "react";
import { ChevronDown } from "lucide-react";

export interface SelectOption<T extends string> {
  value: T;
  label: string;
}

interface SelectProps<T extends string>
  extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "className" | "value" | "onChange"> {
  value: T;
  options: readonly SelectOption<T>[];
  onValueChange: (value: T) => void;
  invalid?: boolean;
}

export function Select<T extends string>({
  value,
  options,
  onValueChange,
  invalid = false,
  disabled,
  ...rest
}: SelectProps<T>) {
  return (
    <div className="relative">
      <select
        value={value}
        disabled={disabled}
        aria-invalid={invalid || undefined}
        onChange={(event) => onValueChange(event.target.value as T)}
        className={[
          "h-control-md w-full appearance-none rounded-md border bg-card",
          "pl-cpx-sm pr-control-md text-base text-body",
          "transition-colors duration-fast ease-out",
          "focus:outline-none focus-visible:ring-input",
          "disabled:is-disabled",
          invalid
            ? "border-hot-solid focus-visible:border-hot-solid"
            : "border-line-strong focus-visible:border-accent-solid",
        ].join(" ")}
        {...rest}
      >
        {options.map((option) => (
          <option key={option.value} value={option.value}>
            {option.label}
          </option>
        ))}
      </select>
      <ChevronDown
        size={18}
        aria-hidden
        className="pointer-events-none absolute inset-y-0 right-cpx-sm my-auto text-muted"
      />
    </div>
  );
}
