//! Laden und Speichern der Einstellungen.
//!
//! ## Erststart und Maschinenvorgaben
//!
//! Existiert `config.json` noch nicht, wird nach
//! `%ProgramData%\leosysr\Luchsr\defaults.json` gesucht. Diese Datei ist
//! **optional** und darf ein Teilausschnitt sein — typischerweise nur
//! Server-URL und Site, ausgerollt über Softwaremanagement. Sie wird über die
//! eingebauten Vorgaben gelegt, nicht an deren Stelle.
//!
//! Danach gehört die Konfiguration dem Benutzer: `defaults.json` wird nie
//! wieder gelesen. Eine spätere Änderung dort wirkt also nicht auf bestehende
//! Installationen — genau das verlangt der Auftrag mit „Danach vom Benutzer
//! überschreibbar".
//!
//! ## Warum atomar geschrieben wird
//!
//! Gespeichert wird in eine Nebendatei, die anschliessend über die echte
//! geschoben wird. Ein Absturz oder Stromausfall mitten im Schreiben
//! hinterlässt damit entweder die alte oder die neue Datei, nie eine halbe.
//! Eine halbe `config.json` würde beim nächsten Start als beschädigt gelten und
//! die Einstellungen des Benutzers kosten.
//!
//! ## Was bei einer beschädigten Datei passiert
//!
//! Sie wird zur Seite gelegt (`config.json.beschaedigt-1`) und die App startet
//! mit Vorgaben. Weder hart scheitern noch stillschweigend überschreiben: das
//! eine lähmt die App, das andere vernichtet Beweise. Der Vorfall wird als
//! Hinweis nach oben gegeben und im Dialog angezeigt.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

use super::error::ConfigError;
use super::paths::ConfigPaths;
use super::schema::Settings;

/// Woher die geladenen Einstellungen stammen.
///
/// Das Frontend entscheidet daran, ob der Ersteinrichtungs-Assistent kommt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingsSource {
    /// Bestehende `config.json` des Benutzers.
    UserConfig,
    /// Erststart, `defaults.json` war vorhanden und wurde übernommen.
    MachineDefaults,
    /// Erststart ohne Vorgabedatei — Ersteinrichtung nötig.
    FirstRun,
}

/// Ergebnis des Ladens.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadOutcome {
    pub settings: Settings,
    pub source: SettingsSource,
    /// Nicht-fatale Vorfälle, etwa eine beschädigte Datei, die zur Seite
    /// gelegt wurde. Werden im Dialog angezeigt.
    pub notices: Vec<String>,
    /// Ob der Ersteinrichtungs-Assistent gezeigt werden soll.
    ///
    /// Ein Feld und keine Methode, weil das Frontend die Antwort braucht und
    /// serde nur Felder serialisiert. Die Alternative — die Bedingung im
    /// Frontend nachbauen — wären zwei Wahrheiten für dieselbe Frage.
    pub needs_setup: bool,
}

impl LoadOutcome {
    fn new(settings: Settings, source: SettingsSource, notices: Vec<String>) -> Self {
        let needs_setup = source == SettingsSource::FirstRun || !settings.active().is_complete();
        Self {
            settings,
            source,
            notices,
            needs_setup,
        }
    }
}

/// Zugriff auf die Konfigurationsdateien.
#[derive(Debug, Clone)]
pub struct ConfigStore {
    paths: ConfigPaths,
}

impl ConfigStore {
    pub fn new(paths: ConfigPaths) -> Self {
        Self { paths }
    }

    /// Aus der Umgebung.
    pub fn from_environment() -> Result<Self, ConfigError> {
        Ok(Self::new(ConfigPaths::from_environment()?))
    }

    pub fn paths(&self) -> &ConfigPaths {
        &self.paths
    }

