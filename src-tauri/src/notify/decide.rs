//! Was aus einem Abzug zu melden ist — reine Entscheidung, ohne Windows.
//!
//! Hier steckt die ganze Logik der Benachrichtigungen. Sie hängt an nichts
//! ausser den Eingaben und ist damit ohne Toasts, ohne Server und ohne Klick
//! prüfbar. Das Versenden macht `notify/mod.rs`.
//!
//! # Das Gedächtnis
//!
//! Gemerkt wird eine Zuordnung **Gegenstand → gemeldeter Zustand**, wobei der
//! Gegenstand Host und Service ist (ohne Zustand). Der Auftrag beschreibt eine
//! Menge aus `(host, service, state)`; als Zuordnung geschrieben leistet sie
//! dasselbe und beantwortet zusätzlich die Frage, die eine Menge nicht
//! beantworten kann: *ist dieser Gegenstand inzwischen weg?*
//!
//! Eine reine Menge hätte einen stillen Fehler: nach `CRIT → OK → CRIT` wäre
//! der Schlüssel noch enthalten, und das zweite CRIT käme nie an.
//!
//! # Warum nicht beim ersten Abzug gemeldet wird
//!
//! Der erste Abzug nach dem Start enthält **alle** offenen Probleme. Bei 40
//! Zeilen wären das 40 Toasts hintereinander — und weil Luchsr im Autostart
//! läuft, bei jeder Anmeldung. Der erste Abzug füllt deshalb nur das
//! Gedächtnis. Siehe [`Decision::first_run`].

use std::collections::BTreeMap;

use crate::checkmk::{Problem, ProblemState, Snapshot};
use crate::config::NotificationLevel;
use crate::i18n::notify as text;

/// Gegenstand einer Meldung: Host und Service, **ohne** Zustand.
///
/// Längenpräfigiert wie [`Problem::notification_key`] und aus demselben Grund
/// (D12): `("host", "a|b")` und `("host|a", "b")` dürfen nicht denselben
/// Gegenstand ergeben.
pub fn subject_of(problem: &Problem) -> String {
    let (kind, service) = match problem.service.as_deref() {
        Some(name) => ('S', name),
        None => ('H', ""),
    };
    format!(
        "{kind}|{}:{}|{}:{}",
        problem.host.len(),
        problem.host,
        service.len(),
        service
    )
}

/// Das Gedächtnis: je Gegenstand der Zustand, in dem gemeldet wurde.
///
/// `BTreeMap` und nicht `HashMap`: die Reihenfolge der Ereignisse soll
/// vorhersagbar sein, sonst sind Tests von der Hash-Streuung abhängig.
pub type Notified = BTreeMap<String, ProblemState>;

/// Art eines Ereignisses.
///
/// Die Stufe steckt mit drin, weil sie den Klang bestimmt — ohne sie müsste
/// `notify::loudest` den Zustand aus dem Text zurücklesen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    /// Neues oder verschlimmertes CRIT, DOWN oder UNREACHABLE.
    Critical,
    /// Neues oder geändertes WARN oder UNKNOWN.
    Warning,
    /// War gemeldet, ist es nicht mehr.
    Recovery,
}

/// Ein fertig formuliertes Ereignis.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NotifyEvent {
    pub kind: EventKind,
    /// Der gemeldete Zustand, `None` bei einer Entwarnung.
    ///
    /// [`EventKind`] fasst CRIT, DOWN und UNREACHABLE zu einer Stufe zusammen,
    /// weil sie denselben Klang bekommen. Das Logo des Toasts unterscheidet
    /// dagegen fünf Farben — CRIT und DOWN sind zwei getrennte Farbtöne (D23).
    /// Ohne dieses Feld müsste der Zustand aus dem Titel zurückgelesen werden,
    /// und der Titel ist Text für Menschen, keine Schnittstelle.
    pub state: Option<ProblemState>,
    /// Kopfzeile des Toasts.
    pub title: String,
    /// Rumpf. Eine bis zwei Zeilen; länger schneidet Windows ab.
    pub body: String,
}

