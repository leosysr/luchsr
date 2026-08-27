/**
 * Einstellungsdialog und Ersteinrichtungs-Assistent.
 *
 * Beides dieselbe Komponente in zwei Modi: die Ersteinrichtung zeigt nur den
 * Verbindungsteil und führt zum Verbindungstest, die Einstellungen zeigen
 * alles. Ein getrennter Assistent hätte dieselben Felder mit derselben Prüfung
 * doppelt gebraucht.
 *
 * ## Wie das Secret behandelt wird
 *
 * Es liegt **nicht** im Einstellungszustand. Es hat sein eigenes, kurzlebiges
 * Feld, das beim Speichern geleert wird. Ob eines gespeichert ist, sagt das
 * Backend als Wahrheitswert. Damit gibt es keinen Ort im Frontend, an dem das
 * Secret länger als bis zum nächsten Speichern liegt.
 *
 * ## Woher die Prüfung kommt
 *
 * Aus dem Backend, nicht aus dem Frontend. Die URL-Prüfung steckt im
 * `checkmk`-Modul und ist dort getestet; sie hier nachzubauen hiesse, zwei
 * Wahrheiten zu pflegen. Der Aufruf ist entprellt, damit nicht jeder Tastendruck
 * über die Brücke geht.
 */

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  KeyRound,
  Moon,
  Monitor,
  Plug,
  RotateCcw,
  Save,
  Sun,
  Trash2,
} from "lucide-react";

import {
  Button,
  Callout,
  Card,
  Checkbox,
  Field,
  Input,
  NumberInput,
  Segmented,
  Select,
  Switch,
} from "@/components";
import {
  asCommandError,
  builtinSounds,
  connectionTest,
  credentialStoreAvailable,
  secretDelete,
  secretExists,
  secretSet,
  settingsSave,
  settingsValidate,
} from "@/lib/api";
import { applyTheme } from "@/lib/theme";
import { t } from "@/i18n";
import { SoundPicker } from "./SoundPicker";
import type {
  BuiltinSoundInfo,
  CommandError,
  Connection,
  ConnectionReport,
  LoadOutcome,
  NotificationLevel,
  SoundSettings,
  ProxyConfig,
  ProxyMode,
  Settings,
  ThemeMode,
  ValidationIssue,
} from "@/lib/types";
import {
  INTERVAL_MAX_SECONDS,
  INTERVAL_MIN_SECONDS,
  TIMEOUT_MAX_SECONDS,
  TIMEOUT_MIN_SECONDS,
} from "@/lib/types";

type Mode = "setup" | "settings";

/**
 * Die Ereignisse mit eigenem Klang, in der Reihenfolge der Anzeige.
 *
 * Muss zu `SoundSettings` in `schema.rs` passen — `keyof SoundSettings`
 * erzwingt das: ein Tippfehler im Namen ist ein Compilerfehler, und ein
 * fehlendes Ereignis fällt beim Vergleich mit dem Typ auf.
 */
const SOUND_EVENTS: readonly {
  key: keyof SoundSettings;
  label: string;
  hint?: string;
}[] = [
  {
    key: "critical",
    label: t("settings.sound.critical"),
    hint: t("settings.sound.criticalHint"),
  },
  {
    key: "warning",
    label: t("settings.sound.warning"),
    hint: t("settings.sound.warningHint"),
  },
  { key: "recovery", label: t("settings.sound.recovery") },
  { key: "acknowledged", label: t("settings.sound.acknowledged") },
  { key: "downtime", label: t("settings.sound.downtime") },
];

interface SettingsViewProps {
  mode: Mode;
  initial: LoadOutcome;
  /** Wird nach erfolgreichem Speichern gerufen. */
  onSaved: (settings: Settings) => void;
}

/* -------------------------------------------------------------------------- */

