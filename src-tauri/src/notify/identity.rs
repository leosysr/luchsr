//! Die AppUserModelID — damit Windows den Toast Luchsr zuordnet.
//!
//! Ein Toast trägt keinen Absender in sich. Windows liest Name und Symbol in
//! der Kopfzeile aus der **AppUserModelID**, die beim Senden mitgegeben wird,
//! und schaut dafür in der Registry nach. Ist dort nichts hinterlegt, bleibt
//! die Kopfzeile leer oder zeigt die Anwendung, deren AUMID gesendet wurde.
//!
//! Genau das war der Zustand vor diesem Modul: es wurde die AUMID von Windows
//! PowerShell gesendet, und der Toast trug deren Namen und Symbol.
//!
//! ## Was hier eingetragen wird
//!
//! `HKCU\Software\Classes\AppUserModelId\<AUMID>` — ein Schlüssel im
//! Benutzerzweig. Er braucht **keine** erhöhten Rechte, gilt für den
//! angemeldeten Benutzer und überlebt eine Neuinstallation.
//!
//! ## Warum bei jedem Start abgeglichen wird
//!
//! Dieselbe Lehre wie beim Autostart (D80): dass ein Eintrag existiert, sagt
//! nichts darüber, ob er auf die richtige Datei zeigt. Der `IconUri` ist ein
//! Pfad, und ein Pfad kann veralten. Der Abgleich schreibt nur, was sich
//! unterscheidet — und protokolliert, was er geschrieben hat, sonst fällt eine
//! Abweichung nie auf.

use std::path::Path;

/// Ein Wert unter dem AUMID-Schlüssel.
///
/// Zwei Typen genügen: Zeichenketten und `DWORD`. Ein Aufzählungstyp statt
/// zweier Listen, damit Name und Typ zusammenbleiben — sonst wäre ein Wert
/// formulierbar, der als Zahl gelesen und als Text geschrieben wird.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RegValue {
    Text(String),
    Number(u32),
}

/// Der Schlüsselpfad zu einer AUMID, relativ zu `HKEY_CURRENT_USER`.
pub fn key_path(aumid: &str) -> String {
    format!(r"Software\Classes\AppUserModelId\{aumid}")
}

/// Was unter dem Schlüssel stehen soll.
///
/// Rein, damit der Inhalt geprüft werden kann, ohne in die Registry zu
/// schreiben.
pub fn desired(display_name: &str, icon: &Path) -> Vec<(&'static str, RegValue)> {
    vec![
        // Die Kopfzeile des Toasts. Ohne diesen Wert steht dort nichts.
        ("DisplayName", RegValue::Text(display_name.to_owned())),
        // Das kleine Symbol neben dem Namen. Muss ein Pfad auf eine
        // vorhandene Datei sein; ein toter Pfad lässt die Stelle leer.
        ("IconUri", RegValue::Text(icon.display().to_string())),
        // Damit Luchsr in den Windows-Benachrichtigungseinstellungen
        // auftaucht. Wer die Toasts dort abschalten, stumm stellen oder aus
        // dem Info-Center nehmen will, findet ohne diesen Wert keinen
        // Eintrag — und hätte keinen Weg, das zu tun.
        ("ShowInSettings", RegValue::Number(1)),
    ]
}

