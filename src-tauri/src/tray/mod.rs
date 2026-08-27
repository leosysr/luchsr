//! Tray-Icon, Kontextmenü und Fensterpositionierung.
//!
//! | Datei          | Inhalt                                                  |
//! |----------------|---------------------------------------------------------|
//! | [`state`]      | Zustandsabbildung, Bilddaten, Tooltip — rein, getestet  |
//! | [`position`]   | Fensterposition am Infobereich — rein, getestet         |
//! | `mod.rs`       | Aufbau und Ereignisse, die Tauri-Schicht                |
//!
//! Wie im `checkmk`-Modul: alles Entscheidbare liegt in reinen Funktionen, hier
//! bleibt nur das Zusammensetzen. Das Zusammensetzen braucht ein laufendes
//! Windows samt Infobereich und ist deshalb nicht testbar — also ist es dünn.

pub mod position;
pub mod state;

use tauri::menu::{Menu, MenuBuilder, MenuEvent};
use tauri::tray::{MouseButton, MouseButtonState, TrayIcon, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Runtime};

use crate::i18n::tray as texte;

pub use position::Rect;
pub use state::{tooltip, tray_state, TrayState};

/// Kennung des Tray-Icons. Wird gebraucht, um es später wiederzufinden.
pub const TRAY_ID: &str = "luchsr";

/// Label des Popup-Fensters, wie in tauri.conf.json.
pub const POPUP_LABEL: &str = "main";

/// Kennungen der Menüeinträge.
mod menu_id {
    pub const OPEN: &str = "open";
    pub const REFRESH: &str = "refresh";
    pub const BROWSER: &str = "browser";
    pub const SETTINGS: &str = "settings";
    pub const QUIT: &str = "quit";
}

/* -------------------------------------------------------------------------- */
/* Aufbau                                                                     */
/* -------------------------------------------------------------------------- */

/// Baut das Kontextmenü in der Reihenfolge des Auftrags.
fn build_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>> {
    MenuBuilder::new(app)
        .text(menu_id::OPEN, texte::OPEN)
        .text(menu_id::REFRESH, texte::REFRESH)
        .text(menu_id::BROWSER, texte::OPEN_IN_BROWSER)
        .separator()
        .text(menu_id::SETTINGS, texte::SETTINGS)
        .separator()
        .text(menu_id::QUIT, texte::QUIT)
        .build()
}

/// Legt das Tray-Icon an.
///
/// Startzustand ist immer [`TrayState::Disconnected`]: vor dem ersten Abruf ist
/// nichts bekannt, und ein grünes Icon wäre eine Behauptung, die niemand
/// geprüft hat.
pub fn setup<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<TrayIcon<R>> {
    let scale = primary_scale_factor(app);
    let start = TrayState::Disconnected;

    TrayIconBuilder::with_id(TRAY_ID)
        .icon(tauri::image::Image::from_bytes(start.icon_bytes(scale))?)
        .tooltip(tooltip(None, None, false))
        .menu(&build_menu(app)?)
        // Linksklick öffnet das Fenster, nicht das Menü — der Auftrag ist da
        // eindeutig, und es ist auch das erwartete Windows-Verhalten.
        .show_menu_on_left_click(false)
        .on_menu_event(on_menu_event)
        .on_tray_icon_event(on_tray_event)
        .build(app)
}

/// Aktualisiert Icon und Tooltip.
///
/// Fehler werden geschluckt und protokolliert: ein nicht aktualisiertes
/// Tray-Icon ist ärgerlich, aber kein Grund, die Abrufschleife zu beenden.
pub fn update<R: Runtime>(app: &AppHandle<R>, state: TrayState, tip: &str) {
    let Some(icon) = app.tray_by_id(TRAY_ID) else {
        log::warn!("Tray-Icon {TRAY_ID} nicht gefunden, Aktualisierung übersprungen");
        return;
    };

    let scale = primary_scale_factor(app);
    match tauri::image::Image::from_bytes(state.icon_bytes(scale)) {
        Ok(image) => {
            if let Err(error) = icon.set_icon(Some(image)) {
                log::warn!("Tray-Icon liess sich nicht setzen: {error}");
            }
        }
        Err(error) => log::warn!("Tray-Bilddaten unlesbar: {error}"),
    }
    if let Err(error) = icon.set_tooltip(Some(tip)) {
        log::warn!("Tray-Tooltip liess sich nicht setzen: {error}");
    }
}

