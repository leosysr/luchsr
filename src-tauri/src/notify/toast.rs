//! Den Windows-Toast selbst bauen.
//!
//! ## Warum nicht über `tauri-plugin-notification`
//!
//! Zwei Gründe, beide im Quelltext der Abhängigkeiten nachgelesen und nicht
//! vermutet:
//!
//! 1. Das Plugin setzt die AppUserModelID **nur**, wenn die ausführbare Datei
//!    nicht unter `target\debug` oder `target\release` liegt. Im
//!    Entwicklungsbau bleibt sie also ungesetzt, und `notify-rust` fällt dann
//!    auf `Toast::POWERSHELL_APP_ID` zurück — der Toast trägt Namen und Symbol
//!    von Windows PowerShell.
//!
//! 2. Die Windows-Umsetzung von `notify-rust` liest für das Bild nur
//!    `path_to_image`. Das Plugin setzt aber `icon`. Das Symbol wird damit
//!    verworfen, egal was man angibt.
//!
//! Über das Plugin ist also weder der Name noch das Logo erreichbar. Deshalb
//! wird `tauri-winrt-notification` hier direkt angesprochen — es lag ohnehin
//! als indirekte Abhängigkeit im Baum, unter dem Plugin.
//!
//! ## Aufbau des Toasts
//!
//! ```text
//! +--------------------------------------------+
//! | Luchsr                                   x |  Absender aus der Registry,
//! | +------+                                   |  siehe `identity`
//! | |Luchs |  CRIT  srv01 - Festplatte /var    |  Kopfzeile, fett
//! | | auf  |  DISK CRITICAL - free space 2%    |  erste Rumpfzeile
//! | |Farbe |  (war WARN)                       |  zweite Rumpfzeile
//! | +------+                                   |
//! +--------------------------------------------+
//! ```
//!
//! Die Farbfläche links ist dieselbe Bildsprache wie das Tray-Icon (D24):
//! gefüllte Kachel in der Zustandsfarbe, Luchs in Ink ausgestanzt. Wer den
//! Toast aus dem Augenwinkel sieht, weiss die Schwere, bevor er liest.
//!
//! Der Zustand steht **zusätzlich als Text** in der Kopfzeile. Das ist keine
//! Doppelung, sondern die Regel des Projekts: Status wird nie allein über
//! Farbe kodiert.
//!
//! ## Warum die Kopfzeile kein Symbol trägt
//!
//! Sie hatte eines, in der Markenfarbe Mint. Nebeneinander war das verwirrend:
//! ein grünes Symbol neben einer roten Fläche, und Grün heisst in diesem
//! Programm „OK". Genau **ein** farbiges Element, und das bedeutet eine Sache.
//!
//! Das Symbol der Zustandsfarbe folgen zu lassen wäre die andere Richtung
//! gewesen — und wäre falsch: es kommt aus `IconUri` der AppUserModelID, also
//! aus **einer** Datei für die ganze Anwendung. Das Info-Center zeichnet alte
//! Toasts daraus neu. Alle vergangenen Meldungen trügen damit die aktuelle
//! Farbe: ein CRIT von vor zehn Minuten stünde nach der Entwarnung in Grün.
//! Das ist der Fehler aus D26 in anderer Gestalt.

use std::fs;
use std::io;
use std::path::Path;

use tauri_winrt_notification::{Duration, IconCrop, Toast};

use super::decide::{EventKind, NotifyEvent};
use crate::checkmk::ProblemState;

/// Kantenlänge der Zustandslogos in Pixeln.
///
/// Windows zeichnet `appLogoOverride` mit 48 px bei 100 % Skalierung und
/// skaliert bis 400 %. Darüber gewinnt nichts mehr dazu.
const LOGO_SIZE: u32 = 192;

/// Ein eingebautes Zustandslogo.
struct Logo {
    key: &'static str,
    data: &'static [u8],
}

/// Bindet ein Logo ein und leitet den Dateinamen aus der Kennung ab.
///
/// Dieselbe Begründung wie beim `klang!`-Makro (D82): stünden Kennung und
/// Dateiname nebeneinander, wäre ein Eintrag formulierbar, der eine andere
/// Datei einbindet als er behauptet — und das fiele erst zur Laufzeit auf, als
/// falsche Farbe im Toast.
macro_rules! logo {
    ($key:literal) => {
        Logo {
            key: $key,
            data: include_bytes!(concat!("../../icons/toast/", $key, "-192.png")),
        }
    };
}

