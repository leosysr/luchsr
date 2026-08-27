//! Datenstrukturen der CheckMK-REST-API und das daraus abgeleitete Domänenmodell.
//!
//! ## Was an den Antworten unangenehm ist
//!
//! Die Nutzdaten liegen unter `value[].extensions`. Die Werte dort kommen aus
//! Livestatus und sind schwach typisiert:
//!
//! * `acknowledged`, `is_flapping` sind **Ganzzahlen** `0`/`1`, nicht Booleans.
//!   Einzelne CheckMK-Versionen liefern trotzdem echte Booleans.
//! * `last_state_change` ist ein Unix-Zeitstempel, mal als Ganzzahl, mal mit
//!   Nachkommastellen. `0` bedeutet „noch nie gewechselt", nicht 1970.
//! * Felder können ganz fehlen, wenn die Spalte nicht angefragt wurde.
//!
//! Deshalb gibt es hier eigene Deserialisierer statt `#[derive]` allein. Sie
//! sind großzügig beim Lesen und streng beim Ergebnis — genau die Richtung, in
//! die Toleranz gehen soll.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt;

use chrono::{DateTime, TimeZone, Utc};
use serde::de::{self, Deserializer};
use serde::{Deserialize, Serialize};

/* -------------------------------------------------------------------------- */
/* Umschlag                                                                   */
/* -------------------------------------------------------------------------- */

/// Der Sammel-Umschlag jeder Collection-Antwort.
#[derive(Debug, Deserialize)]
pub struct Collection<T> {
    #[serde(default = "Vec::new")]
    pub value: Vec<Entry<T>>,
}

/// Ein Element der Collection. Nur `extensions` interessiert; `links`,
/// `members` und `title` werden bewusst verworfen.
#[derive(Debug, Deserialize)]
pub struct Entry<T> {
    pub extensions: T,
}

/// Fehlerantwort von CheckMK (problem+json).
#[derive(Debug, Default, Deserialize)]
pub struct ApiProblem {
    pub title: Option<String>,
    pub detail: Option<String>,
    pub status: Option<u16>,
}

impl ApiProblem {
    /// Die aussagekräftigere der beiden Textfelder.
    pub fn best_detail(&self) -> Option<String> {
        match (&self.detail, &self.title) {
            (Some(d), _) if !d.trim().is_empty() => Some(d.trim().to_string()),
            (_, Some(t)) if !t.trim().is_empty() => Some(t.trim().to_string()),
            _ => None,
        }
    }
}

/// Antwort von `GET /version`. Wird für den Verbindungstest gebraucht:
/// der Auftrag verlangt die *erkannte CheckMK-Version*, nicht nur „OK".
#[derive(Debug, Deserialize)]
pub struct VersionInfo {
    #[serde(default)]
    pub site: Option<String>,
    #[serde(default)]
    pub edition: Option<String>,
    #[serde(default)]
    pub versions: BTreeMap<String, serde_json::Value>,
}

impl VersionInfo {
    /// Die CheckMK-Version als Text, etwa `2.3.0p10`.
    pub fn checkmk_version(&self) -> Option<String> {
        self.versions
            .get("checkmk")
            .and_then(|v| v.as_str())
            .map(str::to_string)
    }

    /// Editionskürzel in Klartext. `cre` heisst Raw, `cee` Enterprise usw.
    pub fn edition_label(&self) -> Option<&'static str> {
        match self.edition.as_deref()? {
            "cre" => Some("Raw Edition"),
            "cee" => Some("Enterprise Edition"),
            "cme" => Some("Managed Services Edition"),
            "cce" => Some("Cloud Edition"),
            "cse" => Some("SaaS Edition"),
            _ => None,
        }
    }
}

/* -------------------------------------------------------------------------- */
/* Zustände                                                                   */
/* -------------------------------------------------------------------------- */

/// Vereinheitlichter Zustand für die Anzeige.
///
/// `Stale` steht hier nicht: das ist kein Zustand, den die API liefert, sondern
/// eine Folge des Alters der Daten. Er entsteht in der Anzeigeschicht.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ProblemState {
    Ok,
    Warn,
    Crit,
    Unknown,
    /// Host antwortet nicht.
    Down,
    /// Host über keinen Pfad erreichbar — Elternhost ist ausgefallen.
    Unreachable,
}

