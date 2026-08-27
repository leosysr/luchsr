//! Positionierung des Popup-Fensters am Infobereich.
//!
//! Der Auftrag verlangt „korrekte Positionierung bei mehreren Monitoren und
//! unterschiedlicher DPI-Skalierung". Das ist reine Rechnung, und genau deshalb
//! steht sie hier als reine Funktion: im Ereignis-Handler wäre sie nur durch
//! Ausprobieren an echter Hardware prüfbar, hier durch Tests.
//!
//! ## Einheiten
//!
//! Alles in **physischen Pixeln**. Tauri liefert das Rechteck des Tray-Icons
//! physisch, und Monitorgrenzen sind physisch. In logische Pixel umzurechnen
//! und zurück wäre eine zusätzliche Fehlerquelle ohne Gewinn — der Aufrufer
//! setzt die Position ebenfalls physisch.

/// Rechteck in physischen Pixeln.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
}

/// Abstand zwischen Fenster und Tray-Icon, in physischen Pixeln bei 100 %.
///
/// Wird mit dem Skalierungsfaktor multipliziert, damit der optische Abstand
/// bei 200 % derselbe bleibt.
pub const GAP_LOGICAL: f64 = 8.0;

/// Berechnet die Fensterposition.
///
/// * `tray` — Rechteck des Tray-Icons, physisch
/// * `window` — gewünschte Fenstergrösse `(breite, höhe)`, physisch
/// * `monitor` — Grenzen des Bildschirms, auf dem das Icon liegt, physisch
/// * `scale_factor` — Skalierung dieses Bildschirms
///
/// ## Regeln
///
/// 1. Waagerecht am Icon **zentriert**, nicht linksbündig — der Infobereich
///    liegt bei einer zentrierten Windows-11-Taskleiste nicht am Rand.
/// 2. Senkrecht **über** dem Icon. Ist dort kein Platz, darunter — bei einer
///    Taskleiste am oberen Rand ist das der Normalfall.
/// 3. Zum Schluss in die Bildschirmgrenzen geklemmt. Ein Fenster, das halb
///    neben dem Bildschirm hängt, ist schlimmer als eines, das nicht ganz
///    mittig sitzt.
pub fn popup_position(
    tray: Rect,
    window: (f64, f64),
    monitor: Rect,
    scale_factor: f64,
) -> (f64, f64) {
    let (window_width, window_height) = window;
    let gap = GAP_LOGICAL * scale_factor.max(1.0);

    // 1 — waagerecht zentriert
    let x = tray.x + tray.width / 2.0 - window_width / 2.0;

    // 2 — bevorzugt oberhalb
    let above = tray.y - gap - window_height;
    let below = tray.bottom() + gap;
    let y = if above >= monitor.y { above } else { below };

    // 3 — klemmen
    (
        clamp_axis(x, window_width, monitor.x, monitor.right()),
        clamp_axis(y, window_height, monitor.y, monitor.bottom()),
    )
}

/// Klemmt eine Achse so, dass das Fenster vollständig sichtbar bleibt.
///
/// Ist das Fenster grösser als der Bildschirm, wird die linke bzw. obere Kante
/// bevorzugt: dort sitzen bei jedem Fenster die wichtigen Elemente.
fn clamp_axis(value: f64, size: f64, min: f64, max: f64) -> f64 {
    if size >= max - min {
        return min;
    }
    value.clamp(min, max - size)
}

