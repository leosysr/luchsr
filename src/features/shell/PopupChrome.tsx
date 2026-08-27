/**
 * Titelzeile des rahmenlosen Fensters.
 *
 * Ein Fenster ohne Dekoration hat keinen Schliessknopf und lässt sich nicht
 * verschieben — beides muss die Anwendung selbst mitbringen. `data-tauri-drag-region`
 * macht eine Fläche zur Ziehfläche; darin liegende Knöpfe brauchen die Angabe
 * nicht, sonst wären sie nicht klickbar.
 *
 * Die Anheftung ist hier und nicht nur in den Einstellungen: sie ist die
 * Entscheidung „bleib offen, ich arbeite gerade damit", und die trifft man in
 * dem Moment, nicht vorher in einem Dialog.
 */

import { Pin, PinOff, RefreshCw, Settings2, X } from "lucide-react";

import { STATUS } from "@/lib/status";
import type { StatusKey } from "@/lib/status";
import { statusLabel, t } from "@/i18n";
import type { StatusPayload } from "@/lib/types";

/** Backend-Kürzel auf den Anzeigeschlüssel. `GETRENNT` hat keinen. */
function statusKeyFor(trayState: string): StatusKey | null {
  switch (trayState) {
    case "OK":
      return "ok";
    case "WARN":
      return "warn";
    case "CRIT":
      return "crit";
    case "DOWN":
      return "down";
    case "UNKNOWN":
      return "unknown";
    default:
      return null;
  }
}

interface PopupChromeProps {
  status: StatusPayload | null;
  pinned: boolean;
  onTogglePin: () => void;
  onRefresh: () => void;
  refreshing: boolean;
  onOpenSettings: () => void;
  onClose: () => void;
  /** Der Reiter, auf dem man gerade ist — der Knopf führt zurück. */
  showingSettings: boolean;
}

export function PopupChrome({
  status,
  pinned,
  onTogglePin,
  onRefresh,
  refreshing,
  onOpenSettings,
  onClose,
  showingSettings,
}: PopupChromeProps) {
  const key = status ? statusKeyFor(status.trayState) : null;
  const meta = key ? STATUS[key] : null;
  const Icon = meta?.icon;
  const PinIcon = pinned ? Pin : PinOff;

  return (
    <header
      data-tauri-drag-region
      className="flex h-control-lg shrink-0 items-center justify-between gap-4 border-b border-line bg-page px-row-x select-none"
    >
      <div className="flex min-w-0 items-center gap-cgap-md" data-tauri-drag-region>
        <span
          className={[
            "flex shrink-0 items-center gap-1 rounded-sm px-badge-x py-badge-y",
            "font-mono text-mono-xs font-semibold tracking-badge uppercase",
            meta ? `${meta.soft} ${meta.fg}` : "bg-state-stale-soft text-state-stale",
          ].join(" ")}
        >
          {Icon ? <Icon size={12} aria-hidden /> : null}
          {status?.trayState ?? "—"}
        </span>
        <p className="min-w-0 truncate text-sm text-muted" data-tauri-drag-region>
          {status?.tooltip?.replace(/^Luchsr — /, "") ??
            (key ? statusLabel(key) : t("status.waiting"))}
        </p>
      </div>

      <div className="flex shrink-0 items-center gap-1">
        <ChromeButton
          icon={RefreshCw}
          label={t("tray.refresh")}
          onClick={onRefresh}
          spinning={refreshing}
        />
        <ChromeButton
          icon={PinIcon}
          label={pinned ? t("popup.unpin") : t("popup.pin")}
          onClick={onTogglePin}
          active={pinned}
        />
        <ChromeButton
          icon={Settings2}
          label={showingSettings ? t("popup.backToList") : t("tray.settings")}
          onClick={onOpenSettings}
          active={showingSettings}
        />
        <ChromeButton icon={X} label={t("popup.close")} onClick={onClose} />
      </div>
    </header>
  );
}

interface ChromeButtonProps {
  icon: React.ComponentType<{ size?: number; "aria-hidden"?: boolean }>;
  label: string;
  onClick: () => void;
  active?: boolean;
  spinning?: boolean;
}

function ChromeButton({
  icon: Icon,
  label,
  onClick,
  active = false,
  spinning = false,
}: ChromeButtonProps) {
  return (
    <button
      type="button"
      aria-label={label}
      aria-pressed={active || undefined}
      title={label}
      onClick={onClick}
      className={[
        "flex size-control-sm items-center justify-center rounded-md",
        "transition-colors duration-fast ease-out press",
        active
          ? "bg-accent-soft text-accent"
          : "text-muted hover:bg-sunken hover:text-body",
      ].join(" ")}
    >
      <span className={spinning ? "animate-spin" : undefined}>
        <Icon size={16} aria-hidden />
      </span>
    </button>
  );
}