/// Skalierungsfaktor des Hauptbildschirms.
///
/// Bestimmt, ob die 16- oder die 32-px-Fassung genommen wird. Fällt auf 1.0
/// zurück: eine falsch gewählte Icongrösse ist unschön, ein Absturz beim
/// Anlegen des Tray-Icons wäre schlimmer.
fn primary_scale_factor<R: Runtime>(app: &AppHandle<R>) -> f64 {
    app.primary_monitor()
        .ok()
        .flatten()
        .map_or(1.0, |monitor| monitor.scale_factor())
}

/* -------------------------------------------------------------------------- */
/* Ereignisse                                                                 */
/* -------------------------------------------------------------------------- */

fn on_menu_event<R: Runtime>(app: &AppHandle<R>, event: MenuEvent) {
    match event.id().as_ref() {
        menu_id::OPEN => show_popup(app, None),
        menu_id::REFRESH => crate::poll::request_refresh(app),
        menu_id::BROWSER => open_in_browser(app),
        menu_id::SETTINGS => {
            show_popup(app, None);
            // Das Fenster entscheidet selbst, welche Ansicht es zeigt.
            if let Err(error) = app.emit_to(POPUP_LABEL, "luchsr://show-settings", ()) {
                log::warn!("Einstellungen liessen sich nicht öffnen: {error}");
            }
        }
        menu_id::QUIT => app.exit(0),
        other => log::warn!("unbekannter Menüeintrag: {other}"),
    }
}

fn on_tray_event<R: Runtime>(icon: &TrayIcon<R>, event: TrayIconEvent) {
    // Nur der Linksklick, und nur beim Loslassen. Auf das Drücken zu reagieren
    // öffnet das Fenster, bevor der Benutzer die Maus wieder anhebt — das
    // fühlt sich unter Windows falsch an.
    if let TrayIconEvent::Click {
        button: MouseButton::Left,
        button_state: MouseButtonState::Up,
        rect,
        ..
    } = event
    {
        show_popup(icon.app_handle(), Some(rect));
    }
}

/// Öffnet das Popup und positioniert es, wenn ein Tray-Rechteck vorliegt.
pub fn show_popup<R: Runtime>(app: &AppHandle<R>, tray_rect: Option<tauri::Rect>) {
    let Some(window) = app.get_webview_window(POPUP_LABEL) else {
        log::warn!("Fenster {POPUP_LABEL} nicht gefunden");
        return;
    };

    // Ein zweiter Klick auf das Icon schliesst das Fenster wieder — so
    // verhalten sich Windows-Tray-Anwendungen.
    if window.is_visible().unwrap_or(false) && tray_rect.is_some() {
        let _ = window.hide();
        return;
    }

    // Der Klick nimmt dem Fenster zuerst den Fokus, der Fokusverlust-Behandler
    // verbirgt es also, bevor der Klick hier ankommt. Ohne diese Abfrage würde
    // der Klick es sofort wieder öffnen und das Fenster liesse sich per Icon
    // nie schliessen. Siehe AppState::just_auto_hidden.
    if tray_rect.is_some() && app.state::<crate::commands::AppState>().just_auto_hidden() {
        return;
    }

    if let Some(rect) = tray_rect {
        if let Err(error) = place_at_tray(app, &window, rect) {
            log::warn!("Fenster liess sich nicht positionieren: {error}");
        }
    }

    let _ = window.show();
    let _ = window.set_focus();
}

/// Rechnet die Position aus und setzt sie.
fn place_at_tray<R: Runtime>(
    app: &AppHandle<R>,
    window: &tauri::WebviewWindow<R>,
    rect: tauri::Rect,
) -> tauri::Result<()> {
    // Tauri liefert das Rechteck als Position+Size in wahlweise logischen oder
    // physischen Einheiten. Für die Rechnung wird alles physisch gebraucht.
    let scale = window.scale_factor()?;
    let position = rect.position.to_physical::<f64>(scale);
    let size = rect.size.to_physical::<f64>(scale);
    let tray = Rect::new(position.x, position.y, size.width, size.height);

    let window_size = window.outer_size()?;
    let wanted = (f64::from(window_size.width), f64::from(window_size.height));

    // Arbeitsbereich statt Bildschirmgrösse: der schliesst die Taskleiste aus.
    // Ohne das läge das Fenster bei einer Taskleiste am unteren Rand darunter.
    let monitor = app
        .monitor_from_point(tray.x + tray.width / 2.0, tray.y + tray.height / 2.0)?
        .or(app.primary_monitor()?);

    let (bounds, monitor_scale) = match &monitor {
        Some(monitor) => {
            let area = monitor.work_area();
            (
                Rect::new(
                    f64::from(area.position.x),
                    f64::from(area.position.y),
                    f64::from(area.size.width),
                    f64::from(area.size.height),
                ),
                monitor.scale_factor(),
            )
        }
        // Ohne Monitorauskunft bleibt nur, das Tray-Rechteck als Anker zu
        // nehmen und nicht zu klemmen.
        None => (
            Rect::new(
                f64::MIN / 4.0,
                f64::MIN / 4.0,
                f64::MAX / 2.0,
                f64::MAX / 2.0,
            ),
            scale,
        ),
    };

    let (x, y) = position::popup_position(tray, wanted, bounds, monitor_scale);
    window.set_position(tauri::PhysicalPosition::new(x, y))?;
    Ok(())
}