export function SettingsView({ mode, initial, onSaved }: SettingsViewProps) {
  const [draft, setDraft] = useState<Settings>(initial.settings);
  const [issues, setIssues] = useState<ValidationIssue[]>([]);
  const [dirty, setDirty] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveError, setSaveError] = useState<CommandError | null>(null);
  const [justSaved, setJustSaved] = useState(false);

  // Secret: eigener, kurzlebiger Zustand. Nie Teil von `draft`.
  const [secretInput, setSecretInput] = useState("");
  const [secretStored, setSecretStored] = useState(false);
  const [storeAvailable, setStoreAvailable] = useState(true);
  const savedUsername = useRef(initial.settings.connections[initial.settings.activeConnection]?.username ?? "");

  const [testing, setTesting] = useState(false);
  const [report, setReport] = useState<ConnectionReport | null>(null);
  const [testError, setTestError] = useState<CommandError | null>(null);

  // Die eingebauten Klänge kommen aus dem Backend, wo sie im Programm liegen.
  // Scheitert der Abruf, bleiben die Auswahlfelder bei „kein Ton" — dann fehlt
  // eine Wahlmöglichkeit, aber der Dialog funktioniert.
  const [builtins, setBuiltins] = useState<BuiltinSoundInfo[]>([]);
  useEffect(() => {
    builtinSounds()
      .then(setBuiltins)
      .catch(() => undefined);
  }, []);

  const connection: Connection =
    draft.connections[draft.activeConnection] ?? draft.connections[0]!;

  /* ------------------------------------------------------------ Änderungen */

  const patch = useCallback((change: Partial<Settings>) => {
    setDraft((current) => ({ ...current, ...change }));
    setDirty(true);
    setJustSaved(false);
  }, []);

  const patchConnection = useCallback(
    (change: Partial<Connection>) => {
      setDraft((current) => {
        const connections = [...current.connections];
        const index = current.activeConnection;
        connections[index] = { ...connections[index]!, ...change };
        return { ...current, connections };
      });
      setDirty(true);
      setJustSaved(false);
      // Eine geänderte Verbindung macht ein altes Testergebnis wertlos.
      setReport(null);
      setTestError(null);
    },
    [],
  );

  /* -------------------------------------------------------------- Prüfung */

  useEffect(() => {
    const timer = window.setTimeout(() => {
      settingsValidate(draft)
        .then(setIssues)
        .catch((error: unknown) => {
          // Die Prüfung darf den Dialog nicht lähmen; im Zweifel keine Meldung.
          console.error("Prüfung fehlgeschlagen", asCommandError(error));
        });
    }, 250);
    return () => window.clearTimeout(timer);
  }, [draft]);

  /* --------------------------------------- Credential Manager und Secret */

  useEffect(() => {
    credentialStoreAvailable()
      .then(() => setStoreAvailable(true))
      .catch(() => setStoreAvailable(false));
  }, []);

  useEffect(() => {
    const username = connection.username.trim();
    if (!username) {
      setSecretStored(false);
      return;
    }
    let aktuell = true;
    secretExists(username)
      .then((exists) => {
        if (aktuell) setSecretStored(exists);
      })
      .catch(() => {
        if (aktuell) setSecretStored(false);
      });
    return () => {
      aktuell = false;
    };
  }, [connection.username]);

  /* ------------------------------------------------------- Theme sofort */

  // Der Farbmodus wirkt beim Auswählen, nicht erst beim Speichern — sonst
  // wählt man blind.
  useEffect(() => {
    applyTheme(draft.appearance.theme);
  }, [draft.appearance.theme]);

  /* ------------------------------------------------------------ Speichern */

  async function handleSave() {
    setSaving(true);
    setSaveError(null);
    try {
      const username = connection.username.trim();
      // Secret zuerst: wenn das scheitert, sollen die Einstellungen nicht so
      // aussehen, als sei alles gespeichert.
      if (secretInput && username) {
        await secretSet(username, secretInput);
        setSecretInput("");
        setSecretStored(true);
      }
      const gespeichert = await settingsSave(draft);
      setDraft(gespeichert);
      savedUsername.current = username;
      setDirty(false);
      setJustSaved(true);
      onSaved(gespeichert);
    } catch (error: unknown) {
      setSaveError(asCommandError(error));
    } finally {
      setSaving(false);
    }
  }

  async function handleDeleteSecret() {
    const username = connection.username.trim();
    if (!username) return;
    try {
      await secretDelete(username);
      setSecretStored(false);
      setSecretInput("");
    } catch (error: unknown) {
      setSaveError(asCommandError(error));
    }
  }

  async function handleTest() {
    setTesting(true);
    setReport(null);
    setTestError(null);
    try {
      const ergebnis = await connectionTest(
        connection,
        secretInput || null,
        draft.polling.timeoutSeconds,
      );
      setReport(ergebnis);
    } catch (error: unknown) {
      setTestError(asCommandError(error));
    } finally {
      setTesting(false);
    }
  }

  /* ------------------------------------------------------------- Ableitung */

  const errorFor = useMemo(() => {
    const map = new Map<string, string>();
    for (const issue of issues) {
      if (issue.severity === "error" && !map.has(issue.field)) {
        map.set(issue.field, issue.message);
      }
    }
    return map;
  }, [issues]);

  const warningFor = useMemo(() => {
    const map = new Map<string, string>();
    for (const issue of issues) {
      if (issue.severity === "warning" && !map.has(issue.field)) {
        map.set(issue.field, issue.message);
      }
    }
    return map;
  }, [issues]);

  const blocking = issues.some((issue) => issue.severity === "error");
  const usernameChanged =
    connection.username.trim() !== savedUsername.current.trim();
  const secretMissing = !secretStored && !secretInput;

  const proxyMode: ProxyMode = connection.proxy.mode;

  /* ------------------------------------------------------------- Darstellung */

  return (
    <div className="mx-auto flex w-full max-w-3xl flex-col gap-6 p-8">
      <header className="flex items-end justify-between gap-6 border-b border-line pb-5">
        <div className="flex flex-col gap-1">
          <p className="font-mono text-mono-xs font-semibold tracking-kicker text-faint uppercase">
            {t("app.name")}
          </p>
          <h1 className="text-h2 font-display font-extrabold tracking-display text-body">
            {mode === "setup" ? t("setup.title") : t("settings.title")}
          </h1>
        </div>
      </header>

      {mode === "setup" ? (
        <Callout tone="info" title={t("setup.title")}>
          <p>{t("setup.intro")}</p>
        </Callout>
      ) : null}

      {initial.notices.map((notice) => (
        <Callout key={notice} tone="warn" title={t("settings.notice")}>
          <p>{notice}</p>
        </Callout>
      ))}

      {!storeAvailable ? (
        <Callout tone="crit" title={t("settings.secret.storeUnavailable")} />
      ) : null}

      {/* =============================================== Verbindung ======= */}
      <Card
        kicker={t("settings.connection.kicker")}
        title={t("settings.connection.section")}
      >
        <div className="flex flex-col gap-5">
          <Field
            label={t("settings.server.label")}
            hint={t("settings.server.hint")}
            error={errorFor.get("connection.server")}
            htmlFor="server"
          >
            <Input
              id="server"
              mono
              value={connection.server}
              placeholder={t("settings.server.placeholder")}
              invalid={errorFor.has("connection.server")}
              onChange={(event) => patchConnection({ server: event.target.value })}
            />
          </Field>

          <div className="grid grid-cols-2 gap-5">
            <Field
              label={t("settings.site.label")}
              hint={t("settings.site.hint")}
              error={errorFor.get("connection.site")}
              htmlFor="site"
            >
              <Input
                id="site"
                mono
                value={connection.site}
                placeholder={t("settings.site.placeholder")}
                invalid={errorFor.has("connection.site")}
                onChange={(event) => patchConnection({ site: event.target.value })}
              />
            </Field>

            <Field
              label={t("settings.username.label")}
              hint={t("settings.username.hint")}
              error={errorFor.get("connection.username")}
              htmlFor="username"
            >
              <Input
                id="username"
                mono
                autoComplete="off"
                value={connection.username}
                placeholder={t("settings.username.placeholder")}
                invalid={errorFor.has("connection.username")}
                onChange={(event) => patchConnection({ username: event.target.value })}
              />
            </Field>
          </div>

          {/* ------------------------------------------------- Secret ----- */}
          <Field
            label={t("settings.secret.label")}
            hint={mode === "setup" ? t("setup.secretHint") : t("settings.secret.hint")}
            warning={
              usernameChanged && secretMissing
                ? t("settings.secret.movedUser")
                : undefined
            }
            htmlFor="secret"
          >
            <div className="flex items-center gap-cgap-md">
              <div className="min-w-0 flex-1">
                <Input
                  id="secret"
                  type="password"
                  autoComplete="new-password"
                  mono
                  value={secretInput}
                  disabled={!storeAvailable}
                  placeholder={
                    secretStored
                      ? t("settings.secret.placeholderStored")
                      : t("settings.secret.placeholderEmpty")
                  }
                  onChange={(event) => {
                    setSecretInput(event.target.value);
                    setDirty(true);
                    setJustSaved(false);
                    setReport(null);
                    setTestError(null);
                  }}
                />
              </div>
              <span
                className={[
                  "inline-flex shrink-0 items-center gap-1 rounded-sm px-badge-x py-badge-y",
                  "font-mono text-mono-xs font-semibold tracking-badge uppercase",
                  secretStored
                    ? "bg-state-ok-soft text-state-ok"
                    : "bg-state-warn-soft text-state-warn",
                ].join(" ")}
              >
                <KeyRound size={12} aria-hidden />
                {secretStored ? t("settings.secret.stored") : t("settings.secret.missing")}
              </span>
              {secretStored ? (
                <Button
                  size="sm"
                  variant="ghost"
                  iconLeft={Trash2}
                  onClick={handleDeleteSecret}
                >
                  {t("settings.secret.delete")}
                </Button>
              ) : null}
            </div>
          </Field>

          {/* ---------------------------------------------------- TLS ----- */}
          <Field
            label={t("settings.verifyTls.label")}
            hint={t("settings.verifyTls.hint")}
            htmlFor="verifyTls"
            inline
          >
            <Switch
              id="verifyTls"
              checked={connection.verifyTls}
              label={t("settings.verifyTls.label")}
              onCheckedChange={(checked) => patchConnection({ verifyTls: checked })}
            />
          </Field>

          {warningFor.has("connection.verifyTls") ? (
            <Callout tone="warn" title={t("settings.verifyTls.warningTitle")}>
              <p>{warningFor.get("connection.verifyTls")}</p>
            </Callout>
          ) : null}

          {/* -------------------------------------------------- Proxy ----- */}
          <Field label={t("settings.proxy.label")} inline>
            <Segmented<ProxyMode>
              label={t("settings.proxy.label")}
              value={proxyMode}
              options={[
                { value: "system", label: t("settings.proxy.system") },
                { value: "none", label: t("settings.proxy.none") },
                { value: "manual", label: t("settings.proxy.manual") },
              ]}
              onValueChange={(next) => {
                const proxy: ProxyConfig =
                  next === "manual"
                    ? {
                        mode: "manual",
                        url:
                          connection.proxy.mode === "manual"
                            ? connection.proxy.url
                            : "",
                      }
                    : { mode: next };
                patchConnection({ proxy });
              }}
            />
          </Field>

          {warningFor.has("connection.proxy") ? (
            <Callout
              tone="warn"
              title={t("settings.proxy.warningTitle")}
              actions={
                <Button
                  size="sm"
                  variant="secondary"
                  iconLeft={Plug}
                  onClick={() => patchConnection({ proxy: { mode: "none" } })}
                >
                  {t("settings.proxy.useNone")}
                </Button>
              }
            >
              <p>{warningFor.get("connection.proxy")}</p>
            </Callout>
          ) : null}

          {connection.proxy.mode === "manual" ? (
            <Field
              label={t("settings.proxy.urlLabel")}
              hint={t("settings.proxy.urlHint")}
              error={errorFor.get("connection.proxy")}
              htmlFor="proxyUrl"
            >
              <Input
                id="proxyUrl"
                mono
                value={connection.proxy.url}
                placeholder={t("settings.proxy.urlPlaceholder")}
                invalid={errorFor.has("connection.proxy")}
                onChange={(event) =>
                  patchConnection({ proxy: { mode: "manual", url: event.target.value } })
                }
              />
            </Field>
          ) : null}

          {/* --------------------------------------------------- Test ----- */}
          <div className="flex flex-col gap-4 border-t border-line pt-5">
            <div className="flex items-center gap-cgap-md">
              <Button
                variant="secondary"
                iconLeft={Plug}
                busy={testing}
                disabled={!connection.server || !connection.site || !connection.username}
                onClick={handleTest}
              >
                {testing ? t("settings.test.running") : t("settings.test.button")}
              </Button>
            </div>

            {report ? <TestSuccess report={report} /> : null}
            {testError ? <TestFailure error={testError} /> : null}
          </div>
        </div>
      </Card>

      {mode === "settings" ? (
        <>
          {/* ============================================== Abruf ========= */}
          <Card
            kicker={t("settings.polling.kicker")}
            title={t("settings.polling.section")}
          >
            <div className="grid grid-cols-2 gap-5">
              <Field
                label={t("settings.interval.label")}
                hint={t("settings.interval.hint")}
                error={errorFor.get("polling.intervalSeconds")}
                htmlFor="interval"
              >
                <NumberInput
                  id="interval"
                  value={draft.polling.intervalSeconds}
                  min={INTERVAL_MIN_SECONDS}
                  max={INTERVAL_MAX_SECONDS}
                  unit={t("settings.unit.seconds")}
                  invalid={errorFor.has("polling.intervalSeconds")}
                  onValueChange={(intervalSeconds) =>
                    patch({ polling: { ...draft.polling, intervalSeconds } })
                  }
                />
              </Field>

              <Field
                label={t("settings.timeout.label")}
                hint={t("settings.timeout.hint")}
                warning={warningFor.get("polling.timeoutSeconds")}
                htmlFor="timeout"
              >
                <NumberInput
                  id="timeout"
                  value={draft.polling.timeoutSeconds}
                  min={TIMEOUT_MIN_SECONDS}
                  max={TIMEOUT_MAX_SECONDS}
                  unit={t("settings.unit.seconds")}
                  onValueChange={(timeoutSeconds) =>
                    patch({ polling: { ...draft.polling, timeoutSeconds } })
                  }
                />
              </Field>
            </div>
          </Card>

          {/* ========================================= Darstellung ======== */}
          <Card
            kicker={t("settings.appearance.kicker")}
            title={t("settings.appearance.section")}
          >
            <div className="flex flex-col gap-5">
              <Field label={t("settings.theme.label")} inline>
                <Segmented<ThemeMode>
                  label={t("settings.theme.label")}
                  value={draft.appearance.theme}
                  options={[
                    { value: "system", label: t("settings.theme.system"), icon: Monitor },
                    { value: "light", label: t("settings.theme.light"), icon: Sun },
                    { value: "dark", label: t("settings.theme.dark"), icon: Moon },
                  ]}
                  onValueChange={(theme) =>
                    patch({ appearance: { ...draft.appearance, theme } })
                  }
                />
              </Field>

              <Field
                label={t("settings.language.label")}
                hint={t("settings.language.hint")}
                htmlFor="language"
              >
                <Select
                  id="language"
                  value={draft.appearance.language}
                  options={[{ value: "de", label: "Deutsch" }]}
                  onValueChange={(language) =>
                    patch({ appearance: { ...draft.appearance, language } })
                  }
                />
              </Field>
            </div>
          </Card>

          {/* ============================================ Verhalten ======= */}
          <Card
            kicker={t("settings.behaviour.kicker")}
            title={t("settings.behaviour.section")}
          >
            <div className="flex flex-col gap-5">
              {(
                [
                  ["autostart", "settings.autostart.label", "settings.autostart.hint"],
                  ["startMinimised", "settings.startMinimised.label", "settings.startMinimised.hint"],
                  ["pinPopup", "settings.pinPopup.label", "settings.pinPopup.hint"],
                  ["hideHandled", "settings.hideHandled.label", "settings.hideHandled.hint"],
                ] as const
              ).map(([key, labelKey, hintKey]) => (
                <Field
                  key={key}
                  label={t(labelKey)}
                  hint={t(hintKey)}
                  htmlFor={key}
                  inline
                >
                  <Switch
                    id={key}
                    checked={draft.behaviour[key]}
                    label={t(labelKey)}
                    onCheckedChange={(checked) =>
                      patch({ behaviour: { ...draft.behaviour, [key]: checked } })
                    }
                  />
                </Field>
              ))}
            </div>
          </Card>

          {/* ==================================== Benachrichtigungen ====== */}
          <Card
            kicker={t("settings.notifications.kicker")}
            title={t("settings.notifications.section")}
          >
            <div className="flex flex-col gap-5">
              <Field label={t("settings.notificationLevel.label")} htmlFor="notifyLevel">
                <Select<NotificationLevel>
                  id="notifyLevel"
                  value={draft.notifications.level}
                  options={[
                    { value: "off", label: t("settings.notificationLevel.off") },
                    {
                      value: "criticalOnly",
                      label: t("settings.notificationLevel.criticalOnly"),
                    },
                    {
                      value: "allChanges",
                      label: t("settings.notificationLevel.allChanges"),
                    },
                  ]}
                  onValueChange={(level) =>
                    patch({ notifications: { ...draft.notifications, level } })
                  }
                />
              </Field>

              <div className="flex flex-col gap-5 border-t border-line pt-5">
                <p className="text-sm text-muted">{t("settings.sound.intro")}</p>
                {SOUND_EVENTS.map(({ key, label, hint }) => (
                  <SoundPicker
                    key={key}
                    id={`sound-${key}`}
                    label={label}
                    hint={hint}
                    value={draft.notifications.sounds[key]}
                    builtins={builtins}
                    warning={warningFor.get(`notifications.sounds.${key}`)}
                    onChange={(choice) =>
                      patch({
                        notifications: {
                          ...draft.notifications,
                          sounds: { ...draft.notifications.sounds, [key]: choice },
                        },
                      })
                    }
                  />
                ))}
              </div>
            </div>
          </Card>

          {/* ========================================= Schreibaktionen ==== */}
          <Card
            kicker={t("settings.permissions.kicker")}
            title={t("settings.permissions.section")}
          >
            <div className="flex flex-col gap-5">
              <p className="text-sm text-muted">{t("settings.permissions.intro")}</p>
              <Checkbox
                checked={draft.permissions.allowAcknowledge}
                label={t("settings.allowAcknowledge.label")}
                hint={t("settings.allowAcknowledge.hint")}
                onCheckedChange={(allowAcknowledge) =>
                  patch({ permissions: { ...draft.permissions, allowAcknowledge } })
                }
              />
              <Checkbox
                checked={draft.permissions.allowDowntime}
                label={t("settings.allowDowntime.label")}
                hint={t("settings.allowDowntime.hint")}
                onCheckedChange={(allowDowntime) =>
                  patch({ permissions: { ...draft.permissions, allowDowntime } })
                }
              />

              {/* Die Vorlagen erscheinen nur, wenn mindestens eine Aktion
                  freigegeben ist — sonst stehen dort zwei Felder für etwas,
                  das gar nicht passieren kann. */}
              {draft.permissions.allowAcknowledge || draft.permissions.allowDowntime ? (
                <div className="flex flex-col gap-5 border-t border-line pt-5">
                  <p className="text-sm text-muted">
                    {t("settings.comment.intro")}{" "}
                    <span className="font-mono text-mono-sm text-body">
                      {"{host} {service} {user} {app}"}
                    </span>
                  </p>
                  {draft.permissions.allowAcknowledge ? (
                    <Field
                      label={t("settings.acknowledgeComment.label")}
                      hint={t("settings.acknowledgeComment.hint")}
                      htmlFor="ackComment"
                    >
                      <Input
                        id="ackComment"
                        mono
                        value={draft.permissions.acknowledgeComment}
                        onChange={(event) =>
                          patch({
                            permissions: {
                              ...draft.permissions,
                              acknowledgeComment: event.target.value,
                            },
                          })
                        }
                      />
                    </Field>
                  ) : null}
                  {draft.permissions.allowDowntime ? (
                    <Field
                      label={t("settings.downtimeComment.label")}
                      hint={t("settings.downtimeComment.hint")}
                      htmlFor="downtimeComment"
                    >
                      <Input
                        id="downtimeComment"
                        mono
                        value={draft.permissions.downtimeComment}
                        onChange={(event) =>
                          patch({
                            permissions: {
                              ...draft.permissions,
                              downtimeComment: event.target.value,
                            },
                          })
                        }
                      />
                    </Field>
                  ) : null}
                </div>
              ) : null}
            </div>
          </Card>
        </>
      ) : null}

      {/* ================================================== Fusszeile ==== */}
      {saveError ? (
        <Callout tone="crit" title={saveError.message}>
          {saveError.details.length ? (
            <details>
              <summary className="cursor-pointer text-sm text-muted">
                {t("action.details")}
              </summary>
              <pre className="selectable mt-2 overflow-x-auto rounded-sm bg-code-bg p-3 font-mono text-mono-xs text-code-text">
                {saveError.details.join("\n")}
              </pre>
            </details>
          ) : null}
        </Callout>
      ) : null}

      <footer className="sticky bottom-0 flex items-center justify-between gap-4 border-t border-line bg-page pt-5 pb-2">
        <p className="text-sm text-muted">
          {justSaved
            ? t("action.saved")
            : dirty
              ? t("settings.unsaved")
              : mode === "setup" && !report
                ? t("setup.testFirst")
                : ""}
        </p>
        <div className="flex gap-cgap-md">
          {mode === "settings" && dirty ? (
            <Button
              variant="ghost"
              iconLeft={RotateCcw}
              onClick={() => {
                setDraft(initial.settings);
                setSecretInput("");
                setDirty(false);
                setSaveError(null);
                setReport(null);
                setTestError(null);
              }}
            >
              {t("action.discard")}
            </Button>
          ) : null}
          <Button
            variant="primary"
            iconLeft={Save}
            busy={saving}
            disabled={blocking || (!dirty && mode === "settings")}
            onClick={handleSave}
          >
            {mode === "setup" ? t("action.continue") : t("action.save")}
          </Button>
        </div>
      </footer>
    </div>
  );
}

