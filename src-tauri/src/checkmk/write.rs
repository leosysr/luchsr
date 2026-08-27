//! Schreibende Aktionen: Quittieren und Wartungszeit.
//!
//! Die Nutzlasten entsprechen wörtlich dem API-Vertrag aus CLAUDE.md. Sie
//! stehen hier als eigene Typen und nicht als handgebautes `serde_json::json!`,
//! damit ein Tippfehler im Feldnamen beim Kompilieren auffällt und nicht erst
//! als HTTP 400 beim Benutzer.
//!
//! ## ETag
//!
//! CheckMK verlangt bei Schreiboperationen auf **einzelne Objekte** eine
//! `If-Match`-Vorbedingung und antwortet ohne sie mit 428. Die hier benutzten
//! Endpunkte sind dagegen Collection-POSTs, die neue Objekte anlegen — dort ist
//! `If-Match` nicht vorgeschrieben.
//!
//! Da sich das zwischen CheckMK-Versionen unterschiedlich verhält, wird
//! `If-Match: *` grundsätzlich mitgesendet: wo der Server es nicht auswertet,
//! ignoriert er den Header, wo er es verlangt, ist er erfüllt. Das ist die
//! robuste Variante, die der Auftrag verlangt. 412 und 428 werden trotzdem
//! ausgewertet und als eigener Fehler gemeldet, damit die Ursache im Fehlerfall
//! benennbar bleibt.

use chrono::{DateTime, Duration, LocalResult, NaiveTime, TimeZone, Utc};
use serde::Serialize;

/// Stunde, die „morgen früh" meint. Bewusst eine Konstante mit Namen statt
/// einer Zahl im Code — falls das konfigurierbar werden soll, ist hier die
/// einzige Stelle.
pub const MORNING_HOUR: u32 = 7;

/* -------------------------------------------------------------------------- */
/* Quittieren                                                                 */
/* -------------------------------------------------------------------------- */

/// Optionen des Quittieren-Dialogs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AcknowledgeOptions {
    /// Bleibt bestehen, bis der Zustand wieder OK ist.
    pub sticky: bool,
    /// Übersteht einen Neustart des Monitoringkerns.
    pub persistent: bool,
    /// Löst eine Benachrichtigung über die Quittierung aus.
    pub notify: bool,
    pub comment: String,
}

impl Default for AcknowledgeOptions {
    /// Die Vorgaben des API-Vertrags: sticky und notify an, persistent aus.
    fn default() -> Self {
        Self {
            sticky: true,
            persistent: false,
            notify: true,
            comment: String::new(),
        }
    }
}

/// Nutzlast zum Quittieren eines Services.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AcknowledgeServiceBody {
    pub acknowledge_type: &'static str,
    pub sticky: bool,
    pub persistent: bool,
    pub notify: bool,
    pub comment: String,
    pub host_name: String,
    pub service_description: String,
}

/// Nutzlast zum Quittieren eines Hosts.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct AcknowledgeHostBody {
    pub acknowledge_type: &'static str,
    pub sticky: bool,
    pub persistent: bool,
    pub notify: bool,
    pub comment: String,
    pub host_name: String,
}

impl AcknowledgeServiceBody {
    pub fn new(host: &str, service: &str, options: &AcknowledgeOptions) -> Self {
        Self {
            acknowledge_type: "service",
            sticky: options.sticky,
            persistent: options.persistent,
            notify: options.notify,
            comment: options.comment.clone(),
            host_name: host.to_string(),
            service_description: service.to_string(),
        }
    }
}

impl AcknowledgeHostBody {
    pub fn new(host: &str, options: &AcknowledgeOptions) -> Self {
        Self {
            acknowledge_type: "host",
            sticky: options.sticky,
            persistent: options.persistent,
            notify: options.notify,
            comment: options.comment.clone(),
            host_name: host.to_string(),
        }
    }
}

/* -------------------------------------------------------------------------- */
/* Wartungszeit                                                               */
/* -------------------------------------------------------------------------- */