/// Welche Stufe ein Zustand auslöst.
///
/// Die Zuordnung stand bis dahin nur im Doc-Kommentar von [`EventKind`] und
/// nicht im Code: `problem_event` setzte ausnahmslos [`EventKind::Critical`].
/// Eine neue WARN bekam damit den Klang für Kritisches, und die Auswahl
/// „Warnung" in den Einstellungen war wirkungslos. Aufgefallen ist es nicht,
/// weil die Klangtests ihre Ereignisse von Hand bauen — die Naht zwischen
/// `decide` und `loudest` war nie durchlaufen.
pub fn kind_of(state: ProblemState) -> EventKind {
    match state {
        ProblemState::Warn | ProblemState::Unknown => EventKind::Warning,
        _ => EventKind::Critical,
    }
}

/// Ergebnis der Entscheidung.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// In der Reihenfolge, in der gemeldet werden soll.
    pub events: Vec<NotifyEvent>,
    /// Ersetzt das bisherige Gedächtnis vollständig.
    pub notified: Notified,
}

impl Decision {
    /// Nichts melden, Gedächtnis übernehmen.
    fn silent(notified: Notified) -> Self {
        Self {
            events: Vec::new(),
            notified,
        }
    }
}

/// Ob ein Zustand als „kritisch" im Sinne der Einstellung gilt.
///
/// CRIT bei Services, DOWN und UNREACHABLE bei Hosts. WARN und UNKNOWN nicht:
/// ein unbekannter Zustand heisst „der Check funktioniert nicht", und dafür
/// nachts geweckt zu werden wäre die falsche Reihenfolge.
fn is_critical(state: ProblemState) -> bool {
    matches!(
        state,
        ProblemState::Crit | ProblemState::Down | ProblemState::Unreachable
    )
}

/// Ob über dieses Problem überhaupt gemeldet wird.
///
/// Bearbeitete Probleme fallen **immer** heraus, unabhängig von der Stufe:
/// quittiert heisst „jemand weiss davon", Wartung heisst „ist geplant". Beides
/// zu melden würde genau das untergraben, wofür die Kennzeichen da sind.
fn is_candidate(problem: &Problem, level: NotificationLevel) -> bool {
    if problem.is_handled() {
        return false;
    }
    match level {
        NotificationLevel::Off => false,
        NotificationLevel::CriticalOnly => is_critical(problem.state),
        NotificationLevel::AllChanges => true,
    }
}

/// Entscheidet, was gemeldet wird.
///
/// `previous` ist das Gedächtnis des letzten Abzugs. `first_run` unterdrückt
/// jede Meldung und füllt nur das Gedächtnis — siehe Modulkommentar.
pub fn decide(
    snapshot: &Snapshot,
    previous: &Notified,
    level: NotificationLevel,
    first_run: bool,
) -> Decision {
    // Bei „aus" wird auch das Gedächtnis geleert. Sonst käme beim
    // Wiedereinschalten eine Nachmeldung für alles, was in der Zwischenzeit
    // passiert ist — und die will niemand.
    if level == NotificationLevel::Off {
        return Decision::silent(Notified::new());
    }

    // Alle Probleme des Abzugs nach Gegenstand, auch die, über die nicht
    // gemeldet wird: für die Begründung einer Entwarnung braucht es sie.
    let mut im_abzug: BTreeMap<String, &Problem> = BTreeMap::new();
    for problem in &snapshot.problems {
        im_abzug.insert(subject_of(problem), problem);
    }

    let mut notified = Notified::new();
    let mut events = Vec::new();

    // Neu und geändert.
    for problem in &snapshot.problems {
        if !is_candidate(problem, level) {
            continue;
        }
        let subject = subject_of(problem);
        let bekannt = previous.get(&subject);
        let geaendert = bekannt != Some(&problem.state);
        notified.insert(subject, problem.state);

        if geaendert && !first_run {
            events.push(problem_event(problem, bekannt.copied()));
        }
    }

    // Entwarnung: war gemeldet, ist jetzt kein Kandidat mehr.
    if !first_run {
        for (subject, alt) in previous {
            if notified.contains_key(subject) {
                continue;
            }
            events.push(recovery_event(
                subject,
                *alt,
                im_abzug.get(subject).copied(),
            ));
        }
    }

    Decision { events, notified }
}

