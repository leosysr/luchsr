//! Die Brücke zwischen Frontend und Backend.
//!
//! ## Was hier nie passiert
//!
//! **Kein Befehl gibt das Automation-Secret zurück.** Es geht nur in eine
//! Richtung: das Frontend schickt es beim Speichern hinein, das Backend legt es
//! im Credential Manager ab. Ob eines gespeichert ist, sagt
//! [`secret_exists`] als Wahrheitswert. Es gibt bewusst kein `secret_get`.
//!
//! Beim Verbindungstest darf das Frontend ein noch nicht gespeichertes Secret
//! mitgeben — sonst müsste man erst speichern, um testen zu können. Es wird
//! benutzt und verworfen, nie zurückgegeben.
//!
//! ## Fehlerform
//!
//! Befehle geben [`CommandError`] zurück statt eines nackten Strings. Der
//! Einstellungsdialog braucht mehr als einen Text: ob es ein Zertifikatsproblem
//! ist (dann zeigt er den Hinweis auf die TLS-Einstellung), ob ein erneuter
//! Versuch Sinn hat, und die technische Fehlerkette zum Aufklappen.

use std::sync::Mutex;
use std::time::Duration;

use serde::Serialize;
use tauri::{Manager, State};

use crate::actions::WriteAction;
use crate::checkmk::{
    AcknowledgeOptions, CheckmkClient, CheckmkError, ClientConfig, ConnectionReport,
    DowntimeDuration, DowntimeHostBody, DowntimeServiceBody, ProxyMode, Secret, Snapshot,
};
use crate::config::{
    ConfigError, ConfigStore, Connection, LoadOutcome, SecretError, SecretStore, Settings,
    ValidationIssue,
};
use crate::poll::{RefreshSignal, StatusPayload};

/* -------------------------------------------------------------------------- */
/* Fehler                                                                     */
/* -------------------------------------------------------------------------- */

/// Fehler in der Form, die das Frontend braucht.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommandError {
    /// Deutscher Klartext, direkt anzeigbar.
    pub message: String,
    /// Zertifikatsproblem. Das UI zeigt dann den Hinweis, dass das
    /// Stammzertifikat in den Windows-Zertifikatspeicher gehört — und weist
    /// auf die TLS-Einstellung als Notlösung hin.
    pub is_tls_problem: bool,
    /// Ob ein erneuter Versuch überhaupt Sinn hat.
    pub retryable: bool,
    /// Betroffene Feldpfade, wenn die Prüfung fehlschlug.
    pub fields: Vec<String>,
    /// Technische Fehlerkette zum Aufklappen. Enthält nie Zugangsdaten.
    pub details: Vec<String>,
}

impl CommandError {
    fn plain(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            is_tls_problem: false,
            retryable: false,
            fields: Vec::new(),
            details: Vec::new(),
        }
    }
}

impl From<CheckmkError> for CommandError {
    fn from(error: CheckmkError) -> Self {
        let (is_tls_problem, details) = match &error {
            CheckmkError::Transport { cause, chain } => (cause.is_tls(), chain.clone()),
            _ => (false, Vec::new()),
        };
        Self {
            message: error.to_string(),
            is_tls_problem,
            retryable: error.is_retryable(),
            fields: Vec::new(),
            details,
        }
    }
}

impl From<ConfigError> for CommandError {
    fn from(error: ConfigError) -> Self {
        let fields = match &error {
            ConfigError::Invalid { fields, .. } => fields.clone(),
            _ => Vec::new(),
        };
        Self {
            message: error.to_string(),
            is_tls_problem: false,
            retryable: false,
            fields,
            details: Vec::new(),
        }
    }
}

impl From<SecretError> for CommandError {
    fn from(error: SecretError) -> Self {
        Self::plain(error.to_string())
    }
}

type CommandResult<T> = Result<T, CommandError>;

/// Zeitfenster, in dem ein Tray-Klick als Fortsetzung des Fokusverlusts gilt.
///
/// Gemessen an der Praxis: der Fokuswechsel und der Klick liegen wenige
/// Millisekunden auseinander. 300 ms sind reichlich Reserve und immer noch
/// kürzer als jedes bewusste zweite Klicken.
const AUTO_HIDE_GRACE: Duration = Duration::from_millis(300);

/* -------------------------------------------------------------------------- */
/* Zustand                                                                   */
/* -------------------------------------------------------------------------- */

/// Zwischengespeicherter Abrufzustand.
///
/// Der Abzug bleibt bei einem Fehler stehen: die Liste soll nicht leer werden,
/// nur weil der Server kurz nicht antwortet. Dass die Daten alt sind, sagen
/// Tray-Icon und Tooltip.
#[derive(Debug, Default)]
struct StatusCache {
    snapshot: Option<Snapshot>,
    error: Option<String>,
    failures: u32,
    configured: bool,
}

/// Gemeinsamer Zustand der Anwendung.
///
/// Die Einstellungen liegen zwischengespeichert vor, weil die Abrufschleife bei
/// jedem Durchlauf das Intervall braucht und dafür nicht die Platte anfassen
/// soll.
///
/// Alle Sperren sind `std::sync::Mutex` und werden **nie über ein `await`
/// gehalten**. Die Abrufschleife liest die Konfiguration, gibt die Sperre frei,
/// wartet auf HTTP und schreibt danach das Ergebnis. Eine Tokio-Mutex wäre
/// hier nur schwerer zu lesen.
pub struct AppState {
    store: ConfigStore,
    settings: Mutex<Settings>,
    status: Mutex<StatusCache>,
    refresh: RefreshSignal,
    /// Wann das Fenster zuletzt **automatisch** wegen Fokusverlust verborgen
    /// wurde. Siehe [`Self::just_auto_hidden`].
    last_auto_hide: Mutex<Option<std::time::Instant>>,
    /// Ob gerade ein Systemdialog offen ist. Siehe [`Self::modal_open`].
    modal: std::sync::atomic::AtomicBool,
    /// Gedächtnis der Benachrichtigungen. `None` heisst **noch kein Abzug** —
    /// siehe [`Self::notified_snapshot`].
    notified: Mutex<Option<crate::notify::Notified>>,
}