/// Die Dauer-Voreinstellungen des Wartungsdialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DowntimeDuration {
    Minutes15,
    Hour1,
    Hours4,
    /// Bis zum nächsten Morgen um [`MORNING_HOUR`].
    UntilMorning,
    /// Frei wählbar, in Minuten.
    Minutes(i64),
}

impl DowntimeDuration {
    /// Berechnet Beginn und Ende der Wartungszeit.
    ///
    /// Beginn ist immer *jetzt*. Rückgabe in UTC, weil die API RFC3339 will;
    /// gerechnet wird aber in der lokalen Zeitzone, denn „morgen früh" ist eine
    /// Aussage über die Ortszeit des Benutzers, nicht über UTC.
    ///
    /// `UntilMorning` liefert das **nächste** Auftreten von [`MORNING_HOUR`]
    /// nach `now`. Wer um 02:00 nachts eine Wartung setzt, meint 07:00 desselben
    /// Tages und nicht 07:00 am Folgetag — sonst entstünde eine 29-Stunden-
    /// Wartung, die niemand wollte.
    pub fn window<Tz: TimeZone>(self, now: DateTime<Tz>) -> (DateTime<Utc>, DateTime<Utc>) {
        let start = now.clone().with_timezone(&Utc);
        let end = match self {
            Self::Minutes15 => start + Duration::minutes(15),
            Self::Hour1 => start + Duration::hours(1),
            Self::Hours4 => start + Duration::hours(4),
            Self::Minutes(minutes) => start + Duration::minutes(minutes.max(1)),
            Self::UntilMorning => next_morning(now),
        };
        // Ein Ende, das nicht nach dem Beginn liegt, weist CheckMK mit 400 ab.
        // Eine Minute Mindestdauer ist harmloser als eine Fehlermeldung.
        let end = if end <= start {
            start + Duration::minutes(1)
        } else {
            end
        };
        (start, end)
    }

    /// Beschriftung für das UI.
    pub fn label(self) -> String {
        match self {
            Self::Minutes15 => "15 Minuten".to_string(),
            Self::Hour1 => "1 Stunde".to_string(),
            Self::Hours4 => "4 Stunden".to_string(),
            Self::UntilMorning => format!("Bis morgen früh ({MORNING_HOUR}:00)"),
            Self::Minutes(minutes) => format!("{minutes} Minuten"),
        }
    }
}

/// Das nächste Auftreten von [`MORNING_HOUR`] nach `now`, in UTC.
fn next_morning<Tz: TimeZone>(now: DateTime<Tz>) -> DateTime<Utc> {
    let zone = now.timezone();
    let target = NaiveTime::from_hms_opt(MORNING_HOUR, 0, 0).expect("MORNING_HOUR ist gültig");
    let local_now = now.naive_local();

    // Heute, wenn die Zielzeit noch aussteht, sonst morgen.
    let mut day = local_now.date();
    if local_now.time() >= target {
        day = day
            .succ_opt()
            .expect("Datum liegt weit vor dem darstellbaren Ende");
    }

    for _ in 0..3 {
        let naive = day.and_time(target);
        match zone.from_local_datetime(&naive) {
            // Eindeutig — der Normalfall.
            LocalResult::Single(value) => return value.with_timezone(&Utc),
            // Zeitumstellung im Herbst: die Stunde existiert zweimal. Die
            // frühere nehmen, damit die Wartung nicht länger dauert als gedacht.
            LocalResult::Ambiguous(earlier, _) => return earlier.with_timezone(&Utc),
            // Zeitumstellung im Frühjahr: die Stunde fällt aus. Nächster Tag.
            LocalResult::None => {
                day = day.succ_opt().expect("Datum überläuft nicht");
            }
        }
    }
    // Kann in der Praxis nicht eintreten; ein sinnvoller Wert ist besser als
    // eine Panik in einer Nebenfunktion.
    now.with_timezone(&Utc) + Duration::hours(12)
}