/// Verbirgt das Fenster bei Fokusverlust — sofern es nicht angeheftet ist.
///
/// Der Auftrag verlangt beides: „verschwindet bei Fokusverlust" und
/// „angeheftet als Option".
///
/// **Während der Ersteinrichtung greift es nicht.** Ist die Verbindung noch
/// unvollständig, würde jeder Blick in ein anderes Fenster den Assistenten
/// wegnehmen — etwa wenn man das Automation-Secret aus CheckMK herüberkopiert,
/// was genau der wahrscheinlichste Ablauf ist.
pub fn watch_focus<R: Runtime>(window: &tauri::WebviewWindow<R>) {
    let handle = window.app_handle().clone();
    window.on_window_event(move |event| {
        if !matches!(event, tauri::WindowEvent::Focused(false)) {
            return;
        }
        let state = handle.state::<crate::commands::AppState>();
        // Ein Systemdialog nimmt dem Popup den Fokus. Würde es dann ausblenden,
        // wäre es nach dem Speichern weg — samt der Meldung, wohin.
        if state.is_modal_open() {
            return;
        }
        let settings = state.settings();
        if settings.behaviour.pin_popup || !settings.active().is_complete() {
            return;
        }
        if let Some(window) = handle.get_webview_window(POPUP_LABEL) {
            // Ist das Fenster schon verborgen, ist der Fokusverlust die Folge
            // des Verbergens und nicht sein Anlass — das passiert nach einem
            // Klick auf den Schliessknopf. Hier trotzdem `note_auto_hide` zu
            // rufen würde die Gnadenfrist setzen, und ein sofortiger Klick auf
            // das Tray-Icon bliebe wirkungslos.
            if !window.is_visible().unwrap_or(false) {
                return;
            }
            state.note_auto_hide();
            let _ = window.hide();
        }
    });
}

/// Öffnet die Problemübersicht der konfigurierten Site im Standardbrowser.
fn open_in_browser<R: Runtime>(app: &AppHandle<R>) {
    let state = app.state::<crate::commands::AppState>();
    let settings = state.settings();
    let connection = settings.active();

    match crate::checkmk::SiteUrl::new(&connection.server, &connection.site) {
        Ok(urls) => match urls.overview_page() {
            Ok(url) => {
                if let Err(error) = tauri_plugin_opener::open_url(url.as_str(), None::<&str>) {
                    log::warn!("Browser liess sich nicht öffnen: {error}");
                }
            }
            Err(error) => log::warn!("Übersichts-URL liess sich nicht bilden: {error}"),
        },
        // Noch nicht eingerichtet — dann ist das Fenster der richtige Ort.
        Err(_) => show_popup(app, None),
    }
}

#[cfg(test)]
mod tests {
    /// Die Menükennungen dürfen sich nicht doppeln — sonst löst ein Eintrag
    /// die Aktion eines anderen aus.
    #[test]
    fn menuekennungen_sind_eindeutig() {
        let ids = [
            super::menu_id::OPEN,
            super::menu_id::REFRESH,
            super::menu_id::BROWSER,
            super::menu_id::SETTINGS,
            super::menu_id::QUIT,
        ];
        let eindeutig: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(eindeutig.len(), ids.len(), "doppelte Menükennung: {ids:?}");
    }

    /// Das Fensterlabel muss zu tauri.conf.json passen, sonst findet
    /// `get_webview_window` nichts und der Klick auf das Icon tut nichts.
    #[test]
    fn fensterlabel_stimmt_mit_der_konfiguration_ueberein() {
        let config = include_str!("../../tauri.conf.json");
        assert!(
            config.contains(&format!("\"label\": \"{}\"", super::POPUP_LABEL)),
            "Label {} fehlt in tauri.conf.json",
            super::POPUP_LABEL
        );
    }
}