/// Hält [`AppState::modal`] gesetzt, solange er lebt.
///
/// Als Wächter und nicht als Paar von Setzern, damit die Marke auch bei einem
/// frühen `return` oder einer Panik zurückgenommen wird. Bliebe sie stehen,
/// würde das Fenster nie wieder von selbst ausblenden.
pub struct ModalGuard<'a>(&'a std::sync::atomic::AtomicBool);

impl Drop for ModalGuard<'_> {
    fn drop(&mut self) {
        self.0.store(false, std::sync::atomic::Ordering::Release);
    }
}

impl AppState {
    /// Baut den Zustand beim Start und lädt die Einstellungen.
    pub fn initialise() -> Result<(Self, LoadOutcome), ConfigError> {
        let store = ConfigStore::from_environment()?;
        let outcome = store.load()?;
        let state = Self {
            store,
            settings: Mutex::new(outcome.settings.clone()),
            status: Mutex::new(StatusCache::default()),
            refresh: RefreshSignal::default(),
            last_auto_hide: Mutex::new(None),
            modal: std::sync::atomic::AtomicBool::new(false),
            notified: Mutex::new(None),
        };
        Ok((state, outcome))
    }

    /// Das Benachrichtigungsgedächtnis und ob dies der **erste** Abzug ist.
    ///
    /// Beides in einem Zugriff, weil beides dieselbe Frage beantwortet und aus
    /// derselben Sperre kommen muss. Als getrennte Flagge könnte sie vom
    /// Gedächtnis abweichen — und dann kämen beim Start entweder alle
    /// Meldungen auf einmal oder gar keine mehr.
    pub fn notified_snapshot(&self) -> (crate::notify::Notified, bool) {
        let slot = self
            .notified
            .lock()
            .expect("Benachrichtigungs-Mutex ist nicht vergiftet");
        match slot.as_ref() {
            Some(map) => (map.clone(), false),
            None => (crate::notify::Notified::new(), true),
        }
    }

    /// Ersetzt das Gedächtnis. Danach gilt der nächste Abruf nicht mehr als
    /// erster.
    pub fn replace_notified(&self, notified: crate::notify::Notified) {
        *self
            .notified
            .lock()
            .expect("Benachrichtigungs-Mutex ist nicht vergiftet") = Some(notified);
    }

    /// Sperrt das Ausblenden bei Fokusverlust, solange der Wächter lebt.
    ///
    /// ## Das Problem, das das löst
    ///
    /// Ein Systemdialog — der Speicherdialog des CSV-Exports — nimmt dem
    /// Popup den Fokus. Der Fokusverlust-Behandler verbirgt es daraufhin. Nach
    /// dem Speichern wäre das Fenster weg, und die Meldung mit dem Zielpfad
    /// hätte niemand gesehen. Beim Anheften ist das kein Thema, aber darauf
    /// darf sich der Export nicht verlassen.
    pub fn modal_open(&self) -> ModalGuard<'_> {
        self.modal.store(true, std::sync::atomic::Ordering::Release);
        ModalGuard(&self.modal)
    }

    /// Ob gerade ein Systemdialog offen ist.
    pub fn is_modal_open(&self) -> bool {
        self.modal.load(std::sync::atomic::Ordering::Acquire)
    }

    /// Merkt, dass das Fenster gerade wegen Fokusverlust verborgen wurde.
    pub fn note_auto_hide(&self) {
        *self
            .last_auto_hide
            .lock()
            .expect("Auto-Hide-Mutex ist nicht vergiftet") = Some(std::time::Instant::now());
    }

    /// Ob das Verbergen so kurz zurückliegt, dass ein Klick auf das Tray-Icon
    /// dieselbe Handlung ist.
    ///
    /// ## Das Problem, das das löst
    ///
    /// Ein Klick auf das Tray-Icon nimmt dem Fenster zuerst den Fokus. Der
    /// Fokusverlust-Behandler verbirgt es also, **bevor** der Klick ankommt.
    /// Danach sieht der Klick ein verborgenes Fenster und öffnet es wieder —
    /// das Fenster liesse sich per Icon nie schliessen.
    ///
    /// Deshalb dieses Zeitfenster: liegt das automatische Verbergen weniger als
    /// [`AUTO_HIDE_GRACE`] zurück, war es dieselbe Handlung, und der Klick tut
    /// nichts. Die Marke wird beim Abfragen verbraucht, damit ein späterer
    /// Klick wieder öffnet.
    pub fn just_auto_hidden(&self) -> bool {
        let mut slot = self
            .last_auto_hide
            .lock()
            .expect("Auto-Hide-Mutex ist nicht vergiftet");
        match *slot {
            Some(at) if at.elapsed() < AUTO_HIDE_GRACE => {
                *slot = None;
                true
            }
            _ => {
                *slot = None;
                false
            }
        }
    }

    /// Kopie der aktuellen Einstellungen.
    pub fn settings(&self) -> Settings {
        self.settings
            .lock()
            .expect("Einstellungs-Mutex ist nicht vergiftet")
            .clone()
    }

    pub fn store(&self) -> &ConfigStore {
        &self.store
    }

    /// Das Signal, mit dem die Abrufschleife geweckt wird.
    pub fn refresh_signal(&self) -> RefreshSignal {
        self.refresh.clone()
    }

    /// Der letzte Abzug, auch wenn der aktuelle Abruf gescheitert ist.
    pub fn snapshot(&self) -> Option<Snapshot> {
        self.status
            .lock()
            .expect("Status-Mutex ist nicht vergiftet")
            .snapshot
            .clone()
    }

    /// Schreibt das Ergebnis eines Abrufs.
    pub fn set_status(
        &self,
        snapshot: Option<Snapshot>,
        error: Option<String>,
        failures: u32,
        configured: bool,
    ) {
        let mut cache = self
            .status
            .lock()
            .expect("Status-Mutex ist nicht vergiftet");
        cache.snapshot = snapshot;
        cache.error = error;
        cache.failures = failures;
        cache.configured = configured;
    }

    /// Der Zustand in der Form, die das Fenster bekommt.
    ///
    /// Wird beim Öffnen gebraucht: das Fenster hat das letzte Ereignis nicht
    /// mitbekommen, wenn es vorher versteckt war.
    pub fn status_payload(&self) -> StatusPayload {
        let cache = self
            .status
            .lock()
            .expect("Status-Mutex ist nicht vergiftet");
        let include_handled = !self.settings().behaviour.hide_handled;
        let tray_state = if cache.configured {
            crate::tray::tray_state(cache.snapshot.as_ref(), cache.failures > 0, include_handled)
        } else {
            crate::tray::TrayState::Disconnected
        };
        StatusPayload {
            snapshot: cache.snapshot.clone(),
            error: cache.error.clone(),
            tray_state: tray_state.label(),
            tooltip: crate::tray::tooltip(
                cache.snapshot.as_ref(),
                cache.error.as_deref(),
                include_handled,
            ),
            failures: cache.failures,
            configured: cache.configured,
        }
    }

    pub(crate) fn replace_settings(&self, settings: Settings) {
        *self
            .settings
            .lock()
            .expect("Einstellungs-Mutex ist nicht vergiftet") = settings;
    }
}