/// Nutzlast für eine Service-Wartungszeit.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DowntimeServiceBody {
    pub downtime_type: &'static str,
    pub start_time: String,
    pub end_time: String,
    pub comment: String,
    pub host_name: String,
    /// Mehrzahl — die API nimmt hier eine Liste, anders als beim Quittieren.
    pub service_descriptions: Vec<String>,
}

/// Nutzlast für eine Host-Wartungszeit.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct DowntimeHostBody {
    pub downtime_type: &'static str,
    pub start_time: String,
    pub end_time: String,
    pub comment: String,
    pub host_name: String,
}

impl DowntimeServiceBody {
    pub fn new(
        host: &str,
        services: &[String],
        comment: &str,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Self {
        Self {
            downtime_type: "service",
            start_time: rfc3339(start),
            end_time: rfc3339(end),
            comment: comment.to_string(),
            host_name: host.to_string(),
            service_descriptions: services.to_vec(),
        }
    }
}

impl DowntimeHostBody {
    pub fn new(host: &str, comment: &str, start: DateTime<Utc>, end: DateTime<Utc>) -> Self {
        Self {
            downtime_type: "host",
            start_time: rfc3339(start),
            end_time: rfc3339(end),
            comment: comment.to_string(),
            host_name: host.to_string(),
        }
    }
}

/// RFC3339 mit Sekundenauflösung und `Z`. CheckMK akzeptiert keine
/// Nanosekunden-Auflösung in allen Versionen zuverlässig.
fn rfc3339(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    // Timelike nur hier: .hour() wird ausschliesslich im Test gebraucht.
    use chrono::{FixedOffset, Timelike};

    /// Mitteleuropäische Winterzeit, UTC+1.
    fn mez() -> FixedOffset {
        FixedOffset::east_opt(3600).unwrap()
    }

    fn lokal(jahr: i32, monat: u32, tag: u32, stunde: u32, minute: u32) -> DateTime<FixedOffset> {
        mez()
            .with_ymd_and_hms(jahr, monat, tag, stunde, minute, 0)
            .unwrap()
    }

    /* ------------------------------------------------------- Quittieren -- */

    #[test]
    fn quittieren_vorgaben_entsprechen_dem_vertrag() {
        let options = AcknowledgeOptions::default();
        assert!(options.sticky, "sticky ist im Vertrag true");
        assert!(!options.persistent, "persistent ist im Vertrag false");
        assert!(options.notify, "notify ist im Vertrag true");
    }

    #[test]
    fn quittieren_nutzlast_hat_die_vertragsfelder() {
        let body = AcknowledgeServiceBody::new(
            "leosys-sql-01",
            "Filesystem /var",
            &AcknowledgeOptions {
                comment: "Platte wird morgen getauscht".into(),
                ..Default::default()
            },
        );
        let json: serde_json::Value = serde_json::to_value(&body).unwrap();

        assert_eq!(json["acknowledge_type"], "service");
        assert_eq!(json["sticky"], true);
        assert_eq!(json["persistent"], false);
        assert_eq!(json["notify"], true);
        assert_eq!(json["comment"], "Platte wird morgen getauscht");
        assert_eq!(json["host_name"], "leosys-sql-01");
        assert_eq!(json["service_description"], "Filesystem /var");

        // Genau sieben Felder — kein zusätzliches, das CheckMK abweisen würde.
        assert_eq!(json.as_object().unwrap().len(), 7);
    }

    #[test]
    fn host_quittieren_hat_kein_service_feld() {
        let body = AcknowledgeHostBody::new("leosys-esxi-03", &AcknowledgeOptions::default());
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["acknowledge_type"], "host");
        assert!(json.get("service_description").is_none());
        assert_eq!(json.as_object().unwrap().len(), 6);
    }