/// Die Zustandslogos.
///
/// `disconnected` fehlt: nach einem Fehlversuch wird nicht gemeldet (D62), es
/// gäbe also nie einen Toast in diesem Zustand.
static LOGOS: &[Logo] = &[
    logo!("ok"),
    logo!("warn"),
    logo!("crit"),
    logo!("down"),
    logo!("unknown"),
];

/// Dateiendung der Logos. Steht hier, weil `extract` danach aufräumt.
const LOGO_EXT: &str = "png";

/// Welches Zustandslogo zu einem Ereignis gehört.
///
/// Eine Entwarnung ist grün, aus welchem Zustand sie auch kommt — sie ist eine
/// gute Nachricht. Sonst entscheidet der Zustand, in derselben Zuordnung wie
/// das Tray-Icon: CRIT und DOWN sind zwei getrennte Farbtöne (D23), und
/// UNREACHABLE teilt den Ton mit DOWN, weil beides „der Host antwortet nicht"
/// heisst.
pub fn logo_key(kind: EventKind, state: Option<ProblemState>) -> &'static str {
    if kind == EventKind::Recovery {
        return "ok";
    }
    match state {
        Some(ProblemState::Crit) => "crit",
        Some(ProblemState::Down | ProblemState::Unreachable) => "down",
        Some(ProblemState::Unknown) => "unknown",
        Some(ProblemState::Warn) => "warn",
        // Ein Problemereignis ohne Zustand gibt es nicht. Käme es doch, ist
        // ein neutrales Logo besser als gar kein Toast.
        Some(ProblemState::Ok) | None => "ok",
    }
}

/// Alternativtext des Logos, für Sprachausgabe und Info-Center.
///
/// Nicht der Titel: der steht schon als Text daneben, und ihn zu wiederholen
/// liest sich in der Sprachausgabe als Stottern.
pub fn logo_alt(key: &str) -> &'static str {
    match key {
        "crit" => "Zustand kritisch",
        "down" => "Host nicht erreichbar",
        "unknown" => "Zustand unbekannt",
        "warn" => "Warnung",
        _ => "Wieder in Ordnung",
    }
}

/// Zerlegt den Rumpf in die zwei Textzeilen des Toasts.
///
/// `ToastGeneric` kennt drei Textfelder: eine fette Kopfzeile und zwei Zeilen
/// darunter. Beide Angaben in ein Feld mit Zeilenumbruch zu schreiben stellt
/// Windows zwar dar, aber ohne den Zeilenabstand — es sähe aus wie ein
/// umgebrochener Satz statt wie zwei Angaben.
pub fn body_lines(body: &str) -> (&str, &str) {
    match body.split_once('\n') {
        Some((erste, rest)) => (erste.trim(), rest.trim()),
        None => (body.trim(), ""),
    }
}

/// Schreibt die eingebauten Logos in ein Verzeichnis.
///
/// Windows lädt das Bild eines Toasts über eine `file:///`-Adresse. Aus dem
/// Speicher geht es nicht — anders als beim Klang, der mit `SND_MEMORY`
/// gespielt wird (D64). Die Dateien müssen also auf der Platte liegen.
///
/// Geschrieben wird bei jedem Start, wenn eine Datei fehlt oder abweicht: eine
/// halbe Installation, ein Virenscanner oder ein aufräumender Benutzer soll
/// den Toast nicht dauerhaft entstellen. Der Vergleich vermeidet, dass bei
/// jedem Start sinnlos 16 KB geschrieben werden.
pub fn extract(dir: &Path) -> io::Result<()> {
    fs::create_dir_all(dir)?;

    let mut gewollt = Vec::with_capacity(LOGOS.len());
    for logo in LOGOS {
        let name = logo_file(logo.key);
        write_if_different(&dir.join(&name), logo.data)?;
        gewollt.push(name);
    }

    remove_orphans(dir, &gewollt);
    Ok(())
}

