//! Startverhalten: Autostart, zweite Instanz, Fenster beim Start.
//!
//! Alles Entscheidende liegt in reinen Funktionen — [`launched_by_autostart`],
//! [`window_on_start`], [`second_instance_action`]. Sie hängen nur an ihren
//! Eingaben und sind damit ohne Anmeldevorgang und ohne zweiten Prozess
//! prüfbar. Das Eintragen in die Registry macht das Autostart-Plugin.
//!
//! # Woran der Autostart erkannt wird
//!
//! Der Autostarteintrag bekommt [`AUTOSTART_FLAG`] als Argument mit. Ein Start
//! über die Anmeldung ist damit von einem Doppelklick unterscheidbar — und das
//! muss er sein, weil beides Verschiedenes bedeutet: bei der Anmeldung soll
//! nichts aufgehen, bei einem Doppelklick will man das Fenster sehen.
//!
//! Der Umweg über ein Argument ist nötig, weil Windows keine Auskunft darüber
//! gibt, ob ein Prozess aus dem `Run`-Schlüssel gestartet wurde.

use tauri::{AppHandle, Manager, Runtime};
use tauri_plugin_autostart::ManagerExt;

use crate::commands::AppState;

/// Argument, das der Autostarteintrag mitgibt.
///
/// Mit zwei Bindestrichen, damit es wie ein Schalter aussieht und nicht wie
/// ein Dateiname, den jemand versehentlich öffnen wollte.
pub const AUTOSTART_FLAG: &str = "--autostart";

/// Was beim Start mit dem Fenster passiert.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupWindow {
    /// Fenster zeigen und fokussieren.
    Show,
    /// Nur ins Tray, kein Fenster.
    Hide,
}

/// Was ein zweiter Start tun soll.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecondInstance {
    /// Vorhandenes Fenster nach vorne holen — der Normalfall.
    Focus,
    /// Nichts tun.
    Ignore,
}

/// Ob die Argumente einen Start durch den Autostart anzeigen.
///
/// Vergleicht genau, nicht mit `contains`: ein Pfad, in dem `--autostart`
/// vorkommt, ist kein Schalter.
pub fn launched_by_autostart<I, S>(args: I) -> bool
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    args.into_iter().any(|a| a.as_ref() == AUTOSTART_FLAG)
}

/// Entscheidet über das Fenster beim Start.
///
/// Drei Bedingungen, in dieser Reihenfolge:
///
/// 1. **Einrichtung fehlt** → zeigen, immer. Ein unkonfiguriertes Programm,
///    das verborgen startet, ist unsichtbar und tut nichts; der Benutzer sähe
///    nur ein Tray-Icon im Zustand „getrennt" und wüsste nicht, warum.
///    Das gilt **auch** beim Autostart.
/// 2. **Autostart** → verbergen. Bei der Anmeldung soll nichts aufgehen, so
///    verlangt es der Auftrag.
/// 3. Sonst gilt die Einstellung `start_minimised`.
pub fn window_on_start(
    launched_by_autostart: bool,
    start_minimised: bool,
    needs_setup: bool,
) -> StartupWindow {
    if needs_setup {
        return StartupWindow::Show;
    }
    if launched_by_autostart || start_minimised {
        return StartupWindow::Hide;
    }
    StartupWindow::Show
}

/// Entscheidet, was ein zweiter Start tut.
///
/// Normalerweise das Fenster nach vorne holen — das ist der Sinn der
/// Einzelinstanz: ein zweiter Doppelklick soll wirken, nicht ins Leere gehen.
///
/// **Ausnahme:** trägt der zweite Start die Autostartmarke, passiert nichts.
/// Der Fall tritt auf, wenn beim Anmelden schon eine Instanz läuft — etwa nach
/// einem Benutzerwechsel oder wenn der `Run`-Schlüssel doppelt eingetragen ist.
/// Dann wäre ein aufspringendes Fenster genau das, was der Autostart nicht tun
/// soll.
pub fn second_instance_action<I, S>(args: I) -> SecondInstance
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    if launched_by_autostart(args) {
        SecondInstance::Ignore
    } else {
        SecondInstance::Focus
    }
}

