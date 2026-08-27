//! Fehlertypen der Konfigurationsschicht.
//!
//! Wie im `checkmk`-Modul: konkrete Meldungen, keine generischen. Diese Texte
//! landen im Einstellungsdialog und müssen dem Benutzer sagen, was zu tun ist —
//! und dem Administrator, wo er nachsehen muss.

use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    /// `%APPDATA%` oder `%ProgramData%` fehlt. Praktisch nur in kaputten
    /// Dienstkontexten oder bei stark eingeschränkten Umgebungen.
    #[error("Die Umgebungsvariable %{variable}% ist nicht verfügbar ({reason}). Luchsr kann seinen Ablageort nicht bestimmen.")]
    MissingEnvironment { variable: String, reason: String },

    #[error("Das Verzeichnis {} liess sich nicht anlegen: {source}", path.display())]
    CreateDir {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Die Datei {} liess sich nicht lesen: {source}", path.display())]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Die Datei {} liess sich nicht schreiben: {source}", path.display())]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Die Datei {} enthält kein gültiges JSON: {source}", path.display())]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },

    /// Die Einstellungen sind unvollständig oder widersprüchlich. Enthält die
    /// Feldpfade, damit der Dialog markieren kann.
    #[error("Die Einstellungen sind unvollständig: {}", summary)]
    Invalid {
        summary: String,
        fields: Vec<String>,
    },
}

/// Fehler beim Zugriff auf den Windows Credential Manager.
#[derive(Debug, thiserror::Error)]
pub enum SecretError {
    /// Der Credential-Store ist gar nicht verfügbar. Sollte unter Windows nicht
    /// vorkommen; wenn doch, ist das eine Systemstörung und keine Fehleingabe.
    #[error("Der Windows Credential Manager ist nicht verfügbar: {reason}. Ohne ihn kann das Automation-Secret nicht gespeichert werden.")]
    StoreUnavailable { reason: String },

    #[error("Für den Benutzer „{username}“ ist kein Automation-Secret gespeichert.")]
    NotFound { username: String },

    #[error("Das Automation-Secret für „{username}“ liess sich nicht speichern: {reason}")]
    WriteFailed { username: String, reason: String },

    #[error("Das Automation-Secret für „{username}“ liess sich nicht lesen: {reason}")]
    ReadFailed { username: String, reason: String },

    #[error("Das Automation-Secret für „{username}“ liess sich nicht löschen: {reason}")]
    DeleteFailed { username: String, reason: String },

    #[error("Es ist kein Benutzername angegeben. Ohne Benutzernamen gibt es keinen Eintrag im Credential Manager.")]
    NoUsername,
}
