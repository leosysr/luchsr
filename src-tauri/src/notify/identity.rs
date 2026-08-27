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

/// Alle Werte, die dieses Modul verwaltet.
///
/// Getrennt von [`desired`], weil der Abgleich auch **entfernen** muss.
/// `IconUri` stand in einer früheren Fassung dort und trug das Markensymbol in
/// die Kopfzeile; die Kopfzeile trägt jetzt keines mehr. Ein Abgleich, der nur
/// schreibt, liesse den Wert stehen — und dann zeigte Windows weiter ein
/// Symbol, das niemand mehr angefordert hat.
///
/// Die Liste ist bewusst vollständig und nicht bloss „was früher da war": so
/// gilt sie auch für die nächste Änderung.
pub const MANAGED: &[&str] = &["DisplayName", "IconUri", "ShowInSettings"];

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
///
/// **Kein `IconUri`.** Die Kopfzeile trägt bewusst kein Symbol — Begründung im
/// Kopf von [`super::toast`]. Der Wert steht trotzdem in [`MANAGED`], damit ein
/// bestehender Eintrag entfernt wird.
pub fn desired(display_name: &str) -> Vec<(&'static str, RegValue)> {
    vec![
        // Der Absender des Toasts. Ohne diesen Wert steht dort nichts.
        ("DisplayName", RegValue::Text(display_name.to_owned())),
        // Damit Luchsr in den Windows-Benachrichtigungseinstellungen
        // auftaucht. Wer die Toasts dort abschalten, stumm stellen oder aus
        // dem Info-Center nehmen will, findet ohne diesen Wert keinen
        // Eintrag — und hätte keinen Weg, das zu tun.
        ("ShowInSettings", RegValue::Number(1)),
    ]
}

/// Was der Abgleich getan hat.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Reconciled {
    /// Werte, die geschrieben werden mussten.
    pub written: Vec<&'static str>,
    /// Verwaltete Werte, die es nicht mehr geben soll und die entfernt wurden.
    pub removed: Vec<&'static str>,
}

impl Reconciled {
    /// Ob nichts zu tun war — es stand schon alles richtig da.
    pub fn is_empty(&self) -> bool {
        self.written.is_empty() && self.removed.is_empty()
    }
}