/* -------------------------------------------------------------------------- */
/* Einstellungen                                                              */
/* -------------------------------------------------------------------------- */

/// Lädt die Einstellungen von der Platte.
///
/// Gibt auch die Herkunft zurück, damit das Frontend entscheiden kann, ob der
/// Ersteinrichtungs-Assistent kommt.
#[tauri::command]
pub fn settings_load(state: State<'_, AppState>) -> CommandResult<LoadOutcome> {
    let outcome = state.store().load()?;
    state.replace_settings(outcome.settings.clone());
    Ok(outcome)
}

/// Die zwischengespeicherten Einstellungen, ohne Plattenzugriff.
#[tauri::command]
pub fn settings_current(state: State<'_, AppState>) -> Settings {
    state.settings()
}

/// Speichert die Einstellungen.
#[tauri::command]
pub fn settings_save(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: Settings,
) -> CommandResult<Settings> {
    state.store().save(&settings)?;
    // Zurücklesen, damit das Frontend die reparierten Werte sieht — etwa ein
    // geklemmtes Intervall. Sonst zeigt der Dialog weiter die Eingabe an,
    // während auf der Platte etwas anderes steht.
    let outcome = state.store().load()?;
    state.replace_settings(outcome.settings.clone());

    // Der Autostart lebt in der Registry, nicht in der Konfiguration. Ohne
    // diesen Aufruf wäre der Schalter im Dialog eine Notiz ohne Wirkung — er
    // würde gespeichert und nichts täte sich.
    if let Err(error) = crate::startup::apply(&app, outcome.settings.behaviour.autostart) {
        // Kein harter Fehler: alles andere ist gespeichert, und das
        // zurückzurollen wäre schlimmer als eine Meldung.
        log::warn!("Autostart liess sich nicht umstellen: {error}");
        return Err(CommandError {
            message: format!(
                "Die Einstellungen sind gespeichert, aber der Autostart liess sich nicht \
                 umstellen: {error}"
            ),
            fields: vec!["behaviour.autostart".to_owned()],
            ..CommandError::plain("")
        });
    }

    // Geänderte Zugangsdaten oder ein geändertes Intervall sollen sofort
    // wirken und nicht erst beim nächsten regulären Abruf. Das ist auch der
    // Weg, auf dem ein korrigiertes Secret unmittelbar greift.
    state.refresh_signal().trigger();
    Ok(outcome.settings)
}

/* -------------------------------------------------------------------------- */
/* Abruf                                                                      */
/* -------------------------------------------------------------------------- */

/// Der zuletzt bekannte Zustand.
///
/// Das Fenster braucht ihn beim Öffnen: war es versteckt, hat es das letzte
/// Ereignis nicht mitbekommen.
#[tauri::command]
pub fn status_current(state: State<'_, AppState>) -> StatusPayload {
    state.status_payload()
}

/// Fordert einen sofortigen Abruf an.
///
/// Bricht einen laufenden Abruf ab — siehe Modulkommentar in `poll`.
#[tauri::command]
pub fn refresh_now(state: State<'_, AppState>) {
    state.refresh_signal().trigger();
}

/// Merkt die Anheftung des Fensters.
///
/// Eigener Befehl statt `settings_save`: das Anheften ist ein Umschalter in der
/// Titelzeile und soll weder einen Abruf auslösen noch die ganze Konfiguration
/// durch die Prüfung schicken. Gespeichert wird ungeprüft — die Flagge kann
/// nichts ungültig machen, und der Benutzer soll sie auch dann umstellen
/// können, wenn woanders noch etwas fehlt.
#[tauri::command]
pub fn set_pin_popup(state: State<'_, AppState>, pinned: bool) -> CommandResult<()> {
    let mut settings = state.settings();
    if settings.behaviour.pin_popup == pinned {
        return Ok(());
    }
    settings.behaviour.pin_popup = pinned;
    state.store().save_unchecked(&settings)?;
    state.replace_settings(settings);
    Ok(())
}

/// Öffnet die passende Ansicht in der CheckMK-Weboberfläche.
///
/// `service` leer oder fehlend heisst: die Host-Ansicht. Die URL-Bildung samt
/// Kodierung liegt im `checkmk`-Modul und ist dort getestet — insbesondere,
/// dass ein Ampersand im Servicenamen keinen zusätzlichen Parameter erzeugt.
#[tauri::command]
pub fn open_in_checkmk(
    state: State<'_, AppState>,
    host: String,
    service: Option<String>,
) -> CommandResult<()> {
    let settings = state.settings();
    let connection = settings.active();
    let urls = crate::checkmk::SiteUrl::new(&connection.server, &connection.site)?;

    let url = match service.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(service) => urls.service_page(&host, service)?,
        None => urls.host_page(&host)?,
    };

    tauri_plugin_opener::open_url(url.as_str(), None::<&str>).map_err(|error| {
        CommandError::plain(format!(
            "Der Browser liess sich nicht öffnen: {error}. Die Adresse wäre: {url}"
        ))
    })
}

