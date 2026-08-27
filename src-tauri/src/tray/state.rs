//! Zustand des Tray-Icons: Abbildung, Bilddaten, Tooltip.
//!
//! Reine Funktionen, alle einzeln testbar. Die Bilddaten sind zur Bauzeit
//! eingebettet — ein Tray-Icon, das erst von der Platte geladen wird, fehlt
//! genau dann, wenn die Installation unvollständig ist.

use crate::checkmk::{ProblemState, Snapshot};

/// Die sechs Zustände, die das Tray-Icon annehmen kann.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayState {
    Ok,
    Warn,
    Crit,
    /// Host nicht erreichbar.
    Down,
    Unknown,
    /// Kein Kontakt zum CheckMK-Server. Nicht dasselbe wie „alles gut".
    Disconnected,
}

/* -------------------------------------------------------------------------- */
/* Bilddaten                                                                  */
/* -------------------------------------------------------------------------- */

// Erzeugt von scripts/make-icons.mjs aus scripts/mark.mjs.
// Je Zustand zwei Grössen, siehe Entscheidung D25.
const OK_16: &[u8] = include_bytes!("../../icons/tray/ok-16.png");
const OK_32: &[u8] = include_bytes!("../../icons/tray/ok-32.png");
const WARN_16: &[u8] = include_bytes!("../../icons/tray/warn-16.png");
const WARN_32: &[u8] = include_bytes!("../../icons/tray/warn-32.png");
const CRIT_16: &[u8] = include_bytes!("../../icons/tray/crit-16.png");
const CRIT_32: &[u8] = include_bytes!("../../icons/tray/crit-32.png");
const DOWN_16: &[u8] = include_bytes!("../../icons/tray/down-16.png");
const DOWN_32: &[u8] = include_bytes!("../../icons/tray/down-32.png");
const UNKNOWN_16: &[u8] = include_bytes!("../../icons/tray/unknown-16.png");
const UNKNOWN_32: &[u8] = include_bytes!("../../icons/tray/unknown-32.png");
const DISCONNECTED_16: &[u8] = include_bytes!("../../icons/tray/disconnected-16.png");
const DISCONNECTED_32: &[u8] = include_bytes!("../../icons/tray/disconnected-32.png");

/// Ab welchem Skalierungsfaktor die 32-px-Fassung genommen wird.
///
/// Windows fragt im Infobereich 16 px bei 100 % und 32 px ab 150 %. Genau bei
/// 1.5 zu wechseln ist die Grenze, an der ein hochskaliertes 16er sichtbar
/// weich wird.
const HIDPI_THRESHOLD: f64 = 1.5;

impl TrayState {
    /// Die eingebetteten PNG-Daten für diesen Zustand.
    ///
    /// Wählt die Grösse nach dem Skalierungsfaktor des Bildschirms. Ein
    /// herunterskaliertes 32er ist bei 100 % unscharf, ein hochskaliertes 16er
    /// bei 200 % matschig — deshalb liegen beide vor.
    pub fn icon_bytes(self, scale_factor: f64) -> &'static [u8] {
        let hidpi = scale_factor >= HIDPI_THRESHOLD;
        match (self, hidpi) {
            (Self::Ok, false) => OK_16,
            (Self::Ok, true) => OK_32,
            (Self::Warn, false) => WARN_16,
            (Self::Warn, true) => WARN_32,
            (Self::Crit, false) => CRIT_16,
            (Self::Crit, true) => CRIT_32,
            (Self::Down, false) => DOWN_16,
            (Self::Down, true) => DOWN_32,
            (Self::Unknown, false) => UNKNOWN_16,
            (Self::Unknown, true) => UNKNOWN_32,
            (Self::Disconnected, false) => DISCONNECTED_16,
            (Self::Disconnected, true) => DISCONNECTED_32,
        }
    }

    /// Kürzel für Protokolle.
    pub fn label(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Crit => "CRIT",
            Self::Down => "DOWN",
            Self::Unknown => "UNKNOWN",
            Self::Disconnected => "GETRENNT",
        }
    }
}