/// Kopfzeile: Zustand, dann Host und Service.
fn headline(problem: &Problem) -> String {
    let service = problem
        .service
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or(text::HOST_PROBLEM);
    format!(
        "{} {}{}{}",
        short_state(problem.state),
        problem.host,
        text::SEPARATOR,
        service
    )
}

/// Kürzel des Zustands, wie es auch das Tray benutzt.
fn short_state(state: ProblemState) -> &'static str {
    match state {
        ProblemState::Ok => "OK",
        ProblemState::Warn => "WARN",
        ProblemState::Crit => "CRIT",
        ProblemState::Unknown => "UNKNOWN",
        ProblemState::Down => "DOWN",
        ProblemState::Unreachable => "UNREACHABLE",
    }
}

fn problem_event(problem: &Problem, vorher: Option<ProblemState>) -> NotifyEvent {
    let mut zeilen = Vec::new();

    // Erste Zeile der Ausgabe. Mehrzeilige plugin_output-Texte kommen vor und
    // würden den Toast sprengen; das Fenster zeigt den vollen Text.
    let ausgabe = problem.output.lines().next().unwrap_or("").trim();
    if !ausgabe.is_empty() {
        zeilen.push(ausgabe.to_owned());
    }

    let mut zusatz = Vec::new();
    if let Some(alt) = vorher {
        zusatz.push(format!("{} {}", text::WAS, short_state(alt)));
    }
    if problem.flapping {
        zusatz.push(text::FLAPPING.to_owned());
    }
    if !zusatz.is_empty() {
        zeilen.push(format!("({})", zusatz.join(", ")));
    }

    NotifyEvent {
        kind: kind_of(problem.state),
        state: Some(problem.state),
        title: headline(problem),
        body: zeilen.join("\n"),
    }
}

/// Entwarnung. Sagt, **warum** nicht mehr gemeldet wird.
///
/// Drei Fälle, und sie bedeuten Verschiedenes: das Problem ist weg, es ist
/// quittiert worden, oder es ist nur noch weniger schlimm. „Wieder in Ordnung"
/// für alle drei zu schreiben wäre falsch — quittiert ist nicht behoben.
fn recovery_event(subject: &str, alt: ProblemState, jetzt: Option<&Problem>) -> NotifyEvent {
    let (host, service) = split_subject(subject);
    let titel = format!("OK {host}{}{service}", text::SEPARATOR);

    let grund = match jetzt {
        None => text::RECOVERED.to_owned(),
        Some(p) if p.acknowledged => text::NOW_ACKNOWLEDGED.to_owned(),
        Some(p) if p.downtime_depth > 0 => text::NOW_DOWNTIME.to_owned(),
        Some(p) => short_state(p.state).to_owned(),
    };

    NotifyEvent {
        kind: EventKind::Recovery,
        // Eine Entwarnung hat keinen gemeldeten Zustand mehr — sie sagt
        // gerade, dass keiner mehr vorliegt. Der alte steht im Rumpf.
        state: None,
        title: titel,
        body: format!("{} {} → {}", text::WAS, short_state(alt), grund),
    }
}

