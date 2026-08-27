//! Das Datenmodell der Einstellungen.
//!
//! ## Zwei harte Regeln
//!
//! **1. Das Automation-Secret kommt hier nicht vor.** Es gibt kein Feld dafür,
//! auch kein optionales. Damit ist es strukturell unmöglich, dass es in
//! `config.json` landet — kein Kommentar und keine Disziplin nötig. Das Secret
//! liegt ausschliesslich im Windows Credential Manager, siehe `secrets.rs`.
//! Ein Test unten prüft die serialisierte Form gegen verdächtige Schlüsselnamen.
//!
//! **2. Keine hartkodierten Werte.** Jeder Parameter des Auftrags ist hier ein
//! Feld und im Einstellungsdialog pflegbar.
//!
//! ## Vorwärts- und Rückwärtskompatibilität
//!
//! Jedes Feld trägt `#[serde(default)]`. Eine `config.json` einer älteren
//! Version lädt damit weiter, fehlende Felder bekommen ihre Vorgabe. Umgekehrt
//! wirft eine neuere Datei mit unbekannten Feldern keinen Fehler. Für eine
//! Desktop-App, die über Softwaremanagement verteilt wird und deren Config auf
//! dem Rechner des Benutzers liegt, ist das keine Bequemlichkeit sondern
//! Voraussetzung: ein Rollback darf die Einstellungen nicht zerstören.
//!
//! ## Mehrere CheckMK-Instanzen
//!
//! Der Auftrag verlangt, das Datenmodell nachrüstbar anzulegen, ohne mehrere
//! Instanzen jetzt zu unterstützen. Deshalb ist [`Settings::connections`] eine
//! Liste und [`Settings::active_connection`] ein Index. Aktuell erzwingt die
//! Prüfung genau einen Eintrag; die Struktur muss dafür später nicht angefasst
//! werden.

use serde::{Deserialize, Serialize};

use crate::checkmk::ProxyMode;

/// Aktuelle Schemaversion. Wird bei einem strukturellen Bruch erhöht, damit
/// eine Migration erkennen kann, was sie vor sich hat.
pub const SCHEMA_VERSION: u32 = 2;

/// Grenzen des Abrufintervalls laut Auftrag.
pub const INTERVAL_MIN_SECONDS: u32 = 15;
pub const INTERVAL_MAX_SECONDS: u32 = 600;
pub const INTERVAL_DEFAULT_SECONDS: u32 = 60;

/// Zeitgrenze eines einzelnen Abrufs laut Auftrag.
pub const TIMEOUT_DEFAULT_SECONDS: u32 = 10;
pub const TIMEOUT_MIN_SECONDS: u32 = 2;
pub const TIMEOUT_MAX_SECONDS: u32 = 120;

/* -------------------------------------------------------------------------- */
/* Verbindung                                                                 */
/* -------------------------------------------------------------------------- */

/// Proxy-Einstellung in serialisierbarer Form.
///
/// Eigener Typ statt [`ProxyMode`] direkt, damit das Wire-Format stabil bleibt,
/// auch wenn sich der Client-Typ ändert.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "mode", rename_all = "camelCase")]
pub enum ProxyConfig {
    /// Proxy-Einstellungen des Systems übernehmen.
    #[default]
    System,
    /// Keinen Proxy verwenden, auch wenn das System einen setzt.
    None,
    /// Fester Proxy.
    Manual { url: String },
}

impl From<&ProxyConfig> for ProxyMode {
    fn from(value: &ProxyConfig) -> Self {
        match value {
            ProxyConfig::System => ProxyMode::System,
            ProxyConfig::None => ProxyMode::Disabled,
            ProxyConfig::Manual { url } => ProxyMode::Manual(url.clone()),
        }
    }
}

/// Eine CheckMK-Instanz.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Connection {
    /// Stabile Kennung. Bleibt gleich, wenn Server oder Benutzer geändert
    /// werden, und dient später als Schlüssel bei mehreren Instanzen.
    pub id: String,
    /// Anzeigename. Leer heisst: den Site-Namen anzeigen.
    pub name: String,
    /// Ohne Pfad, etwa `https://checkmk.example.intern`.
    pub server: String,
    pub site: String,
    pub username: String,
    /// TLS-Prüfung. Abschalten ist möglich, das UI warnt deutlich.
    pub verify_tls: bool,
    pub proxy: ProxyConfig,
}

impl Default for Connection {
    fn default() -> Self {
        Self {
            id: "default".to_string(),
            name: String::new(),
            server: String::new(),
            site: String::new(),
            username: String::new(),
            // Vorgabe an, wie im Auftrag.
            verify_tls: true,
            proxy: ProxyConfig::System,
        }
    }
}

impl Connection {
    /// Anzeigename für Menü und Titel.
    pub fn display_name(&self) -> &str {
        if self.name.trim().is_empty() {
            if self.site.trim().is_empty() {
                "Unbenannt"
            } else {
                &self.site
            }
        } else {
            &self.name
        }
    }

    /// Ob genug eingetragen ist, um überhaupt einen Abruf zu versuchen.
    /// Das Secret ist hier nicht prüfbar — das weiss nur `secrets.rs`.
    pub fn is_complete(&self) -> bool {
        !self.server.trim().is_empty()
            && !self.site.trim().is_empty()
            && !self.username.trim().is_empty()
    }
}

/* -------------------------------------------------------------------------- */
/* Teilbereiche                                                               */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PollingSettings {
    /// 15–600 s, Vorgabe 60.
    pub interval_seconds: u32,
    /// Zeitgrenze eines einzelnen Abrufs.
    pub timeout_seconds: u32,
}

impl Default for PollingSettings {
    fn default() -> Self {
        Self {
            interval_seconds: INTERVAL_DEFAULT_SECONDS,
            timeout_seconds: TIMEOUT_DEFAULT_SECONDS,
        }
    }
}