    /// Lädt die Einstellungen.
    ///
    /// Scheitert nur, wenn nicht einmal die Vorgaben zusammengebaut werden
    /// können. Beschädigte oder unlesbare Dateien führen zu einem Hinweis,
    /// nicht zu einem Fehler.
    pub fn load(&self) -> Result<LoadOutcome, ConfigError> {
        let mut notices = Vec::new();

        if self.paths.config_file.exists() {
            match self.read_json(&self.paths.config_file) {
                Ok(value) => match serde_json::from_value::<Settings>(value) {
                    Ok(mut settings) => {
                        settings.repair();
                        return Ok(LoadOutcome::new(
                            settings,
                            SettingsSource::UserConfig,
                            notices,
                        ));
                    }
                    Err(error) => {
                        notices.push(self.quarantine(
                            &self.paths.config_file,
                            &format!("Struktur nicht lesbar: {error}"),
                        ));
                    }
                },
                Err(error) => {
                    notices.push(self.quarantine(&self.paths.config_file, &error.to_string()));
                }
            }
        }

        // Erststart, oder die bestehende Datei war unbrauchbar.
        let (settings, source) = self.build_defaults(&mut notices);
        Ok(LoadOutcome::new(settings, source, notices))
    }

    /// Baut die Einstellungen aus den eingebauten Vorgaben plus `defaults.json`.
    fn build_defaults(&self, notices: &mut Vec<String>) -> (Settings, SettingsSource) {
        let mut base = serde_json::to_value(Settings::default())
            .expect("Settings::default ist immer serialisierbar");

        if !self.paths.defaults_file.exists() {
            let mut settings = Settings::default();
            settings.repair();
            return (settings, SettingsSource::FirstRun);
        }

        match self.read_json(&self.paths.defaults_file) {
            Ok(overlay) => {
                merge_json(&mut base, &overlay);
                match serde_json::from_value::<Settings>(base) {
                    Ok(mut settings) => {
                        settings.repair();
                        (settings, SettingsSource::MachineDefaults)
                    }
                    Err(error) => {
                        notices.push(format!(
                            "Die Maschinenvorgaben in {} sind nicht verwertbar ({error}). \
                             Luchsr startet mit den eingebauten Vorgaben.",
                            self.paths.defaults_file.display()
                        ));
                        let mut settings = Settings::default();
                        settings.repair();
                        (settings, SettingsSource::FirstRun)
                    }
                }
            }
            Err(error) => {
                notices.push(format!(
                    "Die Maschinenvorgaben konnten nicht gelesen werden ({error}). \
                     Luchsr startet mit den eingebauten Vorgaben."
                ));
                let mut settings = Settings::default();
                settings.repair();
                (settings, SettingsSource::FirstRun)
            }
        }
    }

    /// Speichert die Einstellungen atomar.
    ///
    /// Prüft vorher: ungültige Einstellungen werden nicht auf die Platte
    /// geschrieben. Warnungen blockieren nicht.
    pub fn save(&self, settings: &Settings) -> Result<(), ConfigError> {
        let mut settings = settings.clone();
        settings.repair();

        let issues = settings.validate();
        let fields: Vec<String> = issues
            .iter()
            .filter(|issue| issue.severity == super::schema::IssueSeverity::Error)
            .map(|issue| issue.field.clone())
            .collect();
        if !fields.is_empty() {
            let summary = issues
                .iter()
                .filter(|issue| issue.severity == super::schema::IssueSeverity::Error)
                .map(|issue| issue.message.as_str())
                .collect::<Vec<_>>()
                .join(" ");
            return Err(ConfigError::Invalid { summary, fields });
        }

        self.write_atomically(&settings)
    }

    /// Speichert ohne Prüfung.
    ///
    /// Wird für Zustandsflaggen gebraucht, die das Programm selbst setzt —
    /// etwa `autostart_initialised` beim allerersten Start, der passiert,
    /// bevor überhaupt eine Verbindung eingetragen ist.
    pub fn save_unchecked(&self, settings: &Settings) -> Result<(), ConfigError> {
        let mut settings = settings.clone();
        settings.repair();
        self.write_atomically(&settings)
    }

    fn write_atomically(&self, settings: &Settings) -> Result<(), ConfigError> {
        let dir = self.paths.config_dir();
        fs::create_dir_all(dir).map_err(|source| ConfigError::CreateDir {
            path: dir.to_path_buf(),
            source,
        })?;

        let json = serde_json::to_string_pretty(settings)
            .expect("Settings ist immer serialisierbar")
            // Windows-Zeilenenden: die Datei wird von Administratoren im
            // Notepad angesehen.
            .replace('\n', "\r\n")
            + "\r\n";

        let temp = self.paths.config_file.with_extension("json.tmp");
        fs::write(&temp, json.as_bytes()).map_err(|source| ConfigError::Write {
            path: temp.clone(),
            source,
        })?;

        // std::fs::rename ersetzt unter Windows eine vorhandene Datei.
        fs::rename(&temp, &self.paths.config_file).map_err(|source| {
            // Nebendatei nicht liegen lassen, wenn das Verschieben scheitert.
            let _ = fs::remove_file(&temp);
            ConfigError::Write {
                path: self.paths.config_file.clone(),
                source,
            }
        })
    }