/// Gleicht den Registry-Schlüssel gegen die Sollwerte ab.
///
/// Gibt die Namen der Werte zurück, die geschrieben werden mussten. Eine leere
/// Liste heisst: es stand schon alles richtig da.
pub fn reconcile(
    aumid: &str,
    values: &[(&'static str, RegValue)],
) -> windows_registry::Result<Vec<&'static str>> {
    let key = windows_registry::CURRENT_USER.create(key_path(aumid))?;
    let mut geschrieben = Vec::new();

    for (name, soll) in values {
        // Der gelesene Wert wird in dieselbe Form gebracht wie der Sollwert.
        // Fehlt er oder hat er den falschen Typ, ist `ist` None — beides
        // führt zum Schreiben, und das ist richtig.
        let ist = match soll {
            RegValue::Text(_) => key.get_string(name).ok().map(RegValue::Text),
            RegValue::Number(_) => key.get_u32(name).ok().map(RegValue::Number),
        };
        if ist.as_ref() == Some(soll) {
            continue;
        }
        match soll {
            RegValue::Text(wert) => key.set_string(name, wert)?,
            RegValue::Number(wert) => key.set_u32(name, *wert)?,
        }
        geschrieben.push(*name);
    }

    Ok(geschrieben)
}

/// Entfernt den Schlüssel. Nur für die Selbsttests.
#[cfg(test)]
fn remove(aumid: &str) -> windows_registry::Result<()> {
    windows_registry::CURRENT_USER.remove_tree(key_path(aumid))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /* ------------------------------------------------------------ rein -- */

    #[test]
    fn der_schluesselpfad_liegt_im_klassenzweig() {
        assert_eq!(
            key_path("de.leosysr.luchsr"),
            r"Software\Classes\AppUserModelId\de.leosysr.luchsr"
        );
    }

    #[test]
    fn die_sollwerte_nennen_namen_symbol_und_sichtbarkeit() {
        let werte = desired("Luchsr", &PathBuf::from(r"C:\a\app-64.png"));
        let namen: Vec<_> = werte.iter().map(|(n, _)| *n).collect();
        assert_eq!(namen, vec!["DisplayName", "IconUri", "ShowInSettings"]);
        assert_eq!(werte[0].1, RegValue::Text("Luchsr".into()));
        assert_eq!(werte[1].1, RegValue::Text(r"C:\a\app-64.png".into()));
        assert_eq!(werte[2].1, RegValue::Number(1));
    }

    #[test]
    fn der_symbolpfad_wird_nicht_in_eine_url_verwandelt() {
        // `IconUri` heisst so, nimmt aber einen gewöhnlichen Pfad. Ein
        // vorangestelltes `file:///` hat Windows hier nicht angenommen.
        let werte = desired("Luchsr", &PathBuf::from(r"C:\a\b.png"));
        let RegValue::Text(pfad) = &werte[1].1 else {
            panic!("IconUri muss Text sein");
        };
        assert!(!pfad.contains("file:"), "unerwartetes Schema: {pfad}");
    }

    /* --------------------------------- gegen die echte Registry ---------- */

    /// Räumt den Testschlüssel auf, auch wenn der Test panisch wird.
    struct Aufraeumer(&'static str);
    impl Drop for Aufraeumer {
        fn drop(&mut self) {
            let _ = remove(self.0);
        }
    }

    #[test]
    fn abgleich_schreibt_einmal_und_dann_nicht_mehr() {
        const AUMID: &str = "luchsr-selbsttest-identitaet";
        let _wache = Aufraeumer(AUMID);

        let soll = desired("Luchsr Selbsttest", &PathBuf::from(r"C:\nirgends\x.png"));

        let erste = reconcile(AUMID, &soll).expect("erster Abgleich");
        assert_eq!(
            erste,
            vec!["DisplayName", "IconUri", "ShowInSettings"],
            "beim ersten Mal muss alles geschrieben werden"
        );

        let zweite = reconcile(AUMID, &soll).expect("zweiter Abgleich");
        assert!(
            zweite.is_empty(),
            "unverändert darf nichts geschrieben werden, geschrieben wurde: {zweite:?}"
        );
    }

    #[test]
    fn ein_veralteter_symbolpfad_wird_geheilt() {
        // Das ist der Fall, für den der Abgleich da ist: der Pfad zeigt
        // woandershin als er soll. Nur `IconUri` darf sich ändern.
        const AUMID: &str = "luchsr-selbsttest-heilung";
        let _wache = Aufraeumer(AUMID);

        reconcile(AUMID, &desired("Luchsr", &PathBuf::from(r"C:\alt\x.png"))).expect("Vorlauf");

        let geschrieben =
            reconcile(AUMID, &desired("Luchsr", &PathBuf::from(r"C:\neu\x.png"))).expect("Heilung");
        assert_eq!(geschrieben, vec!["IconUri"]);
    }
}
