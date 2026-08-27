//! Windows-Benachrichtigungen zu neuen und behobenen Problemen.
//!
//! Die Entscheidung, *was* gemeldet wird, steckt vollständig in
//! [`decide`] — rein und ohne Windows, deshalb prüfbar. Hier bleibt nur das
//! Versenden, der Signalton und die Deckelung.
//!
//! # Die Deckelung
//!
//! Kommt ein Server nach einem Ausfall zurück, enthält der erste Abzug
//! womöglich dreissig neue Probleme. Dreissig Toasts hintereinander sind keine
//! Information, sondern eine Sperre für den Bildschirm — Windows stapelt sie,
//! und der Benutzer wischt sie ungelesen weg. Ab [`MAX_TOASTS`] kommt deshalb
//! **eine** Sammelmeldung statt der restlichen.

pub mod decide;
pub mod identity;
pub mod sound;
pub mod toast;

use std::path::PathBuf;

use tauri::{AppHandle, Runtime};

use crate::checkmk::Snapshot;
use crate::commands::AppState;
use crate::config::{NotificationLevel, SoundChoice, SoundSettings};
use crate::i18n::notify as text;
use tauri::Manager;

pub use decide::{decide, Decision, EventKind, Notified, NotifyEvent};

/// Wie viele einzelne Toasts eine Runde höchstens schickt.
///
/// Fünf, weil Windows im Infobereich ohnehin nur wenige gleichzeitig zeigt und
/// der Rest im Info-Center landet. Was dort ungelesen liegt, ist so gut wie
/// nicht gemeldet — die Sammelmeldung sagt zumindest, dass mehr da ist.
pub const MAX_TOASTS: usize = 5;

/// Verteilt die Ereignisse auf einzelne Toasts und den Rest.
///
/// Rückgabe ist `(einzeln, restlich)`. Eigene Funktion, damit die Grenzfälle
/// geprüft werden können, ohne einen Toast zu schicken — genau bei
/// [`MAX_TOASTS`] darf noch **keine** Sammelmeldung kommen, sonst ersetzt sie
/// eine einzelne Meldung durch die Auskunft, dass es eine gäbe.
pub fn split_for_toasts(total: usize) -> (usize, usize) {
    let einzeln = total.min(MAX_TOASTS);
    (einzeln, total - einzeln)
}

/// Formuliert die Sammelmeldung für die abgeschnittenen Ereignisse.
pub fn overflow_body(rest: usize) -> String {
    format!("und {rest} weitere — Fenster öffnen für die vollständige Liste")
}

/// Baut die Sammelmeldung als vollwertiges Ereignis.
///
/// Stufe und Zustand kommen vom **dringlichsten** der übrigen Ereignisse,
/// damit das Logo die Farbe des Schlimmsten trägt. Eine grüne Sammelmeldung,
/// hinter der fünfundzwanzig kritische Probleme stehen, wäre die
/// Fehlinformation aus D26 in anderer Gestalt: sie sähe aus wie eine
/// Entwarnung.
pub fn overflow_event(rest: &[NotifyEvent], count: usize) -> NotifyEvent {
    let schlimmstes = rest
        .iter()
        .find(|e| e.kind == EventKind::Critical)
        .or_else(|| rest.iter().find(|e| e.kind == EventKind::Warning));

    NotifyEvent {
        kind: schlimmstes.map_or(EventKind::Recovery, |e| e.kind),
        state: schlimmstes.and_then(|e| e.state),
        title: text::APP.to_owned(),
        body: overflow_body(count),
    }
}

/// Wo die Toast-Logos liegen.
///
/// `%LOCALAPPDATA%\de.leosysr.luchsr\assets`, also neben dem Protokoll — und
/// bewusst **nicht** neben der ausführbaren Datei: dort wäre es im
/// Entwicklungsbau `target\debug` und nach der Installation ein Ordner unter
/// `%ProgramFiles%`, in den ein gewöhnlicher Benutzer nicht schreiben darf.
pub fn asset_dir<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path()
        .app_local_data_dir()
        .ok()
        .map(|dir| dir.join("assets"))
}

/// Wählt **einen** Klang für eine ganze Runde.
///
/// Bei fünf Meldungen fünfmal zu klingeln wäre Lärm, und der Reihe nach zu
/// spielen ginge nicht — `PlaySoundW` bricht den laufenden Klang ab, man hörte
/// nur den letzten. Also einer, und zwar der **dringlichste**: was in dieser
/// Runde die höchste Stufe erreicht hat, gibt den Ton an. Eine Entwarnung
/// zwischen zwei kritischen Meldungen darf den Alarm nicht ersetzen.
///
/// Ist für die dringlichste Stufe „kein Ton" gewählt, bleibt es still — es wird
/// **nicht** auf eine niedrigere Stufe ausgewichen. Sonst hätte „für Kritisches
/// keinen Ton" die Wirkung, bei Kritischem den Warnton zu spielen.
pub fn loudest<'a>(events: &[NotifyEvent], sounds: &'a SoundSettings) -> &'a SoundChoice {
    if events.iter().any(|e| e.kind == EventKind::Critical) {
        &sounds.critical
    } else if events.iter().any(|e| e.kind == EventKind::Warning) {
        &sounds.warning
    } else {
        &sounds.recovery
    }
}

