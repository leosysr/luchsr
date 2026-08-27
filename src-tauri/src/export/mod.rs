//! CSV-Ausgabe der Problemliste.
//!
//! Alles Entscheidende steckt in [`to_csv`] — einer reinen Funktion über
//! `&[Problem]`. Das Schreiben auf die Platte macht der Befehl in `commands`,
//! damit die Formatierung ohne Datei und ohne Server geprüft werden kann.
//!
//! # Drei Dinge, die eine CSV-Datei auf einem deutschen Windows kaputtmachen
//!
//! **Trennzeichen.** Excel liest in einer deutschen Installation `;` als
//! Trennzeichen, nicht `,`. Eine kommagetrennte Datei landet dort komplett in
//! Spalte A. Deshalb `;` — siehe [`SEPARATOR`].
//!
//! **Kodierung.** Ohne BOM hält Excel die Datei für die Codepage des Systems
//! und macht aus „Wärmefühler" einen „WÃ¤rmefÃ¼hler". Deshalb schreibt
//! [`to_csv`] die Byte-Order-Mark voran.
//!
//! **Formeln.** `plugin_output` kommt aus einem Monitoring-System und ist damit
//! Fremdtext. Beginnt ein Feld mit `=`, `+`, `-` oder `@`, wertet Excel es als
//! Formel — im schlimmsten Fall eine, die etwas ausführt. Siehe
//! [`entschaerfen`].

use crate::checkmk::{Problem, ProblemState, Snapshot};

/// Excel erwartet auf einem deutschen System `;`.
const SEPARATOR: char = ';';

/// Zeichen, mit denen Excel ein Feld als Formel liest.
const FORMELSTART: [char; 4] = ['=', '+', '-', '@'];

/// Byte-Order-Mark. Ohne die rät Excel die Kodierung — und rät falsch.
const BOM: &str = "\u{feff}";

/// Spaltenköpfe, in der Reihenfolge der Ausgabe.
const KOPF: [&str; 9] = [
    "Host",
    "Service",
    "Status",
    "Seit",
    "Dauer",
    "Quittiert",
    "Wartung",
    "Flattert",
    "Ausgabe",
];

/// Deutscher Name eines Zustands. Muss zu `status.*` in `src/i18n/de.ts`
/// passen — eine Exportdatei, die andere Wörter benutzt als die Oberfläche,
/// ist beim Vergleichen wertlos.
fn zustand(state: ProblemState) -> &'static str {
    match state {
        ProblemState::Ok => "OK",
        ProblemState::Warn => "Warnung",
        ProblemState::Crit => "Kritisch",
        ProblemState::Unknown => "Unbekannt",
        ProblemState::Down => "Host nicht erreichbar",
        ProblemState::Unreachable => "Host über keinen Pfad erreichbar",
    }
}

/// Nimmt einem Feld die Formelwirkung, indem ein Apostroph vorangestellt wird.
///
/// Das ist die übliche Gegenmaßnahme (OWASP: „CSV Injection"). Sie verändert
/// den Text sichtbar — das ist der Preis dafür, dass `=cmd|' /c calc'!A0` in
/// einer Zelle nur Text bleibt. Betroffen sind in der Praxis nur Ausgaben, die
/// mit einem Rechenzeichen anfangen; ein normaler `plugin_output` nicht.
fn entschaerfen(feld: &str) -> String {
    match feld.chars().next() {
        Some(erstes) if FORMELSTART.contains(&erstes) => format!("'{feld}"),
        _ => feld.to_owned(),
    }
}

/// Setzt ein Feld in Anführungszeichen, wenn es sein muss.
///
/// Nötig bei Trennzeichen, Anführungszeichen und Zeilenumbrüchen — alle drei
/// kommen in `plugin_output` vor. Innere Anführungszeichen werden nach RFC 4180
/// verdoppelt.
fn feld(inhalt: &str) -> String {
    let text = entschaerfen(inhalt);
    let muss = text.contains(SEPARATOR)
        || text.contains('"')
        || text.contains('\n')
        || text.contains('\r');
    if muss {
        format!("\"{}\"", text.replace('"', "\"\""))
    } else {
        text
    }
}

