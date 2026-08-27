//! Ablageorte der Konfiguration.
//!
//! Windows-only, deshalb direkt über die Umgebungsvariablen statt über eine
//! Plattformabstraktion. Das ist genau die plattformspezifische Vereinfachung,
//! die der Auftrag erlaubt — und `%APPDATA%` ist unter Windows die verbindliche
//! Auskunft, auch bei umgeleiteten Profilen oder Ordnerumleitung per GPO.
//!
//! | Zweck                | Pfad                                          |
//! |----------------------|-----------------------------------------------|
//! | Benutzereinstellungen| `%APPDATA%\leosysr\Luchsr\config.json`         |
//! | Maschinenvorgaben    | `%ProgramData%\leosysr\Luchsr\defaults.json`   |

use std::path::PathBuf;

use super::error::ConfigError;

/// Herstellerordner, siehe Namenskonventionen in CLAUDE.md.
pub const VENDOR_DIR: &str = "leosysr";
/// Produktordner.
pub const PRODUCT_DIR: &str = "Luchsr";
pub const CONFIG_FILE: &str = "config.json";
pub const DEFAULTS_FILE: &str = "defaults.json";

/// Die beiden Ablageorte, gebündelt.
///
/// Als Struktur und nicht als freie Funktionen, damit Tests eigene Pfade
/// einsetzen können, ohne Umgebungsvariablen zu verbiegen — das wäre in
/// parallel laufenden Tests unzuverlässig.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConfigPaths {
    /// `%APPDATA%\leosysr\Luchsr\config.json`
    pub config_file: PathBuf,
    /// `%ProgramData%\leosysr\Luchsr\defaults.json`
    pub defaults_file: PathBuf,
}

impl ConfigPaths {
    /// Ermittelt die Pfade aus der Umgebung.
    pub fn from_environment() -> Result<Self, ConfigError> {
        let appdata = env_dir("APPDATA")?;
        let programdata = env_dir("ProgramData")?;
        Ok(Self {
            config_file: appdata.join(VENDOR_DIR).join(PRODUCT_DIR).join(CONFIG_FILE),
            defaults_file: programdata
                .join(VENDOR_DIR)
                .join(PRODUCT_DIR)
                .join(DEFAULTS_FILE),
        })
    }

    /// Für Tests: beide Dateien unterhalb eines Wurzelverzeichnisses.
    pub fn below(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        Self {
            config_file: root.join("appdata").join(CONFIG_FILE),
            defaults_file: root.join("programdata").join(DEFAULTS_FILE),
        }
    }

    /// Verzeichnis, in dem `config.json` liegt. Wird beim Speichern angelegt.
    pub fn config_dir(&self) -> &std::path::Path {
        self.config_file
            .parent()
            .expect("config_file hat immer ein Elternverzeichnis")
    }
}

fn env_dir(name: &str) -> Result<PathBuf, ConfigError> {
    match std::env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(PathBuf::from(value)),
        Ok(_) => Err(ConfigError::MissingEnvironment {
            variable: name.to_string(),
            reason: "die Variable ist leer".to_string(),
        }),
        Err(error) => Err(ConfigError::MissingEnvironment {
            variable: name.to_string(),
            reason: error.to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pfade_folgen_den_namenskonventionen() {
        let paths = ConfigPaths::below("C:\\temp\\luchsr-test");

        let config = paths.config_file.to_string_lossy().replace('/', "\\");
        assert!(config.ends_with("config.json"), "{config}");

        let defaults = paths.defaults_file.to_string_lossy().replace('/', "\\");
        assert!(defaults.ends_with("defaults.json"), "{defaults}");
    }

    /// Auf diesem Rechner müssen beide Variablen gesetzt sein; der Test hält
    /// fest, dass die Pfadstruktur dem Auftrag entspricht.
    #[test]
    fn umgebungspfade_enthalten_hersteller_und_produkt() {
        let paths = ConfigPaths::from_environment()
            .expect("APPDATA und ProgramData müssen unter Windows gesetzt sein");

        let config = paths.config_file.to_string_lossy().to_string();
        assert!(config.contains(VENDOR_DIR), "Hersteller fehlt in {config}");
        assert!(config.contains(PRODUCT_DIR), "Produkt fehlt in {config}");
        assert!(config.ends_with(CONFIG_FILE), "{config}");

        let defaults = paths.defaults_file.to_string_lossy().to_string();
        assert!(defaults.contains(VENDOR_DIR), "{defaults}");
        assert!(defaults.contains(PRODUCT_DIR), "{defaults}");
        assert!(defaults.ends_with(DEFAULTS_FILE), "{defaults}");

        // Die beiden dürfen nicht im selben Verzeichnis liegen: das eine ist
        // benutzer-, das andere maschinenweit.
        assert_ne!(paths.config_file, paths.defaults_file);
    }

    #[test]
    fn config_dir_ist_das_elternverzeichnis() {
        let paths = ConfigPaths::below("C:\\temp\\x");
        assert_eq!(paths.config_dir(), paths.config_file.parent().unwrap());
    }
}