impl ProblemState {
    /// Schwere für die Sortierung. Höher ist schlimmer.
    ///
    /// Ein ausgefallener Host schlägt jeden Serviceausfall, CRIT schlägt
    /// UNKNOWN. Begründung: ist der Host weg, ist jede Aussage über seine
    /// Services wertlos — und „der Check funktioniert nicht" (UNKNOWN) ist
    /// weniger dringend als „der Dienst ist kaputt" (CRIT).
    ///
    /// Die Zahlen entsprechen den `severity`-Werten in `src/lib/status.ts`,
    /// damit Backend und Frontend dieselbe Ordnung verwenden.
    pub fn severity(self) -> u8 {
        match self {
            Self::Down => 50,
            Self::Unreachable => 45,
            Self::Crit => 40,
            Self::Unknown => 30,
            Self::Warn => 20,
            Self::Ok => 0,
        }
    }

    /// Der Anzeigeschlüssel des Frontends (`StatusKey` in status.ts).
    ///
    /// `Unreachable` fällt mit `Down` zusammen: für den Benutzer ist beides
    /// „der Host ist weg". Die Unterscheidung bleibt im Modell erhalten und
    /// wird im Detail-Panel gezeigt.
    pub fn status_key(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Warn => "warn",
            Self::Crit => "crit",
            Self::Unknown => "unknown",
            Self::Down | Self::Unreachable => "down",
        }
    }

    /// Ob dieser Zustand ein offenes Problem ist.
    pub fn is_problem(self) -> bool {
        self != Self::Ok
    }
}

impl fmt::Display for ProblemState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Ok => "OK",
            Self::Warn => "WARN",
            Self::Crit => "CRIT",
            Self::Unknown => "UNKNOWN",
            Self::Down => "DOWN",
            Self::Unreachable => "UNREACHABLE",
        })
    }
}

/// Servicezustand, wie CheckMK ihn numerisch liefert.
pub fn service_state_from_raw(raw: i64) -> ProblemState {
    match raw {
        0 => ProblemState::Ok,
        1 => ProblemState::Warn,
        2 => ProblemState::Crit,
        // 3 ist UNKNOWN. Alles darüber gibt es nicht — als UNKNOWN behandeln
        // ist ehrlicher als zu raten oder die Zeile zu verwerfen.
        _ => ProblemState::Unknown,
    }
}

/// Hostzustand, wie CheckMK ihn numerisch liefert.
pub fn host_state_from_raw(raw: i64) -> ProblemState {
    match raw {
        0 => ProblemState::Ok,
        1 => ProblemState::Down,
        2 => ProblemState::Unreachable,
        _ => ProblemState::Unknown,
    }
}

/* -------------------------------------------------------------------------- */
/* Rohdaten aus der API                                                       */
/* -------------------------------------------------------------------------- */

/// `extensions` eines Service-Eintrags.
#[derive(Debug, Deserialize)]
pub struct ServiceExtensions {
    pub host_name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default, deserialize_with = "flexible_i64")]
    pub state: i64,
    #[serde(default)]
    pub plugin_output: Option<String>,
    #[serde(default, deserialize_with = "flexible_timestamp")]
    pub last_state_change: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "flexible_bool")]
    pub acknowledged: bool,
    #[serde(default, deserialize_with = "flexible_i64")]
    pub scheduled_downtime_depth: i64,
    #[serde(default, deserialize_with = "flexible_bool")]
    pub is_flapping: bool,
}

/// `extensions` eines Host-Eintrags.
#[derive(Debug, Deserialize)]
pub struct HostExtensions {
    pub name: String,
    #[serde(default, deserialize_with = "flexible_i64")]
    pub state: i64,
    #[serde(default)]
    pub plugin_output: Option<String>,
    #[serde(default, deserialize_with = "flexible_timestamp")]
    pub last_state_change: Option<DateTime<Utc>>,
    #[serde(default, deserialize_with = "flexible_bool")]
    pub acknowledged: bool,
    #[serde(default, deserialize_with = "flexible_i64")]
    pub scheduled_downtime_depth: i64,
}

/* -------------------------------------------------------------------------- */
/* Domänenmodell                                                              */
/* -------------------------------------------------------------------------- */

