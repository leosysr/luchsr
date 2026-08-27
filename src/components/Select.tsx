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
  /**
   * Überschrift, unter der der Eintrag einsortiert wird.
   *
   * Fehlt sie, steht der Eintrag ungruppiert oben — so bleibt „Kein Ton" vor
   * allen Gruppen. Aufeinanderfolgende Einträge mit derselben Gruppe landen in
   * einem `<optgroup>`; die Reihenfolge des Feldes bestimmt also die
   * Reihenfolge der Gruppen. Bei 25 Klängen ist das der Unterschied zwischen
   * einer Liste, die man durchsucht, und einer, die man liest.
   *
   * `| undefined` ausdrücklich, wie bei [`FieldProps`]: `exactOptionalPropertyTypes`
   * ist aktiv, und die Aufrufstelle berechnet den Wert. Ohne das müsste sie die
   * Eigenschaft bedingt weglassen, was den Aufruf unlesbar macht.
   */
  group?: string | undefined;
}

interface SelectProps<T extends string>
  extends Omit<SelectHTMLAttributes<HTMLSelectElement>, "className" | "value" | "onChange"> {
  value: T;
  options: readonly SelectOption<T>[];
  onValueChange: (value: T) => void;
  invalid?: boolean;
}

/**
 * Fasst aufeinanderfolgende Einträge gleicher Gruppe zusammen.
 *
 * Bewusst nur **aufeinanderfolgende**: dann bestimmt die Reihenfolge des
 * Aufrufers die Reihenfolge der Gruppen, und es gibt keine zweite, verborgene
 * Sortierregel. Wer gruppiert übergeben will, übergibt gruppiert.
 */
function gruppieren<T extends string>(
  options: readonly SelectOption<T>[],
): { group: string | undefined; items: SelectOption<T>[] }[] {
  const blocks: { group: string | undefined; items: SelectOption<T>[] }[] = [];
  for (const option of options) {
    const letzter = blocks[blocks.length - 1];
    if (letzter && letzter.group === option.group) letzter.items.push(option);
    else blocks.push({ group: option.group, items: [option] });
  }
  return blocks;
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
        {gruppieren(options).map((block) =>
          block.group === undefined ? (
            block.items.map((option) => (
              <option key={option.value} value={option.value}>
                {option.label}
              </option>
            ))
          ) : (
            <optgroup key={block.group} label={block.group}>
              {block.items.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </optgroup>
          ),
        )}
      </select>
      <ChevronDown
        size={18}
        aria-hidden
        className="pointer-events-none absolute inset-y-0 right-cpx-sm my-auto text-muted"
      />
    </div>
  );
}
