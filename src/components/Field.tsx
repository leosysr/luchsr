/**
 * Formularfeld-Rahmen: Beschriftung, Hinweis, Fehler.
 *
 * Regel des Exports: im Fehlerfall **ersetzt** die Meldung den Hinweis, sie
 * tritt nicht daneben. Sonst stehen zwei Texte unter dem Feld und der Benutzer
 * liest den falschen.
 */

import type { ReactNode } from "react";
import { TriangleAlert } from "lucide-react";

/**
 * Die optionalen Props erlauben ausdrücklich `undefined`.
 *
 * Grund: `exactOptionalPropertyTypes` ist in tsconfig.json aktiv, und die
 * Aufrufstellen holen ihre Meldungen aus einer `Map` — deren `get()` gibt
 * `string | undefined` zurück. Ohne das `| undefined` müsste jede Aufrufstelle
 * die Prop bedingt weglassen, was die Aufrufe unlesbar macht.
 */
interface FieldProps {
  label: string;
  /** Erklärender Hinweis unter dem Feld. */
  hint?: string | undefined;
  /** Fehlermeldung. Ersetzt den Hinweis, wenn gesetzt. */
  error?: string | undefined;
  /** Warnung. Wird zusätzlich gezeigt, in gedämpfter Form. */
  warning?: string | undefined;
  /** id des Eingabeelements, für die Verknüpfung mit dem Label. */
  htmlFor?: string | undefined;
  /** Nebeneinander statt untereinander — für Schalter. */
  inline?: boolean;
  children: ReactNode;
}

export function Field({
  label,
  hint,
  error,
  warning,
  htmlFor,
  inline = false,
  children,
}: FieldProps) {
  const beschreibung = error ?? hint;

  if (inline) {
    return (
      <div className="flex flex-col gap-2">
        <div className="flex items-center justify-between gap-4">
          <label
            htmlFor={htmlFor}
            className="text-base font-bold text-body select-none"
          >
            {label}
          </label>
          {children}
        </div>
        {beschreibung ? (
          <p className={`text-sm ${error ? "text-hot" : "text-muted"}`}>{beschreibung}</p>
        ) : null}
        {warning ? <FieldWarning>{warning}</FieldWarning> : null}
      </div>
    );
  }

  return (
    <div className="flex flex-col gap-2">
      <label htmlFor={htmlFor} className="text-base font-bold text-body select-none">
        {label}
      </label>
      {children}
      {beschreibung ? (
        <p className={`text-sm ${error ? "text-hot" : "text-muted"}`}>{beschreibung}</p>
      ) : null}
      {warning ? <FieldWarning>{warning}</FieldWarning> : null}
    </div>
  );
}

function FieldWarning({ children }: { children: string }) {
  return (
    <p className="flex items-start gap-2 text-sm text-muted">
      <TriangleAlert
        size={16}
        className="mt-1 shrink-0 text-state-warn"
        aria-hidden
      />
      <span>{children}</span>
    </p>
  );
}