/// Ein Problem, wie es in der Liste erscheint.
///
/// Serialisiert nach camelCase, weil es unverändert ans Frontend geht.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub host: String,
    /// `None` bedeutet: das Problem betrifft den Host selbst, nicht einen Service.
    pub service: Option<String>,
    pub state: ProblemState,
    pub output: String,
    /// `None`, wenn CheckMK `0` geliefert hat — dann gab es noch keinen Wechsel.
    pub last_state_change: Option<DateTime<Utc>>,
    pub acknowledged: bool,
    pub downtime_depth: i64,
    pub flapping: bool,
}

impl Problem {
    /// Ob der Zustand als „bearbeitet" gilt: quittiert oder in Wartungszeit.
    /// Solche Zeilen sind standardmässig ausgeblendet.
    pub fn is_handled(&self) -> bool {
        self.acknowledged || self.downtime_depth > 0
    }

    /// Ob es ein Hostproblem ist.
    pub fn is_host_problem(&self) -> bool {
        self.service.is_none()
    }

    /// Stabiler Schlüssel für die Benachrichtigungs-Merkmenge.
    ///
    /// Der Auftrag will Wiederholungen unterdrücken über eine Menge aus
    /// `(host, service, state)` — genau das ist dieser Schlüssel.
    ///
    /// Die Bestandteile werden **längenpräfigiert** zusammengesetzt und nicht
    /// mit einem Trennzeichen verkettet. Sonst wären
    /// `("host", "a|b")` und `("host|a", "b")` derselbe Schlüssel — und eine
    /// Kollision hier bedeutet eine unterdrückte Benachrichtigung, also einen
    /// Fehler, der niemandem auffällt. Mit Längenpräfix ist die Kodierung
    /// eindeutig, unabhängig vom Inhalt.
    ///
    /// Das führende `H`/`S` trennt zusätzlich ein Hostproblem von einem Service
    /// mit leerem Namen — beides gibt es in echten Antworten.
    pub fn notification_key(&self) -> String {
        let (kind, service) = match self.service.as_deref() {
            Some(name) => ('S', name),
            None => ('H', ""),
        };
        format!(
            "{kind}|{}:{}|{}:{}|{}",
            self.host.len(),
            self.host,
            service.len(),
            service,
            self.state
        )
    }

    /// Dauer seit dem Statuswechsel, bezogen auf `now`.
    pub fn duration_since_change(&self, now: DateTime<Utc>) -> Option<chrono::Duration> {
        self.last_state_change.map(|then| now - then)
    }

    /// Vergleich für die Standardsortierung der Liste:
    /// Status absteigend, dann Dauer absteigend (das Älteste zuerst).
    ///
    /// Zeilen ohne Zeitstempel landen hinter denen mit — „unbekannt seit wann"
    /// ist weniger dringend als „nachweislich seit drei Stunden".
    pub fn compare_for_list(&self, other: &Self) -> Ordering {
        other
            .state
            .severity()
            .cmp(&self.state.severity())
            .then_with(|| match (self.last_state_change, other.last_state_change) {
                (Some(a), Some(b)) => a.cmp(&b),
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            })
            .then_with(|| self.host.cmp(&other.host))
            .then_with(|| self.service.cmp(&other.service))
    }
}

impl From<ServiceExtensions> for Problem {
    fn from(raw: ServiceExtensions) -> Self {
        Self {
            host: raw.host_name,
            service: Some(raw.description.unwrap_or_default()),
            state: service_state_from_raw(raw.state),
            output: normalise_output(raw.plugin_output),
            last_state_change: raw.last_state_change,
            acknowledged: raw.acknowledged,
            downtime_depth: raw.scheduled_downtime_depth,
            flapping: raw.is_flapping,
        }
    }
}

impl From<HostExtensions> for Problem {
    fn from(raw: HostExtensions) -> Self {
        Self {
            host: raw.name,
            service: None,
            state: host_state_from_raw(raw.state),
            output: normalise_output(raw.plugin_output),
            last_state_change: raw.last_state_change,
            acknowledged: raw.acknowledged,
            downtime_depth: raw.scheduled_downtime_depth,
            flapping: false,
        }
    }
}