    fn read_json(&self, path: &Path) -> Result<Value, ConfigError> {
        let text = fs::read_to_string(path).map_err(|source| ConfigError::Read {
            path: path.to_path_buf(),
            source,
        })?;
        // Eine leere Datei ist wie eine fehlende Datei, nicht wie ein Fehler.
        if text.trim().is_empty() {
            return Ok(Value::Object(serde_json::Map::new()));
        }
        serde_json::from_str(&text).map_err(|source| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })
    }

    /// Legt eine unbrauchbare Datei zur Seite und gibt den Hinweistext zurück.
    fn quarantine(&self, path: &Path, reason: &str) -> String {
        match free_quarantine_name(path) {
            Some(target) => match fs::rename(path, &target) {
                Ok(()) => format!(
                    "Die Konfigurationsdatei war unbrauchbar ({reason}). Sie wurde nach \
                     {} verschoben; Luchsr startet mit Vorgaben.",
                    target.display()
                ),
                Err(error) => format!(
                    "Die Konfigurationsdatei ist unbrauchbar ({reason}) und liess sich \
                     nicht zur Seite legen ({error}). Luchsr startet mit Vorgaben, \
                     überschreibt die Datei aber beim nächsten Speichern."
                ),
            },
            None => format!(
                "Die Konfigurationsdatei ist unbrauchbar ({reason}). Es liess sich kein \
                 freier Name für eine Sicherung finden; Luchsr startet mit Vorgaben."
            ),
        }
    }
}