    /// Sonderzeichen im Kommentar dürfen nicht das JSON zerlegen.
    #[test]
    fn kommentar_mit_sonderzeichen_bleibt_gueltiges_json() {
        let body = AcknowledgeServiceBody::new(
            "h",
            "s",
            &AcknowledgeOptions {
                comment: "Größe \"kritisch\", siehe Ticket #42\\n\nZeile 2".into(),
                ..Default::default()
            },
        );
        let text = serde_json::to_string(&body).unwrap();
        let zurueck: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(
            zurueck["comment"],
            "Größe \"kritisch\", siehe Ticket #42\\n\nZeile 2"
        );
    }

    /* ----------------------------------------------------- Wartungszeit -- */

    #[test]
    fn feste_dauern_rechnen_richtig() {
        let now = lokal(2026, 1, 15, 14, 30);
        for (dauer, minuten) in [
            (DowntimeDuration::Minutes15, 15),
            (DowntimeDuration::Hour1, 60),
            (DowntimeDuration::Hours4, 240),
            (DowntimeDuration::Minutes(90), 90),
        ] {
            let (start, end) = dauer.window(now);
            assert_eq!(
                (end - start).num_minutes(),
                minuten,
                "{dauer:?} ergibt die falsche Dauer"
            );
        }
    }

    #[test]
    fn beginn_ist_immer_jetzt() {
        let now = lokal(2026, 1, 15, 14, 30);
        let (start, _) = DowntimeDuration::Hour1.window(now);
        assert_eq!(start, now.with_timezone(&Utc));
    }

    /// Vormittags gesetzt heisst: morgen um 7.
    #[test]
    fn bis_morgen_frueh_am_vormittag_ist_der_folgetag() {
        let now = lokal(2026, 1, 15, 9, 0);
        let (_, end) = DowntimeDuration::UntilMorning.window(now);
        let lokal_ende = end.with_timezone(&mez());
        assert_eq!(lokal_ende, lokal(2026, 1, 16, 7, 0));
    }

    /// Nachts um 2 gesetzt heisst: heute um 7, nicht morgen um 7.
    /// Sonst wären es 29 Stunden Wartung statt 5.
    #[test]
    fn bis_morgen_frueh_nachts_ist_derselbe_tag() {
        let now = lokal(2026, 1, 15, 2, 0);
        let (start, end) = DowntimeDuration::UntilMorning.window(now);
        let lokal_ende = end.with_timezone(&mez());
        assert_eq!(lokal_ende, lokal(2026, 1, 15, 7, 0));
        assert_eq!(
            (end - start).num_hours(),
            5,
            "Nachts gesetzt darf keine 29-Stunden-Wartung ergeben"
        );
    }

    /// Genau um 07:00 gesetzt: die Zielzeit ist erreicht, also der Folgetag.
    #[test]
    fn bis_morgen_frueh_genau_um_sieben_ist_der_folgetag() {
        let now = lokal(2026, 1, 15, 7, 0);
        let (_, end) = DowntimeDuration::UntilMorning.window(now);
        assert_eq!(end.with_timezone(&mez()), lokal(2026, 1, 16, 7, 0));
    }

    #[test]
    fn bis_morgen_frueh_ueber_den_monatswechsel() {
        let now = lokal(2026, 1, 31, 22, 0);
        let (_, end) = DowntimeDuration::UntilMorning.window(now);
        assert_eq!(end.with_timezone(&mez()), lokal(2026, 2, 1, 7, 0));
    }

    #[test]
    fn bis_morgen_frueh_ueber_den_jahreswechsel() {
        let now = lokal(2026, 12, 31, 23, 30);
        let (_, end) = DowntimeDuration::UntilMorning.window(now);
        assert_eq!(end.with_timezone(&mez()), lokal(2027, 1, 1, 7, 0));
    }

    /// Eine freie Dauer von 0 oder negativ darf keine Wartung erzeugen, die
    /// CheckMK mit 400 abweist.
    #[test]
    fn nicht_positive_dauer_wird_auf_eine_minute_angehoben() {
        let now = lokal(2026, 1, 15, 14, 30);
        for minuten in [0, -1, -600] {
            let (start, end) = DowntimeDuration::Minutes(minuten).window(now);
            assert!(
                end > start,
                "{minuten} Minuten ergaben ein Ende vor dem Beginn"
            );
            assert_eq!((end - start).num_minutes(), 1);
        }
    }