/* -------------------------------------------------------------------------- */
/* Über das Programm                                                          */
/* -------------------------------------------------------------------------- */

/// Adresse des Quelltexts.
///
/// **Fest verdrahtet, mit Absicht.** Es gibt bewusst keinen Befehl, der eine
/// beliebige URL öffnet: das wäre eine Stelle, an der ein Fehler im Frontend
/// oder eine untergeschobene Zeichenkette den Standardbrowser auf etwas
/// Beliebiges lenken könnte. Für den einen Link, den die Fusszeile braucht,
/// genügt eine Konstante.
pub const PROJECT_URL: &str = "https://github.com/leosysr/luchsr";

/// Angaben für die Fusszeile.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AboutInfo {
    /// Aus `tauri.conf.json`, der einzigen Stelle mit der Version.
    pub version: String,
    pub project_url: String,
}

/// Version und Projektadresse.
///
/// Die Version kommt aus dem Paket und nicht aus dem Frontend — sie steht
/// ausschliesslich in `tauri.conf.json`, und sie dort **und** im Frontend zu
/// pflegen wären zwei Wahrheiten, die beim nächsten Release auseinanderlaufen.
#[tauri::command]
pub fn about_info(app: tauri::AppHandle) -> AboutInfo {
    AboutInfo {
        version: app.package_info().version.to_string(),
        project_url: PROJECT_URL.to_owned(),
    }
}

/// Öffnet die Projektseite im Standardbrowser.
///
/// Ohne Parameter — siehe [`PROJECT_URL`].
#[tauri::command]
pub fn open_project_page() -> CommandResult<()> {
    tauri_plugin_opener::open_url(PROJECT_URL, None::<&str>).map_err(|error| {
        CommandError::plain(format!(
            "Der Browser liess sich nicht öffnen: {error}. Die Adresse ist: {PROJECT_URL}"
        ))
    })
}

/* -------------------------------------------------------------------------- */
/* Klänge                                                                     */
/* -------------------------------------------------------------------------- */

/// Ein eingebauter Klang, wie das Frontend ihn braucht.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuiltinSoundInfo {
    pub id: String,
    pub label: String,
}

/// Die Liste der eingebauten Klänge für die Auswahlfelder.
///
/// Kommt aus dem Backend, weil die Klänge dort eingebaut sind. Sie im Frontend
/// nachzuschreiben wären zwei Wahrheiten — und eine Kennung, die auseinander
/// läuft, ergibt eine Auswahl, die stumm bleibt.
#[tauri::command]
pub fn builtin_sounds() -> Vec<BuiltinSoundInfo> {
    crate::notify::sound::BUILTIN
        .iter()
        .map(|s| BuiltinSoundInfo {
            id: s.id.to_owned(),
            label: s.label.to_owned(),
        })
        .collect()
}

/// Spielt eine Auswahl zum Vorhören.
///
/// Ohne das müsste der Benutzer blind wählen, speichern und auf ein Ereignis
/// warten, um zu hören, was er eingestellt hat. Nimmt die Auswahl direkt aus
/// dem Dialog, nicht aus den gespeicherten Einstellungen — sonst liesse sich
/// nur Gespeichertes probieren.
#[tauri::command]
pub fn play_sound(choice: crate::config::SoundChoice) {
    crate::notify::sound::play(&choice);
}

/// Schreibt die **vollständige** Problemliste des aktuellen Abzugs als CSV.
///
/// Vollständig heisst: alle Zeilen des Abzugs, auch quittierte und solche in
/// Wartung. Die Filter der Oberfläche wirken **nicht** — eine Datei, die man
/// weitergibt, soll den Stand vollständig zeigen und nicht davon abhängen, was
/// beim Klicken gerade eingestellt war. Die Merkmale stehen als eigene Spalten
/// drin, man kann in Excel also nachfiltern.
///
/// Gibt den geschriebenen Pfad zurück, oder `None`, wenn der Benutzer den
/// Speicherdialog abgebrochen hat. Ein Abbruch ist kein Fehler.
///
/// `async`, weil `blocking_save_file` nicht im Hauptthread laufen darf — Tauri
/// führt `async`-Befehle auf einem eigenen Thread aus.
#[tauri::command]
pub async fn export_csv(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
) -> CommandResult<Option<String>> {
    use tauri_plugin_dialog::DialogExt;

    let (csv, vorschlag, zeilen) = {
        let cache = state
            .status
            .lock()
            .expect("Status-Mutex ist nicht vergiftet");
        let Some(snapshot) = cache.snapshot.as_ref() else {
            return Err(CommandError::plain(
                "Es liegt noch kein Abzug vor. Erst aktualisieren, dann exportieren.",
            ));
        };
        (
            crate::export::to_csv(&snapshot.problems, snapshot.fetched_at),
            crate::export::dateiname(snapshot),
            snapshot.problems.len(),
        )
    };

    // Solange der Dialog offen ist, darf der Fokusverlust das Popup nicht
    // ausblenden. Der Wächter nimmt die Sperre am Ende der Funktion zurück,
    // auch bei einem frühen `return`.
    let _modal = state.modal_open();

    let mut dialog = app
        .dialog()
        .file()
        .set_title("Problemliste als CSV speichern")
        .set_file_name(&vorschlag)
        .add_filter("CSV", &["csv"]);

    // Elternfenster setzen: das Popup steht auf `alwaysOnTop` und läge sonst
    // womöglich über dem Dialog.
    if let Some(window) = app.get_webview_window(crate::tray::POPUP_LABEL) {
        dialog = dialog.set_parent(&window);
    }

    let Some(ziel) = dialog.blocking_save_file() else {
        return Ok(None);
    };
    let pfad = ziel
        .into_path()
        .map_err(|error| CommandError::plain(format!("Der Zielpfad ist unbrauchbar: {error}")))?;

    std::fs::write(&pfad, csv.as_bytes()).map_err(|error| {
        CommandError::plain(format!(
            "Die Datei „{}“ liess sich nicht schreiben: {error}",
            pfad.display()
        ))
    })?;

    log::info!("{zeilen} Zeilen nach {} exportiert", pfad.display());
    Ok(Some(pfad.display().to_string()))
}