/// Wann benachrichtigt wird.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum NotificationLevel {
    /// Keine Benachrichtigungen.
    Off,
    /// Nur CRIT und Host DOWN — die Vorgabe des Auftrags.
    #[default]
    CriticalOnly,
    /// Jede Statusänderung.
    AllChanges,
}

/// Woher der Klang für ein Ereignis kommt.
///
/// Getaggte Aufzählung wie [`ProxyConfig`], damit das Wire-Format lesbar und
/// erweiterbar bleibt: `{"kind":"builtin","id":"kritisch"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum SoundChoice {
    /// Kein Ton für dieses Ereignis. Muss es für **jedes** geben.
    #[default]
    None,
    /// Einer der eingebauten Klänge, siehe `notify::sound::BUILTIN`.
    Builtin { id: String },
    /// Eigene WAV-Datei.
    File { path: String },
}

impl SoundChoice {
    fn builtin(id: &str) -> Self {
        Self::Builtin { id: id.to_owned() }
    }

    pub fn is_none(&self) -> bool {
        matches!(self, Self::None)
    }
}

/// Ereignisse, die einen eigenen Klang bekommen können.
///
/// Fünf Felder statt einer Zuordnung: so ist im Typ festgehalten, welche
/// Ereignisse es gibt, und ein Tippfehler im Namen wird zum Compilerfehler
/// statt zu einem Ereignis, das stillschweigend nie klingt.
///
/// Was **keinen** Klang bekommt, ist der Verbindungsfehler: der wiederholt sich
/// bei einem längeren Ausfall jede Minute, und ein Ton dazu wäre nach zehn
/// Minuten der Grund, alle Töne abzuschalten. Er ist am Tray-Icon zu sehen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct SoundSettings {
    /// Neues CRIT, DOWN oder UNREACHABLE.
    pub critical: SoundChoice,
    /// Neues WARN oder UNKNOWN. Kommt nur bei „jede Statusänderung" vor.
    pub warning: SoundChoice,
    /// Ein gemeldetes Problem ist weg, quittiert, in Wartung oder milder.
    pub recovery: SoundChoice,
    /// Eigene Aktion: Quittieren war erfolgreich.
    pub acknowledged: SoundChoice,
    /// Eigene Aktion: Wartungszeit wurde gesetzt.
    pub downtime: SoundChoice,
}

impl SoundSettings {
    /// Alle fünf Auswahlen der Reihe nach, veränderbar.
    ///
    /// Damit Prüfung und Reparatur nicht fünf Zeilen abschreiben müssen und
    /// beim Hinzufügen eines sechsten Ereignisses **eine** Stelle zu ändern
    /// ist. Die Reihenfolge entspricht der Anzeige im Dialog.
    pub fn alle_mut(&mut self) -> [&mut SoundChoice; 5] {
        [
            &mut self.critical,
            &mut self.warning,
            &mut self.recovery,
            &mut self.acknowledged,
            &mut self.downtime,
        ]
    }

