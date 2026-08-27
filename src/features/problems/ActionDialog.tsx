/**
 * Der Dialog für Quittieren und Wartungszeit.
 *
 * Bewusst **kein** Systemfenster, sondern eine Fläche über dem Popup: ein
 * eigenes Fenster nähme dem Popup den Fokus, und der Fokusverlust-Behandler
 * blendet es aus (siehe D37 und D43). Für einen Systemdialog braucht es die
 * Sperre im Backend; für zwei Eingabefelder ist das der falsche Aufwand.
 *
 * Der Kommentar kommt **vorbelegt** aus dem Backend — dort steht die Vorlage
 * samt Platzhaltern, und sie ist in den Einstellungen pflegbar.
 */

import { useEffect, useRef, useState } from "react";
import { Check, Wrench, X } from "lucide-react";

import { Button, Callout } from "@/components";
import { actionComment, acknowledge, asCommandError, setDowntime } from "@/lib/api";
import { STATUS } from "@/lib/status";
import { statusKeyOf } from "./grouping";
import { t } from "@/i18n";
import type { DowntimeChoice, Problem, WriteAction } from "@/lib/types";

interface ActionDialogProps {
  action: WriteAction;
  problem: Problem;
  onClose: () => void;
  /** Nach Erfolg: der Aufrufer schliesst und meldet. */
  onDone: (message: string) => void;
}

/**
 * Die angebotenen Dauern.
 *
 * Dieselben, die `DowntimeDuration` in `checkmk/write.rs` kennt — die Liste hier
 * nur um Beschriftungen erweitert. Eine fünfte Möglichkeit im Frontend, die das
 * Backend nicht kennt, wäre ein Laufzeitfehler.
 */
const DAUERN: readonly { value: DowntimeChoice; label: string }[] = [
  { value: "minutes15", label: t("downtime.minutes15") },
  { value: "hour1", label: t("downtime.hour1") },
  { value: "hours4", label: t("downtime.hours4") },
  { value: "untilMorning", label: t("downtime.untilMorning") },
  { value: "custom", label: t("downtime.custom") },
];

/** Vorgabe für die freie Angabe. Etwas, das man selten ändern muss. */
const CUSTOM_DEFAULT_MINUTES = 30;

