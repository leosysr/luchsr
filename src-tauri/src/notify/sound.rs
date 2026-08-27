//! Hinweistöne.
//!
//! # Woher die Klänge kommen
//!
//! Die eingebauten sind **erzeugt**, nicht besorgt: `scripts/make-sounds.mjs`
//! synthetisiert sie und legt sie nach `src-tauri/sounds/`. Damit gibt es keine
//! Herkunft und keine Lizenz mitzuführen. Sie sind hier per `include_bytes!`
//! ins Programm eingebaut.
//!
//! # Warum aus dem Speicher gespielt wird
//!
//! `PlaySoundW` kann mit `SND_MEMORY` ein WAV-Abbild direkt aus dem Speicher
//! spielen. Das ist der Grund, warum die Klänge eingebaut und nicht als
//! Ressourcendateien mitgeliefert werden: eine Datei, die es zur Laufzeit gibt,
//! kann fehlen — nach einer halben Installation, nach einem Virenscanner, nach
//! einem Benutzer, der aufgeräumt hat. Ein `&'static [u8]` kann das nicht.
//!
//! # Nur WAV
//!
//! `PlaySoundW` kann ausschliesslich WAV. Eine MP3 ergibt **keinen Fehler,
//! sondern Stille** — die schlimmste Variante, weil niemand merkt, woran es
//! liegt. Deshalb prüft [`is_supported`] die Endung und `Settings::validate`
//! warnt im Dialog. Eine Audiobibliothek mit MP3-Decoder wöge mehr als der
//! halbe Rest der Anwendung.

use std::path::Path;

use windows_sys::Win32::Media::Audio::{
    PlaySoundW, SND_ASYNC, SND_FILENAME, SND_MEMORY, SND_NODEFAULT,
};

use crate::config::SoundChoice;

/// Die einzige Endung, die `PlaySoundW` verarbeiten kann.
pub const SUPPORTED_EXTENSION: &str = "wav";

/// Ein eingebauter Klang.
pub struct BuiltinSound {
    /// Kennung in der Konfiguration. **Nicht ändern** — eine gespeicherte
    /// Auswahl zeigt sonst ins Leere.
    pub id: &'static str,
    /// Anzeige im Einstellungsdialog. Darf sich ändern.
    pub label: &'static str,
    /// Das WAV-Abbild.
    pub data: &'static [u8],
}

/// Alle eingebauten Klänge, in der Reihenfolge der Anzeige.
///
/// Muss zu den Entwürfen in `scripts/make-sounds.mjs` passen. Ein Skript kann
/// kein Rust lesen, deshalb prüft ein Test unten, dass jeder Eintrag ein
/// plausibles WAV enthält — eine falsche Einbindung wäre sonst erst zur
/// Laufzeit als Stille zu merken.
pub const BUILTIN: [BuiltinSound; 6] = [
    BuiltinSound {
        id: "hinweis",
        label: "Hinweis (zwei Töne, aufwärts)",
        data: include_bytes!("../../sounds/hinweis.wav"),
    },
    BuiltinSound {
        id: "warnung",
        label: "Warnung (zwei Töne, abwärts)",
        data: include_bytes!("../../sounds/warnung.wav"),
    },
    BuiltinSound {
        id: "kritisch",
        label: "Kritisch (drei Töne, abwärts)",
        data: include_bytes!("../../sounds/kritisch.wav"),
    },
    BuiltinSound {
        id: "alarm",
        label: "Alarm (drei kurze, einer tief)",
        data: include_bytes!("../../sounds/alarm.wav"),
    },
    BuiltinSound {
        id: "entwarnung",
        label: "Entwarnung (zwei Töne, aufwärts)",
        data: include_bytes!("../../sounds/entwarnung.wav"),
    },
    BuiltinSound {
        id: "bestaetigung",
        label: "Bestätigung (zwei sehr kurze)",
        data: include_bytes!("../../sounds/bestaetigung.wav"),
    },
];

/// Sucht einen eingebauten Klang.
pub fn builtin(id: &str) -> Option<&'static BuiltinSound> {
    BUILTIN.iter().find(|s| s.id == id)
}