/// Setzt den Autostart beim **allerersten** Start und vermerkt das.
///
/// Der Auftrag verlangt beides: Vorgabe an, und ein Vermerk, damit eine
/// spätere Abschaltung durch den Benutzer nicht bei jedem Start überschrieben
/// wird. Ohne den Vermerk wäre die Einstellung nicht abschaltbar — man würde
/// sie ausschalten und beim nächsten Anmelden wäre sie wieder an.
///
/// Läuft **nach** dem Fenster: ein Fehlschlag beim Registrierungszugriff darf
/// den Start nicht verhindern. Er wird protokolliert, und die Einstellung
/// bleibt dann unvermerkt, sodass es beim nächsten Start erneut versucht wird.
pub fn initialise_autostart<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<AppState>();
    let settings = state.settings();
    let gewuenscht = settings.behaviour.autostart;

    // Bei **jedem** Start, nicht nur beim ersten: der Eintrag trägt den Pfad
    // der ausführbaren Datei, und der ändert sich mit einer Installation oder
    // einem Verschieben. Siehe [`autostart_action`].
    if let Err(error) = apply(app, gewuenscht) {
        log::warn!("Autostart liess sich nicht setzen: {error}");
        return;
    }

    if settings.behaviour.autostart_initialised {
        return;
    }

    // Erstmalig: vermerken, damit eine spätere Abschaltung durch den Benutzer
    // nicht bei jedem Start überschrieben wird.
    let mut neu = settings;
    neu.behaviour.autostart_initialised = true;
    if let Err(error) = state.store().save_unchecked(&neu) {
        // Der Autostart ist gesetzt, der Vermerk nicht. Beim nächsten Start
        // passiert dasselbe noch einmal — unschön, aber harmlos, und besser
        // als eine Einstellung, die als vermerkt gilt, ohne es zu sein.
        log::warn!("Autostart gesetzt, Vermerk liess sich nicht speichern: {error}");
        return;
    }
    state.replace_settings(neu);
    log::info!(
        "Autostart beim ersten Start {} und vermerkt",
        if gewuenscht { "aktiviert" } else { "abgelehnt" }
    );
}

/// Was am Registrierungseintrag zu tun ist.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AutostartAction {
    /// Eintrag schreiben — auch wenn schon einer da ist.
    Register,
    /// Eintrag entfernen.
    Unregister,
    /// Nichts.
    Nothing,
}

/// Entscheidet, was am Eintrag zu tun ist.
///
/// # Warum bei „an" **immer** geschrieben wird
///
/// Der Eintrag enthält den **Pfad** der ausführbaren Datei. Ob einer existiert,
/// sagt nichts darüber, ob er auf *diese* Datei zeigt. Genau das ist einmal
/// passiert: der Eintrag zeigte nach der Installation weiter auf den
/// Entwicklungsbau unter `target\debug\`, weil die alte Prüfung nur „ist ein
/// Eintrag da?" verglich. Verschwindet der alte Pfad, startet nichts mehr — und
/// niemand merkt es, weil ein fehlender Autostart keine Meldung erzeugt.
///
/// Deshalb: solange „an" gewünscht ist, wird bei jedem Start neu geschrieben.
/// Das kostet einen kleinen Registrierungszugriff und macht den Eintrag
/// selbstheilend. Der Pfad zeigt danach immer auf die Datei, die zuletzt lief.
pub fn autostart_action(wanted: bool, currently_enabled: bool) -> AutostartAction {
    match (wanted, currently_enabled) {
        (true, _) => AutostartAction::Register,
        (false, true) => AutostartAction::Unregister,
        (false, false) => AutostartAction::Nothing,
    }
}