/// Dateiname eines Logos.
fn logo_file(key: &str) -> String {
    format!("{key}-{LOGO_SIZE}.{LOGO_EXT}")
}

fn write_if_different(path: &Path, data: &[u8]) -> io::Result<()> {
    if fs::read(path).is_ok_and(|vorhanden| vorhanden == data) {
        return Ok(());
    }
    fs::write(path, data)
}

/// Räumt Bilder weg, die dieses Modul nicht mehr auslegt.
///
/// Dasselbe Vorgehen wie bei den Klängen in `make-sounds.mjs`, und aus
/// demselben Grund: eine frühere Fassung legte hier ein `app-64.png` für die
/// Toast-Kopfzeile ab. Ohne diesen Schritt bliebe es liegen, ohne dass es
/// jemand noch erklären könnte. Eine Namensliste veralteter Dateien zu pflegen
/// wäre die Alternative — die wäre nach der zweiten Änderung ein Friedhof.
///
/// Fehler werden übergangen: das Auslegen ist gelungen, und ein nicht
/// gelöschter Rest ist kein Grund, den Start zu behelligen.
fn remove_orphans(dir: &Path, gewollt: &[String]) {
    let Ok(inhalt) = fs::read_dir(dir) else {
        return;
    };
    for eintrag in inhalt.flatten() {
        let pfad = eintrag.path();
        if pfad.extension().and_then(|e| e.to_str()) != Some(LOGO_EXT) {
            continue;
        }
        let Some(name) = pfad.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if gewollt.iter().any(|g| g == name) {
            continue;
        }
        match fs::remove_file(&pfad) {
            Ok(()) => log::info!("verwaistes Toast-Logo entfernt: {name}"),
            Err(fehler) => log::debug!("verwaistes Toast-Logo {name} bleibt liegen: {fehler}"),
        }
    }
}

/// Schickt einen Toast.
pub fn send(
    aumid: &str,
    dir: &Path,
    event: &NotifyEvent,
) -> Result<(), tauri_winrt_notification::Error> {
    let (zeile1, zeile2) = body_lines(&event.body);
    let key = logo_key(event.kind, event.state);
    let logo = dir.join(logo_file(key));

    let mut toast = Toast::new(aumid)
        .title(&event.title)
        // Der Klang kommt aus `sound.rs`, einmal je Runde und nur für die
        // dringlichste Stufe (D66). Ohne dieses `None` legte Windows seinen
        // eigenen Toast-Klang darüber: man hörte zwei Töne übereinander, und
        // die Auswahl in den Einstellungen wäre eine Empfehlung statt einer
        // Entscheidung.
        .sound(None)
        .duration(Duration::Short);

    if !zeile1.is_empty() {
        toast = toast.text1(zeile1);
    }
    if !zeile2.is_empty() {
        toast = toast.text2(zeile2);
    }
    // Fehlt die Datei, kommt der Toast ohne Logo — aber er kommt. Eine
    // Meldung wegen eines fehlenden Bildes zu verschlucken wäre die falsche
    // Reihenfolge der Wichtigkeiten.
    if logo.is_file() {
        toast = toast.icon(&logo, IconCrop::Square, logo_alt(key));
    }

    toast.show()
}