    /// Dasselbe lesend, mit dem Feldpfad für Prüfmeldungen.
    pub fn alle(&self) -> [(&'static str, &SoundChoice); 5] {
        [
            ("notifications.sounds.critical", &self.critical),
            ("notifications.sounds.warning", &self.warning),
            ("notifications.sounds.recovery", &self.recovery),
            ("notifications.sounds.acknowledged", &self.acknowledged),
            ("notifications.sounds.downtime", &self.downtime),
        ]
    }
}

impl Default for SoundSettings {
    /// Nur das Kritische klingt.
    ///
    /// Alles klingen zu lassen wäre der schnellste Weg dazu, dass der Benutzer
    /// alles abschaltet. Warnungen kommen häufig, Entwarnungen sind gute
    /// Nachrichten, und eigene Aktionen hat man selbst ausgelöst — für die drei
    /// ist Stille die bessere Vorgabe. Wer sie will, wählt sie.
    fn default() -> Self {
        Self {
            critical: SoundChoice::builtin("kritisch"),
            warning: SoundChoice::None,
            recovery: SoundChoice::None,
            acknowledged: SoundChoice::None,
            downtime: SoundChoice::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct NotificationSettings {
    pub level: NotificationLevel,
    pub sounds: SoundSettings,
    /// **Veraltet.** Bis Schemaversion 1 gab es genau einen Klang für alles.
    /// Wird von [`Settings::repair`] nach `sounds.critical` überführt und
    /// danach geleert. Das Feld bleibt, damit eine alte Datei nicht ihre
    /// Einstellung verliert.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sound_path: Option<String>,
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            level: NotificationLevel::CriticalOnly,
            sounds: SoundSettings::default(),
            sound_path: None,
        }
    }
}

/// Theme-Wahl. Entspricht `ThemePreference` in `src/lib/theme.ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum ThemePreference {
    #[default]
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct AppearanceSettings {
    pub theme: ThemePreference,
    /// Sprachkürzel. Ausgeliefert wird nur `de`, das Feld hält den Weg offen.
    pub language: String,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self {
            theme: ThemePreference::System,
            language: "de".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct BehaviourSettings {
    /// Vorgabe an. Wird beim allerersten Start aktiviert.
    pub autostart: bool,
    /// Ob die Erstaktivierung des Autostarts schon passiert ist.
    ///
    /// Ohne dieses Feld würde eine spätere Deaktivierung durch den Benutzer
    /// bei jedem Start überschrieben — genau das schliesst der Auftrag aus.
    pub autostart_initialised: bool,
    /// Auch bei einem **manuellen** Start kein Fenster öffnen.
    ///
    /// Beim Autostart bleibt es ohnehin zu — das erkennt `startup` an einer
    /// Marke im Registrierungseintrag und braucht diese Einstellung nicht.
    /// Sie wirkt deshalb nur noch auf den Start per Doppelklick oder
    /// Verknüpfung.
    ///
    /// Vorgabe **an**, so steht es in der Parametertabelle des Auftrags. Der
    /// Nebeneffekt: ein Doppelklick öffnet dann kein Fenster, sondern legt nur
    /// das Tray-Icon an. Für eine Tray-Anwendung vertretbar, aber beim ersten
    /// Mal erklärungsbedürftig — die Ersteinrichtung ist davon ausgenommen.
    pub start_minimised: bool,
    /// Popup angeheftet lassen statt bei Fokusverlust zu schliessen.
    pub pin_popup: bool,
    /// Quittierte Zustände und Wartungszeiten ausblenden. Vorgabe an.
    pub hide_handled: bool,
}

impl Default for BehaviourSettings {
    fn default() -> Self {
        Self {
            autostart: true,
            autostart_initialised: false,
            start_minimised: true,
            pin_popup: false,
            hide_handled: true,
        }
    }
}

/// Vorbelegung des Kommentars beim Quittieren.
///
/// Getrennt von der Wartungszeit, weil die beiden Sätze verschiedene Dinge
/// aussagen: „ist bekannt, wird bearbeitet“ gegen „ist geplant, bitte nicht
/// alarmieren“. Eine gemeinsame Vorlage würde für beide zu allgemein.
pub const DEFAULT_ACK_COMMENT: &str = "{service} auf {host} — bekannt, wird bearbeitet ({user})";

/// Vorbelegung des Kommentars bei einer Wartungszeit.
pub const DEFAULT_DOWNTIME_COMMENT: &str = "{service} auf {host} — geplante Wartung ({user})";

/// Freigabe der Schreibaktionen. Beide Vorgaben **aus**, wie im Auftrag.
///
/// Die Kommentarvorlagen stehen hier und nicht als Konstanten im Code, weil der
/// Auftrag „alles variabel, keine hartkodierten Werte“ verlangt. Ersetzt werden
/// die Platzhalter aus [`crate::actions::PLACEHOLDERS`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct PermissionSettings {
    pub allow_acknowledge: bool,
    pub allow_downtime: bool,
    pub acknowledge_comment: String,
    pub downtime_comment: String,
}

impl Default for PermissionSettings {
    fn default() -> Self {
        Self {
            allow_acknowledge: false,
            allow_downtime: false,
            acknowledge_comment: DEFAULT_ACK_COMMENT.to_owned(),
            downtime_comment: DEFAULT_DOWNTIME_COMMENT.to_owned(),
        }
    }
}

/* -------------------------------------------------------------------------- */
/* Gesamteinstellungen                                                        */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    /// Schemaversion der Datei, nicht die Programmversion.
    pub schema_version: u32,
    /// Aktuell genau ein Eintrag. Liste, damit mehrere Instanzen nachrüstbar
    /// sind, ohne die Struktur zu ändern.
    pub connections: Vec<Connection>,
    /// Index in `connections`.
    pub active_connection: usize,
    pub polling: PollingSettings,
    pub notifications: NotificationSettings,
    pub appearance: AppearanceSettings,
    pub behaviour: BehaviourSettings,
    pub permissions: PermissionSettings,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: SCHEMA_VERSION,
            connections: vec![Connection::default()],
            active_connection: 0,
            polling: PollingSettings::default(),
            notifications: NotificationSettings::default(),
            appearance: AppearanceSettings::default(),
            behaviour: BehaviourSettings::default(),
            permissions: PermissionSettings::default(),
        }
    }
}

impl Settings {
    /// Die aktive Verbindung. Fällt auf die erste zurück, wenn der Index
    /// nicht passt — eine kaputte Datei soll die App nicht lähmen.
    pub fn active(&self) -> &Connection {
        self.connections
            .get(self.active_connection)
            .or_else(|| self.connections.first())
            .unwrap_or_else(|| {
                // Kann nach `repair` nicht eintreten; der Zweig existiert nur,
                // damit die Signatur ohne Option auskommt.
                unreachable!("repair() stellt mindestens eine Verbindung sicher")
            })
    }

    pub fn active_mut(&mut self) -> &mut Connection {
        self.repair();
        let index = self.active_connection;
        &mut self.connections[index]
    }