/// Bringt den Autostart des Systems auf den gewünschten Stand.
///
/// Fehler werden weitergegeben; der Aufrufer entscheidet, ob sie den Benutzer
/// erreichen.
pub fn apply<R: Runtime>(app: &AppHandle<R>, enabled: bool) -> Result<(), String> {
    let manager = app.autolaunch();
    let ist = manager.is_enabled().map_err(|e| e.to_string())?;
    match autostart_action(enabled, ist) {
        AutostartAction::Register => manager.enable().map_err(|e| e.to_string()),
        AutostartAction::Unregister => manager.disable().map_err(|e| e.to_string()),
        AutostartAction::Nothing => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /* ------------------------------------------------- Autostart erkennen -- */

    #[test]
    fn die_marke_wird_erkannt() {
        assert!(launched_by_autostart(["luchsr.exe", AUTOSTART_FLAG]));
        assert!(launched_by_autostart([AUTOSTART_FLAG]));
    }

    #[test]
    fn ohne_marke_gilt_es_als_manueller_start() {
        assert!(!launched_by_autostart(["luchsr.exe"]));
        assert!(!launched_by_autostart(Vec::<String>::new()));
    }

    /// Ein Pfad, in dem die Marke vorkommt, ist kein Schalter. Mit `contains`
    /// statt Gleichheit wäre `C:\--autostart\luchsr.exe` ein Autostart.
    #[test]
    fn ein_pfad_mit_der_marke_im_namen_zaehlt_nicht() {
        assert!(!launched_by_autostart([r"C:\--autostart\luchsr.exe"]));
        assert!(!launched_by_autostart(["--autostartx"]));
        assert!(!launched_by_autostart(["autostart"]));
    }

    /* --------------------------------------------------- Fenster beim Start */

    /// Der Kern des Auftrags: bei der Anmeldung geht nichts auf.
    #[test]
    fn beim_autostart_bleibt_das_fenster_zu() {
        assert_eq!(window_on_start(true, false, false), StartupWindow::Hide);
        assert_eq!(window_on_start(true, true, false), StartupWindow::Hide);
    }

    /// Wer doppelklickt, will das Fenster sehen. „Ich habe es gestartet und es
    /// ist nichts passiert" ist die schlechteste Vorgabe.
    #[test]
    fn ein_manueller_start_zeigt_das_fenster() {
        assert_eq!(window_on_start(false, false, false), StartupWindow::Show);
    }

    #[test]
    fn minimiert_starten_wirkt_auch_manuell() {
        assert_eq!(window_on_start(false, true, false), StartupWindow::Hide);
    }

    /// Ein unkonfiguriertes Programm, das verborgen startet, ist unsichtbar und
    /// tut nichts. Deshalb schlägt die fehlende Einrichtung **alles**.
    #[test]
    fn fehlende_einrichtung_zeigt_das_fenster_immer() {
        for autostart in [false, true] {
            for minimiert in [false, true] {
                assert_eq!(
                    window_on_start(autostart, minimiert, true),
                    StartupWindow::Show,
                    "autostart={autostart}, minimiert={minimiert}"
                );
            }
        }
    }

    /* ----------------------------------------------------- Zweite Instanz -- */

    #[test]
    fn ein_zweiter_start_holt_das_fenster_nach_vorne() {
        assert_eq!(
            second_instance_action(["luchsr.exe"]),
            SecondInstance::Focus
        );
    }

    /// Läuft beim Anmelden schon eine Instanz, darf der Autostart kein Fenster
    /// aufreissen — sonst tut er genau das, was er nicht tun soll.
    #[test]
    fn ein_zweiter_autostart_bleibt_still() {
        assert_eq!(
            second_instance_action(["luchsr.exe", AUTOSTART_FLAG]),
            SecondInstance::Ignore
        );
    }

    /* -------------------------------------------------- Registrierungseintrag */

    /// Der Fehler, den erst der Installationstest gezeigt hat: der Eintrag
    /// zeigte nach der Installation weiter auf `target\debug\`, weil nur
    /// geprüft wurde, **ob** einer da ist — nicht, worauf er zeigt.
    #[test]
    fn bei_an_wird_immer_neu_geschrieben() {
        assert_eq!(autostart_action(true, false), AutostartAction::Register);
        assert_eq!(
            autostart_action(true, true),
            AutostartAction::Register,
            "auch wenn schon einer da ist — er könnte auf eine andere Datei zeigen"
        );
    }

    #[test]
    fn bei_aus_wird_nur_entfernt_wenn_etwas_da_ist() {
        assert_eq!(autostart_action(false, true), AutostartAction::Unregister);
        assert_eq!(autostart_action(false, false), AutostartAction::Nothing);
    }

    /* ------------------------------------------------------------- Marke -- */

    /// Die Marke muss als Schalter erkennbar sein und darf nicht mit einem
    /// Tauri-eigenen Argument kollidieren.
    #[test]
    fn die_marke_ist_ein_schalter() {
        assert!(AUTOSTART_FLAG.starts_with("--"));
        assert!(!AUTOSTART_FLAG.contains(' '));
        assert!(AUTOSTART_FLAG.len() > 2);
    }
}
