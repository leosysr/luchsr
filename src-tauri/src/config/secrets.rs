//! Das Automation-Secret im Windows Credential Manager.
//!
//! Der Auftrag ist hier eindeutig: das Secret liegt **ausschliesslich** im
//! Credential Manager, nie in `config.json`, nie in Logs, nie im
//! Frontend-State. Diese Datei ist die einzige Stelle im Programm, die es
//! anfasst.
//!
//! | Eigenschaft | Wert                       |
//! |-------------|----------------------------|
//! | Service     | `leosysr.Luchsr`           |
//! | Account     | der Benutzername           |
//!
//! ## Wie keyring 4 den Store initialisiert
//!
//! Anders als in einer früheren Notiz vermutet, muss der Default-Store **nicht**
//! von Hand registriert werden. Mit dem Feature `v1` erledigt
//! `keyring::Entry::new` das beim ersten Aufruf über einen `LazyLock`.
//!
//! Es gibt aber `keyring::Entry::store_status()`, das die Initialisierung
//! anstösst und ihr Ergebnis zurückgibt, ohne einen Eintrag anzulegen. Genau
//! das nutzt [`SecretStore::availability`] für eine Prüfung beim Start: fehlt
//! der Credential Manager, soll der Benutzer das als klare Meldung sehen und
//! nicht erst beim Speichern seines Secrets.
//!
//! ## Mehrere CheckMK-Instanzen
//!
//! Der Auftrag legt „Account = Benutzername" fest, und genau das macht
//! [`account_for`]. Bei mehreren Instanzen mit demselben Benutzernamen auf
//! verschiedenen Servern würden sich die Einträge überschreiben. Damit das
//! später eine Einzeiländerung bleibt, läuft **jeder** Zugriff über diese eine
//! Funktion — es gibt keine zweite Stelle, die den Schlüssel bildet.

use keyring::Entry;
use keyring::Error as KeyringError;

use crate::checkmk::Secret;

use super::error::SecretError;

/// Service-Name im Credential Manager, siehe Namenskonventionen.
pub const SERVICE_NAME: &str = "leosysr.Luchsr";

/// Bildet den Account-Schlüssel.
///
/// Aktuell der Benutzername, wie im Auftrag festgelegt. Die einzige Stelle,
/// die den Schlüssel bildet — siehe Modulkommentar.
pub fn account_for(username: &str) -> String {
    username.trim().to_string()
}

/// Zugriff auf das Automation-Secret.
///
/// Zustandslos: der Credential Manager ist die Quelle der Wahrheit, ein
/// Zwischenspeicher wäre eine zweite Kopie des Secrets im Prozessspeicher.
#[derive(Debug, Clone, Copy, Default)]
pub struct SecretStore;

impl SecretStore {
    /// Prüft, ob der Credential Manager überhaupt nutzbar ist.
    ///
    /// Stösst die einmalige Initialisierung an, ohne einen Eintrag anzulegen.
    pub fn availability() -> Result<(), SecretError> {
        match Entry::store_status() {
            Ok(()) => Ok(()),
            Err(error) => Err(SecretError::StoreUnavailable {
                reason: error.to_string(),
            }),
        }
    }

    /// Speichert das Secret.
    ///
    /// Ein leeres Secret löscht den Eintrag statt einen leeren zu schreiben:
    /// ein leerer Eintrag würde später als „vorhanden" gelten und einen
    /// 401 erzeugen, dessen Ursache nicht zu erraten ist.
    pub fn store(username: &str, secret: &Secret) -> Result<(), SecretError> {
        let account = Self::checked_account(username)?;

        if secret.is_empty() {
            return match Self::delete(username) {
                Ok(()) | Err(SecretError::NotFound { .. }) => Ok(()),
                Err(other) => Err(other),
            };
        }

        let entry = Self::entry(&account)?;
        entry
            .set_password(secret.expose())
            .map_err(|error| SecretError::WriteFailed {
                username: account,
                reason: describe(&error),
            })
    }

    /// Liest das Secret.
    pub fn load(username: &str) -> Result<Secret, SecretError> {
        let account = Self::checked_account(username)?;
        let entry = Self::entry(&account)?;
        match entry.get_password() {
            Ok(value) => Ok(Secret::new(value)),
            Err(KeyringError::NoEntry) => Err(SecretError::NotFound { username: account }),
            Err(error) => Err(SecretError::ReadFailed {
                username: account,
                reason: describe(&error),
            }),
        }
    }

    /// Ob ein Secret gespeichert ist.
    ///
    /// Gibt **nur** einen Wahrheitswert zurück. Die Oberfläche soll anzeigen
    /// können „ist gesetzt", ohne dass das Secret dafür durch die Schichten
    /// wandert.
    pub fn exists(username: &str) -> Result<bool, SecretError> {
        match Self::load(username) {
            Ok(secret) => Ok(!secret.is_empty()),
            Err(SecretError::NotFound { .. }) => Ok(false),
            Err(SecretError::NoUsername) => Ok(false),
            Err(other) => Err(other),
        }
    }