    #[test]
    fn wartungs_nutzlast_hat_die_vertragsfelder() {
        let start = Utc.with_ymd_and_hms(2026, 1, 15, 13, 30, 0).unwrap();
        let end = Utc.with_ymd_and_hms(2026, 1, 15, 17, 30, 0).unwrap();
        let body = DowntimeServiceBody::new(
            "leosys-sql-01",
            &["Filesystem /var".to_string(), "Memory".to_string()],
            "Wartungsfenster Storage",
            start,
            end,
        );
        let json = serde_json::to_value(&body).unwrap();

        assert_eq!(json["downtime_type"], "service");
        assert_eq!(json["start_time"], "2026-01-15T13:30:00Z");
        assert_eq!(json["end_time"], "2026-01-15T17:30:00Z");
        assert_eq!(json["comment"], "Wartungsfenster Storage");
        assert_eq!(json["host_name"], "leosys-sql-01");
        assert_eq!(
            json["service_descriptions"],
            serde_json::json!(["Filesystem /var", "Memory"])
        );
        assert_eq!(json.as_object().unwrap().len(), 6);
    }

    #[test]
    fn host_wartung_hat_keine_serviceliste() {
        let start = Utc.with_ymd_and_hms(2026, 1, 15, 13, 30, 0).unwrap();
        let end = start + Duration::hours(1);
        let body = DowntimeHostBody::new("leosys-esxi-03", "Reboot", start, end);
        let json = serde_json::to_value(&body).unwrap();
        assert_eq!(json["downtime_type"], "host");
        assert!(json.get("service_descriptions").is_none());
        assert_eq!(json.as_object().unwrap().len(), 5);
    }

    /// Zeitstempel müssen ohne Nachkommastellen und mit Z gehen — manche
    /// CheckMK-Versionen stolpern über Nanosekunden.
    #[test]
    fn zeitstempel_sind_sekundengenau_mit_z() {
        // Aus einem lesbaren Datum gebaut, nicht aus einer Epoch-Zahl: eine
        // Magic Number hier hätte man gegen die falsche Erwartung geprüft.
        let mit_nanos = Utc
            .with_ymd_and_hms(2026, 1, 15, 15, 0, 0)
            .unwrap()
            .with_nanosecond(123_456_789)
            .unwrap();
        let text = rfc3339(mit_nanos);
        assert_eq!(text, "2026-01-15T15:00:00Z");
        assert!(!text.contains('.'), "Nachkommastellen in: {text}");
        assert!(text.ends_with('Z'), "kein Z-Suffix in: {text}");
    }

    #[test]
    fn beschriftungen_sind_deutsch_und_nennen_die_stunde() {
        assert_eq!(DowntimeDuration::Minutes15.label(), "15 Minuten");
        assert_eq!(DowntimeDuration::Hour1.label(), "1 Stunde");
        assert_eq!(DowntimeDuration::Hours4.label(), "4 Stunden");
        assert_eq!(DowntimeDuration::Minutes(90).label(), "90 Minuten");
        let morgen = DowntimeDuration::UntilMorning.label();
        assert!(
            morgen.contains(&MORNING_HOUR.to_string()),
            "Stunde fehlt in: {morgen}"
        );
    }

    /// Die Zeitzonenrechnung darf nicht in UTC stattfinden: in MEZ ist 07:00
    /// lokal 06:00 UTC.
    #[test]
    fn morgenstunde_wird_in_ortszeit_gerechnet_nicht_in_utc() {
        let now = lokal(2026, 1, 15, 9, 0);
        let (_, end) = DowntimeDuration::UntilMorning.window(now);
        assert_eq!(end.hour(), 6, "07:00 MEZ muss 06:00 UTC sein, ist: {end}");
    }
}