/// Zerlegt einen Gegenstand zurück in Host und Service.
///
/// Möglich, weil [`subject_of`] längenpräfigiert schreibt: die Länge sagt, wie
/// weit zu lesen ist, unabhängig davon, welche Zeichen im Namen stehen. Bei
/// einem unerwarteten Aufbau kommt der Rohtext zurück statt einer Panik — eine
/// verstümmelte Entwarnung ist besser als ein Absturz der Abrufschleife.
fn split_subject(subject: &str) -> (String, String) {
    fn nimm(rest: &str) -> Option<(String, &str)> {
        let (len, rest) = rest.split_once(':')?;
        let len: usize = len.parse().ok()?;
        if rest.len() < len {
            return None;
        }
        // Byteweise schneiden ist hier richtig: `len` ist die Byte-Länge aus
        // `str::len()`, nicht eine Zeichenzahl.
        let (wert, rest) = rest.split_at(len);
        Some((wert.to_owned(), rest.strip_prefix('|').unwrap_or(rest)))
    }

    let Some(rest) = subject
        .strip_prefix("S|")
        .or_else(|| subject.strip_prefix("H|"))
    else {
        return (subject.to_owned(), String::new());
    };
    let Some((host, rest)) = nimm(rest) else {
        return (subject.to_owned(), String::new());
    };
    let service = nimm(rest).map(|(s, _)| s).unwrap_or_default();
    if service.is_empty() {
        (host, text::HOST_PROBLEM.to_owned())
    } else {
        (host, service)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    fn snapshot(problems: Vec<Problem>) -> Snapshot {
        Snapshot {
            problems,
            fetched_at: Utc.with_ymd_and_hms(2026, 8, 24, 12, 0, 0).unwrap(),
        }
    }

    fn problem(host: &str, service: Option<&str>, state: ProblemState) -> Problem {
        Problem {
            host: host.to_owned(),
            service: service.map(str::to_owned),
            state,
            output: "CRIT - Used: 96.41%".to_owned(),
            last_state_change: Some(Utc.with_ymd_and_hms(2026, 8, 24, 11, 0, 0).unwrap()),
            acknowledged: false,
            downtime_depth: 0,
            flapping: false,
        }
    }

    /* ------------------------------------------------------- Gegenstand -- */

    /// Dieselbe Falle wie D12, eine Ebene tiefer.
    #[test]
    fn gegenstand_ist_eindeutig_auch_mit_trennzeichen_im_namen() {
        let a = subject_of(&problem("host", Some("a|b"), ProblemState::Crit));
        let b = subject_of(&problem("host|a", Some("b"), ProblemState::Crit));
        assert_ne!(a, b);
    }

    #[test]
    fn gegenstand_ignoriert_den_zustand() {
        let a = subject_of(&problem("h", Some("s"), ProblemState::Crit));
        let b = subject_of(&problem("h", Some("s"), ProblemState::Warn));
        assert_eq!(a, b, "der Zustand darf nicht im Gegenstand stecken");
    }

    /* -------------------------------------------------------- Stufe ----- */

    #[test]
    fn warn_und_unknown_sind_warnungen_alles_andere_ist_kritisch() {
        assert_eq!(kind_of(ProblemState::Warn), EventKind::Warning);
        assert_eq!(kind_of(ProblemState::Unknown), EventKind::Warning);
        assert_eq!(kind_of(ProblemState::Crit), EventKind::Critical);
        assert_eq!(kind_of(ProblemState::Down), EventKind::Critical);
        assert_eq!(kind_of(ProblemState::Unreachable), EventKind::Critical);
    }

    /// Der Fehler, den dieser Test festhält: `problem_event` setzte
    /// ausnahmslos `Critical`. Eine neue WARN bekam damit den Klang für
    /// Kritisches, und die Auswahl „Warnung" in den Einstellungen war ohne
    /// Wirkung. Geprüft wird hier durch `decide` hindurch — genau die Naht,
    /// die vorher nicht durchlaufen wurde.
    #[test]
    fn eine_neue_warnung_wird_nicht_als_kritisch_gemeldet() {
        let leer = Notified::new();
        let erst = decide(
            &snapshot(vec![]),
            &leer,
            NotificationLevel::AllChanges,
            true,
        );
        let dann = decide(
            &snapshot(vec![problem("h", Some("s"), ProblemState::Warn)]),
            &erst.notified,
            NotificationLevel::AllChanges,
            false,
        );
        assert_eq!(dann.events.len(), 1);
        assert_eq!(dann.events[0].kind, EventKind::Warning);
    }

    /* -------------------------------------------------- Zustand am Ereignis */

    #[test]
    fn ein_problemereignis_fuehrt_seinen_zustand_mit() {
        let leer = Notified::new();
        let erst = decide(
            &snapshot(vec![]),
            &leer,
            NotificationLevel::CriticalOnly,
            true,
        );
        let dann = decide(
            &snapshot(vec![problem("h", Some("s"), ProblemState::Down)]),
            &erst.notified,
            NotificationLevel::CriticalOnly,
            false,
        );
        assert_eq!(dann.events[0].state, Some(ProblemState::Down));
    }

    #[test]
    fn eine_entwarnung_fuehrt_keinen_zustand_mit() {
        let leer = Notified::new();
        let erst = decide(
            &snapshot(vec![problem("h", Some("s"), ProblemState::Crit)]),
            &leer,
            NotificationLevel::CriticalOnly,
            true,
        );
        let dann = decide(
            &snapshot(vec![]),
            &erst.notified,
            NotificationLevel::CriticalOnly,
            false,
        );
        assert_eq!(dann.events[0].kind, EventKind::Recovery);
        assert_eq!(dann.events[0].state, None);
    }

    #[test]
    fn hostproblem_und_service_mit_leerem_namen_sind_verschieden() {
        let host = subject_of(&problem("h", None, ProblemState::Down));
        let service = subject_of(&problem("h", Some(""), ProblemState::Crit));
        assert_ne!(host, service);
    }

    #[test]
    fn gegenstand_laesst_sich_zurueckzerlegen() {
        for (host, service) in [
            ("leosys-sql-01", Some("Filesystem /var")),
            ("host", Some("a|b")),
            ("host|a", Some("b")),
            ("umlaut-hößt", Some("Prüfung ä")),
        ] {
            let s = subject_of(&problem(host, service, ProblemState::Crit));
            let (h, sv) = split_subject(&s);
            assert_eq!(h, host, "Host falsch aus {s}");
            assert_eq!(sv, service.unwrap(), "Service falsch aus {s}");
        }
    }

    #[test]
    fn zerlegung_eines_hostproblems_nennt_es_als_solches() {
        let s = subject_of(&problem("srv", None, ProblemState::Down));
        assert_eq!(split_subject(&s), ("srv".into(), text::HOST_PROBLEM.into()));
    }

    /// Kein Absturz bei Unsinn — die Abrufschleife darf nicht daran hängen.
    #[test]
    fn zerlegung_von_unsinn_panikt_nicht() {
        for müll in ["", "X|", "S|abc", "S|999:kurz", "S|3:abc"] {
            let _ = split_subject(müll);
        }
    }

    /* ------------------------------------------------------- Erster Lauf -- */

    #[test]
    fn erster_lauf_meldet_nichts_und_merkt_alles() {
        let snap = snapshot(vec![
            problem("a", Some("s1"), ProblemState::Crit),
            problem("b", None, ProblemState::Down),
        ]);
        let d = decide(&snap, &Notified::new(), NotificationLevel::AllChanges, true);
        assert!(d.events.is_empty(), "beim Start darf nichts aufpoppen");
        assert_eq!(d.notified.len(), 2, "gemerkt werden muss trotzdem alles");
    }

    /// Der Fall, der das Ganze rechtfertigt: 40 Probleme beim Autostart.
    #[test]
    fn erster_lauf_mit_vielen_problemen_bleibt_still() {
        let problems: Vec<Problem> = (0..40)
            .map(|i| problem(&format!("h{i}"), Some("s"), ProblemState::Crit))
            .collect();
        let d = decide(
            &snapshot(problems),
            &Notified::new(),
            NotificationLevel::CriticalOnly,
            true,
        );
        assert!(d.events.is_empty());
        assert_eq!(d.notified.len(), 40);
    }

    /* ------------------------------------------------------------- Stufen -- */

    #[test]
    fn aus_meldet_nichts_und_leert_das_gedaechtnis() {
        let mut vorher = Notified::new();
        vorher.insert("irgendwas".into(), ProblemState::Crit);
        let snap = snapshot(vec![problem("a", Some("s"), ProblemState::Crit)]);
        let d = decide(&snap, &vorher, NotificationLevel::Off, false);
        assert!(d.events.is_empty());
        assert!(
            d.notified.is_empty(),
            "sonst kommt beim Wiedereinschalten eine Nachmeldung"
        );
    }

    #[test]
    fn nur_kritisch_laesst_warn_und_unknown_liegen() {
        let snap = snapshot(vec![
            problem("a", Some("s"), ProblemState::Warn),
            problem("b", Some("s"), ProblemState::Unknown),
            problem("c", Some("s"), ProblemState::Crit),
            problem("d", None, ProblemState::Down),
            problem("e", None, ProblemState::Unreachable),
        ]);
        let d = decide(
            &snap,
            &Notified::new(),
            NotificationLevel::CriticalOnly,
            false,
        );
        assert_eq!(d.events.len(), 3, "CRIT, DOWN und UNREACHABLE");
        assert_eq!(d.notified.len(), 3);
    }

    #[test]
    fn alle_aenderungen_meldet_auch_warn() {
        let snap = snapshot(vec![problem("a", Some("s"), ProblemState::Warn)]);
        let d = decide(
            &snap,
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        assert_eq!(d.events.len(), 1);
    }

    /* -------------------------------------------------------- Bearbeitete -- */

    #[test]
    fn quittierte_und_wartung_werden_nie_gemeldet() {
        let mut quittiert = problem("a", Some("s"), ProblemState::Crit);
        quittiert.acknowledged = true;
        let mut wartung = problem("b", Some("s"), ProblemState::Crit);
        wartung.downtime_depth = 1;

        let d = decide(
            &snapshot(vec![quittiert, wartung]),
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        assert!(d.events.is_empty());
        assert!(d.notified.is_empty());
    }

    /* ----------------------------------------------------- Wiederholungen -- */

    #[test]
    fn unveraenderter_zustand_meldet_nicht_erneut() {
        let p = problem("a", Some("s"), ProblemState::Crit);
        let snap = snapshot(vec![p.clone()]);
        let erst = decide(
            &snap,
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        assert_eq!(erst.events.len(), 1);

        let zweit = decide(&snap, &erst.notified, NotificationLevel::AllChanges, false);
        assert!(
            zweit.events.is_empty(),
            "zweimal dasselbe zu melden ist Lärm"
        );
        assert_eq!(zweit.notified, erst.notified);
    }

    /// Der stille Fehler, den eine reine Menge hätte.
    #[test]
    fn nach_entwarnung_wird_ein_rueckfall_wieder_gemeldet() {
        let p = problem("a", Some("s"), ProblemState::Crit);
        let mit = snapshot(vec![p.clone()]);
        let ohne = snapshot(vec![]);

        let a = decide(&mit, &Notified::new(), NotificationLevel::AllChanges, false);
        let b = decide(&ohne, &a.notified, NotificationLevel::AllChanges, false);
        let c = decide(&mit, &b.notified, NotificationLevel::AllChanges, false);

        assert_eq!(a.events.len(), 1, "erstes Auftreten");
        assert_eq!(b.events.len(), 1, "Entwarnung");
        assert_eq!(b.events[0].kind, EventKind::Recovery);
        assert_eq!(c.events.len(), 1, "Rückfall muss wieder ankommen");
        assert_eq!(c.events[0].kind, EventKind::Critical);
    }

    #[test]
    fn zustandswechsel_wird_gemeldet_und_nennt_den_alten() {
        let warn = problem("a", Some("s"), ProblemState::Warn);
        let mut crit = warn.clone();
        crit.state = ProblemState::Crit;

        let a = decide(
            &snapshot(vec![warn]),
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        let b = decide(
            &snapshot(vec![crit]),
            &a.notified,
            NotificationLevel::AllChanges,
            false,
        );

        assert_eq!(b.events.len(), 1, "eine Meldung, keine Entwarnung dazu");
        assert_eq!(b.events[0].kind, EventKind::Critical);
        assert!(b.events[0].title.starts_with("CRIT "), "{:?}", b.events[0]);
        assert!(b.events[0].body.contains("WARN"), "{:?}", b.events[0]);
    }

    /// WARN → CRIT darf **keine** Entwarnung für WARN erzeugen. Genau das
    /// passiert, wenn das Gedächtnis den Zustand im Schlüssel trägt.
    #[test]
    fn verschlimmerung_erzeugt_keine_entwarnung() {
        let warn = problem("a", Some("s"), ProblemState::Warn);
        let mut crit = warn.clone();
        crit.state = ProblemState::Crit;

        let a = decide(
            &snapshot(vec![warn]),
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        let b = decide(
            &snapshot(vec![crit]),
            &a.notified,
            NotificationLevel::AllChanges,
            false,
        );
        assert!(
            !b.events.iter().any(|e| e.kind == EventKind::Recovery),
            "{:?}",
            b.events
        );
    }

    /* -------------------------------------------------------- Entwarnungen -- */

    #[test]
    fn verschwundenes_problem_gilt_als_behoben() {
        let a = decide(
            &snapshot(vec![problem("a", Some("s"), ProblemState::Crit)]),
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        let b = decide(
            &snapshot(vec![]),
            &a.notified,
            NotificationLevel::AllChanges,
            false,
        );
        assert_eq!(b.events.len(), 1);
        assert!(
            b.events[0].body.contains(text::RECOVERED),
            "{:?}",
            b.events[0]
        );
        assert!(b.notified.is_empty());
    }

    /// Quittiert ist **nicht** behoben. Die Entwarnung muss das sagen.
    #[test]
    fn quittieren_erzeugt_eine_entwarnung_mit_der_richtigen_begruendung() {
        let p = problem("a", Some("s"), ProblemState::Crit);
        let a = decide(
            &snapshot(vec![p.clone()]),
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );

        let mut quittiert = p;
        quittiert.acknowledged = true;
        let b = decide(
            &snapshot(vec![quittiert]),
            &a.notified,
            NotificationLevel::AllChanges,
            false,
        );

        assert_eq!(b.events.len(), 1);
        assert_eq!(b.events[0].kind, EventKind::Recovery);
        assert!(
            b.events[0].body.contains(text::NOW_ACKNOWLEDGED),
            "„behoben\" wäre falsch: {:?}",
            b.events[0]
        );
    }

    #[test]
    fn wartungszeit_erzeugt_die_entsprechende_begruendung() {
        let p = problem("a", Some("s"), ProblemState::Crit);
        let a = decide(
            &snapshot(vec![p.clone()]),
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        let mut wartung = p;
        wartung.downtime_depth = 2;
        let b = decide(
            &snapshot(vec![wartung]),
            &a.notified,
            NotificationLevel::AllChanges,
            false,
        );
        assert!(
            b.events[0].body.contains(text::NOW_DOWNTIME),
            "{:?}",
            b.events[0]
        );
    }

    /// Bei „nur kritisch": CRIT → WARN ist keine Behebung, sondern eine
    /// Besserung. Die Entwarnung muss den neuen Zustand nennen.
    #[test]
    fn abstieg_unter_die_schwelle_nennt_den_neuen_zustand() {
        let crit = problem("a", Some("s"), ProblemState::Crit);
        let a = decide(
            &snapshot(vec![crit.clone()]),
            &Notified::new(),
            NotificationLevel::CriticalOnly,
            false,
        );
        let mut warn = crit;
        warn.state = ProblemState::Warn;
        let b = decide(
            &snapshot(vec![warn]),
            &a.notified,
            NotificationLevel::CriticalOnly,
            false,
        );

        assert_eq!(b.events.len(), 1);
        assert_eq!(b.events[0].kind, EventKind::Recovery);
        assert!(b.events[0].body.contains("WARN"), "{:?}", b.events[0]);
        assert!(
            !b.events[0].body.contains(text::RECOVERED),
            "nicht behoben, nur besser: {:?}",
            b.events[0]
        );
    }

    /* ------------------------------------------------------------ Texte -- */

    #[test]
    fn der_rumpf_nimmt_nur_die_erste_ausgabezeile() {
        let mut p = problem("a", Some("s"), ProblemState::Crit);
        p.output = "erste Zeile\nzweite Zeile\ndritte".into();
        let d = decide(
            &snapshot(vec![p]),
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        assert!(d.events[0].body.starts_with("erste Zeile"));
        assert!(!d.events[0].body.contains("zweite Zeile"));
    }

    #[test]
    fn flattern_steht_im_rumpf() {
        let mut p = problem("a", Some("s"), ProblemState::Crit);
        p.flapping = true;
        let d = decide(
            &snapshot(vec![p]),
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        assert!(
            d.events[0].body.contains(text::FLAPPING),
            "{:?}",
            d.events[0]
        );
    }

    #[test]
    fn ein_hostproblem_wird_als_solches_benannt() {
        let d = decide(
            &snapshot(vec![problem("srv", None, ProblemState::Down)]),
            &Notified::new(),
            NotificationLevel::CriticalOnly,
            false,
        );
        assert!(
            d.events[0].title.contains(text::HOST_PROBLEM),
            "{:?}",
            d.events[0]
        );
        assert!(
            d.events[0].title.starts_with("DOWN srv"),
            "{:?}",
            d.events[0]
        );
    }

    #[test]
    fn kein_ereignis_hat_leere_texte() {
        let mut leer = problem("a", Some("s"), ProblemState::Crit);
        leer.output = String::new();
        let d = decide(
            &snapshot(vec![leer]),
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        // Der Rumpf darf leer sein, die Kopfzeile nicht — ein Toast ohne
        // Kopfzeile zeigt Windows als leeren Kasten.
        assert!(!d.events[0].title.trim().is_empty());
    }

    /* ---------------------------------------------------- Reihenfolge -- */

    /// Die Reihenfolge muss reproduzierbar sein, sonst sind Tests von der
    /// Hash-Streuung abhängig und Toasts erscheinen in wechselnder Folge.
    #[test]
    fn die_reihenfolge_ist_stabil() {
        let problems: Vec<Problem> = ["c", "a", "b"]
            .iter()
            .map(|h| problem(h, Some("s"), ProblemState::Crit))
            .collect();
        let snap = snapshot(problems);
        let erst = decide(
            &snap,
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        let nochmal = decide(
            &snap,
            &Notified::new(),
            NotificationLevel::AllChanges,
            false,
        );
        assert_eq!(erst.events, nochmal.events);
        // Neue Probleme folgen der Abzugsreihenfolge, die schon nach Schwere
        // sortiert ist — nicht alphabetisch.
        assert_eq!(erst.events[0].title, "CRIT c · s");
    }
}