    /// Bringt eine geladene Datei in einen benutzbaren Zustand.
    ///
    /// Wird nach jedem Laden aufgerufen. Absicht: die App startet auch mit
    /// einer von Hand verpfuschten `config.json` und zeigt dann Vorgaben,
    /// statt sich zu verweigern. Was nicht reparabel ist, meldet
    /// [`Self::validate`] als Problem im Dialog.
    pub fn repair(&mut self) {
        if self.connections.is_empty() {
            self.connections.push(Connection::default());
        }
        if self.active_connection >= self.connections.len() {
            self.active_connection = 0;
        }
        // Doppelte Kennungen würden bei mehreren Instanzen kollidieren.
        for index in 0..self.connections.len() {
            if self.connections[index].id.trim().is_empty() {
                self.connections[index].id = format!("connection-{index}");
            }
        }

        self.polling.interval_seconds = self
            .polling
            .interval_seconds
            .clamp(INTERVAL_MIN_SECONDS, INTERVAL_MAX_SECONDS);
        self.polling.timeout_seconds = self
            .polling
            .timeout_seconds
            .clamp(TIMEOUT_MIN_SECONDS, TIMEOUT_MAX_SECONDS);

        if self.appearance.language.trim().is_empty() {
            self.appearance.language = "de".to_string();
        }

        // Migration von Schemaversion 1: dort gab es einen Klang für alles.
        // Er wird zum Klang für kritische Probleme — die Stufe, bei der die
        // Vorgabe ohnehin meldet — und das alte Feld wird geleert. Idempotent,
        // weil danach `None` steht.
        // Das `take()` steht ausserhalb der Bedingung: das alte Feld wird in
        // jedem Fall geleert, auch wenn der Wert nicht übernommen wird.
        let alter_pfad = self.notifications.sound_path.take();
        if let Some(path) = alter_pfad {
            let path = path.trim();
            // Eine schon getroffene Wahl gewinnt, und ein leerer Pfad war auch
            // vorher dasselbe wie kein Ton.
            if !path.is_empty() && self.notifications.sounds.critical.is_none() {
                self.notifications.sounds.critical = SoundChoice::File {
                    path: path.to_owned(),
                };
            }
        }

        // Ein leerer Pfad in einer Auswahl ist kein Pfad.
        for choice in self.notifications.sounds.alle_mut() {
            if let SoundChoice::File { path } = &*choice {
                if path.trim().is_empty() {
                    *choice = SoundChoice::None;
                }
            }
            if let SoundChoice::Builtin { id } = &*choice {
                if id.trim().is_empty() {
                    *choice = SoundChoice::None;
                }
            }
        }

        // Eine leere Vorlage ist keine Vorlage. Sie leer zu lassen hiesse, dem
        // Benutzer beim Quittieren ein leeres Feld vorzusetzen, das CheckMK
        // dann ablehnt — mit dem Standardsatz kommt er weiter.
        if self.permissions.acknowledge_comment.trim().is_empty() {
            self.permissions.acknowledge_comment = DEFAULT_ACK_COMMENT.to_owned();
        }
        if self.permissions.downtime_comment.trim().is_empty() {
            self.permissions.downtime_comment = DEFAULT_DOWNTIME_COMMENT.to_owned();
        }

        self.schema_version = SCHEMA_VERSION;
    }

    /// Prüft die Einstellungen für den Dialog.
    ///
    /// Gibt **alle** Probleme zurück, nicht nur das erste: der Benutzer soll
    /// nicht Feld für Feld durch Fehlermeldungen geführt werden.
    pub fn validate(&self) -> Vec<ValidationIssue> {
        let mut issues = Vec::new();
        let connection = self.active();

        if connection.server.trim().is_empty() {
            issues.push(ValidationIssue::error(
                "connection.server",
                "Die Server-URL fehlt.",
            ));
        }
        if connection.site.trim().is_empty() {
            issues.push(ValidationIssue::error(
                "connection.site",
                "Der Site-Name fehlt.",
            ));
        }
        if connection.username.trim().is_empty() {
            issues.push(ValidationIssue::error(
                "connection.username",
                "Der Benutzername fehlt.",
            ));
        }

        // Server und Site nur gemeinsam prüfen — die URL-Prüfung braucht beide.
        if !connection.server.trim().is_empty() && !connection.site.trim().is_empty() {
            if let Err(error) = crate::checkmk::SiteUrl::new(&connection.server, &connection.site) {
                issues.push(ValidationIssue::error(
                    "connection.server",
                    error.to_string(),
                ));
            }
        }

        if let ProxyConfig::Manual { url } = &connection.proxy {
            if url.trim().is_empty() {
                issues.push(ValidationIssue::error(
                    "connection.proxy",
                    "Für einen manuellen Proxy fehlt die Adresse.",
                ));
            }
        }

        // Die Warnung, die der Auftrag ausdrücklich verlangt.
        if !connection.verify_tls {
            issues.push(ValidationIssue::warning(
                "connection.verifyTls",
                "Die TLS-Prüfung ist abgeschaltet. Die Verbindung ist damit gegen \
                 Manipulation ungeschützt: ein Angreifer im Netz kann sich als \
                 CheckMK-Server ausgeben und Zugangsdaten mitlesen. Besser ist es, \
                 das Stammzertifikat der internen CA in den Windows-Zertifikatspeicher \
                 aufzunehmen.",
            ));
        }

        if self.polling.interval_seconds < INTERVAL_MIN_SECONDS
            || self.polling.interval_seconds > INTERVAL_MAX_SECONDS
        {
            issues.push(ValidationIssue::error(
                "polling.intervalSeconds",
                format!(
                    "Das Abrufintervall muss zwischen {INTERVAL_MIN_SECONDS} und \
                     {INTERVAL_MAX_SECONDS} Sekunden liegen."
                ),
            ));
        }

        if self.polling.timeout_seconds >= self.polling.interval_seconds {
            issues.push(ValidationIssue::warning(
                "polling.timeoutSeconds",
                "Die Zeitgrenze ist nicht kleiner als das Abrufintervall. Bei einem \
                 langsamen Server können sich Abrufe dann überlappen.",
            ));
        }

        // Jede Klangauswahl einzeln prüfen. Ohne diese Warnungen bleibt es
        // einfach still, und niemand findet den Grund: die Windows-Funktion,
        // die den Ton spielt, meldet weder ein falsches Format noch eine
        // fehlende Datei.
        for (feld, choice) in self.notifications.sounds.alle() {
            match choice {
                SoundChoice::None => {}
                SoundChoice::Builtin { id } => {
                    if !crate::notify::sound::builtin_exists(id) {
                        issues.push(ValidationIssue::warning(
                            feld,
                            format!(
                                "Der eingebaute Klang „{id}“ ist unbekannt. \
                                 Es wird kein Ton gespielt."
                            ),
                        ));
                    }
                }
                SoundChoice::File { path } => {
                    if !std::path::Path::new(path).is_file() {
                        issues.push(ValidationIssue::warning(
                            feld,
                            format!(
                                "Die Klangdatei „{path}“ ist nicht vorhanden. \
                                 Es wird kein Ton gespielt."
                            ),
                        ));
                    } else if !crate::notify::sound::is_supported(path) {
                        issues.push(ValidationIssue::warning(
                            feld,
                            format!(
                                "„{path}“ ist keine .{}-Datei. Es wird kein Ton gespielt — \
                                 Windows kann hier nur WAV.",
                                crate::notify::sound::SUPPORTED_EXTENSION
                            ),
                        ));
                    }
                }
            }
        }

        issues
    }