/// Ob die Kennung zu einem eingebauten Klang gehört.
pub fn builtin_exists(id: &str) -> bool {
    builtin(id).is_some()
}

/// Ob die Datei von der Form her spielbar ist.
///
/// Prüft nur die Endung, nicht den Inhalt. Eine umbenannte MP3 fällt damit
/// durch das Raster — dafür braucht es keinen Dateikopfleser, denn wer eine
/// Datei umbenennt, hat eine Absicht.
pub fn is_supported(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| e.eq_ignore_ascii_case(SUPPORTED_EXTENSION))
}

/// Spielt, was die Auswahl vorgibt. `None` tut nichts.
///
/// Fehlschläge werden protokolliert und nicht weitergegeben: ein Ton, der nicht
/// kommt, darf die Benachrichtigung nicht verhindern — die Meldung ist die
/// Hauptsache, der Ton die Beigabe.
pub fn play(choice: &SoundChoice) {
    match choice {
        SoundChoice::None => {}
        SoundChoice::Builtin { id } => match builtin(id) {
            Some(s) => play_memory(s.data),
            None => log::warn!("eingebauter Klang „{id}“ ist unbekannt"),
        },
        SoundChoice::File { path } => play_file(path),
    }
}

/// Spielt ein WAV-Abbild aus dem Speicher.
fn play_memory(data: &'static [u8]) {
    // SICHERHEIT: `SND_ASYNC` gibt sofort zurück und Windows liest den Puffer
    // danach weiter. Das ist hier zulässig, weil `data` 'static ist — bei einem
    // ausgeliehenen Puffer wäre es ein Fehler nach dem Ende der Funktion.
    let ok = unsafe {
        PlaySoundW(
            data.as_ptr().cast(),
            std::ptr::null_mut(),
            SND_MEMORY | SND_ASYNC | SND_NODEFAULT,
        )
    };
    if ok == 0 {
        log::warn!("eingebauter Klang liess sich nicht spielen");
    }
}