export function ActionDialog({ action, problem, onClose, onDone }: ActionDialogProps) {
  const [comment, setComment] = useState("");
  const [duration, setDuration] = useState<DowntimeChoice>("hour1");
  const [minutes, setMinutes] = useState(CUSTOM_DEFAULT_MINUTES);
  const [running, setRunning] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const feld = useRef<HTMLTextAreaElement>(null);

  // Vorbelegung holen. Scheitert das, bleibt das Feld leer und der Benutzer
  // schreibt selbst — das ist besser als den Dialog nicht zu öffnen.
  useEffect(() => {
    let verworfen = false;
    actionComment(action, problem.host, problem.service)
      .then((text) => {
        if (!verworfen) setComment(text);
      })
      .catch(() => undefined);
    return () => {
      verworfen = true;
    };
  }, [action, problem.host, problem.service]);

  // Fokus ins Kommentarfeld, damit man sofort tippen kann.
  useEffect(() => {
    feld.current?.focus();
  }, []);

  const meta = STATUS[statusKeyOf(problem.state)];
  const titel =
    action === "acknowledge" ? t("action.acknowledge") : t("action.downtime");
  const Icon = action === "acknowledge" ? Check : Wrench;

  function ausfuehren() {
    setError(null);
    setRunning(true);
    const laufend =
      action === "acknowledge"
        ? acknowledge(problem.host, problem.service, comment)
        : setDowntime(
            problem.host,
            problem.service,
            comment,
            duration,
            duration === "custom" ? minutes : null,
          );
    laufend
      .then(() => onDone(titel))
      .catch((raw: unknown) => {
        setError(asCommandError(raw).message);
        setRunning(false);
      });
  }

  return (
    // Die Fläche fängt Klicks daneben ab und schliesst. Kein `onKeyDown` hier,
    // sondern auf dem Kasten: dort landen die Tastendrücke der Eingabefelder.
    <div
      role="presentation"
      onClick={onClose}
      className="absolute inset-0 z-10 flex items-center justify-center bg-scrim p-6"
    >
      <div
        role="dialog"
        aria-modal="true"
        aria-label={titel}
        onClick={(event) => event.stopPropagation()}
        onKeyDown={(event) => {
          if (event.key === "Escape") onClose();
          // Strg+Enter statt Enter allein: im Kommentarfeld ist Enter ein
          // Zeilenumbruch, und eine Schreibaktion soll kein Versehen sein.
          if (event.key === "Enter" && (event.ctrlKey || event.metaKey)) {
            if (!running) ausfuehren();
          }
        }}
        className="flex w-full max-w-lg flex-col gap-4 rounded-lg border border-line-strong bg-card p-5 shadow-card"
      >
        <header className="flex items-start justify-between gap-4">
          <div className="flex min-w-0 flex-col gap-1">
            <h2 className="flex items-center gap-cgap-md text-h3 font-display font-bold text-body">
              <Icon size={18} aria-hidden />
              {titel}
            </h2>
            <p className="selectable min-w-0 truncate font-mono text-mono-sm text-muted">
              <span className={meta.fg}>{meta.short}</span> {problem.host}
              {problem.service ? ` · ${problem.service}` : ` · ${t("detail.hostProblem")}`}
            </p>
          </div>
          <button
            type="button"
            aria-label={t("action.cancel")}
            onClick={onClose}
            className="flex size-control-sm shrink-0 items-center justify-center rounded-md text-muted transition-colors duration-fast ease-out press hover:bg-sunken hover:text-body"
          >
            <X size={16} aria-hidden />
          </button>
        </header>

        {action === "downtime" ? (
          <div className="flex flex-col gap-2">
            <span className="text-base font-bold text-body">
              {t("downtime.duration")}
            </span>
            <div role="group" aria-label={t("downtime.duration")} className="flex flex-wrap gap-1">
              {DAUERN.map((option) => (
                <button
                  key={option.value}
                  type="button"
                  aria-pressed={duration === option.value}
                  onClick={() => setDuration(option.value)}
                  className={[
                    "inline-flex h-control-sm items-center rounded-md border px-cpx-sm text-sm font-bold",
                    "transition-colors duration-fast ease-out press",
                    duration === option.value
                      ? "border-accent-solid bg-accent-solid text-on-accent"
                      : "border-line bg-card text-muted hover:bg-sunken",
                  ].join(" ")}
                >
                  {option.label}
                </button>
              ))}
            </div>
            {duration === "custom" ? (
              <label className="mt-1 flex items-center gap-cgap-md">
                <input
                  type="number"
                  min={1}
                  value={minutes}
                  onChange={(event) => setMinutes(Number(event.target.value))}
                  className="h-control-sm w-24 rounded-md border border-line-strong bg-card px-cpx-sm font-mono text-mono-sm text-body focus:outline-none focus-visible:ring-input"
                />
                <span className="text-sm text-muted">{t("downtime.minutes")}</span>
              </label>
            ) : null}
          </div>
        ) : null}

        <label className="flex flex-col gap-2">
          <span className="text-base font-bold text-body">{t("action.comment")}</span>
          <textarea
            ref={feld}
            rows={3}
            value={comment}
            onChange={(event) => setComment(event.target.value)}
            className="w-full resize-none rounded-md border border-line-strong bg-page p-3 text-base text-body focus:outline-none focus-visible:border-accent-solid focus-visible:ring-input"
          />
          <span className="text-sm text-muted">{t("action.commentHint")}</span>
        </label>

        {error ? (
          <Callout tone="crit" title={t("action.failed")}>
            <p className="selectable">{error}</p>
          </Callout>
        ) : null}

        <footer className="flex items-center justify-end gap-cgap-md">
          <Button variant="ghost" size="sm" onClick={onClose} disabled={running}>
            {t("action.cancel")}
          </Button>
          <Button
            variant="primary"
            size="sm"
            iconLeft={Icon}
            onClick={ausfuehren}
            // Ein leerer Kommentar wird vom Backend abgelehnt. Den Knopf zu
            // sperren sagt das vorher, statt es als Fehler nachzuliefern.
            disabled={running || comment.trim().length === 0}
          >
            {running ? t("action.running") : titel}
          </Button>
        </footer>
      </div>
    </div>
  );
}