/// `plugin_output` kann mehrzeilig sein. Die Liste zeigt eine Zeile, das
/// Detail-Panel den vollen Text — deshalb wird hier nur normalisiert, nicht
/// gekürzt. Kürzen ist Sache der Anzeige.
fn normalise_output(raw: Option<String>) -> String {
    raw.unwrap_or_default().trim().to_string()
}

/* -------------------------------------------------------------------------- */
/* Abzug                                                                      */
/* -------------------------------------------------------------------------- */

/// Zähler je Zustand.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
pub struct StateCounts {
    pub down: usize,
    pub unreachable: usize,
    pub crit: usize,
    pub unknown: usize,
    pub warn: usize,
}

impl StateCounts {
    pub fn total(&self) -> usize {
        self.down + self.unreachable + self.crit + self.unknown + self.warn
    }

    /// Tooltip-Text für das Tray-Icon, etwa `3 CRIT, 7 WARN`.
    ///
    /// Nur belegte Zustände werden genannt, in absteigender Schwere. Ohne
    /// Probleme kommt ein eigener Satz statt einer leeren Zeichenkette.
    pub fn tooltip(&self) -> String {
        let parts: Vec<String> = [
            (self.down, "DOWN"),
            (self.unreachable, "UNREACHABLE"),
            (self.crit, "CRIT"),
            (self.unknown, "UNKNOWN"),
            (self.warn, "WARN"),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, label)| format!("{count} {label}"))
        .collect();

        if parts.is_empty() {
            "Keine offenen Probleme".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// Ergebnis eines Abrufs: Host- und Serviceprobleme zu einem Zeitpunkt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub problems: Vec<Problem>,
    pub fetched_at: DateTime<Utc>,
}

impl Snapshot {
    pub fn new(mut problems: Vec<Problem>, fetched_at: DateTime<Utc>) -> Self {
        problems.sort_by(Problem::compare_for_list);
        Self {
            problems,
            fetched_at,
        }
    }

    /// Sichtbare Probleme. `include_handled = false` blendet Quittiertes und
    /// Wartungszeiten aus — so ist der Standard laut Auftrag.
    pub fn visible(&self, include_handled: bool) -> impl Iterator<Item = &Problem> {
        self.problems
            .iter()
            .filter(move |p| include_handled || !p.is_handled())
    }

    pub fn counts(&self, include_handled: bool) -> StateCounts {
        let mut counts = StateCounts::default();
        for problem in self.visible(include_handled) {
            match problem.state {
                ProblemState::Down => counts.down += 1,
                ProblemState::Unreachable => counts.unreachable += 1,
                ProblemState::Crit => counts.crit += 1,
                ProblemState::Unknown => counts.unknown += 1,
                ProblemState::Warn => counts.warn += 1,
                ProblemState::Ok => {}
            }
        }
        counts
    }

    /// Der schlimmste sichtbare Zustand — bestimmt das Tray-Icon.
    pub fn worst(&self, include_handled: bool) -> Option<ProblemState> {
        self.visible(include_handled)
            .map(|p| p.state)
            .max_by_key(|state| state.severity())
    }

    /// Hostnamen, die selbst ausgefallen sind.
    ///
    /// Slice 6 klappt deren Services darunter zusammen, statt vierzig rote
    /// Zeilen zu zeigen.
    ///
    /// Nimmt denselben Filter wie [`Self::visible`] und [`Self::counts`]: ein
    /// ausgefallener Host in Wartungszeit ist in der Standardansicht nicht
    /// sichtbar, also gibt es dort auch keine Gruppe für ihn. Wird der Filter
    /// umgeschaltet, erscheint Zeile und Gruppe gemeinsam. Ohne diesen
    /// Parameter würde die Gruppierung auf einen Host zeigen, den die Liste
    /// gerade nicht zeigt.
    pub fn failed_hosts(&self, include_handled: bool) -> Vec<&str> {
        self.visible(include_handled)
            .filter(|p| {
                p.is_host_problem()
                    && matches!(p.state, ProblemState::Down | ProblemState::Unreachable)
            })
            .map(|p| p.host.as_str())
            .collect()
    }
}

/* -------------------------------------------------------------------------- */
/* Großzügige Deserialisierer                                                 */
/* -------------------------------------------------------------------------- */

/// Liest `0`/`1`, `true`/`false`, `"0"`/`"1"`, `"true"`/`"false"` und `null`.
fn flexible_bool<'de, D: Deserializer<'de>>(deserializer: D) -> Result<bool, D::Error> {
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Bool(value) => Ok(value),
        serde_json::Value::Null => Ok(false),
        serde_json::Value::Number(number) => Ok(number.as_f64().unwrap_or(0.0) != 0.0),
        serde_json::Value::String(text) => match text.trim().to_ascii_lowercase().as_str() {
            "1" | "true" | "yes" | "on" => Ok(true),
            "" | "0" | "false" | "no" | "off" => Ok(false),
            other => Err(de::Error::custom(format!(
                "erwartet wurde ein Wahrheitswert, gelesen wurde {other:?}"
            ))),
        },
        other => Err(de::Error::custom(format!(
            "erwartet wurde ein Wahrheitswert, gelesen wurde {other}"
        ))),
    }
}