impl From<ProblemState> for TrayState {
    fn from(value: ProblemState) -> Self {
        match value {
            ProblemState::Ok => Self::Ok,
            ProblemState::Warn => Self::Warn,
            ProblemState::Crit => Self::Crit,
            // Wie in ProblemState::status_key: für den Benutzer ist beides
            // „der Host ist weg".
            ProblemState::Down | ProblemState::Unreachable => Self::Down,
            ProblemState::Unknown => Self::Unknown,
        }
    }
}

/* -------------------------------------------------------------------------- */
/* Abbildung                                                                  */
/* -------------------------------------------------------------------------- */

/// Bestimmt den Tray-Zustand.
///
/// Reihenfolge der Prüfungen ist Absicht:
///
/// 1. **Verbindungsfehler schlägt alles.** Ein grünes Icon, während der Server
///    nicht antwortet, wäre die schlimmste Fehlinformation, die dieses
///    Programm liefern kann — es sähe aus wie „alles in Ordnung".
/// 2. **Kein Abzug** heisst ebenfalls „getrennt", nicht „OK". Beim Start ist
///    noch nichts bekannt.
/// 3. Sonst der schlimmste sichtbare Zustand; ohne Probleme OK.
pub fn tray_state(
    snapshot: Option<&Snapshot>,
    connection_failing: bool,
    include_handled: bool,
) -> TrayState {
    if connection_failing {
        return TrayState::Disconnected;
    }
    match snapshot {
        None => TrayState::Disconnected,
        Some(snapshot) => snapshot
            .worst(include_handled)
            .map_or(TrayState::Ok, TrayState::from),
    }
}

/* -------------------------------------------------------------------------- */
/* Tooltip                                                                    */
/* -------------------------------------------------------------------------- */

/// Windows begrenzt `NOTIFYICONDATA::szTip` auf 128 Zeichen inklusive
/// Nullterminierung. Längere Texte werden stillschweigend abgeschnitten —
/// besser selbst kürzen und ein Auslassungszeichen setzen.
pub const TOOLTIP_MAX_CHARS: usize = 127;

/// Baut den Tooltip.
///
/// `error` ist die Meldung des letzten fehlgeschlagenen Abrufs. Sie hat
/// Vorrang: wenn keine Verbindung steht, ist die Problemanzahl von vorgestern
/// und der Benutzer soll die Ursache sehen.
pub fn tooltip(snapshot: Option<&Snapshot>, error: Option<&str>, include_handled: bool) -> String {
    let body = match (error, snapshot) {
        (Some(message), _) => format!("Keine Verbindung — {message}"),
        (None, None) => "Noch nicht abgerufen".to_string(),
        (None, Some(snapshot)) => snapshot.counts(include_handled).tooltip(),
    };
    truncate_chars(&format!("Luchsr — {body}"), TOOLTIP_MAX_CHARS)
}