/// Sucht den Bildschirm, auf dem der Mittelpunkt des Tray-Icons liegt.
///
/// Fällt auf den ersten Bildschirm zurück, wenn keiner passt — das kann bei
/// einem gerade abgezogenen Monitor kurz vorkommen und darf nicht dazu führen,
/// dass sich das Fenster nirgends öffnet.
pub fn monitor_for<'a>(tray: &Rect, monitors: &'a [Rect]) -> Option<&'a Rect> {
    let cx = tray.x + tray.width / 2.0;
    let cy = tray.y + tray.height / 2.0;
    monitors
        .iter()
        .find(|m| cx >= m.x && cx < m.right() && cy >= m.y && cy < m.bottom())
        .or_else(|| monitors.first())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Voller HD-Bildschirm, Taskleiste unten, Icon rechts unten.
    fn hd() -> Rect {
        Rect::new(0.0, 0.0, 1920.0, 1080.0)
    }

    const FENSTER: (f64, f64) = (720.0, 520.0);

    /* --------------------------------------------------------- Normalfall */

    /// Zentrierung, wo sie überhaupt möglich ist.
    #[test]
    fn oeffnet_ueber_dem_icon_und_daran_zentriert() {
        let tray = Rect::new(900.0, 1048.0, 24.0, 24.0);
        let (x, y) = popup_position(tray, FENSTER, hd(), 1.0);

        // Zentriert: 900 + 12 - 360 = 552
        assert_eq!(x, 552.0);
        // Oberhalb: 1048 - 8 - 520 = 520
        assert_eq!(y, 520.0);
    }

    /// Der realistische Fall: der Infobereich liegt rechts. Ein 720 px breites
    /// Fenster kann dort nicht zentriert werden, ohne über den Rand zu ragen —
    /// die Klemmung ist die richtige Antwort, nicht die Zentrierung.
    #[test]
    fn am_infobereich_rechts_klemmt_es_statt_zu_zentrieren() {
        let tray = Rect::new(1800.0, 1048.0, 24.0, 24.0);
        let (x, y) = popup_position(tray, FENSTER, hd(), 1.0);

        assert_eq!(x, 1200.0, "1920 - 720 = 1200, rechte Kante schliesst ab");
        assert_eq!(y, 520.0);
        assert!(x + FENSTER.0 <= 1920.0);
    }

    #[test]
    fn bleibt_vollstaendig_auf_dem_bildschirm() {
        // Icon ganz am rechten Rand — zentriert würde das Fenster überstehen.
        let tray = Rect::new(1910.0, 1048.0, 10.0, 24.0);
        let (x, y) = popup_position(tray, FENSTER, hd(), 1.0);

        assert_eq!(x, 1920.0 - 720.0, "rechte Kante muss klemmen");
        assert!(x + 720.0 <= 1920.0);
        assert!(y >= 0.0 && y + 520.0 <= 1080.0);
    }

    #[test]
    fn klemmt_auch_am_linken_rand() {
        let tray = Rect::new(2.0, 1048.0, 24.0, 24.0);
        let (x, _) = popup_position(tray, FENSTER, hd(), 1.0);
        assert_eq!(x, 0.0);
    }

    /* ------------------------------------------------- Taskleiste oben --- */

    /// Bei einer Taskleiste am oberen Rand ist oberhalb kein Platz — dann
    /// gehört das Fenster darunter.
    #[test]
    fn oeffnet_unterhalb_wenn_oben_kein_platz_ist() {
        let tray = Rect::new(1800.0, 8.0, 24.0, 24.0);
        let (_, y) = popup_position(tray, FENSTER, hd(), 1.0);
        // Oberhalb wäre 8 - 8 - 520 = -520, also darunter: 32 + 8 = 40
        assert_eq!(y, 40.0);
    }

    /// Genau an der Grenze: passt es oberhalb noch, wird oberhalb genommen.
    #[test]
    fn nutzt_oberhalb_wenn_es_genau_passt() {
        // y so, dass above == monitor.y == 0
        let tray = Rect::new(900.0, 528.0, 24.0, 24.0);
        let (_, y) = popup_position(tray, FENSTER, hd(), 1.0);
        assert_eq!(y, 0.0, "528 - 8 - 520 = 0, passt genau");
    }

    /* --------------------------------------------------- Mehrere Monitore */

    /// Zweiter Monitor links vom Hauptmonitor hat negative Koordinaten. Das
    /// ist unter Windows der Normalfall und bricht naive Rechnungen.
    #[test]
    fn rechnet_auf_einem_monitor_mit_negativen_koordinaten() {
        let links = Rect::new(-1920.0, 0.0, 1920.0, 1080.0);
        // Icon nahe der Kante zum Hauptmonitor.
        let tray = Rect::new(-200.0, 1048.0, 24.0, 24.0);
        let (x, y) = popup_position(tray, FENSTER, links, 1.0);

        // Zentriert wären -548, damit ragte das Fenster bis +172 und läge halb
        // auf dem Nachbarmonitor. Ein Popup, das über zwei Bildschirme mit
        // womöglich verschiedener DPI läuft, sieht kaputt aus.
        assert_eq!(x, -720.0, "das Fenster bleibt auf seinem Monitor");
        assert_eq!(y, 520.0);
        assert!(x >= -1920.0, "darf nicht über den linken Rand hinaus");
        assert!(
            x + FENSTER.0 <= 0.0,
            "darf nicht auf den Nachbarmonitor ragen"
        );
    }

    /// Weiter innen auf demselben Monitor wird wieder zentriert.
    #[test]
    fn zentriert_auch_bei_negativen_koordinaten() {
        let links = Rect::new(-1920.0, 0.0, 1920.0, 1080.0);
        let tray = Rect::new(-1000.0, 1048.0, 24.0, 24.0);
        let (x, _) = popup_position(tray, FENSTER, links, 1.0);
        assert_eq!(x, -1000.0 + 12.0 - 360.0);
    }

    #[test]
    fn findet_den_monitor_unter_dem_icon() {
        let links = Rect::new(-1920.0, 0.0, 1920.0, 1080.0);
        let rechts = Rect::new(0.0, 0.0, 2560.0, 1440.0);
        let monitors = [links, rechts];

        let auf_links = Rect::new(-500.0, 1048.0, 24.0, 24.0);
        assert_eq!(monitor_for(&auf_links, &monitors), Some(&links));

        let auf_rechts = Rect::new(2400.0, 1400.0, 24.0, 24.0);
        assert_eq!(monitor_for(&auf_rechts, &monitors), Some(&rechts));
    }

    /// Ein gerade abgezogener Monitor darf nicht dazu führen, dass sich das
    /// Fenster nirgends öffnet.
    #[test]
    fn faellt_auf_den_ersten_monitor_zurueck() {
        let monitors = [hd()];
        let irgendwo = Rect::new(9000.0, 9000.0, 24.0, 24.0);
        assert_eq!(monitor_for(&irgendwo, &monitors), Some(&hd()));
        assert_eq!(monitor_for(&irgendwo, &[]), None);
    }

    /* --------------------------------------------------------------- DPI */

    /// Der Abstand muss mit der Skalierung mitwachsen, sonst klebt das Fenster
    /// bei 200 % optisch am Icon.
    #[test]
    fn abstand_skaliert_mit_der_dpi() {
        let monitor = Rect::new(0.0, 0.0, 3840.0, 2160.0);
        let tray = Rect::new(3700.0, 2100.0, 48.0, 48.0);
        let fenster = (1440.0, 1040.0);

        let (_, y100) = popup_position(tray, fenster, monitor, 1.0);
        let (_, y200) = popup_position(tray, fenster, monitor, 2.0);

        assert_eq!(y100, 2100.0 - 8.0 - 1040.0);
        assert_eq!(y200, 2100.0 - 16.0 - 1040.0);
        assert!(y200 < y100, "bei 200 % ist der Abstand doppelt so gross");
    }

    /// Ein Skalierungsfaktor unter 1 darf den Abstand nicht schrumpfen lassen.
    #[test]
    fn abstand_wird_nicht_kleiner_als_bei_hundert_prozent() {
        let tray = Rect::new(900.0, 1048.0, 24.0, 24.0);
        let (_, normal) = popup_position(tray, FENSTER, hd(), 1.0);
        let (_, klein) = popup_position(tray, FENSTER, hd(), 0.5);
        assert_eq!(normal, klein);
    }

    /* ------------------------------------------------------- Grenzfälle */

    /// Ein Fenster grösser als der Bildschirm: obere linke Ecke bevorzugen.
    #[test]
    fn fenster_groesser_als_der_bildschirm_landet_oben_links() {
        let klein = Rect::new(0.0, 0.0, 800.0, 600.0);
        let tray = Rect::new(700.0, 570.0, 24.0, 24.0);
        let (x, y) = popup_position(tray, (1200.0, 900.0), klein, 1.0);
        assert_eq!((x, y), (0.0, 0.0));
    }

    /// Auf einem Monitor mit Versatz muss die Klemmung dessen Ursprung
    /// benutzen, nicht (0,0).
    #[test]
    fn klemmung_nutzt_den_monitor_ursprung_nicht_null() {
        let versetzt = Rect::new(1920.0, 200.0, 1280.0, 1024.0);
        let tray = Rect::new(3180.0, 1200.0, 24.0, 24.0);
        let (x, y) = popup_position(tray, FENSTER, versetzt, 1.0);

        assert!(x >= 1920.0, "x = {x} liegt links des Monitors");
        assert!(x + 720.0 <= 3200.0, "x = {x} ragt rechts heraus");
        assert!(y >= 200.0, "y = {y} liegt über dem Monitor");
        assert!(y + 520.0 <= 1224.0, "y = {y} ragt unten heraus");
    }

    /// Über alle plausiblen Icon-Positionen hinweg darf das Fenster den
    /// Bildschirm nie verlassen.
    #[test]
    fn fenster_verlaesst_den_bildschirm_nie() {
        let monitor = hd();
        for x in (0..1920).step_by(37) {
            for y in [0.0, 8.0, 540.0, 1048.0, 1056.0] {
                let tray = Rect::new(f64::from(x), y, 24.0, 24.0);
                let (wx, wy) = popup_position(tray, FENSTER, monitor, 1.0);
                assert!(
                    wx >= monitor.x
                        && wx + FENSTER.0 <= monitor.right()
                        && wy >= monitor.y
                        && wy + FENSTER.1 <= monitor.bottom(),
                    "Icon ({x}, {y}) ergab Fenster ({wx}, {wy})"
                );
            }
        }
    }
}
