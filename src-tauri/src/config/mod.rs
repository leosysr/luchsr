//! Konfiguration und Zugangsdaten.
//!
//! | Datei         | Inhalt                                                    |
//! |---------------|-----------------------------------------------------------|
//! | [`schema`]    | Datenmodell der Einstellungen, Vorgaben, Prüfung          |
//! | [`paths`]     | Ablageorte unter `%APPDATA%` und `%ProgramData%`          |
//! | [`store`]     | Laden, atomares Speichern, Maschinenvorgaben              |
//! | [`secrets`]   | Automation-Secret im Windows Credential Manager            |
//! | [`error`]     | Fehlertypen beider Bereiche                               |
//!
//! ## Die Trennlinie, auf die es ankommt
//!
//! [`schema::Settings`] hat **kein Feld für das Secret**. Es ist damit
//! strukturell unmöglich, dass es in `config.json` landet — das ist keine
//! Frage der Disziplin, sondern des Typs. Das Secret geht ausschliesslich über
//! [`secrets::SecretStore`], und das ist die einzige Stelle im Programm, die es
//! anfasst.

pub mod error;
pub mod paths;
pub mod schema;
pub mod secrets;
pub mod store;

pub use error::{ConfigError, SecretError};
pub use paths::ConfigPaths;
pub use schema::{
    AppearanceSettings, BehaviourSettings, Connection, IssueSeverity, NotificationLevel,
    NotificationSettings, PermissionSettings, PollingSettings, ProxyConfig, Settings, SoundChoice,
    SoundSettings, ThemePreference, ValidationIssue, INTERVAL_DEFAULT_SECONDS,
    INTERVAL_MAX_SECONDS, INTERVAL_MIN_SECONDS, SCHEMA_VERSION,
};
pub use secrets::{SecretStore, SERVICE_NAME};
pub use store::{ConfigStore, LoadOutcome, SettingsSource};