/// Kurzform für das Protokoll: wie viele je Art.
fn zusammenfassung(events: &[NotifyEvent]) -> String {
    let zaehle = |k: EventKind| events.iter().filter(|e| e.kind == k).count();
    format!(
        "{} kritisch, {} Warnung, {} Entwarnung",
        zaehle(EventKind::Critical),
        zaehle(EventKind::Warning),
        zaehle(EventKind::Recovery)
    )
}

/// Meldet, was sich seit dem letzten Abzug geändert hat.
///
/// Wird nach jedem **erfolgreichen** Abruf gerufen. Nach einem Fehlversuch
/// bewusst nicht: der Abzug ist dann der alte, und daraus lässt sich keine
/// Änderung ableiten. Ein Verbindungsfehler ist am Tray-Icon zu sehen (D26),
/// ein Toast dafür wäre bei einem längeren Ausfall eine Meldung pro Minute.
pub fn announce<R: Runtime>(app: &AppHandle<R>, snapshot: &Snapshot) {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let level = settings.notifications.level;

    if level == NotificationLevel::Off {
        // Auch dann durchlaufen: `decide` leert das Gedächtnis, damit beim
        // Wiedereinschalten keine Nachmeldung kommt.
        state.replace_notified(Notified::new());
        return;
    }

    let (previous, first_run) = state.notified_snapshot();
    let outcome = decide(snapshot, &previous, level, first_run);
    state.replace_notified(outcome.notified);

    if outcome.events.is_empty() {
        return;
    }

    let (einzeln, restlich) = split_for_toasts(outcome.events.len());
    for event in outcome.events.iter().take(einzeln) {
        send(app, event);
    }
    if restlich > 0 {
        send(app, &overflow_event(&outcome.events[einzeln..], restlich));
    }

    // Der Ton kommt **einmal** je Runde, nicht je Meldung — fünf Toasts sollen
    // nicht fünfmal klingeln. Welcher, entscheidet `loudest`.
    let klang = loudest(&outcome.events, &settings.notifications.sounds);
    sound::play(klang);

    log::info!(
        "{} Benachrichtigung(en) gemeldet: {}",
        outcome.events.len(),
        zusammenfassung(&outcome.events)
    );
}