    /// Löscht das Secret.
    pub fn delete(username: &str) -> Result<(), SecretError> {
        let account = Self::checked_account(username)?;
        let entry = Self::entry(&account)?;
        match entry.delete_credential() {
            Ok(()) => Ok(()),
            Err(KeyringError::NoEntry) => Err(SecretError::NotFound { username: account }),
            Err(error) => Err(SecretError::DeleteFailed {
                username: account,
                reason: describe(&error),
            }),
        }
    }

    fn checked_account(username: &str) -> Result<String, SecretError> {
        let account = account_for(username);
        if account.is_empty() {
            return Err(SecretError::NoUsername);
        }
        Ok(account)
    }

    fn entry(account: &str) -> Result<Entry, SecretError> {
        Entry::new(SERVICE_NAME, account).map_err(|error| match error {
            KeyringError::NoDefaultStore => SecretError::StoreUnavailable {
                reason: "der plattformeigene Credential-Store liess sich nicht initialisieren"
                    .to_string(),
            },
            other => SecretError::StoreUnavailable {
                reason: describe(&other),
            },
        })
    }
}

/// Übersetzt einen keyring-Fehler in deutschen Klartext.
///
/// Die Rohmeldungen sind englisch und teils sehr technisch; im
/// Einstellungsdialog hilft das niemandem.
fn describe(error: &KeyringError) -> String {
    match error {
        KeyringError::NoEntry => "es ist kein Eintrag vorhanden".to_string(),
        KeyringError::NoDefaultStore => {
            "der plattformeigene Credential-Store ist nicht verfügbar".to_string()
        }
        KeyringError::NoStorageAccess(inner) => {
            format!("der Zugriff auf den Credential Manager wurde verweigert ({inner})")
        }
        KeyringError::PlatformFailure(inner) => {
            format!("der Credential Manager hat einen Fehler gemeldet ({inner})")
        }
        KeyringError::Ambiguous(entries) => format!(
            "es gibt {} widersprüchliche Einträge mit denselben Merkmalen; \
             bitte im Windows Credential Manager unter „{SERVICE_NAME}“ aufräumen",
            entries.len()
        ),
        KeyringError::TooLong(what, limit) => {
            format!("„{what}“ ist zu lang, erlaubt sind höchstens {limit} Zeichen")
        }
        KeyringError::Invalid(what, why) => format!("„{what}“ ist unzulässig: {why}"),
        KeyringError::BadEncoding(_) => {
            "der gespeicherte Wert ist kein gültiger Text und stammt vermutlich nicht von Luchsr"
                .to_string()
        }
        KeyringError::BadDataFormat(_, inner) => {
            format!("der gespeicherte Wert hat ein unerwartetes Format ({inner})")
        }
        KeyringError::BadStoreFormat(what) => {
            format!("der Credential-Store hat ein unerwartetes Format ({what})")
        }
        KeyringError::NotSupportedByStore(what) => {
            format!("der Credential Manager unterstützt „{what}“ nicht")
        }
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Eindeutiger Account je Test, damit parallele Tests sich nicht ins
    /// Gehege kommen. Das Präfix macht Rückstände im Credential Manager
    /// sofort erkennbar.
    fn test_account(suffix: &str) -> String {
        format!("luchsr-selbsttest-{suffix}")
    }

    /// Räumt am Ende auf, auch wenn ein Test panisch wird.
    struct Cleanup(String);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            let _ = SecretStore::delete(&self.0);
        }
    }

    /* ------------------------------------------------ Reine Funktionen --- */

    #[test]
    fn service_name_entspricht_den_namenskonventionen() {
        assert_eq!(SERVICE_NAME, "leosysr.Luchsr");
    }

    /// Der Auftrag legt „Account = Benutzername" fest.
    #[test]
    fn account_ist_der_benutzername() {
        assert_eq!(account_for("m.mustermann"), "m.mustermann");
        assert_eq!(account_for("  m.mustermann  "), "m.mustermann");
        assert_eq!(account_for(""), "");
        assert_eq!(account_for("   "), "");
    }

    #[test]
    fn leerer_benutzername_wird_abgewiesen() {
        for leer in ["", "   ", "\t"] {
            assert!(matches!(
                SecretStore::store(leer, &Secret::new("x")),
                Err(SecretError::NoUsername)
            ));
            assert!(matches!(
                SecretStore::load(leer),
                Err(SecretError::NoUsername)
            ));
            assert!(matches!(
                SecretStore::delete(leer),
                Err(SecretError::NoUsername)
            ));
            // exists ist gutmütig: ohne Benutzernamen gibt es kein Secret.
            assert!(!SecretStore::exists(leer).unwrap());
        }
    }

    /* --------------------------------------- Gegen den echten Store ------ */

    /// Ohne verfügbaren Credential Manager sind die folgenden Tests sinnlos.
    /// Unter Windows muss er verfügbar sein — schlägt das fehl, ist das ein
    /// echter Fund und kein Grund zum Überspringen.
    #[test]
    fn credential_manager_ist_verfuegbar() {
        SecretStore::availability().expect("der Windows Credential Manager muss verfügbar sein");
    }

    /// Die vollständige Runde gegen den echten Credential Manager. Validiert
    /// die keyring-4-API, die sonst nirgends geprüft wäre.
    #[test]
    fn speichern_lesen_loeschen_gegen_den_echten_store() {
        let account = test_account("rundreise");
        let _cleanup = Cleanup(account.clone());
        let secret = Secret::new("Test-Secret-!\"§$%&/()=?-äöüß-1234567890");

        // Vorher nichts.
        let _ = SecretStore::delete(&account);
        assert!(!SecretStore::exists(&account).unwrap());

        // Speichern und unverändert zurücklesen.
        SecretStore::store(&account, &secret).unwrap();
        assert!(SecretStore::exists(&account).unwrap());
        assert_eq!(
            SecretStore::load(&account).unwrap().expose(),
            secret.expose(),
            "Sonderzeichen und Umlaute müssen unverändert zurückkommen"
        );

        // Überschreiben.
        let neu = Secret::new("zweites-secret");
        SecretStore::store(&account, &neu).unwrap();
        assert_eq!(
            SecretStore::load(&account).unwrap().expose(),
            "zweites-secret"
        );

        // Löschen.
        SecretStore::delete(&account).unwrap();
        assert!(!SecretStore::exists(&account).unwrap());
        assert!(matches!(
            SecretStore::load(&account),
            Err(SecretError::NotFound { .. })
        ));
    }

    /// Ein leeres Secret muss den Eintrag entfernen, nicht einen leeren
    /// Eintrag hinterlassen — sonst gilt er später als „gesetzt" und erzeugt
    /// einen 401 ohne erkennbare Ursache.
    #[test]
    fn leeres_secret_loescht_den_eintrag() {
        let account = test_account("leer");
        let _cleanup = Cleanup(account.clone());

        SecretStore::store(&account, &Secret::new("etwas")).unwrap();
        assert!(SecretStore::exists(&account).unwrap());

        SecretStore::store(&account, &Secret::new("")).unwrap();
        assert!(
            !SecretStore::exists(&account).unwrap(),
            "der Eintrag hätte gelöscht werden müssen"
        );
    }

    /// Ein leeres Secret zu speichern, wenn gar keiner existiert, ist kein
    /// Fehler — das passiert, wenn der Benutzer das Feld leer lässt.
    #[test]
    fn leeres_secret_ohne_vorhandenen_eintrag_ist_kein_fehler() {
        let account = test_account("leer-ohne-eintrag");
        let _cleanup = Cleanup(account.clone());
        let _ = SecretStore::delete(&account);

        SecretStore::store(&account, &Secret::new("")).unwrap();
        assert!(!SecretStore::exists(&account).unwrap());
    }

    #[test]
    fn loeschen_eines_fehlenden_eintrags_meldet_nicht_gefunden() {
        let account = test_account("fehlt");
        let _ = SecretStore::delete(&account);
        assert!(matches!(
            SecretStore::delete(&account),
            Err(SecretError::NotFound { .. })
        ));
    }

    /// Ein sehr langes Secret muss durchgehen — Automation-Secrets sind
    /// zufällige Zeichenketten und können lang sein.
    #[test]
    fn langes_secret_geht_durch() {
        let account = test_account("lang");
        let _cleanup = Cleanup(account.clone());
        let lang = Secret::new("A".repeat(512));

        SecretStore::store(&account, &lang).unwrap();
        assert_eq!(SecretStore::load(&account).unwrap().expose().len(), 512);
    }

    /// Der Benutzername wird getrimmt — sonst legt ein versehentliches
    /// Leerzeichen im Dialog einen zweiten, unerreichbaren Eintrag an.
    #[test]
    fn benutzername_mit_leerzeichen_trifft_denselben_eintrag() {
        let account = test_account("trim");
        let _cleanup = Cleanup(account.clone());

        SecretStore::store(&format!("  {account}  "), &Secret::new("wert")).unwrap();
        assert_eq!(SecretStore::load(&account).unwrap().expose(), "wert");
        assert!(SecretStore::exists(&format!("{account} ")).unwrap());
    }

    /// Die Fehlermeldungen gehen in den Einstellungsdialog. Sie müssen
    /// deutsch und konkret sein.
    #[test]
    fn fehlermeldungen_sind_deutsch_und_nennen_den_benutzer() {
        let text = SecretError::NotFound {
            username: "m.mustermann".into(),
        }
        .to_string();
        assert!(text.contains("m.mustermann"), "{text}");
        assert!(text.contains("Automation-Secret"), "{text}");

        let text = SecretError::StoreUnavailable { reason: "x".into() }.to_string();
        assert!(text.contains("Credential Manager"), "{text}");
    }
}