/* -------------------------------------------------------------------------- */
/* Schreibaktionen                                                            */
/* -------------------------------------------------------------------------- */

/// Vorbelegter Kommentar für den Aktionsdialog.
///
/// Wird gebraucht, bevor der Benutzer etwas eingibt, und die Vorlage samt
/// Platzhalterersetzung liegt im Backend — also holt das Frontend den fertigen
/// Text hier ab statt die Ersetzung nachzubauen.
#[tauri::command]
pub fn action_comment(
    state: State<'_, AppState>,
    action: WriteAction,
    host: String,
    service: Option<String>,
) -> String {
    let settings = state.settings();
    let template = match action {
        WriteAction::Acknowledge => &settings.permissions.acknowledge_comment,
        WriteAction::Downtime => &settings.permissions.downtime_comment,
    };
    crate::actions::render_comment(
        template,
        &host,
        service.as_deref().filter(|s| !s.trim().is_empty()),
        &settings.active().username,
    )
}

/// Quittiert ein Problem in CheckMK.
///
/// `service` leer oder fehlend heisst: das Hostproblem. Läuft nur, wenn
/// `permissions.allowAcknowledge` gesetzt ist — geprüft **hier**, nicht im
/// Frontend, siehe Modulkommentar in `actions`.
#[tauri::command]
pub async fn acknowledge(
    state: State<'_, AppState>,
    host: String,
    service: Option<String>,
    comment: String,
) -> CommandResult<()> {
    let settings = state.settings();
    crate::actions::ensure_allowed(WriteAction::Acknowledge, &settings)?;
    let comment = crate::actions::ensure_comment(&comment)?.to_owned();

    let client = build_client(&settings)?;
    let options = AcknowledgeOptions {
        comment,
        ..AcknowledgeOptions::default()
    };

    match service.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(service) => client.acknowledge_service(&host, service, &options).await?,
        None => client.acknowledge_host(&host, &options).await?,
    }

    // Bestätigung, dass es angekommen ist. Vorgabe ist kein Ton — wer geklickt
    // hat, weiss ohnehin, was er getan hat; hörbar ist es nur, wenn gewählt.
    crate::notify::sound::play(&settings.notifications.sounds.acknowledged);

    // Sofort neu abrufen: sonst steht die Zeile bis zum nächsten Intervall
    // unverändert da, und es sieht aus, als hätte die Aktion nichts getan.
    state.refresh_signal().trigger();
    Ok(())
}

/// Setzt eine Wartungszeit in CheckMK.
///
/// `minutes` ist gesetzt, wenn `duration` auf eine freie Angabe zeigt; sonst
/// ergibt sich das Fenster aus der Auswahl. Beginn ist immer *jetzt* — die
/// Berechnung liegt in [`crate::checkmk::DowntimeDuration::window`].
#[tauri::command]
pub async fn set_downtime(
    state: State<'_, AppState>,
    host: String,
    service: Option<String>,
    comment: String,
    duration: DowntimeChoice,
    minutes: Option<i64>,
) -> CommandResult<()> {
    let settings = state.settings();
    crate::actions::ensure_allowed(WriteAction::Downtime, &settings)?;
    let comment = crate::actions::ensure_comment(&comment)?;
    let duration = duration.resolve(minutes)?;

    let client = build_client(&settings)?;
    let (start, end) = duration.window(chrono::Local::now());

    match service.as_deref().filter(|s| !s.trim().is_empty()) {
        Some(service) => {
            let body = DowntimeServiceBody::new(&host, &[service.to_owned()], comment, start, end);
            client.downtime_service(&body).await?
        }
        None => {
            let body = DowntimeHostBody::new(&host, comment, start, end);
            client.downtime_host(&body).await?
        }
    }

    crate::notify::sound::play(&settings.notifications.sounds.downtime);
    state.refresh_signal().trigger();
    Ok(())
}

/// Die Dauerauswahl, wie sie aus dem Dialog kommt.
///
/// Eigener Typ statt [`DowntimeDuration`] direkt, weil `Minutes(i64)` über die
/// IPC-Grenze zwei Felder bräuchte. Hier ist die Auswahl eine schlichte
/// Aufzählung und die Zahl ein eigener Parameter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DowntimeChoice {
    Minutes15,
    Hour1,
    Hours4,
    UntilMorning,
    Custom,
}

impl DowntimeChoice {
    /// Grenzen der freien Angabe.
    ///
    /// Unten 1 Minute — kürzer ist sinnlos, und 0 ergäbe ein leeres Fenster,
    /// das CheckMK annimmt und das nichts tut. Oben 90 Tage: eine Wartung, die
    /// länger läuft, ist keine Wartung mehr, sondern ein vergessener Eintrag.
    const CUSTOM_MIN: i64 = 1;
    const CUSTOM_MAX: i64 = 90 * 24 * 60;

    fn resolve(self, minutes: Option<i64>) -> CommandResult<DowntimeDuration> {
        Ok(match self {
            Self::Minutes15 => DowntimeDuration::Minutes15,
            Self::Hour1 => DowntimeDuration::Hour1,
            Self::Hours4 => DowntimeDuration::Hours4,
            Self::UntilMorning => DowntimeDuration::UntilMorning,
            Self::Custom => {
                let minutes = minutes.ok_or_else(|| {
                    CommandError::plain("Für eine freie Dauer fehlt die Angabe in Minuten.")
                })?;
                if !(Self::CUSTOM_MIN..=Self::CUSTOM_MAX).contains(&minutes) {
                    return Err(CommandError::plain(format!(
                        "Die Dauer muss zwischen {} Minute und {} Tagen liegen.",
                        Self::CUSTOM_MIN,
                        Self::CUSTOM_MAX / (24 * 60)
                    )));
                }
                DowntimeDuration::Minutes(minutes)
            }
        })
    }
}