/// Schickt einen Toast. Fehlschläge werden protokolliert, nicht behandelt.
///
/// Ein nicht angekommener Toast ist ärgerlich, aber kein Grund, die
/// Abrufschleife zu stören — die Liste im Fenster zeigt den Zustand ohnehin.
///
/// Die AppUserModelID ist der Bundle-Identifier aus `tauri.conf.json`. Sie
/// hier abzulesen statt sie zu wiederholen hält beide Stellen zusammen: unter
/// derselben Kennung trägt [`identity`] Name und Symbol in die Registry ein,
/// und laufen die zwei auseinander, bleibt die Kopfzeile des Toasts leer.
fn send<R: Runtime>(app: &AppHandle<R>, event: &NotifyEvent) {
    let Some(dir) = asset_dir(app) else {
        log::warn!("Kein lokales Datenverzeichnis — Toast ohne Logo unterbleibt");
        return;
    };
    if let Err(error) = toast::send(&app.config().identifier, &dir, event) {
        log::warn!(
            "Benachrichtigung „{}“ liess sich nicht zeigen: {error:?}",
            event.title
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn die_sammelmeldung_nennt_die_anzahl() {
        assert!(overflow_body(25).contains("25"));
    }

    #[test]
    fn ohne_ereignisse_wird_nichts_verteilt() {
        assert_eq!(split_for_toasts(0), (0, 0));
    }

    #[test]
    fn wenige_ereignisse_kommen_alle_einzeln() {
        assert_eq!(split_for_toasts(1), (1, 0));
        assert_eq!(split_for_toasts(MAX_TOASTS - 1), (MAX_TOASTS - 1, 0));
    }

    /// Der Grenzfall: genau die Höchstzahl darf **keine** Sammelmeldung
    /// auslösen. Sonst ersetzt „und 0 weitere" eine echte Meldung.
    #[test]
    fn genau_die_hoechstzahl_erzeugt_keine_sammelmeldung() {
        assert_eq!(split_for_toasts(MAX_TOASTS), (MAX_TOASTS, 0));
    }

    #[test]
    fn darueber_wird_gedeckelt_und_der_rest_gezaehlt() {
        assert_eq!(split_for_toasts(MAX_TOASTS + 1), (MAX_TOASTS, 1));
        let (einzeln, rest) = split_for_toasts(30);
        assert_eq!(einzeln, MAX_TOASTS);
        assert_eq!(einzeln + rest, 30, "es darf nichts verloren gehen");
    }

    /* -------------------------------------------------- Sammelmeldung --- */

    #[test]
    fn die_sammelmeldung_traegt_die_farbe_des_schlimmsten() {
        let rest = [
            ereignis(EventKind::Recovery),
            ereignis(EventKind::Critical),
            ereignis(EventKind::Warning),
        ];
        let sammel = overflow_event(&rest, 3);
        assert_eq!(sammel.kind, EventKind::Critical);
        assert_eq!(sammel.state, Some(crate::checkmk::ProblemState::Crit));
    }

    #[test]
    fn ohne_kritisches_reicht_die_warnung() {
        let rest = [ereignis(EventKind::Recovery), ereignis(EventKind::Warning)];
        assert_eq!(overflow_event(&rest, 2).kind, EventKind::Warning);
    }

    #[test]
    fn nur_entwarnungen_ergeben_eine_entwarnung() {
        let rest = [ereignis(EventKind::Recovery)];
        let sammel = overflow_event(&rest, 1);
        assert_eq!(sammel.kind, EventKind::Recovery);
        assert_eq!(sammel.state, None);
    }

    #[test]
    fn die_sammelmeldung_nennt_luchsr_als_titel() {
        assert_eq!(overflow_event(&[], 7).title, text::APP);
        assert!(overflow_event(&[], 7).body.contains('7'));
    }

    /* ----------------------------------------------------- Klangauswahl -- */

    fn ereignis(kind: EventKind) -> NotifyEvent {
        NotifyEvent {
            kind,
            state: match kind {
                EventKind::Critical => Some(crate::checkmk::ProblemState::Crit),
                EventKind::Warning => Some(crate::checkmk::ProblemState::Warn),
                EventKind::Recovery => None,
            },
            title: "t".into(),
            body: "b".into(),
        }
    }

    /// Alle fünf verschieden belegt, damit jede Verwechslung auffällt.
    fn klaenge() -> SoundSettings {
        SoundSettings {
            critical: SoundChoice::Builtin {
                id: "kritisch".into(),
            },
            warning: SoundChoice::Builtin {
                id: "warnung".into(),
            },
            recovery: SoundChoice::Builtin {
                id: "entwarnung".into(),
            },
            acknowledged: SoundChoice::Builtin {
                id: "bestaetigung".into(),
            },
            downtime: SoundChoice::Builtin {
                id: "hinweis".into(),
            },
        }
    }

    fn id_von(choice: &SoundChoice) -> Option<&str> {
        match choice {
            SoundChoice::Builtin { id } => Some(id),
            _ => None,
        }
    }

    #[test]
    fn eine_stufe_allein_waehlt_ihren_klang() {
        let s = klaenge();
        assert_eq!(
            id_von(loudest(&[ereignis(EventKind::Critical)], &s)),
            Some("kritisch")
        );
        assert_eq!(
            id_von(loudest(&[ereignis(EventKind::Warning)], &s)),
            Some("warnung")
        );
        assert_eq!(
            id_von(loudest(&[ereignis(EventKind::Recovery)], &s)),
            Some("entwarnung")
        );
    }

    /// Der Kern: eine Entwarnung in derselben Runde darf den Alarm nicht
    /// verdrängen. Die Reihenfolge im Vektor darf keine Rolle spielen.
    #[test]
    fn das_dringlichste_gibt_den_ton_an() {
        let s = klaenge();
        let gemischt = [
            ereignis(EventKind::Recovery),
            ereignis(EventKind::Warning),
            ereignis(EventKind::Critical),
        ];
        assert_eq!(id_von(loudest(&gemischt, &s)), Some("kritisch"));

        let umgekehrt = [ereignis(EventKind::Critical), ereignis(EventKind::Recovery)];
        assert_eq!(id_von(loudest(&umgekehrt, &s)), Some("kritisch"));

        let ohne_kritisch = [ereignis(EventKind::Recovery), ereignis(EventKind::Warning)];
        assert_eq!(id_von(loudest(&ohne_kritisch, &s)), Some("warnung"));
    }

    /// „Für Kritisches keinen Ton" muss **still** bedeuten — nicht: dann eben
    /// den Warnton. Sonst wäre das Abschalten wirkungslos.
    #[test]
    fn kein_ton_fuer_die_stufe_weicht_nicht_aus() {
        let mut s = klaenge();
        s.critical = SoundChoice::None;
        let gemischt = [ereignis(EventKind::Critical), ereignis(EventKind::Warning)];
        assert!(
            loudest(&gemischt, &s).is_none(),
            "hat auf die Warnstufe ausgewichen"
        );
    }

    #[test]
    fn die_zusammenfassung_zaehlt_je_art() {
        let events = [
            ereignis(EventKind::Critical),
            ereignis(EventKind::Critical),
            ereignis(EventKind::Recovery),
        ];
        let text = zusammenfassung(&events);
        assert!(text.contains("2 kritisch"), "{text}");
        assert!(text.contains("0 Warnung"), "{text}");
        assert!(text.contains("1 Entwarnung"), "{text}");
    }
}