/// Gleicht den Registry-Schlüssel gegen die Sollwerte ab.
///
/// Schreibt, was abweicht, und entfernt jeden verwalteten Wert, der nicht mehr
/// gewollt ist. Nicht verwaltete Werte bleiben unangetastet: der Schlüssel
/// gehört Windows, nicht diesem Modul.
pub fn reconcile(
    aumid: &str,
    values: &[(&'static str, RegValue)],
) -> windows_registry::Result<Reconciled> {
    let key = windows_registry::CURRENT_USER.create(key_path(aumid))?;
    let mut ergebnis = Reconciled::default();

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
        ergebnis.written.push(*name);
    }

    for name in MANAGED {
        if values.iter().any(|(gewollt, _)| gewollt == name) {
            continue;
        }
        // `get_type` ist die günstigste Frage nach „gibt es den Wert" — sie
        // liest ihn nicht. Fehlt er, ist nichts zu entfernen und der Abgleich
        // meldet richtigerweise keine Änderung.
        if key.get_type(name).is_err() {
            continue;
        }
        key.remove_value(name)?;
        ergebnis.removed.push(*name);
    }

    Ok(ergebnis)
}

/// Entfernt den Schlüssel. Nur für die Selbsttests.
#[cfg(test)]
fn remove(aumid: &str) -> windows_registry::Result<()> {
    windows_registry::CURRENT_USER.remove_tree(key_path(aumid))
}

#[cfg(test)]
mod tests {
    use super::*;

    /* ------------------------------------------------------------ rein -- */

    #[test]
    fn der_schluesselpfad_liegt_im_klassenzweig() {
        assert_eq!(
            key_path("de.leosysr.luchsr"),
            r"Software\Classes\AppUserModelId\de.leosysr.luchsr"
        );
    }

    #[test]
    fn die_sollwerte_nennen_absender_und_sichtbarkeit() {
        let werte = desired("Luchsr");
        let namen: Vec<_> = werte.iter().map(|(n, _)| *n).collect();
        assert_eq!(namen, vec!["DisplayName", "ShowInSettings"]);
        assert_eq!(werte[0].1, RegValue::Text("Luchsr".into()));
        assert_eq!(werte[1].1, RegValue::Number(1));
    }

    /// Die Kopfzeile trägt bewusst kein Symbol — siehe `super::toast`. Der Wert
    /// muss aber verwaltet bleiben, sonst wird ein bestehender nie entfernt.
    #[test]
    fn iconuri_ist_nicht_gewollt_aber_verwaltet() {
        assert!(!desired("Luchsr").iter().any(|(n, _)| *n == "IconUri"));
        assert!(MANAGED.contains(&"IconUri"));
    }

    #[test]
    fn jeder_sollwert_ist_verwaltet() {
        // Sonst gäbe es einen Wert, den der Abgleich schreibt und nie wieder
        // aufräumen könnte.
        for (name, _) in desired("Luchsr") {
            assert!(MANAGED.contains(&name), "{name} fehlt in MANAGED");
        }
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

        let soll = desired("Luchsr Selbsttest");

        let erste = reconcile(AUMID, &soll).expect("erster Abgleich");
        assert_eq!(
            erste.written,
            vec!["DisplayName", "ShowInSettings"],
            "beim ersten Mal muss alles geschrieben werden"
        );
        assert!(erste.removed.is_empty(), "es gab nichts zu entfernen");

        let zweite = reconcile(AUMID, &soll).expect("zweiter Abgleich");
        assert!(
            zweite.is_empty(),
            "unverändert darf nichts geschehen, geschehen ist: {zweite:?}"
        );
    }

    #[test]
    fn ein_veralteter_absendername_wird_geheilt() {
        // Der Fall, für den der Abgleich da ist: der Wert steht da, aber
        // falsch. Nur `DisplayName` darf sich ändern.
        const AUMID: &str = "luchsr-selbsttest-heilung";
        let _wache = Aufraeumer(AUMID);

        reconcile(AUMID, &desired("Alter Name")).expect("Vorlauf");

        let ergebnis = reconcile(AUMID, &desired("Luchsr")).expect("Heilung");
        assert_eq!(ergebnis.written, vec!["DisplayName"]);
        assert!(ergebnis.removed.is_empty());
    }

    /// Der Fall, der den Entfernen-Zweig überhaupt nötig macht: eine frühere
    /// Fassung hat `IconUri` eingetragen. Ohne Entfernen zeigte Windows weiter
    /// ein Symbol, das niemand mehr angefordert hat.
    #[test]
    fn ein_bestehendes_iconuri_wird_entfernt() {
        const AUMID: &str = "luchsr-selbsttest-altlast";
        let _wache = Aufraeumer(AUMID);

        // Zustand der alten Fassung nachstellen.
        let alt = vec![
            ("DisplayName", RegValue::Text("Luchsr".into())),
            ("IconUri", RegValue::Text(r"C:\alt\app-64.png".into())),
            ("ShowInSettings", RegValue::Number(1)),
        ];
        let vorlauf = reconcile(AUMID, &alt).expect("Vorlauf");
        assert!(vorlauf.written.contains(&"IconUri"));

        let ergebnis = reconcile(AUMID, &desired("Luchsr")).expect("Abgleich");
        assert_eq!(ergebnis.removed, vec!["IconUri"]);
        assert!(ergebnis.written.is_empty(), "der Rest stand schon richtig");

        // Und ein zweiter Lauf meldet nichts mehr — nicht wiederholt
        // entfernen, was schon weg ist.
        let nochmal = reconcile(AUMID, &desired("Luchsr")).expect("zweiter Abgleich");
        assert!(nochmal.is_empty(), "geschehen ist: {nochmal:?}");
    }

    /// Werte, die dieses Modul nicht verwaltet, gehören nicht ihm.
    #[test]
    fn fremde_werte_bleiben_stehen() {
        const AUMID: &str = "luchsr-selbsttest-fremd";
        let _wache = Aufraeumer(AUMID);

        let key = windows_registry::CURRENT_USER
            .create(key_path(AUMID))
            .expect("Schlüssel");
        key.set_string("CustomActivator", "{irgendwas}")
            .expect("Fremdwert");

        reconcile(AUMID, &desired("Luchsr")).expect("Abgleich");

        assert_eq!(
            key.get_string("CustomActivator").ok().as_deref(),
            Some("{irgendwas}")
        );
    }
}