impl From<crate::actions::ActionRefusal> for CommandError {
    fn from(refusal: crate::actions::ActionRefusal) -> Self {
        let mut error = Self::plain(refusal.to_string());
        if let crate::actions::ActionRefusal::NotPermitted(action) = refusal {
            error.fields.push(action.setting_path().to_owned());
        }
        error
    }
}

/// Baut den API-Client aus den gespeicherten Einstellungen.
///
/// Das Secret kommt aus dem Credential Manager und wird nicht durchgereicht —
/// eine Schreibaktion darf keinen Weg öffnen, auf dem es durch das Frontend
/// läuft.
fn build_client(settings: &Settings) -> CommandResult<CheckmkClient> {
    let connection = settings.active();
    if !connection.is_complete() {
        return Err(CommandError::plain(
            "Die Verbindung ist nicht vollständig eingerichtet.",
        ));
    }
    let secret = SecretStore::load(&connection.username).map_err(|error| match error {
        SecretError::NotFound { .. } => CommandError::plain(
            "Es ist kein Automation-Secret gespeichert. Bitte in den Einstellungen \
             eines eintragen.",
        ),
        other => CommandError::from(other),
    })?;

    let config = ClientConfig {
        server: connection.server.clone(),
        site: connection.site.clone(),
        username: connection.username.clone(),
        secret,
        verify_tls: connection.verify_tls,
        proxy: ProxyMode::from(&connection.proxy),
        timeout: Duration::from_secs(u64::from(settings.polling.timeout_seconds)),
    };
    Ok(CheckmkClient::new(&config)?)
}

/// Prüft Einstellungen, ohne sie zu speichern.
///
/// Für die laufende Anzeige im Dialog: Fehler und Warnungen erscheinen beim
/// Tippen, nicht erst beim Speichern.
#[tauri::command]
pub fn settings_validate(settings: Settings) -> Vec<ValidationIssue> {
    let mut settings = settings;
    settings.repair();
    let mut issues = settings.validate();
    issues.extend(proxy_environment_issue(&settings));
    issues
}

/// Warnt, wenn Anfragen an den konfigurierten Server über einen Proxy laufen
/// würden, obwohl „System" gewählt ist.
///
/// ## Warum es diese Prüfung gibt
///
/// `reqwest` liest bei „System" die Umgebungsvariablen `HTTP_PROXY` und
/// Verwandte. Der Browser und .NET nehmen dagegen WinINET bzw. WinHTTP. In
/// Firmennetzen laufen diese Quellen auseinander: ein gesetztes `HTTP_PROXY`,
/// dessen `NO_PROXY` den internen Server nicht führt, während der Proxy in den
/// Internetoptionen abgeschaltet ist.
///
/// Die Folge ist ein `403 Forbidden` mit HTML-Rumpf vom Proxy. Für den
/// Benutzer sieht das aus wie ein Berechtigungsproblem in CheckMK — und ohne
/// diesen Hinweis sucht man dort auch. Genau das ist einmal passiert.
///
/// Steht der Modus auf „Keiner" oder „Manuell", greift die Prüfung nicht: dann
/// hat der Benutzer die Entscheidung schon getroffen.
fn proxy_environment_issue(settings: &Settings) -> Option<ValidationIssue> {
    let connection = settings.active();
    if connection.proxy != crate::config::ProxyConfig::System {
        return None;
    }

    // Host und Schema aus der zusammengesetzten URL holen — dieselbe Prüfung,
    // die auch der Client durchläuft.
    let urls = crate::checkmk::SiteUrl::new(&connection.server, &connection.site).ok()?;
    let base = urls.api_base();
    let host = base.host_str()?;

    let proxy = crate::checkmk::proxy_for_host(
        host,
        base.scheme(),
        &crate::checkmk::ProxyEnv::from_environment(),
    )?;

    Some(ValidationIssue::warning(
        "connection.proxy",
        format!(
            "Die Umgebung dieses Rechners setzt einen Proxy ({proxy}), und „{host}“ steht \
             nicht in NO_PROXY. Luchsr würde alle Anfragen über diesen Proxy schicken. \
             Weist der Proxy interne Adressen ab, kommt ein HTTP 403 zurück, das wie ein \
             Berechtigungsproblem in CheckMK aussieht. Bei einem CheckMK-Server im eigenen \
             Netz ist meist „Keiner“ richtig. Zur Kontrolle: der Browser nutzt womöglich \
             eine PAC-Datei und erreicht den Server deshalb direkt."
        ),
    ))
}

/* -------------------------------------------------------------------------- */
/* Automation-Secret                                                          */
/* -------------------------------------------------------------------------- */

/// Ob der Credential Manager nutzbar ist. Wird beim Öffnen des Dialogs geprüft.
#[tauri::command]
pub fn credential_store_available() -> CommandResult<()> {
    SecretStore::availability().map_err(CommandError::from)
}

/// Legt das Automation-Secret ab.
///
/// Ein leerer Wert löscht den Eintrag. Es gibt keinen Weg zurück: das Secret
/// verlässt den Credential Manager nie wieder in Richtung Frontend.
#[tauri::command]
pub fn secret_set(username: String, secret: String) -> CommandResult<()> {
    SecretStore::store(&username, &Secret::new(secret)).map_err(CommandError::from)
}

/// Ob für diesen Benutzer ein Secret gespeichert ist. Nur ja oder nein.
#[tauri::command]
pub fn secret_exists(username: String) -> CommandResult<bool> {
    SecretStore::exists(&username).map_err(CommandError::from)
}