/// Zeitstempel als `TT.MM.JJJJ HH:MM:SS` in **Ortszeit**.
///
/// Zwei Gründe gegen ISO-8601 in UTC, das hier zuerst stand:
///
/// Erstens zeigt die Liste Ortszeit (`toLocaleString("de-DE")` in
/// `duration.ts`). Eine Exportdatei, die andere Zeiten nennt als die Ansicht,
/// aus der sie kommt, lädt zu Rechenfehlern ein — im Sommer um zwei Stunden.
///
/// Zweitens liest Excel diese Form als **Datum**; ein ISO-Zeitstempel mit
/// Zonenversatz bleibt dort Text und lässt sich nicht als Zeit sortieren oder
/// filtern.
///
/// Die Zone geht damit verloren. Für einen Bericht, der im eigenen Haus
/// gelesen wird, ist das der bessere Tausch.
fn zeitpunkt(t: chrono::DateTime<chrono::Utc>) -> String {
    chrono::DateTime::<chrono::Local>::from(t)
        .format("%d.%m.%Y %H:%M:%S")
        .to_string()
}

/// Dauer seit dem Statuswechsel, als `HH:MM:SS` bzw. `Nd HH:MM`.
///
/// Bewusst dieselbe Form wie `formatDuration` in `src/features/problems/
/// duration.ts`: die Datei soll aussehen wie die Liste, aus der sie kommt.
fn dauer(problem: &Problem, bezug: chrono::DateTime<chrono::Utc>) -> String {
    let Some(seit) = problem.last_state_change else {
        return String::new();
    };
    let sekunden = (bezug - seit).num_seconds();
    if sekunden < 0 {
        // Uhren dürfen auseinanderlaufen; eine negative Dauer auszugeben wäre
        // schlechter als sie weglassen.
        return String::new();
    }
    let tage = sekunden / 86_400;
    let stunden = (sekunden % 86_400) / 3_600;
    let minuten = (sekunden % 3_600) / 60;
    if tage > 0 {
        format!("{tage}d {stunden:02}:{minuten:02}")
    } else {
        format!("{stunden:02}:{minuten:02}:{:02}", sekunden % 60)
    }
}

/// „ja" oder leer. Ein leeres Feld filtert sich in Excel bequemer als „nein".
fn ja(wert: bool) -> &'static str {
    if wert {
        "ja"
    } else {
        ""
    }
}

/// Baut die vollständige Datei.
///
/// `bezug` ist der Zeitpunkt, gegen den die Dauern gerechnet werden — als
/// Parameter, damit der Test nicht von der Uhr abhängt. Sinnvoll ist der
/// `fetched_at` des Abzugs: die Dauern gehören zum Stand der Daten, nicht zum
/// Zeitpunkt des Klicks.
pub fn to_csv(problems: &[Problem], bezug: chrono::DateTime<chrono::Utc>) -> String {
    let mut aus = String::from(BOM);
    aus.push_str(&KOPF.join(&SEPARATOR.to_string()));
    // CRLF, weil die Datei auf einem Windows-Rechner in Excel landet.
    aus.push_str("\r\n");

    for problem in problems {
        let spalten = [
            feld(&problem.host),
            feld(problem.service.as_deref().unwrap_or("")),
            feld(zustand(problem.state)),
            problem.last_state_change.map(zeitpunkt).unwrap_or_default(),
            dauer(problem, bezug),
            ja(problem.acknowledged).to_owned(),
            ja(problem.downtime_depth > 0).to_owned(),
            ja(problem.flapping).to_owned(),
            feld(&problem.output),
        ];
        aus.push_str(&spalten.join(&SEPARATOR.to_string()));
        aus.push_str("\r\n");
    }

    aus
}