/// Sucht einen freien Namen für die Quarantäne.
///
/// Durchnummeriert statt Zeitstempel: das ist ohne Uhr testbar und die
/// Reihenfolge bleibt beim Draufschauen erkennbar.
fn free_quarantine_name(path: &Path) -> Option<PathBuf> {
    let base = path.to_string_lossy().to_string();
    for index in 1..=99 {
        let candidate = PathBuf::from(format!("{base}.beschaedigt-{index}"));
        if !candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Legt `overlay` rekursiv über `base`.
///
/// Objekte werden verschmolzen, alles andere ersetzt. Arrays absichtlich
/// ersetzt und nicht angehängt: eine `defaults.json`, die `connections` setzt,
/// meint „diese Verbindungen", nicht „zusätzlich zu den eingebauten".
fn merge_json(base: &mut Value, overlay: &Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, overlay_value) in overlay_map {
                match base_map.get_mut(key) {
                    Some(base_value) => merge_json(base_value, overlay_value),
                    None => {
                        base_map.insert(key.clone(), overlay_value.clone());
                    }
                }
            }
        }
        (base_slot, overlay_value) => *base_slot = overlay_value.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{NotificationLevel, ProxyConfig, ThemePreference};
    use tempfile::TempDir;

    fn store() -> (TempDir, ConfigStore) {
        let dir = TempDir::new().expect("Temp-Verzeichnis");
        let store = ConfigStore::new(ConfigPaths::below(dir.path()));
        (dir, store)
    }

    fn schreibe(path: &Path, inhalt: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, inhalt).unwrap();
    }

    fn vollstaendig() -> Settings {
        let mut settings = Settings::default();
        let connection = settings.active_mut();
        connection.server = "https://checkmk.example.intern".into();
        connection.site = "leosys".into();
        connection.username = "m.mustermann".into();
        settings
    }

    /* ------------------------------------------------------- JSON-Merge --- */

    #[test]
    fn merge_verschmilzt_objekte_in_der_tiefe() {
        let mut base = serde_json::json!({
            "a": 1,
            "nested": { "x": 1, "y": 2 }
        });
        merge_json(
            &mut base,
            &serde_json::json!({ "nested": { "y": 99, "z": 3 } }),
        );
        assert_eq!(
            base,
            serde_json::json!({ "a": 1, "nested": { "x": 1, "y": 99, "z": 3 } })
        );
    }

    /// Arrays ersetzen, nicht anhängen: sonst hätte eine defaults.json mit
    /// einer Verbindung am Ende zwei.
    #[test]
    fn merge_ersetzt_arrays_statt_sie_anzuhaengen() {
        let mut base = serde_json::json!({ "list": [1, 2, 3] });
        merge_json(&mut base, &serde_json::json!({ "list": [9] }));
        assert_eq!(base, serde_json::json!({ "list": [9] }));
    }

    #[test]
    fn merge_ueberschreibt_mit_null_wenn_ausdruecklich() {
        let mut base = serde_json::json!({ "a": 1 });
        merge_json(&mut base, &serde_json::json!({ "a": null }));
        assert_eq!(base, serde_json::json!({ "a": null }));
    }

    /* ---------------------------------------------------------- Erststart */

    #[test]
    fn erststart_ohne_vorgabedatei_verlangt_ersteinrichtung() {
        let (_dir, store) = store();
        let outcome = store.load().unwrap();

        assert_eq!(outcome.source, SettingsSource::FirstRun);
        assert!(outcome.needs_setup, "Assistent muss kommen");
        assert!(outcome.notices.is_empty());
        assert_eq!(outcome.settings, {
            let mut s = Settings::default();
            s.repair();
            s
        });
    }

    /// Der Fall der Massenverteilung: defaults.json setzt nur Server und Site.
    #[test]
    fn maschinenvorgaben_werden_beim_erststart_uebernommen() {
        let (_dir, store) = store();
        schreibe(
            &store.paths().defaults_file,
            r#"{
                "connections": [{
                    "id": "default",
                    "server": "https://checkmk.leosys.intern",
                    "site": "leosys"
                }]
            }"#,
        );

        let outcome = store.load().unwrap();
        assert_eq!(outcome.source, SettingsSource::MachineDefaults);
        assert_eq!(
            outcome.settings.active().server,
            "https://checkmk.leosys.intern"
        );
        assert_eq!(outcome.settings.active().site, "leosys");

        // Nicht gesetzte Felder behalten die eingebauten Vorgaben.
        assert!(outcome.settings.active().verify_tls);
        assert_eq!(outcome.settings.polling.interval_seconds, 60);
        assert!(!outcome.settings.permissions.allow_acknowledge);

        // Der Benutzername fehlt noch, also weiterhin Ersteinrichtung.
        assert!(outcome.needs_setup);
    }

    #[test]
    fn maschinenvorgaben_koennen_auch_andere_bereiche_setzen() {
        let (_dir, store) = store();
        schreibe(
            &store.paths().defaults_file,
            r#"{
                "polling": { "intervalSeconds": 120 },
                "permissions": { "allowAcknowledge": true },
                "notifications": { "level": "allChanges" },
                "appearance": { "theme": "dark" }
            }"#,
        );

        let settings = store.load().unwrap().settings;
        assert_eq!(settings.polling.interval_seconds, 120);
        assert!(settings.permissions.allow_acknowledge);
        assert_eq!(settings.notifications.level, NotificationLevel::AllChanges);
        assert_eq!(settings.appearance.theme, ThemePreference::Dark);
    }

    /// Ein unbrauchbares defaults.json darf den Start nicht verhindern.
    #[test]
    fn kaputte_maschinenvorgaben_werden_gemeldet_nicht_fatal() {
        let (_dir, store) = store();
        schreibe(&store.paths().defaults_file, "{ kein json");

        let outcome = store.load().unwrap();
        assert_eq!(outcome.source, SettingsSource::FirstRun);
        assert_eq!(outcome.notices.len(), 1);
        assert!(
            outcome.notices[0].contains("Maschinenvorgaben"),
            "{:?}",
            outcome.notices
        );
    }

    /// Maschinenvorgaben werden nach dem Erststart nie wieder gelesen.
    #[test]
    fn maschinenvorgaben_wirken_nicht_mehr_wenn_config_existiert() {
        let (_dir, store) = store();
        schreibe(
            &store.paths().defaults_file,
            r#"{ "polling": { "intervalSeconds": 300 } }"#,
        );

        // Benutzer speichert eigene Einstellungen.
        let mut eigene = vollstaendig();
        eigene.polling.interval_seconds = 30;
        store.save(&eigene).unwrap();

        let outcome = store.load().unwrap();
        assert_eq!(outcome.source, SettingsSource::UserConfig);
        assert_eq!(
            outcome.settings.polling.interval_seconds, 30,
            "die Vorgabe der Maschine darf die Wahl des Benutzers nicht überschreiben"
        );
    }

    /* ----------------------------------------------------------- Rundreise */

    #[test]
    fn speichern_und_laden_erhaelt_alles() {
        let (_dir, store) = store();
        let mut original = vollstaendig();
        original.polling.interval_seconds = 45;
        original.notifications.level = NotificationLevel::AllChanges;
        original.notifications.sound_path = None;
        original.appearance.theme = ThemePreference::Dark;
        original.permissions.allow_downtime = true;
        original.behaviour.pin_popup = true;
        original.active_mut().proxy = ProxyConfig::Manual {
            url: "http://proxy.intern:8080".into(),
        };

        store.save(&original).unwrap();
        let geladen = store.load().unwrap();

        assert_eq!(geladen.source, SettingsSource::UserConfig);
        assert_eq!(geladen.settings, original);
        assert!(!geladen.needs_setup);
    }

    #[test]
    fn speichern_legt_das_verzeichnis_an() {
        let (_dir, store) = store();
        assert!(!store.paths().config_dir().exists());
        store.save(&vollstaendig()).unwrap();
        assert!(store.paths().config_file.exists());
    }

    /// Die Datei wird von Administratoren im Notepad angesehen.
    #[test]
    fn gespeicherte_datei_hat_windows_zeilenenden_und_ist_eingerueckt() {
        let (_dir, store) = store();
        store.save(&vollstaendig()).unwrap();
        let text = fs::read_to_string(&store.paths().config_file).unwrap();

        assert!(text.contains("\r\n"), "keine CRLF-Zeilenenden");
        assert!(
            !text.contains("\n\n") && !text.replace("\r\n", "").contains('\n'),
            "es gibt nackte LF-Zeilenenden"
        );
        assert!(text.contains("  \"schemaVersion\"") || text.contains("\"schemaVersion\""));
        assert!(text.ends_with("\r\n"));
    }

    /// Die Zusicherung aus dem Schema, hier gegen die echte Datei geprüft.
    #[test]
    fn gespeicherte_datei_enthaelt_kein_secret() {
        let (_dir, store) = store();
        store.save(&vollstaendig()).unwrap();
        let text = fs::read_to_string(&store.paths().config_file)
            .unwrap()
            .to_lowercase();
        for verdaechtig in ["secret", "password", "passwort", "kennwort", "token"] {
            assert!(
                !text.contains(verdaechtig),
                "„{verdaechtig}“ steht in der Datei"
            );
        }
    }

    #[test]
    fn keine_nebendatei_bleibt_liegen() {
        let (_dir, store) = store();
        store.save(&vollstaendig()).unwrap();
        let temp = store.paths().config_file.with_extension("json.tmp");
        assert!(!temp.exists(), "Nebendatei wurde nicht aufgeräumt");
    }

    /* -------------------------------------------------------- Beschädigung */

    #[test]
    fn beschaedigte_datei_wird_zur_seite_gelegt_und_gemeldet() {
        let (_dir, store) = store();
        schreibe(&store.paths().config_file, "{ das ist kein json");

        let outcome = store.load().unwrap();

        assert_eq!(outcome.source, SettingsSource::FirstRun);
        assert_eq!(outcome.notices.len(), 1);
        assert!(
            outcome.notices[0].contains("beschaedigt-1")
                || outcome.notices[0].contains("beschädigt-1"),
            "{:?}",
            outcome.notices
        );
        assert!(
            !store.paths().config_file.exists(),
            "die kaputte Datei muss weg sein"
        );
        let sicherung = PathBuf::from(format!(
            "{}.beschaedigt-1",
            store.paths().config_file.to_string_lossy()
        ));
        assert!(sicherung.exists(), "die Sicherung fehlt");
        assert_eq!(
            fs::read_to_string(&sicherung).unwrap(),
            "{ das ist kein json",
            "der Inhalt muss unverändert erhalten bleiben"
        );
    }

    /// Gültiges JSON, aber falsche Struktur — etwa ein Zahlenfeld als Text.
    #[test]
    fn strukturell_falsche_datei_wird_ebenfalls_zur_seite_gelegt() {
        let (_dir, store) = store();
        schreibe(
            &store.paths().config_file,
            r#"{ "polling": { "intervalSeconds": "sechzig" } }"#,
        );

        let outcome = store.load().unwrap();
        assert_eq!(outcome.source, SettingsSource::FirstRun);
        assert!(outcome.notices[0].contains("Struktur") || !outcome.notices.is_empty());
    }

    #[test]
    fn mehrere_beschaedigungen_durchnummerieren_sich() {
        let (_dir, store) = store();
        for _ in 0..3 {
            schreibe(&store.paths().config_file, "{ kaputt");
            store.load().unwrap();
        }
        let base = store.paths().config_file.to_string_lossy().to_string();
        for index in 1..=3 {
            assert!(
                PathBuf::from(format!("{base}.beschaedigt-{index}")).exists(),
                "Sicherung {index} fehlt"
            );
        }
    }

    /// Eine leere Datei ist wie keine Datei — kein Fehler, keine Quarantäne.
    #[test]
    fn leere_datei_ergibt_vorgaben_ohne_quarantaene() {
        let (_dir, store) = store();
        schreibe(&store.paths().config_file, "   \r\n  ");

        let outcome = store.load().unwrap();
        assert!(
            outcome.notices.is_empty(),
            "leere Datei ist kein Vorfall: {:?}",
            outcome.notices
        );
        assert_eq!(outcome.settings.polling.interval_seconds, 60);
    }

    /* ------------------------------------------------------------ Prüfung */

    #[test]
    fn ungueltige_einstellungen_landen_nicht_auf_der_platte() {
        let (_dir, store) = store();
        let leer = Settings::default(); // ohne Server, Site, Benutzer

        let error = store.save(&leer).unwrap_err();
        match &error {
            ConfigError::Invalid { fields, .. } => {
                assert!(
                    fields.contains(&"connection.server".to_string()),
                    "{fields:?}"
                );
                assert!(
                    fields.contains(&"connection.username".to_string()),
                    "{fields:?}"
                );
            }
            other => panic!("erwartet wurde Invalid, gelesen wurde {other:?}"),
        }
        assert!(
            !store.paths().config_file.exists(),
            "es darf nichts geschrieben worden sein"
        );
    }

    /// Warnungen dürfen nicht blockieren — sonst liesse sich die
    /// TLS-Prüfung nie abschalten.
    #[test]
    fn warnungen_blockieren_das_speichern_nicht() {
        let (_dir, store) = store();
        let mut settings = vollstaendig();
        settings.active_mut().verify_tls = false;

        store.save(&settings).unwrap();
        assert!(!store.load().unwrap().settings.active().verify_tls);
    }

    /// Zustandsflaggen des Programms müssen auch vor der Ersteinrichtung
    /// gespeichert werden können.
    #[test]
    fn save_unchecked_speichert_auch_unvollstaendiges() {
        let (_dir, store) = store();
        let mut settings = Settings::default();
        settings.behaviour.autostart_initialised = true;

        store.save_unchecked(&settings).unwrap();
        let geladen = store.load().unwrap();
        assert!(geladen.settings.behaviour.autostart_initialised);
        assert!(geladen.needs_setup, "unvollständig bleibt unvollständig");
    }

    /// Beim Speichern wird geklemmt, nicht abgelehnt.
    #[test]
    fn speichern_repariert_werte_ausserhalb_der_grenzen() {
        let (_dir, store) = store();
        let mut settings = vollstaendig();
        settings.polling.interval_seconds = 5;

        store.save(&settings).unwrap();
        assert_eq!(store.load().unwrap().settings.polling.interval_seconds, 15);
    }

    /* ------------------------------------------------- Ersteinrichtung ---- */

    /// Eine vollständige Verbindung braucht keinen Assistenten, eine
    /// unvollständige schon — auch wenn config.json existiert.
    #[test]
    fn ersteinrichtung_haengt_an_der_vollstaendigkeit_nicht_nur_an_der_datei() {
        let (_dir, store) = store();
        let mut settings = vollstaendig();
        store.save(&settings).unwrap();
        assert!(!store.load().unwrap().needs_setup);

        settings.active_mut().username = String::new();
        store.save_unchecked(&settings).unwrap();
        let outcome = store.load().unwrap();
        assert_eq!(outcome.source, SettingsSource::UserConfig);
        assert!(
            outcome.needs_setup,
            "ohne Benutzernamen ist die Einrichtung nicht fertig"
        );
    }
}