/// Spielt eine Datei, ohne zu warten.
///
/// `SND_ASYNC` gibt sofort zurück; ohne das würde die Abrufschleife für die
/// Dauer des Klangs stehen. `SND_NODEFAULT` verhindert, dass Windows bei einer
/// fehlenden oder kaputten Datei den Standardklang spielt — dann klingt es, als
/// hätte es funktioniert.
fn play_file(path: &str) {
    if !is_supported(path) {
        log::warn!(
            "Klangdatei „{path}“ wird nicht gespielt: nur .{SUPPORTED_EXTENSION} ist möglich"
        );
        return;
    }
    if !Path::new(path).is_file() {
        log::warn!("Klangdatei „{path}“ ist nicht vorhanden");
        return;
    }

    let mut wide: Vec<u16> = path.encode_utf16().collect();
    wide.push(0);

    // SICHERHEIT: `wide` ist nullterminiert. Bei `SND_FILENAME` liest Windows
    // den Namen vor der Rückkehr, auch mit SND_ASYNC — der Puffer muss also
    // nur bis hierhin leben.
    let ok = unsafe {
        PlaySoundW(
            wide.as_ptr(),
            std::ptr::null_mut(),
            SND_FILENAME | SND_ASYNC | SND_NODEFAULT,
        )
    };
    if ok == 0 {
        log::warn!("Klangdatei „{path}“ liess sich nicht spielen");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /* -------------------------------------------------- Eingebaute Klänge -- */

    /// Jede Kennung darf nur einmal vorkommen, sonst entscheidet die
    /// Reihenfolge, welcher Klang gewinnt.
    #[test]
    fn die_kennungen_sind_eindeutig() {
        let mut ids: Vec<&str> = BUILTIN.iter().map(|s| s.id).collect();
        let anzahl = ids.len();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), anzahl, "doppelte Kennung in BUILTIN");
    }

    /// Der Test, der eine falsche Einbindung fängt: ein `include_bytes!` auf
    /// die falsche Datei fällt sonst erst als Stille auf.
    #[test]
    fn jeder_eingebaute_klang_ist_ein_plausibles_wav() {
        for s in &BUILTIN {
            assert!(s.data.len() > 1000, "{} ist verdächtig klein", s.id);
            assert_eq!(&s.data[0..4], b"RIFF", "{}: kein RIFF-Kopf", s.id);
            assert_eq!(&s.data[8..12], b"WAVE", "{}: kein WAVE", s.id);
            // Blockformat 1 = PCM, an Position 20.
            let format = u16::from_le_bytes([s.data[20], s.data[21]]);
            assert_eq!(format, 1, "{}: nicht PCM", s.id);
        }
    }

    /// Kurz heisst kurz. Bei 22050 Hz, 16 Bit, mono sind das 44100 Byte je
    /// Sekunde — mehr als 400 ms wäre gegen die Absicht der Entwürfe.
    #[test]
    fn kein_eingebauter_klang_ist_zu_lang() {
        for s in &BUILTIN {
            let ms = ((s.data.len() - 44) as f64 / 44100.0) * 1000.0;
            assert!(ms <= 400.0, "{} dauert {ms:.0} ms", s.id);
        }
    }

    #[test]
    fn kein_label_ist_leer() {
        for s in &BUILTIN {
            assert!(!s.label.trim().is_empty(), "{} ohne Beschriftung", s.id);
        }
    }

    /// Die Vorgabe der Einstellungen zeigt auf einen eingebauten Klang. Wäre
    /// die Kennung falsch geschrieben, wäre die Vorgabe stumm.
    #[test]
    fn die_vorgabe_der_einstellungen_ist_auffindbar() {
        let vorgabe = crate::config::SoundSettings::default();
        for (feld, choice) in vorgabe.alle() {
            if let SoundChoice::Builtin { id } = choice {
                assert!(builtin_exists(id), "{feld} zeigt auf unbekanntes „{id}“");
            }
        }
    }

    #[test]
    fn unbekannte_kennung_wird_nicht_gefunden() {
        assert!(!builtin_exists("gibtsnicht"));
        assert!(!builtin_exists(""));
        // Gross- und Kleinschreibung zählt: die Kennung steht in einer Datei,
        // nicht in einer Benutzereingabe.
        assert!(!builtin_exists("Kritisch"));
    }

    /* ------------------------------------------------------ Dateiformate -- */

    #[test]
    fn wav_wird_akzeptiert_unabhaengig_von_der_schreibweise() {
        for p in [
            r"C:\ton.wav",
            r"C:\ton.WAV",
            r"C:\Pfad mit Leerzeichen\Ton.Wav",
            "relativ.wav",
        ] {
            assert!(is_supported(p), "{p} sollte spielbar sein");
        }
    }

    /// Der Fall, der sonst zu stiller Ratlosigkeit führt.
    #[test]
    fn andere_formate_werden_abgelehnt() {
        for p in [
            r"C:\ton.mp3",
            r"C:\ton.ogg",
            r"C:\ton.m4a",
            r"C:\ton.flac",
            r"C:\ohne-endung",
            "",
        ] {
            assert!(!is_supported(p), "{p} sollte abgelehnt werden");
        }
    }

    /// Ein Punkt im Verzeichnisnamen darf nicht als Endung gelesen werden.
    #[test]
    fn punkt_im_verzeichnisnamen_taeuscht_nicht() {
        assert!(!is_supported(r"C:\v1.2\ton"));
        assert!(is_supported(r"C:\v1.2\ton.wav"));
    }

    /* ------------------------------------------------------------ Spielen -- */

    /// `play` darf bei jeder Eingabe zurückkehren — es läuft in der
    /// Abrufschleife und in Befehlen.
    #[test]
    fn play_ueberlebt_jede_auswahl() {
        play(&SoundChoice::None);
        play(&SoundChoice::Builtin {
            id: "gibtsnicht".into(),
        });
        play(&SoundChoice::File {
            path: String::new(),
        });
        play(&SoundChoice::File {
            path: "nicht-vorhanden.wav".into(),
        });
        play(&SoundChoice::File {
            path: r"C:\datei.mp3".into(),
        });
    }
}
