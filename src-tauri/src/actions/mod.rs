//! Quittieren und Wartungszeit.
//!
//! Die Nutzlasten und ihre Zeitfenster stehen in [`crate::checkmk::write`] und
//! sind dort geprüft. Hier liegt, was darüber hinaus entschieden werden muss:
//!
//! * die **Kommentarvorlage** — [`render_comment`], rein und getestet
//! * die **Berechtigungsprüfung** — [`ensure_allowed`], damit sie im Backend
//!   liegt und nicht nur ein ausgeblendeter Knopf ist
//!
//! # Warum die Berechtigung hier geprüft wird und nicht im Frontend
//!
//! Im Dialog stehen zwei Schalter, die standardmässig **aus** sind. Wären sie
//! nur eine Anzeigebedingung, käme jeder an den Befehl heran, der ihn direkt
//! aufruft — und ein Fehlgriff schreibt in ein Produktionsmonitoring. Die
//! Prüfung gehört auf die Seite, die den Aufruf ausführt.

use serde::{Deserialize, Serialize};

use crate::config::Settings;

/// Welche Schreibaktion gemeint ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WriteAction {
    Acknowledge,
    Downtime,
}

impl WriteAction {
    /// Name für Meldungen. Deutsch, weil er in einer Fehlermeldung landet.
    pub fn label(self) -> &'static str {
        match self {
            Self::Acknowledge => "Quittieren",
            Self::Downtime => "Wartungszeit setzen",
        }
    }

    /// Der Einstellungspfad, der sie freigibt. Wird in der Fehlermeldung
    /// genannt, damit der Benutzer weiss, wo er nachsehen muss.
    pub fn setting_path(self) -> &'static str {
        match self {
            Self::Acknowledge => "permissions.allowAcknowledge",
            Self::Downtime => "permissions.allowDowntime",
        }
    }

    fn is_allowed(self, settings: &Settings) -> bool {
        match self {
            Self::Acknowledge => settings.permissions.allow_acknowledge,
            Self::Downtime => settings.permissions.allow_downtime,
        }
    }
}

/// Grund, aus dem eine Aktion nicht ausgeführt wurde.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionRefusal {
    /// Der Schalter im Einstellungsdialog ist aus.
    NotPermitted(WriteAction),
    /// Kommentar leer. CheckMK verlangt das Feld, und ein leerer Kommentar
    /// wäre in der Historie wertlos.
    EmptyComment,
}

impl std::fmt::Display for ActionRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotPermitted(action) => write!(
                f,
                "„{}“ ist nicht freigegeben. Der Schalter dafür steht in den \
                 Einstellungen unter „Schreibaktionen“.",
                action.label()
            ),
            Self::EmptyComment => write!(
                f,
                "Der Kommentar darf nicht leer sein — CheckMK verlangt ihn, und \
                 in der Historie ist er die einzige Auskunft darüber, warum die \
                 Aktion stattfand."
            ),
        }
    }
}

/// Prüft, ob die Aktion laufen darf.
pub fn ensure_allowed(action: WriteAction, settings: &Settings) -> Result<(), ActionRefusal> {
    if action.is_allowed(settings) {
        Ok(())
    } else {
        Err(ActionRefusal::NotPermitted(action))
    }
}

/// Platzhalter, die [`render_comment`] ersetzt.
///
/// Absichtlich wenige und alle aus Daten, die zum Zeitpunkt der Aktion sicher
/// vorliegen. Ein Platzhalter, der manchmal leer bleibt, erzeugt Kommentare wie
/// „Quittiert durch  auf “ — schlimmer als keine Vorlage.
pub const PLACEHOLDERS: [&str; 4] = ["{host}", "{service}", "{user}", "{app}"];

/// Was in `{app}` eingesetzt wird.
const APP_NAME: &str = "Luchsr";

/// Setzt die Platzhalter der Vorlage ein.
///
/// `service` ist `None` bei einem Hostproblem. `{service}` wird dann durch
/// **„Host“** ersetzt und nicht durch Leerraum: der Kommentar soll auch dann
/// einen Satz ergeben.
///
/// Unbekannte geschweifte Ausdrücke bleiben **stehen**. Sie stillschweigend zu
/// entfernen würde einen Tippfehler in der Vorlage unsichtbar machen; so sieht
/// der Benutzer im vorbelegten Feld sofort, dass etwas nicht ersetzt wurde.
pub fn render_comment(template: &str, host: &str, service: Option<&str>, user: &str) -> String {
    template
        .replace("{host}", host)
        .replace("{service}", service.unwrap_or("Host"))
        .replace("{user}", user)
        .replace("{app}", APP_NAME)
        .trim()
        .to_owned()
}