/* -------------------------------------------------------------------------- */
/* Testergebnis                                                               */
/* -------------------------------------------------------------------------- */

function TestSuccess({ report }: { report: ConnectionReport }) {
  const zeilen: [string, string][] = [
    ["HTTP", String(report.httpStatus)],
    ["CheckMK", report.checkmkVersion ?? "—"],
    ["Edition", report.editionLabel ?? report.edition ?? "—"],
    ["Site", report.site ?? "—"],
    ["Antwortzeit", `${report.elapsedMs} ms`],
  ];

  return (
    <Callout tone="ok" title={t("settings.test.successTitle")}>
      <dl className="grid grid-cols-[auto_1fr] gap-x-5 gap-y-1">
        {zeilen.map(([label, wert]) => (
          <div key={label} className="col-span-2 grid grid-cols-subgrid">
            <dt className="font-mono text-mono-xs text-muted uppercase">{label}</dt>
            <dd className="selectable font-mono text-mono-sm text-body">{wert}</dd>
          </div>
        ))}
      </dl>
      {report.tlsVerificationDisabled ? (
        <p className="mt-3 text-sm text-state-warn">
          {t("settings.verifyTls.warningTitle")}
        </p>
      ) : null}
    </Callout>
  );
}

function TestFailure({ error }: { error: CommandError }) {
  return (
    <Callout tone="crit" title={t("settings.test.failureTitle")}>
      <p className="selectable">{error.message}</p>
      {error.isTlsProblem ? (
        <p className="mt-3 text-sm text-muted">{t("settings.test.tlsHint")}</p>
      ) : null}
      {error.details.length ? (
        <details className="mt-3">
          <summary className="cursor-pointer text-sm text-muted">
            {t("action.details")}
          </summary>
          <pre className="selectable mt-2 overflow-x-auto rounded-sm bg-code-bg p-3 font-mono text-mono-xs text-code-text">
            {error.details.join("\n")}
          </pre>
        </details>
      ) : null}
    </Callout>
  );
}