/// Vorgeschlagener Dateiname, etwa `luchsr-2026-08-24_1432.csv`.
///
/// Der Zeitstempel kommt aus dem Abzug und nicht aus der aktuellen Uhrzeit:
/// zwei Exporte desselben Abzugs sollen denselben Namen vorschlagen.
pub fn dateiname(snapshot: &Snapshot) -> String {
    format!("luchsr-{}.csv", snapshot.fetched_at.format("%Y-%m-%d_%H%M"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn utc(tag: u32, stunde: u32, minute: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, tag, stunde, minute, 0)
            .unwrap()
    }

    fn problem(host: &str, service: Option<&str>, state: ProblemState) -> Problem {
        Problem {
            host: host.to_owned(),
            service: service.map(str::to_owned),
            state,
            output: "alles ruhig".to_owned(),
            last_state_change: Some(utc(24, 12, 0)),
            acknowledged: false,
            downtime_depth: 0,
            flapping: false,
        }
    }

    #[test]
    fn kopfzeile_und_bom_stehen_vorn() {
        let csv = to_csv(&[], utc(24, 14, 0));
        assert!(csv.starts_with(BOM), "BOM fehlt — Excel rät die Kodierung");
        assert!(csv.contains("Host;Service;Status;Seit;Dauer"));
        assert!(csv.ends_with("\r\n"));
    }

    /// Geprüft wird die **Form**, nicht der Wert: die Ortszeit hängt von der
    /// Einstellung des Rechners ab, und ein Test, der eine Zone festnagelt,
    /// prüft die Zone und nicht die Formatierung.
    #[test]
    fn seit_steht_in_deutscher_form_damit_excel_es_als_datum_liest() {
        let csv = to_csv(
            &[problem("srv01", Some("CPU"), ProblemState::Crit)],
            utc(24, 14, 0),
        );
        let spalten: Vec<&str> = csv.lines().nth(1).unwrap().split(SEPARATOR).collect();
        let seit = spalten[3];
        assert_eq!(seit.len(), 19, "unerwartete Länge: {seit}");
        let ziffern: Vec<char> = seit.chars().collect();
        assert_eq!(ziffern[2], '.', "{seit}");
        assert_eq!(ziffern[5], '.', "{seit}");
        assert_eq!(ziffern[10], ' ', "{seit}");
        assert_eq!(ziffern[13], ':', "{seit}");
        assert_eq!(ziffern[16], ':', "{seit}");
        assert!(
            !seit.contains('T') && !seit.contains('+'),
            "sieht noch nach ISO aus: {seit}"
        );
    }

    #[test]
    fn leere_liste_gibt_nur_die_kopfzeile() {
        let csv = to_csv(&[], utc(24, 14, 0));
        assert_eq!(csv.lines().count(), 1);
    }

    #[test]
    fn hostproblem_hat_eine_leere_servicespalte() {
        let csv = to_csv(
            &[problem("srv01", None, ProblemState::Down)],
            utc(24, 14, 0),
        );
        let zeile = csv.lines().nth(1).unwrap();
        assert!(zeile.starts_with("srv01;;Host nicht erreichbar;"));
    }

    #[test]
    fn dauer_wird_gegen_den_bezug_gerechnet() {
        let csv = to_csv(
            &[problem("srv01", Some("CPU"), ProblemState::Crit)],
            utc(24, 14, 30),
        );
        assert!(csv.contains(";02:30:00;"), "Dauer falsch: {csv}");
    }

    #[test]
    fn dauer_ab_einem_tag_wechselt_die_form() {
        let csv = to_csv(
            &[problem("srv01", Some("CPU"), ProblemState::Crit)],
            utc(26, 15, 45),
        );
        assert!(csv.contains(";2d 03:45;"), "Dauer falsch: {csv}");
    }

    #[test]
    fn ohne_statuswechsel_bleiben_seit_und_dauer_leer() {
        let mut p = problem("srv01", Some("CPU"), ProblemState::Warn);
        p.last_state_change = None;
        let csv = to_csv(&[p], utc(24, 14, 0));
        assert!(csv.contains("Warnung;;;"), "Seit/Dauer nicht leer: {csv}");
    }

    /// Uhren dürfen auseinanderlaufen. Eine negative Dauer wäre Unsinn.
    ///
    /// Geprüft wird die Spalte, nicht die ganze Zeile: der ISO-Zeitstempel in
    /// „Seit" enthält selbst Bindestriche.
    #[test]
    fn zukuenftiger_statuswechsel_gibt_keine_negative_dauer() {
        let csv = to_csv(
            &[problem("srv01", Some("CPU"), ProblemState::Warn)],
            utc(24, 11, 0),
        );
        let zeile = csv.lines().nth(1).unwrap();
        let spalten: Vec<&str> = zeile.split(SEPARATOR).collect();
        assert_eq!(spalten[4], "", "Dauer nicht leer: {zeile}");
    }

    #[test]
    fn trennzeichen_in_der_ausgabe_wird_gequotet() {
        let mut p = problem("srv01", Some("Disk"), ProblemState::Crit);
        p.output = "belegt: 91%; Grenze 90%".to_owned();
        let csv = to_csv(&[p], utc(24, 14, 0));
        assert!(csv.contains("\"belegt: 91%; Grenze 90%\""), "{csv}");
    }

    #[test]
    fn anfuehrungszeichen_werden_verdoppelt() {
        let mut p = problem("srv01", Some("Disk"), ProblemState::Crit);
        p.output = "Dienst \"Spooler\" tot".to_owned();
        let csv = to_csv(&[p], utc(24, 14, 0));
        assert!(csv.contains("\"Dienst \"\"Spooler\"\" tot\""), "{csv}");
    }

    #[test]
    fn zeilenumbruch_in_der_ausgabe_bleibt_im_feld() {
        let mut p = problem("srv01", Some("Disk"), ProblemState::Crit);
        p.output = "Zeile 1\nZeile 2".to_owned();
        let csv = to_csv(&[p], utc(24, 14, 0));
        // Gequotet, also gehört der Umbruch zum Feld — die Datei hat trotzdem
        // nur eine Datenzeile im Sinne von CSV.
        assert!(csv.contains("\"Zeile 1\nZeile 2\""), "{csv}");
    }

    /// Der Kern der Formelabwehr.
    #[test]
    fn formelstart_wird_entschaerft() {
        for gift in ["=1+1", "+42", "-cmd", "@SUM(A1)"] {
            let mut p = problem("srv01", Some("Disk"), ProblemState::Crit);
            p.output = gift.to_owned();
            let csv = to_csv(&[p], utc(24, 14, 0));
            assert!(
                csv.contains(&format!("'{gift}")),
                "nicht entschärft: {gift} in {csv}"
            );
        }
    }

    /// Auch der Hostname kommt vom Server und ist damit Fremdtext.
    #[test]
    fn formelabwehr_gilt_auch_fuer_host_und_service() {
        let mut p = problem("=böse", Some("=auch"), ProblemState::Crit);
        p.output = "harmlos".to_owned();
        let csv = to_csv(&[p], utc(24, 14, 0));
        assert!(csv.contains("'=böse;'=auch;"), "{csv}");
    }

    #[test]
    fn merkmale_erscheinen_als_ja_oder_leer() {
        let mut p = problem("srv01", Some("Disk"), ProblemState::Crit);
        p.acknowledged = true;
        p.downtime_depth = 2;
        p.flapping = false;
        let csv = to_csv(&[p], utc(24, 14, 0));
        // Quittiert=ja, Wartung=ja, Flattert=leer
        assert!(csv.contains(";ja;ja;;"), "{csv}");
    }

    #[test]
    fn jede_zeile_hat_gleich_viele_trennzeichen() {
        let problems = vec![
            problem("a", None, ProblemState::Down),
            problem("b", Some("S"), ProblemState::Warn),
        ];
        let csv = to_csv(&problems, utc(24, 14, 0));
        // Zählen geht nur, weil in diesem Fall kein Feld gequotet ist.
        let zaehlungen: Vec<usize> = csv
            .lines()
            .map(|zeile| zeile.matches(SEPARATOR).count())
            .collect();
        assert_eq!(zaehlungen, vec![KOPF.len() - 1; 3], "{csv}");
    }

    #[test]
    fn dateiname_kommt_aus_dem_abzug() {
        let snapshot = Snapshot {
            problems: vec![],
            fetched_at: utc(24, 14, 32),
        };
        assert_eq!(dateiname(&snapshot), "luchsr-2026-08-24_1432.csv");
    }
}

#[cfg(test)]
mod gegen_fixture {
    use super::*;
    use crate::checkmk::Snapshot;
    use chrono::TimeZone;

    /// Ein Durchlauf gegen die **aufgezeichnete Serverantwort** statt gegen
    /// handgebaute Strukturen. Damit ist geprüft, dass die Formatierung auch
    /// die Eigenheiten echter `plugin_output`-Texte übersteht.
    #[test]
    fn echte_serverantwort_wird_zu_lesbarem_csv() {
        let problems = crate::checkmk::parse_services(include_str!(
            "../checkmk/fixtures/services_problems.json"
        ))
        .expect("Fixture liess sich nicht auswerten");
        assert!(!problems.is_empty(), "Fixture enthält keine Probleme");

        let snapshot = Snapshot {
            fetched_at: chrono::Utc
                .with_ymd_and_hms(2026, 8, 24, 14, 32, 0)
                .unwrap(),
            problems,
        };
        let csv = to_csv(&snapshot.problems, snapshot.fetched_at);

        // Kopfzeile plus je eine Zeile pro Problem. Zählt nur, solange kein
        // Feld einen Umbruch enthält — die Fixture hat keinen.
        assert_eq!(
            csv.matches("\r\n").count(),
            snapshot.problems.len() + 1,
            "Zeilenzahl passt nicht:\n{csv}"
        );

        // Und jeder Host taucht namentlich auf. Ein Export, der Zeilen
        // verschluckt, wäre schlimmer als keiner.
        for problem in &snapshot.problems {
            assert!(
                csv.contains(&problem.host),
                "Host {} fehlt in der Datei",
                problem.host
            );
        }
    }
}
