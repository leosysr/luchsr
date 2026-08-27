//! Texte der nativen Oberflächenteile.
//!
//! Das Tray-Menü wird von Windows gezeichnet, nicht vom WebView — das
//! Wörterbuch in `src/i18n/de.ts` ist von hier aus nicht erreichbar. Deshalb
//! gibt es diese kleine zweite Tabelle.
//!
//! Sie enthält **nur**, was nativ gebraucht wird. Alles, was im Fenster
//! erscheint, gehört ins Frontend-Wörterbuch; zwei Quellen für denselben Text
//! wären eine Einladung, sie auseinanderlaufen zu lassen. Die Schlüssel sind
//! absichtlich dieselben wie dort (`tray.*`), damit ein Abgleich möglich ist.

/// Deutsche Texte des Tray-Menüs.
///
/// Die Reihenfolge entspricht dem Auftrag: Öffnen, Jetzt aktualisieren,
/// CheckMK im Browser öffnen, Einstellungen, Beenden.
pub mod tray {
    pub const OPEN: &str = "Öffnen";
    pub const REFRESH: &str = "Jetzt aktualisieren";
    pub const OPEN_IN_BROWSER: &str = "CheckMK im Browser öffnen";
    pub const SETTINGS: &str = "Einstellungen";
    pub const QUIT: &str = "Beenden";
}

/// Meldungen, die im Tooltip oder in Protokollen landen.
pub mod status {
    pub const NOT_CONFIGURED: &str =
        "Nicht eingerichtet — Server, Site und Automation-Secret fehlen";
    pub const NO_SECRET: &str = "Kein Automation-Secret gespeichert";
}

/// Bausteine der Windows-Benachrichtigungen.
///
/// Auch die zeichnet Windows, nicht das WebView — dieselbe Begründung wie beim
/// Traymenü. Die Texte sind kurz gehalten: ein Toast bricht nach wenigen
/// Zeilen ab, und was abgeschnitten wird, hat niemand gelesen.
pub mod notify {
    /// Vorangestellt, damit im Info-Center erkennbar ist, wer meldet.
    pub const APP: &str = "Luchsr";
    /// Ein Problem ist aufgetreten oder hat den Zustand gewechselt.
    pub const RECOVERED: &str = "wieder in Ordnung";
    /// Trennt Host und Service in der Kopfzeile.
    pub const SEPARATOR: &str = " · ";
    /// Ein Hostproblem hat keinen Servicenamen.
    pub const HOST_PROBLEM: &str = "Host";
    /// Zusatz, wenn CheckMK das Problem als flatternd meldet.
    pub const FLAPPING: &str = "flattert";
    /// Vorher-Nachher bei einem Zustandswechsel.
    pub const WAS: &str = "vorher";
    /// Ein Problem ist nicht mehr im Meldebereich, aber auch nicht weg.
    pub const NOW_ACKNOWLEDGED: &str = "jetzt quittiert";
    pub const NOW_DOWNTIME: &str = "jetzt in Wartung";
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Die fünf Menüeinträge aus dem Auftrag müssen alle belegt sein.
    #[test]
    fn traymenue_texte_sind_gesetzt() {
        for text in [
            tray::OPEN,
            tray::REFRESH,
            tray::OPEN_IN_BROWSER,
            tray::SETTINGS,
            tray::QUIT,
        ] {
            assert!(!text.is_empty());
        }
    }

    /// Sie müssen sich unterscheiden — ein doppelter Text wäre ein
    /// Kopierfehler, der im Menü sofort auffällt, im Test aber früher.
    #[test]
    fn traymenue_texte_sind_eindeutig() {
        let texte = [
            tray::OPEN,
            tray::REFRESH,
            tray::OPEN_IN_BROWSER,
            tray::SETTINGS,
            tray::QUIT,
        ];
        let eindeutig: std::collections::HashSet<_> = texte.iter().collect();
        assert_eq!(
            eindeutig.len(),
            texte.len(),
            "doppelter Menütext: {texte:?}"
        );
    }

    /// Die Schlüssel müssen zum Frontend-Wörterbuch passen. Der Test liest
    /// `src/i18n/de.ts` und vergleicht die Texte — läuft ein Text auseinander,
    /// fällt es hier auf und nicht erst dem Benutzer.
    #[test]
    fn traymenue_texte_stimmen_mit_dem_frontend_ueberein() {
        let de = include_str!("../../src/i18n/de.ts");
        for (schluessel, erwartet) in [
            ("tray.open", tray::OPEN),
            ("tray.refresh", tray::REFRESH),
            ("tray.openInBrowser", tray::OPEN_IN_BROWSER),
            ("tray.settings", tray::SETTINGS),
            ("tray.quit", tray::QUIT),
        ] {
            let nadel = format!("\"{schluessel}\": \"{erwartet}\"");
            assert!(
                de.contains(&nadel),
                "in src/i18n/de.ts fehlt oder weicht ab: {nadel}"
            );
        }
    }
}