/// Löscht das Secret.
#[tauri::command]
pub fn secret_delete(username: String) -> CommandResult<()> {
    match SecretStore::delete(&username) {
        // „War schon weg" ist für den Aufrufer dasselbe wie „gelöscht".
        Ok(()) | Err(SecretError::NotFound { .. }) => Ok(()),
        Err(other) => Err(CommandError::from(other)),
    }
}

/* -------------------------------------------------------------------------- */
/* Verbindungstest                                                            */
/* -------------------------------------------------------------------------- */

/// Prüft eine Verbindung.
///
/// `secret` darf ein noch nicht gespeicherter Wert aus dem Dialog sein. Fehlt
/// er, wird der gespeicherte genommen — so lässt sich eine bestehende
/// Konfiguration prüfen, ohne das Secret erneut eingeben zu müssen.
#[tauri::command]
pub async fn connection_test(
    connection: Connection,
    secret: Option<String>,
    timeout_seconds: Option<u32>,
) -> CommandResult<ConnectionReport> {
    let secret = match secret {
        Some(value) if !value.is_empty() => Secret::new(value),
        _ => SecretStore::load(&connection.username).map_err(|error| match error {
            SecretError::NotFound { .. } => CommandError::plain(
                "Es ist kein Automation-Secret gespeichert. Bitte eines eintragen \
                 und den Test erneut ausführen.",
            ),
            other => CommandError::from(other),
        })?,
    };

    let config = ClientConfig {
        server: connection.server.clone(),
        site: connection.site.clone(),
        username: connection.username.clone(),
        secret,
        verify_tls: connection.verify_tls,
        proxy: ProxyMode::from(&connection.proxy),
        timeout: Duration::from_secs(u64::from(
            timeout_seconds.unwrap_or(crate::config::schema::TIMEOUT_DEFAULT_SECONDS),
        )),
    };

    let client = CheckmkClient::new(&config)?;
    Ok(client.test_connection().await?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkmk::TransportCause;
    use std::sync::atomic::{AtomicBool, Ordering};

    /* ------------------------------------------------------ Modalwächter -- */

    #[test]
    fn modalwaechter_setzt_und_nimmt_zurueck() {
        let flagge = AtomicBool::new(false);
        {
            let _guard = ModalGuard({
                flagge.store(true, Ordering::Release);
                &flagge
            });
            assert!(
                flagge.load(Ordering::Acquire),
                "während des Dialogs gesetzt"
            );
        }
        assert!(
            !flagge.load(Ordering::Acquire),
            "nach dem Dialog muss die Sperre weg sein — sonst blendet das \
             Fenster nie wieder von selbst aus"
        );
    }

    /// Der Fall, für den es überhaupt ein Wächter und kein Setzerpaar ist.
    #[test]
    fn modalwaechter_nimmt_auch_bei_panik_zurueck() {
        let flagge = AtomicBool::new(false);
        let ergebnis = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = ModalGuard({
                flagge.store(true, Ordering::Release);
                &flagge
            });
            panic!("mitten im Dialog");
        }));
        assert!(ergebnis.is_err(), "die Panik muss angekommen sein");
        assert!(!flagge.load(Ordering::Acquire), "Sperre blieb stehen");
    }

    /* ------------------------------------------------- Wartungsdauern -- */

    #[test]
    fn feste_dauern_brauchen_keine_minutenangabe() {
        for wahl in [
            DowntimeChoice::Minutes15,
            DowntimeChoice::Hour1,
            DowntimeChoice::Hours4,
            DowntimeChoice::UntilMorning,
        ] {
            assert!(wahl.resolve(None).is_ok(), "{wahl:?} verlangte Minuten");
        }
    }

    #[test]
    fn freie_dauer_ohne_angabe_wird_abgelehnt() {
        let fehler = DowntimeChoice::Custom.resolve(None).unwrap_err();
        assert!(fehler.message.contains("Minuten"), "{}", fehler.message);
    }

    /// Null Minuten ergäbe ein leeres Fenster: CheckMK nimmt es an, und es tut
    /// nichts. Ein Wartungseintrag, der nicht wirkt, ist schlimmer als ein
    /// abgelehnter — man verlässt sich darauf.
    #[test]
    fn null_und_negative_minuten_werden_abgelehnt() {
        for minuten in [0, -1, -600] {
            assert!(
                DowntimeChoice::Custom.resolve(Some(minuten)).is_err(),
                "{minuten} Minuten wurden angenommen"
            );
        }
    }

    #[test]
    fn die_grenzen_der_freien_dauer_sind_beidseitig_erlaubt() {
        assert_eq!(
            DowntimeChoice::Custom
                .resolve(Some(DowntimeChoice::CUSTOM_MIN))
                .unwrap(),
            DowntimeDuration::Minutes(1)
        );
        assert!(DowntimeChoice::Custom
            .resolve(Some(DowntimeChoice::CUSTOM_MAX))
            .is_ok());
        assert!(DowntimeChoice::Custom
            .resolve(Some(DowntimeChoice::CUSTOM_MAX + 1))
            .is_err());
    }

    /// Die Meldung soll die Grenze in Tagen nennen, nicht in 129600 Minuten.
    #[test]
    fn die_grenzmeldung_nennt_tage() {
        let fehler = DowntimeChoice::Custom.resolve(Some(999_999)).unwrap_err();
        assert!(fehler.message.contains("90 Tagen"), "{}", fehler.message);
    }

    /* ------------------------------------------------ Berechtigungen -- */

    /// Eine abgelehnte Aktion muss den Feldpfad mitgeben, damit der Dialog die
    /// richtige Stelle hervorheben kann.
    #[test]
    fn die_ablehnung_traegt_den_feldpfad() {
        let error = CommandError::from(crate::actions::ActionRefusal::NotPermitted(
            WriteAction::Acknowledge,
        ));
        assert_eq!(error.fields, vec!["permissions.allowAcknowledge"]);
    }

    #[test]
    fn ein_leerer_kommentar_traegt_keinen_feldpfad() {
        let error = CommandError::from(crate::actions::ActionRefusal::EmptyComment);
        assert!(error.fields.is_empty());
        assert!(error.message.contains("Kommentar"), "{}", error.message);
    }

    /* --------------------------------------------------- Fehlerabbildung -- */

    /// Ein Zertifikatsproblem muss als solches markiert sein, damit der Dialog
    /// den richtigen Hinweis zeigt.
    #[test]
    fn tls_fehler_wird_als_tls_problem_markiert() {
        let error = CommandError::from(CheckmkError::Transport {
            cause: TransportCause::TlsUntrustedRoot,
            chain: vec!["a".into(), "b".into()],
        });
        assert!(error.is_tls_problem);
        assert!(
            !error.retryable,
            "kaputtes Zertifikat wiederholt sich nicht"
        );
        assert_eq!(error.details, vec!["a", "b"]);
        assert!(
            error.message.contains("Stammzertifikat"),
            "{}",
            error.message
        );
    }

    #[test]
    fn dns_fehler_ist_kein_tls_problem_aber_wiederholbar() {
        let error = CommandError::from(CheckmkError::Transport {
            cause: TransportCause::Dns,
            chain: vec![],
        });
        assert!(!error.is_tls_problem);
        assert!(error.retryable);
    }

    #[test]
    fn ungueltige_einstellungen_liefern_die_feldpfade_mit() {
        let error = CommandError::from(ConfigError::Invalid {
            summary: "fehlt".into(),
            fields: vec!["connection.server".into(), "connection.site".into()],
        });
        assert_eq!(error.fields, vec!["connection.server", "connection.site"]);
    }

    #[test]
    fn falsches_secret_ist_nicht_wiederholbar() {
        let error = CommandError::from(CheckmkError::Unauthorized { detail: None });
        assert!(!error.retryable);
        assert!(
            error.message.contains("Automation-Secret"),
            "{}",
            error.message
        );
    }

    /* ---------------------------------------------- Form für das Frontend */

    /// Das Frontend liest camelCase. Ein Umbenennen hier bricht den Dialog.
    #[test]
    fn fehlerform_ist_camelcase() {
        let json = serde_json::to_value(CommandError::plain("x")).unwrap();
        let object = json.as_object().unwrap();
        for key in ["message", "isTlsProblem", "retryable", "fields", "details"] {
            assert!(object.contains_key(key), "{key} fehlt in {json}");
        }
        assert_eq!(object.len(), 5, "unerwartete Felder in {json}");
    }

    /// Kein Befehlsergebnis darf ein Secret enthalten. Stichprobe über die
    /// serialisierten Formen, die tatsächlich zurückgehen.
    #[test]
    fn befehlsergebnisse_enthalten_kein_secret() {
        let mut settings = Settings::default();
        {
            let connection = settings.active_mut();
            connection.server = "https://checkmk.example.intern".into();
            connection.site = "leosys".into();
            connection.username = "m.mustermann".into();
        }

        let ausgaben = [
            serde_json::to_string(&settings).unwrap(),
            serde_json::to_string(&settings.validate()).unwrap(),
            serde_json::to_string(&CommandError::plain("Fehler")).unwrap(),
        ];

        for ausgabe in ausgaben {
            let klein = ausgabe.to_lowercase();
            for verdaechtig in ["secret", "password", "passwort", "kennwort", "token"] {
                assert!(
                    !klein.contains(verdaechtig),
                    "„{verdaechtig}“ steht in einer Befehlsantwort: {ausgabe}"
                );
            }
        }
    }

    /// Es darf keinen Befehl geben, der das Secret ausliest.
    ///
    /// Zwei Fallstricke, über die die naive Fassung gestolpert ist:
    /// der Suchbegriff wird zusammengesetzt, damit der Test nicht sich selbst
    /// findet, und Kommentarzeilen werden übersprungen — die Dokumentation
    /// darf benennen, was der Code nicht enthalten soll.
    #[test]
    fn es_gibt_keinen_lesenden_secret_befehl() {
        let verboten = concat!("secret", "_get");
        for (name, quelle) in [
            ("commands.rs", include_str!("commands.rs")),
            ("lib.rs", include_str!("lib.rs")),
        ] {
            let treffer: Vec<&str> = quelle
                .lines()
                .map(str::trim)
                .filter(|line| !line.starts_with("//") && !line.starts_with('*'))
                .filter(|line| line.contains(verboten))
                .collect();
            assert!(
                treffer.is_empty(),
                "in {name} steht ein lesender Secret-Befehl: {treffer:?}"
            );
        }
    }

    /// Die registrierten Befehle sind die öffentliche Oberfläche des Backends.
    /// Der Test hält sie fest, damit nicht versehentlich etwas dazukommt.
    #[test]
    fn nur_die_erwarteten_befehle_sind_registriert() {
        let lib = include_str!("lib.rs");
        let erwartet = [
            "settings_load",
            "settings_current",
            "settings_save",
            "settings_validate",
            "credential_store_available",
            "secret_set",
            "secret_exists",
            "secret_delete",
            "connection_test",
            "status_current",
            "refresh_now",
            "open_in_checkmk",
            "set_pin_popup",
            "export_csv",
            "action_comment",
            "acknowledge",
            "set_downtime",
            "builtin_sounds",
            "play_sound",
            "about_info",
            "open_project_page",
        ];

        // Jeder erwartete Befehl ist registriert.
        for name in erwartet {
            assert!(
                lib.contains(&format!("commands::{name},")),
                "{name} ist nicht registriert"
            );
        }

        // Und es sind keine anderen registriert.
        let registriert = lib
            .lines()
            .filter_map(|line| line.trim().strip_prefix("commands::"))
            .filter_map(|rest| rest.strip_suffix(','))
            .collect::<Vec<_>>();
        assert_eq!(
            registriert.len(),
            erwartet.len(),
            "unerwartete Befehlsliste: {registriert:?}"
        );
    }
}