/// Prüft einen Kommentar, wie er aus dem Dialog kommt.
///
/// Nur auf Leerheit — die Länge begrenzt CheckMK selbst, und eine eigene
/// Obergrenze zu erfinden hiesse, sie irgendwann falsch geraten zu haben.
pub fn ensure_comment(comment: &str) -> Result<&str, ActionRefusal> {
    let getrimmt = comment.trim();
    if getrimmt.is_empty() {
        Err(ActionRefusal::EmptyComment)
    } else {
        Ok(getrimmt)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(ack: bool, downtime: bool) -> Settings {
        let mut s = Settings::default();
        s.permissions.allow_acknowledge = ack;
        s.permissions.allow_downtime = downtime;
        s
    }

    /* ------------------------------------------------- Berechtigungen -- */

    /// Die Vorgabe des Auftrags: beide Schalter aus.
    #[test]
    fn ohne_freigabe_laeuft_keine_der_beiden_aktionen() {
        let s = Settings::default();
        assert!(ensure_allowed(WriteAction::Acknowledge, &s).is_err());
        assert!(ensure_allowed(WriteAction::Downtime, &s).is_err());
    }

    /// Die beiden Freigaben sind getrennt. Wer quittieren darf, darf nicht
    /// automatisch Wartungszeiten setzen.
    #[test]
    fn die_freigaben_wirken_einzeln() {
        let nur_ack = settings(true, false);
        assert!(ensure_allowed(WriteAction::Acknowledge, &nur_ack).is_ok());
        assert!(ensure_allowed(WriteAction::Downtime, &nur_ack).is_err());

        let nur_downtime = settings(false, true);
        assert!(ensure_allowed(WriteAction::Acknowledge, &nur_downtime).is_err());
        assert!(ensure_allowed(WriteAction::Downtime, &nur_downtime).is_ok());
    }

    /// Die Meldung muss sagen, *welche* Aktion gemeint ist und *wo* man sie
    /// freigibt. „Nicht erlaubt“ allein schickt den Benutzer auf die Suche.
    #[test]
    fn die_ablehnung_benennt_aktion_und_ort() {
        let text = ActionRefusal::NotPermitted(WriteAction::Downtime).to_string();
        assert!(text.contains("Wartungszeit setzen"), "{text}");
        assert!(text.contains("Einstellungen"), "{text}");
    }

    /* ------------------------------------------------ Kommentarvorlage -- */

    #[test]
    fn platzhalter_werden_eingesetzt() {
        let text = render_comment(
            "{app}: {service} auf {host} bearbeitet von {user}",
            "leosys-sql-01",
            Some("Filesystem /var"),
            "m.mustermann",
        );
        assert_eq!(
            text,
            "Luchsr: Filesystem /var auf leosys-sql-01 bearbeitet von m.mustermann"
        );
    }

    /// Ein Hostproblem hat keinen Service. Der Satz muss trotzdem stehen.
    #[test]
    fn ohne_service_steht_dort_host_und_nicht_leerraum() {
        let text = render_comment("{service} auf {host}", "leosys-dc-02", None, "m.mustermann");
        assert_eq!(text, "Host auf leosys-dc-02");
    }

    #[test]
    fn mehrfaches_vorkommen_wird_ueberall_ersetzt() {
        let text = render_comment("{host} {host} {host}", "srv01", None, "u");
        assert_eq!(text, "srv01 srv01 srv01");
    }

    /// Ein Tippfehler in der Vorlage soll sichtbar sein, nicht verschwinden.
    #[test]
    fn unbekannter_platzhalter_bleibt_stehen() {
        let text = render_comment("{hosst} ist kaputt", "srv01", None, "u");
        assert_eq!(text, "{hosst} ist kaputt");
    }

    /// Eine Vorlage ohne Platzhalter ist erlaubt — jemand will vielleicht
    /// immer denselben Satz.
    #[test]
    fn vorlage_ohne_platzhalter_bleibt_unveraendert() {
        let text = render_comment("Bekannt, wird bearbeitet", "srv01", Some("S"), "u");
        assert_eq!(text, "Bekannt, wird bearbeitet");
    }

    /// Kein Platzhalter darf einen Teil eines anderen enthalten, sonst hängt
    /// das Ergebnis von der Ersetzungsreihenfolge ab.
    #[test]
    fn kein_platzhalter_ist_teil_eines_anderen() {
        for (i, a) in PLACEHOLDERS.iter().enumerate() {
            for (j, b) in PLACEHOLDERS.iter().enumerate() {
                if i != j {
                    assert!(!a.contains(b), "{a} enthält {b} — Reihenfolge zählt");
                }
            }
        }
    }

    /// Jeder dokumentierte Platzhalter muss tatsächlich ersetzt werden.
    #[test]
    fn jeder_dokumentierte_platzhalter_wird_ersetzt() {
        for p in PLACEHOLDERS {
            let text = render_comment(p, "H", Some("S"), "U");
            assert_ne!(text, p, "{p} wurde nicht ersetzt");
            assert!(!text.contains('{'), "{p} liess eine Klammer stehen: {text}");
        }
    }

    /* ----------------------------------------------- Kommentarprüfung -- */

    #[test]
    fn leerer_kommentar_wird_abgelehnt() {
        assert_eq!(ensure_comment(""), Err(ActionRefusal::EmptyComment));
        assert_eq!(ensure_comment("   \t\n "), Err(ActionRefusal::EmptyComment));
    }

    #[test]
    fn kommentar_wird_getrimmt_durchgelassen() {
        assert_eq!(ensure_comment("  Grund  "), Ok("Grund"));
    }
}