/// Kürzt auf `max` **Zeichen**, nicht Bytes.
///
/// Auf Bytes zu kürzen würde einen Umlaut in der Mitte zerlegen; Windows zählt
/// ohnehin UTF-16-Zeichen.
fn truncate_chars(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let keep = max.saturating_sub(1);
    let mut out: String = value.chars().take(keep).collect();
    out.push('…');
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkmk::Problem;
    use chrono::{TimeZone, Utc};

    fn problem(state: ProblemState, host: &str, service: Option<&str>) -> Problem {
        Problem {
            host: host.to_string(),
            service: service.map(str::to_string),
            state,
            output: String::new(),
            last_state_change: None,
            acknowledged: false,
            downtime_depth: 0,
            flapping: false,
        }
    }

    fn snapshot(problems: Vec<Problem>) -> Snapshot {
        Snapshot::new(problems, Utc.timestamp_opt(1_700_000_000, 0).unwrap())
    }

    /* ------------------------------------------------------------ Abbildung */

    /// Die wichtigste Zusicherung dieses Moduls: ein Verbindungsfehler darf
    /// niemals als grünes Icon erscheinen.
    #[test]
    fn verbindungsfehler_schlaegt_jeden_abzug() {
        let alles_gut = snapshot(vec![]);
        assert_eq!(
            tray_state(Some(&alles_gut), true, false),
            TrayState::Disconnected,
            "ein grünes Icon bei fehlender Verbindung wäre die schlimmste Fehlinformation"
        );

        // Auch wenn der letzte Abzug Probleme enthielt.
        let mit_problemen = snapshot(vec![problem(ProblemState::Crit, "h", Some("s"))]);
        assert_eq!(
            tray_state(Some(&mit_problemen), true, false),
            TrayState::Disconnected
        );
    }

    /// Vor dem ersten Abruf ist nichts bekannt — das ist nicht „OK".
    #[test]
    fn ohne_abzug_ist_getrennt_nicht_ok() {
        assert_eq!(tray_state(None, false, false), TrayState::Disconnected);
    }

    #[test]
    fn leerer_abzug_ist_ok() {
        assert_eq!(
            tray_state(Some(&snapshot(vec![])), false, false),
            TrayState::Ok
        );
    }

    #[test]
    fn schlimmster_zustand_bestimmt_das_icon() {
        let s = snapshot(vec![
            problem(ProblemState::Warn, "a", Some("x")),
            problem(ProblemState::Crit, "b", Some("y")),
            problem(ProblemState::Unknown, "c", Some("z")),
        ]);
        assert_eq!(tray_state(Some(&s), false, false), TrayState::Crit);

        let mit_host = snapshot(vec![
            problem(ProblemState::Crit, "b", Some("y")),
            problem(ProblemState::Down, "d", None),
        ]);
        assert_eq!(tray_state(Some(&mit_host), false, false), TrayState::Down);
    }

    #[test]
    fn unreachable_zeigt_dasselbe_icon_wie_down() {
        let s = snapshot(vec![problem(ProblemState::Unreachable, "d", None)]);
        assert_eq!(tray_state(Some(&s), false, false), TrayState::Down);
        assert_eq!(TrayState::from(ProblemState::Unreachable), TrayState::Down);
    }

    /// Sind nur noch quittierte Probleme übrig, muss das Icon auf grün gehen —
    /// sonst leuchtet es dauerhaft rot.
    #[test]
    fn nur_quittierte_probleme_ergeben_ok() {
        let s = snapshot(vec![Problem {
            acknowledged: true,
            ..problem(ProblemState::Crit, "h", Some("s"))
        }]);
        assert_eq!(tray_state(Some(&s), false, false), TrayState::Ok);
        assert_eq!(
            tray_state(Some(&s), false, true),
            TrayState::Crit,
            "mit eingeblendeten Bearbeiteten wieder CRIT"
        );
    }

    /* ---------------------------------------------------------- Bilddaten */

    #[test]
    fn jeder_zustand_hat_beide_groessen_und_sie_unterscheiden_sich() {
        let states = [
            TrayState::Ok,
            TrayState::Warn,
            TrayState::Crit,
            TrayState::Down,
            TrayState::Unknown,
            TrayState::Disconnected,
        ];
        for state in states {
            let klein = state.icon_bytes(1.0);
            let gross = state.icon_bytes(2.0);
            assert!(!klein.is_empty(), "{:?}: 16 px fehlt", state);
            assert!(!gross.is_empty(), "{:?}: 32 px fehlt", state);
            assert_ne!(
                klein, gross,
                "{:?}: beide Grössen sind identisch, da wurde dieselbe Datei eingebettet",
                state
            );
            // PNG-Signatur.
            assert_eq!(
                &klein[..8],
                &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
            );
            assert_eq!(
                &gross[..8],
                &[0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a]
            );
        }
    }

    /// Alle sechs Zustände müssen unterschiedliche Bilddaten haben — sonst
    /// wurde beim Einbetten eine Datei verwechselt und zwei Zustände sähen
    /// gleich aus, ohne dass es auffällt.
    #[test]
    fn alle_zustaende_haben_unterschiedliche_bilder() {
        let states = [
            TrayState::Ok,
            TrayState::Warn,
            TrayState::Crit,
            TrayState::Down,
            TrayState::Unknown,
            TrayState::Disconnected,
        ];
        for scale in [1.0, 2.0] {
            let mut gesehen = std::collections::HashSet::new();
            for state in states {
                assert!(
                    gesehen.insert(state.icon_bytes(scale)),
                    "{:?} bei Skalierung {scale} teilt sein Bild mit einem anderen Zustand",
                    state
                );
            }
        }
    }

    #[test]
    fn groessenwahl_folgt_dem_skalierungsfaktor() {
        let state = TrayState::Crit;
        let klein = state.icon_bytes(1.0);
        let gross = state.icon_bytes(2.0);

        assert_eq!(state.icon_bytes(1.0), klein);
        assert_eq!(state.icon_bytes(1.25), klein, "125 % nutzt noch 16 px");
        assert_eq!(state.icon_bytes(1.5), gross, "ab 150 % die 32er");
        assert_eq!(state.icon_bytes(3.0), gross);
    }

    /* ------------------------------------------------------------- Tooltip */

    /// Das Beispiel aus dem Auftrag.
    #[test]
    fn tooltip_nennt_die_anzahl_wie_im_auftrag() {
        let mut problems = vec![];
        for i in 0..3 {
            problems.push(problem(ProblemState::Crit, &format!("c{i}"), Some("s")));
        }
        for i in 0..7 {
            problems.push(problem(ProblemState::Warn, &format!("w{i}"), Some("s")));
        }
        let text = tooltip(Some(&snapshot(problems)), None, false);
        assert_eq!(text, "Luchsr — 3 CRIT, 7 WARN");
    }

    #[test]
    fn tooltip_ohne_probleme() {
        assert_eq!(
            tooltip(Some(&snapshot(vec![])), None, false),
            "Luchsr — Keine offenen Probleme"
        );
    }

    #[test]
    fn tooltip_vor_dem_ersten_abruf() {
        assert_eq!(tooltip(None, None, false), "Luchsr — Noch nicht abgerufen");
    }

    /// Bei einem Fehler ist die Problemanzahl veraltet — die Ursache hat Vorrang.
    #[test]
    fn tooltip_zeigt_bei_fehler_die_ursache_nicht_die_alte_anzahl() {
        let s = snapshot(vec![problem(ProblemState::Crit, "h", Some("s"))]);
        let text = tooltip(
            Some(&s),
            Some("Der Hostname konnte nicht aufgelöst werden."),
            false,
        );
        assert!(text.contains("Keine Verbindung"), "{text}");
        assert!(text.contains("Hostname"), "{text}");
        assert!(
            !text.contains("CRIT"),
            "veraltete Anzahl darf nicht erscheinen: {text}"
        );
    }

    /// Windows schneidet stillschweigend ab. Lieber selbst kürzen.
    #[test]
    fn tooltip_wird_auf_die_windows_grenze_gekuerzt() {
        let lang = "x".repeat(500);
        let text = tooltip(None, Some(&lang), false);
        assert_eq!(text.chars().count(), TOOLTIP_MAX_CHARS);
        assert!(text.ends_with('…'));
    }

    /// Auf Bytes zu kürzen würde einen Umlaut zerlegen.
    #[test]
    fn kuerzung_zerlegt_keine_umlaute() {
        let text = truncate_chars(&"ä".repeat(200), 50);
        assert_eq!(text.chars().count(), 50);
        assert!(text.chars().take(49).all(|c| c == 'ä'));
        // Und der String ist gültiges UTF-8, sonst wäre er nicht konstruierbar.
        assert!(text.ends_with('…'));
    }

    #[test]
    fn kuerzung_laesst_kurze_texte_unberuehrt() {
        assert_eq!(truncate_chars("kurz", 50), "kurz");
        assert_eq!(truncate_chars("", 50), "");
    }

    #[test]
    fn zustandskuerzel_sind_gesetzt() {
        for state in [
            TrayState::Ok,
            TrayState::Warn,
            TrayState::Crit,
            TrayState::Down,
            TrayState::Unknown,
            TrayState::Disconnected,
        ] {
            assert!(!state.label().is_empty());
        }
        assert_eq!(TrayState::Disconnected.label(), "GETRENNT");
    }
}