/// Liest Ganzzahlen, auch wenn sie als Fliesskommazahl, Text oder `null` kommen.
fn flexible_i64<'de, D: Deserializer<'de>>(deserializer: D) -> Result<i64, D::Error> {
    match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => Ok(0),
        serde_json::Value::Bool(value) => Ok(i64::from(value)),
        serde_json::Value::Number(number) => number
            .as_i64()
            .or_else(|| number.as_f64().map(|f| f.trunc() as i64))
            .ok_or_else(|| de::Error::custom("Zahl liess sich nicht als Ganzzahl lesen")),
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Ok(0);
            }
            trimmed
                .parse::<i64>()
                .or_else(|_| trimmed.parse::<f64>().map(|f| f.trunc() as i64))
                .map_err(|_| {
                    de::Error::custom(format!(
                        "erwartet wurde eine Zahl, gelesen wurde {trimmed:?}"
                    ))
                })
        }
        other => Err(de::Error::custom(format!(
            "erwartet wurde eine Zahl, gelesen wurde {other}"
        ))),
    }
}

/// Liest einen Unix-Zeitstempel. `0`, `null` und fehlend ergeben `None` —
/// CheckMK meint damit „noch nie gewechselt", nicht den 1.1.1970.
fn flexible_timestamp<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<DateTime<Utc>>, D::Error> {
    let seconds = match serde_json::Value::deserialize(deserializer)? {
        serde_json::Value::Null => return Ok(None),
        serde_json::Value::Number(number) => number
            .as_f64()
            .ok_or_else(|| de::Error::custom("Zeitstempel liess sich nicht lesen"))?,
        serde_json::Value::String(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }
            // Manche Installationen liefern ISO8601 statt Epoch.
            if let Ok(parsed) = DateTime::parse_from_rfc3339(trimmed) {
                return Ok(Some(parsed.with_timezone(&Utc)));
            }
            trimmed
                .parse::<f64>()
                .map_err(|_| de::Error::custom(format!("unlesbarer Zeitstempel: {trimmed:?}")))?
        }
        other => {
            return Err(de::Error::custom(format!(
                "erwartet wurde ein Zeitstempel, gelesen wurde {other}"
            )))
        }
    };

    if seconds <= 0.0 {
        return Ok(None);
    }
    let whole = seconds.trunc() as i64;
    let nanos = ((seconds - seconds.trunc()) * 1e9).round() as u32;
    match Utc.timestamp_opt(whole, nanos.min(999_999_999)) {
        chrono::LocalResult::Single(value) => Ok(Some(value)),
        _ => Err(de::Error::custom(format!(
            "Zeitstempel liegt ausserhalb des darstellbaren Bereichs: {seconds}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /* ------------------------------------------------ Zustandsabbildung -- */

    #[test]
    fn servicezustaende_werden_richtig_abgebildet() {
        assert_eq!(service_state_from_raw(0), ProblemState::Ok);
        assert_eq!(service_state_from_raw(1), ProblemState::Warn);
        assert_eq!(service_state_from_raw(2), ProblemState::Crit);
        assert_eq!(service_state_from_raw(3), ProblemState::Unknown);
    }

    #[test]
    fn hostzustaende_werden_richtig_abgebildet() {
        assert_eq!(host_state_from_raw(0), ProblemState::Ok);
        assert_eq!(host_state_from_raw(1), ProblemState::Down);
        assert_eq!(host_state_from_raw(2), ProblemState::Unreachable);
    }

    /// Ein unerwarteter Zustandswert darf die Zeile nicht verschlucken.
    #[test]
    fn unerwarteter_zustandswert_wird_unknown_nicht_verworfen() {
        assert_eq!(service_state_from_raw(99), ProblemState::Unknown);
        assert_eq!(service_state_from_raw(-1), ProblemState::Unknown);
        assert_eq!(host_state_from_raw(42), ProblemState::Unknown);
    }

    /// Die Schwere-Reihenfolge muss der von src/lib/status.ts entsprechen,
    /// sonst sortieren Backend und Frontend unterschiedlich.
    #[test]
    fn schwere_stimmt_mit_dem_frontend_ueberein() {
        assert_eq!(ProblemState::Down.severity(), 50);
        assert_eq!(ProblemState::Crit.severity(), 40);
        assert_eq!(ProblemState::Unknown.severity(), 30);
        assert_eq!(ProblemState::Warn.severity(), 20);
        assert_eq!(ProblemState::Ok.severity(), 0);
        // Unreachable liegt zwischen Down und Crit.
        assert!(ProblemState::Down.severity() > ProblemState::Unreachable.severity());
        assert!(ProblemState::Unreachable.severity() > ProblemState::Crit.severity());
    }

    #[test]
    fn unreachable_und_down_teilen_den_anzeigeschluessel() {
        assert_eq!(ProblemState::Down.status_key(), "down");
        assert_eq!(ProblemState::Unreachable.status_key(), "down");
        assert_eq!(ProblemState::Crit.status_key(), "crit");
        assert_eq!(ProblemState::Unknown.status_key(), "unknown");
    }

    /* ------------------------------------------------------ Sortierung -- */

    #[test]
    fn sortiert_status_absteigend_dann_dauer_absteigend() {
        let alt = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let neu = Utc.timestamp_opt(1_700_009_000, 0).unwrap();

        let mut list = [
            Problem {
                last_state_change: Some(neu),
                ..problem(ProblemState::Warn, "b", Some("neu"))
            },
            Problem {
                last_state_change: Some(neu),
                ..problem(ProblemState::Crit, "c", Some("neu"))
            },
            Problem {
                last_state_change: Some(alt),
                ..problem(ProblemState::Crit, "a", Some("alt"))
            },
        ];
        list.sort_by(Problem::compare_for_list);

        // Erst CRIT, davon das ältere zuerst; dann WARN.
        assert_eq!(list[0].service.as_deref(), Some("alt"));
        assert_eq!(list[0].state, ProblemState::Crit);
        assert_eq!(list[1].state, ProblemState::Crit);
        assert_eq!(list[2].state, ProblemState::Warn);
    }

    #[test]
    fn zeilen_ohne_zeitstempel_landen_hinten() {
        let zeit = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let mut list = [
            problem(ProblemState::Crit, "ohne", Some("x")),
            Problem {
                last_state_change: Some(zeit),
                ..problem(ProblemState::Crit, "mit", Some("y"))
            },
        ];
        list.sort_by(Problem::compare_for_list);
        assert_eq!(list[0].host, "mit");
        assert_eq!(list[1].host, "ohne");
    }

    #[test]
    fn sortierung_ist_bei_gleichstand_stabil_und_nachvollziehbar() {
        let mut list = [
            problem(ProblemState::Warn, "zeta", Some("a")),
            problem(ProblemState::Warn, "alpha", Some("b")),
            problem(ProblemState::Warn, "alpha", Some("a")),
        ];
        list.sort_by(Problem::compare_for_list);
        assert_eq!(
            list.iter()
                .map(|p| (p.host.as_str(), p.service.as_deref().unwrap()))
                .collect::<Vec<_>>(),
            vec![("alpha", "a"), ("alpha", "b"), ("zeta", "a")]
        );
    }

    /* ----------------------------------------------------- Bearbeitung -- */

    #[test]
    fn quittiert_und_wartung_gelten_als_bearbeitet() {
        let mut p = problem(ProblemState::Crit, "h", Some("s"));
        assert!(!p.is_handled());

        p.acknowledged = true;
        assert!(p.is_handled());

        p.acknowledged = false;
        p.downtime_depth = 1;
        assert!(p.is_handled());
    }

    /* --------------------------------------------------------- Zähler -- */

    #[test]
    fn tooltip_nennt_nur_belegte_zustaende_absteigend() {
        let counts = StateCounts {
            crit: 3,
            warn: 7,
            ..Default::default()
        };
        assert_eq!(counts.tooltip(), "3 CRIT, 7 WARN");
        assert_eq!(counts.total(), 10);
    }

    #[test]
    fn tooltip_ohne_probleme_ist_ein_satz_kein_leerstring() {
        assert_eq!(StateCounts::default().tooltip(), "Keine offenen Probleme");
    }

    #[test]
    fn tooltip_reihenfolge_ist_absteigend_nach_schwere() {
        let counts = StateCounts {
            down: 1,
            unreachable: 2,
            crit: 3,
            unknown: 4,
            warn: 5,
        };
        assert_eq!(
            counts.tooltip(),
            "1 DOWN, 2 UNREACHABLE, 3 CRIT, 4 UNKNOWN, 5 WARN"
        );
    }

    /* -------------------------------------------------------- Snapshot -- */

    fn beispiel_snapshot() -> Snapshot {
        let now = Utc.timestamp_opt(1_700_100_000, 0).unwrap();
        Snapshot::new(
            vec![
                problem(ProblemState::Warn, "h1", Some("Memory")),
                Problem {
                    acknowledged: true,
                    ..problem(ProblemState::Crit, "h2", Some("Disk"))
                },
                Problem {
                    downtime_depth: 2,
                    ..problem(ProblemState::Crit, "h3", Some("CPU"))
                },
                problem(ProblemState::Down, "h4", None),
            ],
            now,
        )
    }

    #[test]
    fn snapshot_blendet_bearbeitete_standardmaessig_aus() {
        let snapshot = beispiel_snapshot();

        let sichtbar = snapshot.counts(false);
        assert_eq!(sichtbar.total(), 2, "quittiert und Wartung müssen raus");
        assert_eq!(sichtbar.down, 1);
        assert_eq!(sichtbar.warn, 1);
        assert_eq!(sichtbar.crit, 0);

        let alle = snapshot.counts(true);
        assert_eq!(alle.total(), 4);
        assert_eq!(alle.crit, 2);
    }

    #[test]
    fn snapshot_findet_den_schlimmsten_zustand() {
        let snapshot = beispiel_snapshot();
        assert_eq!(snapshot.worst(false), Some(ProblemState::Down));

        // Ohne Probleme gibt es keinen schlimmsten Zustand.
        let leer = Snapshot::new(vec![], Utc.timestamp_opt(0, 0).unwrap());
        assert_eq!(leer.worst(false), None);
        assert_eq!(leer.counts(false).tooltip(), "Keine offenen Probleme");
    }

    /// Wenn nur noch quittierte Probleme übrig sind, muss das Tray-Icon auf
    /// „alles gut" gehen — sonst leuchtet es dauerhaft rot.
    #[test]
    fn nur_quittierte_probleme_ergeben_keinen_schlimmsten_zustand() {
        let snapshot = Snapshot::new(
            vec![Problem {
                acknowledged: true,
                ..problem(ProblemState::Crit, "h", Some("s"))
            }],
            Utc.timestamp_opt(1, 0).unwrap(),
        );
        assert_eq!(snapshot.worst(false), None);
        assert_eq!(snapshot.worst(true), Some(ProblemState::Crit));
    }

    #[test]
    fn snapshot_sortiert_beim_anlegen() {
        let snapshot = beispiel_snapshot();
        let schweren: Vec<u8> = snapshot
            .problems
            .iter()
            .map(|p| p.state.severity())
            .collect();
        let mut absteigend = schweren.clone();
        absteigend.sort_unstable_by(|a, b| b.cmp(a));
        assert_eq!(schweren, absteigend, "Snapshot muss vorsortiert sein");
    }

    #[test]
    fn findet_ausgefallene_hosts_fuer_die_gruppierung() {
        let snapshot = Snapshot::new(
            vec![
                problem(ProblemState::Down, "esxi-03", None),
                problem(ProblemState::Unreachable, "vm-77", None),
                problem(ProblemState::Crit, "sql-01", Some("Disk")),
                // Ein Host-Eintrag mit UNKNOWN ist kein Ausfall.
                problem(ProblemState::Unknown, "print-01", None),
            ],
            Utc.timestamp_opt(1, 0).unwrap(),
        );
        let mut hosts = snapshot.failed_hosts(false);
        hosts.sort_unstable();
        assert_eq!(hosts, vec!["esxi-03", "vm-77"]);
    }

    /// Ein ausgefallener Host in Wartungszeit gehört nur dann in die
    /// Gruppierung, wenn die Liste ihn auch zeigt. Sonst zeigte die
    /// Gruppierung auf eine Zeile, die gar nicht da ist.
    #[test]
    fn ausgefallener_host_in_wartung_folgt_dem_filter() {
        let snapshot = Snapshot::new(
            vec![
                problem(ProblemState::Down, "offen", None),
                Problem {
                    downtime_depth: 1,
                    ..problem(ProblemState::Down, "in-wartung", None)
                },
                Problem {
                    acknowledged: true,
                    ..problem(ProblemState::Down, "quittiert", None)
                },
            ],
            Utc.timestamp_opt(1, 0).unwrap(),
        );

        assert_eq!(snapshot.failed_hosts(false), vec!["offen"]);

        let mut alle = snapshot.failed_hosts(true);
        alle.sort_unstable();
        assert_eq!(alle, vec!["in-wartung", "offen", "quittiert"]);
    }

    /* ------------------------------------------- Benachrichtigungsschlüssel */

    #[test]
    fn benachrichtigungsschluessel_unterscheidet_die_drei_bestandteile() {
        let a = problem(ProblemState::Crit, "h", Some("s"));
        let mut b = a.clone();
        b.state = ProblemState::Warn;
        let mut c = a.clone();
        c.service = Some("anders".into());
        let mut d = a.clone();
        d.host = "anders".into();

        let keys = [
            a.notification_key(),
            b.notification_key(),
            c.notification_key(),
            d.notification_key(),
        ];
        let eindeutig: std::collections::HashSet<_> = keys.iter().collect();
        assert_eq!(eindeutig.len(), 4, "Schlüssel kollidieren: {keys:?}");
    }

    /// Kollisionsfreiheit unabhängig vom Inhalt: dieselben Zeichen, anders
    /// zwischen Host und Service verteilt, müssen verschiedene Schlüssel geben.
    /// Ohne Längenpräfix wäre das derselbe String.
    #[test]
    fn benachrichtigungsschluessel_ist_gegen_trennzeichen_gefeit() {
        for trenner in ["|", "\u{1f}", ":", "|3:"] {
            let a = problem(ProblemState::Crit, "host", Some(&format!("a{trenner}b")));
            let b = problem(ProblemState::Crit, &format!("host{trenner}a"), Some("b"));
            assert_ne!(
                a.notification_key(),
                b.notification_key(),
                "Trennzeichen „{trenner}“ kollidiert"
            );
        }
    }

    /// Ein Hostproblem und ein Service mit leerem Namen auf demselben Host
    /// sind zwei verschiedene Dinge. Beides kommt in echten Antworten vor.
    #[test]
    fn hostproblem_und_service_ohne_namen_kollidieren_nicht() {
        let host_problem = problem(ProblemState::Crit, "h", None);
        let service_ohne_namen = problem(ProblemState::Crit, "h", Some(""));
        assert_ne!(
            host_problem.notification_key(),
            service_ohne_namen.notification_key()
        );
    }

    /* ---------------------------------------------------------- Dauer -- */

    #[test]
    fn berechnet_dauer_seit_statuswechsel() {
        let then = Utc.timestamp_opt(1_700_000_000, 0).unwrap();
        let now = Utc.timestamp_opt(1_700_003_661, 0).unwrap();
        let p = Problem {
            last_state_change: Some(then),
            ..problem(ProblemState::Crit, "h", Some("s"))
        };
        let dauer = p.duration_since_change(now).unwrap();
        assert_eq!(dauer.num_seconds(), 3661);

        let ohne = problem(ProblemState::Crit, "h", Some("s"));
        assert!(ohne.duration_since_change(now).is_none());
    }
}