/// Wo die Logos zur Laufzeit liegen, ohne `AppHandle`.
///
/// Nur für den Augenschein-Test unten. Im Betrieb kommt der Pfad aus
/// `notify::asset_dir`, damit Tauri die eine Wahrheit über seine Verzeichnisse
/// behält — zwei Wege zu demselben Ordner wären zwei Stellen, die auseinander
/// laufen können.
#[cfg(test)]
fn asset_dir_fuer_test(identifier: &str) -> Option<std::path::PathBuf> {
    std::env::var_os("LOCALAPPDATA").map(|base| {
        std::path::PathBuf::from(base)
            .join(identifier)
            .join("assets")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Signatur einer PNG-Datei.
    const PNG: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

    /* ------------------------------------------------ Logo-Zuordnung ----- */

    #[test]
    fn jeder_zustand_hat_ein_eingebautes_logo() {
        for state in [
            ProblemState::Ok,
            ProblemState::Warn,
            ProblemState::Crit,
            ProblemState::Unknown,
            ProblemState::Down,
            ProblemState::Unreachable,
        ] {
            let key = logo_key(EventKind::Critical, Some(state));
            assert!(
                LOGOS.iter().any(|l| l.key == key),
                "kein Logo für {state:?} (Kennung {key})"
            );
        }
    }

    #[test]
    fn crit_und_down_bekommen_verschiedene_logos() {
        // Der Kern von D23: zwei getrennte Farbtöne, nicht zwei Rottöne.
        assert_ne!(
            logo_key(EventKind::Critical, Some(ProblemState::Crit)),
            logo_key(EventKind::Critical, Some(ProblemState::Down))
        );
    }

    #[test]
    fn eine_entwarnung_ist_gruen_egal_woher() {
        for state in [ProblemState::Crit, ProblemState::Down, ProblemState::Warn] {
            assert_eq!(logo_key(EventKind::Recovery, Some(state)), "ok");
        }
    }

    #[test]
    fn jedes_logo_hat_einen_alternativtext() {
        for logo in LOGOS {
            assert!(!logo_alt(logo.key).is_empty());
        }
    }

    /* ----------------------------------------------- eingebaute Dateien -- */

    #[test]
    fn die_eingebauten_logos_sind_pngs() {
        for logo in LOGOS {
            assert!(logo.data.starts_with(&PNG), "{} ist kein PNG", logo.key);
        }
    }

    #[test]
    fn keine_zwei_logos_teilen_eine_kennung() {
        let mut keys: Vec<_> = LOGOS.iter().map(|l| l.key).collect();
        let vorher = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), vorher, "doppelte Kennung in LOGOS");
    }

    /* ------------------------------------------------------ Textzeilen --- */

    #[test]
    fn ein_einzeiliger_rumpf_fuellt_nur_die_erste_zeile() {
        assert_eq!(body_lines("DISK CRITICAL"), ("DISK CRITICAL", ""));
    }

    #[test]
    fn zwei_zeilen_werden_getrennt() {
        assert_eq!(
            body_lines("DISK CRITICAL - free 2%\n(war WARN)"),
            ("DISK CRITICAL - free 2%", "(war WARN)")
        );
    }

    #[test]
    fn ein_leerer_rumpf_ergibt_zwei_leere_zeilen() {
        assert_eq!(body_lines(""), ("", ""));
        assert_eq!(body_lines("   "), ("", ""));
    }

    /* --------------------------------------------------- Auspacken ------- */

    #[test]
    fn auspacken_legt_alle_dateien_an_und_wiederholt_sich_nicht() {
        let dir = std::env::temp_dir().join("luchsr-selbsttest-toast-logos");
        let _ = fs::remove_dir_all(&dir);

        extract(&dir).expect("erstes Auspacken");
        for logo in LOGOS {
            let pfad = dir.join(logo_file(logo.key));
            assert_eq!(fs::read(&pfad).expect("Logo lesbar"), logo.data);
        }

        // Zweites Auspacken darf nichts anfassen: der Zeitstempel bleibt.
        let probe = dir.join(logo_file("crit"));
        let vorher = fs::metadata(&probe)
            .and_then(|m| m.modified())
            .expect("Zeitstempel");
        extract(&dir).expect("zweites Auspacken");
        let nachher = fs::metadata(&probe)
            .and_then(|m| m.modified())
            .expect("Zeitstempel");
        assert_eq!(vorher, nachher, "unverändert darf nicht geschrieben werden");

        let _ = fs::remove_dir_all(&dir);
    }

    /// Der Fall, für den `remove_orphans` da ist: eine frühere Fassung legte
    /// hier ein `app-64.png` für die Toast-Kopfzeile ab. Es soll verschwinden.
    #[test]
    fn ein_verwaistes_bild_wird_weggeraeumt() {
        let dir = std::env::temp_dir().join("luchsr-selbsttest-toast-verwaist");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("Verzeichnis");

        let altlast = dir.join("app-64.png");
        fs::write(&altlast, "alt").expect("Vorlauf");

        extract(&dir).expect("Auspacken");
        assert!(!altlast.exists(), "app-64.png liegt noch da");

        // Die gewollten Logos sind trotzdem alle da.
        for logo in LOGOS {
            assert!(dir.join(logo_file(logo.key)).is_file(), "{}", logo.key);
        }

        let _ = fs::remove_dir_all(&dir);
    }

    /// Fremde Dateien gehen das Modul nichts an — nur Bilder räumt es auf.
    #[test]
    fn fremde_dateien_bleiben_liegen() {
        let dir = std::env::temp_dir().join("luchsr-selbsttest-toast-fremd");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("Verzeichnis");

        let fremd = dir.join("notizen.txt");
        fs::write(&fremd, "nicht anfassen").expect("Vorlauf");

        extract(&dir).expect("Auspacken");
        assert_eq!(
            fs::read_to_string(&fremd).ok().as_deref(),
            Some("nicht anfassen")
        );

        let _ = fs::remove_dir_all(&dir);
    }

    /* ----------------------------------------------------- Augenschein -- */

    /// Schickt drei echte Toasts, einen je Stufe.
    ///
    /// Absichtlich `#[ignore]`: ein Test, der ungefragt Benachrichtigungen auf
    /// den Bildschirm wirft, gehört nicht in einen Durchlauf, den jemand
    /// nebenbei startet — und auf einem Bauläufer gibt es niemanden, der
    /// hinsieht. Er ist ein Werkzeug, kein Wächter.
    ///
    /// Ob ein Toast *richtig aussieht*, kann keine Zusicherung beantworten.
    /// Deshalb prüft dieser Test nur, dass das Senden gelingt; das Urteil
    /// fällt das Auge:
    ///
    /// ```text
    /// cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored toast_augenschein --nocapture
    /// ```
    ///
    /// Er schreibt in dieselben Pfade wie die Anwendung — Logos nach
    /// `%LOCALAPPDATA%`, Identität in die Registry. Das ist kein Nebeneffekt,
    /// den man verstecken müsste: es ist genau der Zustand, den der nächste
    /// Start ohnehin herstellt.
    #[test]
    #[ignore = "wirft echte Benachrichtigungen auf den Bildschirm"]
    fn toast_augenschein() {
        const AUMID: &str = "de.leosysr.luchsr";

        let dir = asset_dir_fuer_test(AUMID).expect("LOCALAPPDATA");
        extract(&dir).expect("Logos auspacken");

        let soll = super::super::identity::desired("Luchsr");
        super::super::identity::reconcile(AUMID, &soll).expect("Identität eintragen");

        let faelle = [
            NotifyEvent {
                kind: EventKind::Critical,
                state: Some(ProblemState::Crit),
                title: "CRIT  srv01 · Festplatte /var".into(),
                body: "DISK CRITICAL - free space: /var 2% (2.1 GB)\n(war WARN)".into(),
            },
            NotifyEvent {
                kind: EventKind::Warning,
                state: Some(ProblemState::Warn),
                title: "WARN  srv02 · CPU-Last".into(),
                body: "CPU load average 15min: 8.42".into(),
            },
            NotifyEvent {
                kind: EventKind::Recovery,
                state: None,
                title: "OK  srv01 · Festplatte /var".into(),
                body: "war CRIT → wieder in Ordnung".into(),
            },
        ];

        for fall in &faelle {
            send(AUMID, &dir, fall).unwrap_or_else(|e| panic!("„{}“: {e:?}", fall.title));
            println!("geschickt: {}", fall.title);
            // Ohne Pause stapelt Windows sie so schnell, dass man den ersten
            // nicht zu sehen bekommt.
            std::thread::sleep(std::time::Duration::from_millis(1200));
        }
    }

    #[test]
    fn eine_beschaedigte_datei_wird_ersetzt() {
        let dir = std::env::temp_dir().join("luchsr-selbsttest-toast-heilung");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).expect("Verzeichnis");

        let pfad = dir.join(format!("crit-{LOGO_SIZE}.png"));
        fs::write(&pfad, "kaputt").expect("Vorlauf");

        extract(&dir).expect("Auspacken");
        assert!(fs::read(&pfad).expect("lesbar").starts_with(&PNG));

        let _ = fs::remove_dir_all(&dir);
    }
}