    /// Ob gespeichert werden kann. Warnungen blockieren nicht.
    pub fn is_valid(&self) -> bool {
        !self
            .validate()
            .iter()
            .any(|issue| issue.severity == IssueSeverity::Error)
    }
}

/* -------------------------------------------------------------------------- */
/* Prüfergebnisse                                                             */
/* -------------------------------------------------------------------------- */

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum IssueSeverity {
    /// Blockiert das Speichern.
    Error,
    /// Wird angezeigt, blockiert aber nicht.
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ValidationIssue {
    /// Feldpfad in camelCase, damit das Frontend direkt zuordnen kann.
    pub field: String,
    pub message: String,
    pub severity: IssueSeverity,
}

impl ValidationIssue {
    pub fn error(field: &str, message: impl Into<String>) -> Self {
        Self {
            field: field.to_string(),
            message: message.into(),
            severity: IssueSeverity::Error,
        }
    }

    pub fn warning(field: &str, message: impl Into<String>) -> Self {
        Self {
            field: field.to_string(),
            message: message.into(),
            severity: IssueSeverity::Warning,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vollstaendig() -> Settings {
        let mut settings = Settings::default();
        let connection = settings.active_mut();
        connection.server = "https://checkmk.example.intern".into();
        connection.site = "leosys".into();
        connection.username = "m.mustermann".into();
        settings
    }

    /* ------------------------------------------- Die wichtigste Zusicherung */

    /// Das Automation-Secret darf unter keinen Umständen in der
    /// serialisierten Form auftauchen. Es gibt kein Feld dafür — dieser Test
    /// hält fest, dass auch keins hinzukommt.
    #[test]
    fn serialisierte_einstellungen_enthalten_kein_secret_feld() {
        let json = serde_json::to_string_pretty(&vollstaendig()).unwrap();
        let klein = json.to_lowercase();
        for verdaechtig in [
            "secret",
            "password",
            "passwort",
            "kennwort",
            "token",
            "credential",
            "apikey",
            "api_key",
            "pass",
        ] {
            assert!(
                !klein.contains(verdaechtig),
                "„{verdaechtig}“ steht in der Konfiguration:\n{json}"
            );
        }
    }

    /* -------------------------------------------------------- Vorgabewerte */

    /// Die Vorgaben aus der Parametertabelle des Auftrags.
    #[test]
    fn vorgaben_entsprechen_dem_auftrag() {
        let settings = Settings::default();
        assert_eq!(settings.polling.interval_seconds, 60);
        assert_eq!(settings.polling.timeout_seconds, 10);
        assert!(settings.active().verify_tls, "TLS-Prüfung: an");
        assert_eq!(settings.active().proxy, ProxyConfig::System);
        assert!(settings.behaviour.autostart, "Autostart: an");
        assert!(!settings.behaviour.autostart_initialised);
        assert!(settings.behaviour.start_minimised, "Start minimiert: an");
        assert!(
            settings.behaviour.hide_handled,
            "Ack./Wartung ausblenden: an"
        );
        assert!(!settings.behaviour.pin_popup, "Popup nicht angeheftet");
        assert_eq!(
            settings.notifications.level,
            NotificationLevel::CriticalOnly,
            "Benachrichtigungen: nur CRIT+DOWN"
        );
        // Das Altfeld ist leer — es gibt es nur noch für die Migration.
        assert!(settings.notifications.sound_path.is_none());
        // Abweichung von der Parametertabelle, bewusst und abgestimmt (D65):
        // dort stand „Ton: aus". Seit es Klänge je Ereignis gibt, ist genau
        // einer vorbelegt — kritische Probleme. Alles klingen zu lassen wäre
        // der schnellste Weg dazu, dass der Benutzer alles abschaltet; keinen
        // Klang vorzubelegen versteckt die Möglichkeit.
        assert_eq!(
            settings.notifications.sounds.critical,
            SoundChoice::Builtin {
                id: "kritisch".into()
            },
            "kritische Probleme klingen vorbelegt"
        );
        for (feld, choice) in settings.notifications.sounds.alle() {
            if feld != "notifications.sounds.critical" {
                assert!(choice.is_none(), "{feld} soll still sein");
            }
        }
        assert_eq!(settings.appearance.theme, ThemePreference::System);
        assert_eq!(settings.appearance.language, "de");
        assert_eq!(settings.schema_version, SCHEMA_VERSION);
    }

    /// Beide Schreibaktionen sind laut Auftrag standardmässig gesperrt.
    #[test]
    fn schreibaktionen_sind_standardmaessig_gesperrt() {
        let settings = Settings::default();
        assert!(!settings.permissions.allow_acknowledge);
        assert!(!settings.permissions.allow_downtime);
    }

    /* ------------------------------------------------------ Kompatibilität */

    /// Eine leere Datei muss die Vorgaben ergeben, nicht einen Fehler.
    #[test]
    fn leeres_json_ergibt_die_vorgaben() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings, Settings::default());
    }

    /// Eine Datei einer älteren Version, die nur wenige Felder kennt, muss
    /// laden — sonst zerstört ein Rollback die Einstellungen.
    #[test]
    fn teilweise_datei_laedt_mit_vorgaben_fuer_den_rest() {
        let json = r#"{
            "connections": [{ "server": "https://alt.intern", "site": "alt" }],
            "polling": { "intervalSeconds": 120 }
        }"#;
        let mut settings: Settings = serde_json::from_str(json).unwrap();
        settings.repair();

        assert_eq!(settings.active().server, "https://alt.intern");
        assert_eq!(settings.active().site, "alt");
        assert_eq!(settings.polling.interval_seconds, 120);
        // Nicht genannte Felder tragen ihre Vorgabe.
        assert_eq!(settings.polling.timeout_seconds, TIMEOUT_DEFAULT_SECONDS);
        assert!(settings.active().verify_tls);
        assert!(settings.behaviour.hide_handled);
    }

    /// Eine Datei einer neueren Version mit unbekannten Feldern darf nicht
    /// scheitern.
    #[test]
    fn unbekannte_felder_werden_ignoriert() {
        let json = r#"{ "schemaVersion": 99, "zukunftsfeld": true, "polling": { "intervalSeconds": 30, "neuesFeld": "x" } }"#;
        let settings: Settings = serde_json::from_str(json).unwrap();
        assert_eq!(settings.polling.interval_seconds, 30);
    }

    /* --------------------------------------------------------- Reparatur -- */

    #[test]
    fn reparatur_stellt_mindestens_eine_verbindung_sicher() {
        let mut settings = Settings {
            connections: Vec::new(),
            active_connection: 7,
            ..Default::default()
        };
        settings.repair();
        assert_eq!(settings.connections.len(), 1);
        assert_eq!(settings.active_connection, 0);
        // active() darf danach nicht panisch werden.
        assert_eq!(settings.active().id, "default");
    }

    #[test]
    fn reparatur_korrigiert_einen_index_ausserhalb_der_liste() {
        let mut settings = vollstaendig();
        settings.active_connection = 99;
        settings.repair();
        assert_eq!(settings.active_connection, 0);
    }

    #[test]
    fn reparatur_klemmt_das_intervall_in_die_grenzen() {
        for (eingabe, erwartet) in [
            (0u32, INTERVAL_MIN_SECONDS),
            (5, INTERVAL_MIN_SECONDS),
            (60, 60),
            (600, 600),
            (9999, INTERVAL_MAX_SECONDS),
        ] {
            let mut settings = Settings::default();
            settings.polling.interval_seconds = eingabe;
            settings.repair();
            assert_eq!(
                settings.polling.interval_seconds, erwartet,
                "{eingabe} wurde falsch geklemmt"
            );
        }
    }

    #[test]
    fn reparatur_klemmt_die_zeitgrenze() {
        let mut settings = Settings::default();
        settings.polling.timeout_seconds = 0;
        settings.repair();
        assert_eq!(settings.polling.timeout_seconds, TIMEOUT_MIN_SECONDS);

        settings.polling.timeout_seconds = 9999;
        settings.repair();
        assert_eq!(settings.polling.timeout_seconds, TIMEOUT_MAX_SECONDS);
    }

    #[test]
    fn reparatur_ersetzt_leere_kennungen_und_sprache() {
        let mut settings = vollstaendig();
        settings.connections[0].id = "  ".into();
        settings.appearance.language = "".into();
        settings.repair();
        assert_eq!(settings.connections[0].id, "connection-0");
        assert_eq!(settings.appearance.language, "de");
    }

    /// Ein leerer Tonpfad ist kein Pfad. Sonst würde die Prüfung eine
    /// Warnung über eine Datei namens "" erzeugen.
    #[test]
    fn reparatur_macht_aus_leerem_tonpfad_keinen_ton() {
        let mut settings = Settings::default();
        settings.notifications.sound_path = Some("   ".into());
        settings.repair();
        assert!(settings.notifications.sound_path.is_none());
        assert!(settings
            .validate()
            .iter()
            .all(|i| i.field != "notifications.soundPath"));
    }

    /// Eine vorhandene Datei im falschen Format bleibt sonst **still**: die
    /// Windows-Funktion, die den Ton spielt, kann nur WAV und meldet einen
    /// Formatfehler nicht. Ohne diese Warnung sucht der Benutzer den Fehler an
    /// der falschen Stelle.
    #[test]
    fn eine_vorhandene_datei_im_falschen_format_wird_gemeldet() {
        let dir = tempfile::TempDir::new().expect("Temp-Verzeichnis");
        let mp3 = dir.path().join("alarm.mp3");
        std::fs::write(&mp3, b"kein echtes MP3, nur vorhanden").unwrap();

        let mut settings = vollstaendig();
        settings.notifications.sounds.critical = SoundChoice::File {
            path: mp3.display().to_string(),
        };
        let issues = settings.validate();

        let ton: Vec<_> = issues
            .iter()
            .filter(|i| i.field == "notifications.sounds.critical")
            .collect();
        assert_eq!(ton.len(), 1, "genau eine Warnung erwartet: {issues:?}");
        assert_eq!(
            ton[0].severity,
            IssueSeverity::Warning,
            "darf nicht blockieren"
        );
        assert!(ton[0].message.contains("wav"), "{:?}", ton[0]);
        assert!(
            settings.is_valid(),
            "eine Warnung sperrt das Speichern nicht"
        );
    }

    /// Die Gegenprobe: eine vorhandene WAV-Datei erzeugt keine Warnung.
    #[test]
    fn eine_vorhandene_wav_datei_wird_nicht_bemaengelt() {
        let dir = tempfile::TempDir::new().expect("Temp-Verzeichnis");
        let wav = dir.path().join("alarm.wav");
        std::fs::write(&wav, b"kein echtes WAV, nur vorhanden").unwrap();

        let mut settings = vollstaendig();
        settings.notifications.sounds.critical = SoundChoice::File {
            path: wav.display().to_string(),
        };
        assert!(settings
            .validate()
            .iter()
            .all(|i| !i.field.starts_with("notifications.sounds")));
    }

    /// Migration von Schemaversion 1: der eine Klangpfad wird zum Klang für
    /// kritische Probleme. Ohne das verliert eine bestehende Konfiguration
    /// ihre Einstellung stillschweigend.
    #[test]
    fn der_alte_einzelne_tonpfad_wird_uebernommen() {
        let mut settings = Settings::default();
        settings.notifications.sound_path = Some(r"C:\ton.wav".into());
        // Die Vorgabe belegt `critical` mit einem eingebauten Klang; die
        // Migration darf sie nur ersetzen, wenn dort nichts gewählt ist.
        settings.notifications.sounds.critical = SoundChoice::None;
        settings.repair();

        assert_eq!(
            settings.notifications.sounds.critical,
            SoundChoice::File {
                path: r"C:\ton.wav".into()
            }
        );
        assert!(
            settings.notifications.sound_path.is_none(),
            "das alte Feld muss geleert werden, sonst migriert es jedes Mal neu"
        );
        assert_eq!(settings.schema_version, SCHEMA_VERSION);
    }

    /// Zweimal reparieren darf nichts kaputtmachen — `repair` läuft bei jedem
    /// Laden.
    #[test]
    fn die_migration_ist_wiederholbar() {
        let mut settings = Settings::default();
        settings.notifications.sound_path = Some(r"C:\ton.wav".into());
        settings.notifications.sounds.critical = SoundChoice::None;
        settings.repair();
        let nachher = settings.clone();
        settings.repair();
        assert_eq!(settings, nachher);
    }

    /// Eine schon getroffene Wahl wird von der Migration **nicht** überschrieben.
    #[test]
    fn die_migration_ueberschreibt_keine_bestehende_wahl() {
        let mut settings = Settings::default();
        settings.notifications.sound_path = Some(r"C:\alt.wav".into());
        settings.notifications.sounds.critical = SoundChoice::Builtin { id: "alarm".into() };
        settings.repair();
        assert_eq!(
            settings.notifications.sounds.critical,
            SoundChoice::Builtin { id: "alarm".into() }
        );
        assert!(settings.notifications.sound_path.is_none());
    }

    /// Ein leerer Pfad in einer Auswahl ist kein Pfad. Sonst entstünde eine
    /// Warnung über eine Datei namens "".
    #[test]
    fn reparatur_macht_aus_leeren_auswahlen_kein_ton() {
        let mut settings = Settings::default();
        settings.notifications.sounds.critical = SoundChoice::File { path: "  ".into() };
        settings.notifications.sounds.warning = SoundChoice::Builtin { id: "".into() };
        settings.repair();
        assert!(settings.notifications.sounds.critical.is_none());
        assert!(settings.notifications.sounds.warning.is_none());
    }

    /// Ein unbekannter eingebauter Klang muss auffallen — sonst bleibt es
    /// stumm und niemand weiss warum.
    #[test]
    fn eine_unbekannte_klangkennung_wird_gemeldet() {
        let mut settings = vollstaendig();
        settings.notifications.sounds.warning = SoundChoice::Builtin {
            id: "gibtsnicht".into(),
        };
        let issues = settings.validate();
        let ton: Vec<_> = issues
            .iter()
            .filter(|i| i.field == "notifications.sounds.warning")
            .collect();
        assert_eq!(ton.len(), 1, "{issues:?}");
        assert_eq!(ton[0].severity, IssueSeverity::Warning);
    }

    /// „Kein Ton" ist überall erlaubt und erzeugt keine Meldung.
    #[test]
    fn ueberall_kein_ton_ist_gueltig() {
        let mut settings = vollstaendig();
        for choice in settings.notifications.sounds.alle_mut() {
            *choice = SoundChoice::None;
        }
        assert!(settings
            .validate()
            .iter()
            .all(|i| !i.field.starts_with("notifications.sounds")));
    }

    /* ----------------------------------------------------------- Prüfung -- */

    #[test]
    fn vollstaendige_einstellungen_sind_gueltig() {
        let settings = vollstaendig();
        let issues = settings.validate();
        assert!(
            issues.iter().all(|i| i.severity == IssueSeverity::Warning),
            "unerwartete Fehler: {issues:?}"
        );
        assert!(settings.is_valid());
    }

    /// Alle fehlenden Felder auf einmal, nicht eines nach dem anderen.
    #[test]
    fn pruefung_meldet_alle_fehlenden_felder_gleichzeitig() {
        let settings = Settings::default();
        let issues = settings.validate();
        let felder: Vec<&str> = issues
            .iter()
            .filter(|i| i.severity == IssueSeverity::Error)
            .map(|i| i.field.as_str())
            .collect();
        assert!(felder.contains(&"connection.server"), "{felder:?}");
        assert!(felder.contains(&"connection.site"), "{felder:?}");
        assert!(felder.contains(&"connection.username"), "{felder:?}");
        assert!(!settings.is_valid());
    }

    /// Die URL-Prüfung des checkmk-Moduls muss durchschlagen.
    #[test]
    fn pruefung_erkennt_pfad_in_der_server_url() {
        let mut settings = vollstaendig();
        settings.active_mut().server = "https://checkmk.example.intern/leosys/check_mk".into();
        let issues = settings.validate();
        let fehler = issues
            .iter()
            .find(|i| i.field == "connection.server" && i.severity == IssueSeverity::Error)
            .expect("Fehler zur Server-URL fehlt");
        assert!(fehler.message.contains("Pfad"), "{}", fehler.message);
        assert!(!settings.is_valid());
    }

    /// Der Auftrag verlangt eine deutliche Warnung. Sie muss die Folge
    /// benennen, nicht nur „unsicher" sagen.
    #[test]
    fn abgeschaltete_tls_pruefung_warnt_deutlich() {
        let mut settings = vollstaendig();
        settings.active_mut().verify_tls = false;
        let warnung = settings
            .validate()
            .into_iter()
            .find(|i| i.field == "connection.verifyTls")
            .expect("Warnung zur TLS-Prüfung fehlt");

        assert_eq!(warnung.severity, IssueSeverity::Warning);
        assert!(
            warnung.message.contains("Zugangsdaten"),
            "Warnung benennt die Folge nicht: {}",
            warnung.message
        );
        // Eine Warnung darf das Speichern nicht blockieren.
        assert!(settings.is_valid());
    }

    #[test]
    fn manueller_proxy_ohne_adresse_ist_ein_fehler() {
        let mut settings = vollstaendig();
        settings.active_mut().proxy = ProxyConfig::Manual { url: "  ".into() };
        assert!(!settings.is_valid());
        assert!(settings
            .validate()
            .iter()
            .any(|i| i.field == "connection.proxy"));
    }

    #[test]
    fn zeitgrenze_groesser_als_intervall_warnt() {
        let mut settings = vollstaendig();
        settings.polling.interval_seconds = 15;
        settings.polling.timeout_seconds = 30;
        let warnung = settings
            .validate()
            .into_iter()
            .find(|i| i.field == "polling.timeoutSeconds")
            .expect("Warnung zur Zeitgrenze fehlt");
        assert_eq!(warnung.severity, IssueSeverity::Warning);
        assert!(settings.is_valid(), "Warnung darf nicht blockieren");
    }

    /* ---------------------------------------------------- Mehrinstanzfähig */

    /// Die Struktur muss mehrere Instanzen tragen, ohne geändert zu werden.
    #[test]
    fn struktur_traegt_mehrere_verbindungen() {
        let mut settings = vollstaendig();
        settings.connections.push(Connection {
            id: "zweite".into(),
            name: "Aussenstelle".into(),
            server: "https://checkmk2.example.intern".into(),
            site: "aus".into(),
            username: "m.mustermann".into(),
            ..Default::default()
        });
        settings.active_connection = 1;
        settings.repair();

        assert_eq!(settings.connections.len(), 2);
        assert_eq!(settings.active().display_name(), "Aussenstelle");

        // Rundreise durch JSON muss beides erhalten.
        let json = serde_json::to_string(&settings).unwrap();
        let zurueck: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(zurueck, settings);
    }

    #[test]
    fn anzeigename_faellt_auf_den_sitenamen_zurueck() {
        let mut connection = Connection {
            site: "leosys".into(),
            ..Default::default()
        };
        assert_eq!(connection.display_name(), "leosys");

        connection.name = "Zentrale".into();
        assert_eq!(connection.display_name(), "Zentrale");

        connection.name = "   ".into();
        assert_eq!(connection.display_name(), "leosys");

        connection.site = String::new();
        assert_eq!(connection.display_name(), "Unbenannt");
    }

    #[test]
    fn vollstaendigkeit_einer_verbindung() {
        let mut connection = Connection::default();
        assert!(!connection.is_complete());

        connection.server = "https://x.intern".into();
        connection.site = "s".into();
        assert!(!connection.is_complete(), "Benutzername fehlt noch");

        connection.username = "u".into();
        assert!(connection.is_complete());

        connection.username = "   ".into();
        assert!(!connection.is_complete(), "Leerzeichen sind kein Name");
    }

    /* ----------------------------------------------------- Proxy-Mapping -- */

    #[test]
    fn proxy_wird_auf_den_client_typ_abgebildet() {
        assert_eq!(ProxyMode::from(&ProxyConfig::System), ProxyMode::System);
        assert_eq!(ProxyMode::from(&ProxyConfig::None), ProxyMode::Disabled);
        assert_eq!(
            ProxyMode::from(&ProxyConfig::Manual {
                url: "http://proxy.intern:8080".into()
            }),
            ProxyMode::Manual("http://proxy.intern:8080".into())
        );
    }

    /// Das Wire-Format des Proxy muss stabil bleiben — es steht in der
    /// Konfigurationsdatei auf der Platte des Benutzers.
    #[test]
    fn proxy_wire_format_ist_stabil() {
        assert_eq!(
            serde_json::to_value(ProxyConfig::System).unwrap(),
            serde_json::json!({ "mode": "system" })
        );
        assert_eq!(
            serde_json::to_value(ProxyConfig::None).unwrap(),
            serde_json::json!({ "mode": "none" })
        );
        assert_eq!(
            serde_json::to_value(ProxyConfig::Manual {
                url: "http://p:8080".into()
            })
            .unwrap(),
            serde_json::json!({ "mode": "manual", "url": "http://p:8080" })
        );
    }

    /// Auch die Aufzählungen landen in der Datei. camelCase muss stabil sein.
    #[test]
    fn aufzaehlungen_serialisieren_in_camelcase() {
        assert_eq!(
            serde_json::to_value(NotificationLevel::CriticalOnly).unwrap(),
            serde_json::json!("criticalOnly")
        );
        assert_eq!(
            serde_json::to_value(NotificationLevel::AllChanges).unwrap(),
            serde_json::json!("allChanges")
        );
        assert_eq!(
            serde_json::to_value(NotificationLevel::Off).unwrap(),
            serde_json::json!("off")
        );
        assert_eq!(
            serde_json::to_value(ThemePreference::System).unwrap(),
            serde_json::json!("system")
        );
    }

    /* ------------------------------------------------------- Rundreise ---- */

    #[test]
    fn rundreise_durch_json_veraendert_nichts() {
        let mut original = vollstaendig();
        original.notifications.level = NotificationLevel::AllChanges;
        original.notifications.sound_path = Some("C:\\Windows\\Media\\Alarm01.wav".into());
        original.appearance.theme = ThemePreference::Dark;
        original.permissions.allow_acknowledge = true;
        original.behaviour.pin_popup = true;
        original.active_mut().proxy = ProxyConfig::Manual {
            url: "http://proxy.intern:8080".into(),
        };

        let json = serde_json::to_string_pretty(&original).unwrap();
        let zurueck: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(zurueck, original);
    }
}
