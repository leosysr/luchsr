//! Zeitplanung des Abrufs: Jitter, Backoff, Standby-Erkennung.
//!
//! Alles hier sind **reine Funktionen**. Der Zufallswert und die gemessenen
//! Zeiten kommen als Parameter herein, nicht aus `rand` oder der Uhr. Sonst
//! liesse sich weder das Backoff noch die Standby-Erkennung prüfen, ohne
//! tatsächlich fünf Minuten zu warten oder den Rechner schlafen zu legen.

use std::time::Duration;

/// Obergrenze des Backoffs laut Auftrag.
pub const BACKOFF_CAP: Duration = Duration::from_secs(300);

/// Streuung des Abrufintervalls, ±10 %.
pub const JITTER_FRACTION: f64 = 0.10;

/// Untergrenze, unter die keine Wartezeit fällt. Schützt gegen eine
/// Fehlkonfiguration, die den Server mit Anfragen überziehen würde.
pub const MIN_DELAY: Duration = Duration::from_secs(5);

/// Berechnet die Wartezeit bis zum nächsten Abruf.
///
/// `failures` ist die Anzahl der unmittelbar vorangegangenen Fehlversuche;
/// `0` heisst „der letzte Abruf war erfolgreich".
///
/// `random` muss in `[0, 1)` liegen. Der Wert wird als Parameter übergeben,
/// damit die Funktion rein bleibt — der Aufrufer zieht ihn aus `rand`.
///
/// ## Warum der Deckel nicht einfach 5 Minuten ist
///
/// Der Auftrag sagt „exponentielles Backoff bis maximal 5 Minuten". Bei einem
/// Basisintervall von 600 s wäre ein Deckel von 300 s aber eine *Verkürzung* —
/// nach einem Fehler würde häufiger abgefragt als im Normalbetrieb. Deshalb
/// greift der Deckel nie unter das Basisintervall.
pub fn next_delay(base: Duration, failures: u32, random: f64) -> Duration {
    let cap = BACKOFF_CAP.max(base);

    let raw = if failures == 0 {
        base
    } else {
        // 2^failures, gegen Überlauf begrenzt. Ab 2^20 ist der Deckel längst
        // erreicht, weiteres Verdoppeln wäre nur ein Weg in den Panic.
        let factor = 2u64.saturating_pow(failures.min(20));
        Duration::from_secs(base.as_secs().saturating_mul(factor)).min(cap)
    };

    apply_jitter(raw, random).max(MIN_DELAY)
}

/// Legt ±[`JITTER_FRACTION`] auf eine Dauer.
///
/// Der Auftrag begründet das selbst: damit nicht alle Clients synchron feuern.
/// Ohne Jitter würden nach einem Serverausfall alle Arbeitsplätze im
/// Sekundentakt gemeinsam wieder anklopfen.
fn apply_jitter(value: Duration, random: f64) -> Duration {
    // NaN klemmt nicht: `clamp` gibt NaN zurück, weil jeder Vergleich mit NaN
    // falsch ist. Ohne diese Zeile fällt die Wartezeit über einen NaN-Faktor
    // still auf MIN_DELAY — der Server würde alle fünf Sekunden angeklopft.
    // Der Aufrufer kann kein NaN liefern, aber ein stiller Kollaps auf die
    // kürzeste Wartezeit ist der falsche Umgang mit einem unmöglichen Wert.
    let clamped = if random.is_nan() {
        0.5
    } else {
        random.clamp(0.0, 1.0)
    };
    // random = 0 -> 0.9, random = 1 -> 1.1
    let factor = 1.0 - JITTER_FRACTION + clamped * 2.0 * JITTER_FRACTION;
    let millis = (value.as_millis() as f64 * factor).round();
    Duration::from_millis(millis.max(0.0) as u64)
}

/// Ob zwischen zwei Abrufen offenbar geschlafen wurde.
///
/// `expected` ist die geplante Wartezeit, `actual` die tatsächlich verstrichene
/// Wanduhrzeit. Ein Ausreisser nach oben heisst: der Prozess war eingefroren.
///
/// ## Warum Wanduhrzeit und keine Power-Ereignisse
///
/// Windows sendet `WM_POWERBROADCAST`, aber das abzufangen heisst, in die
/// Nachrichtenschleife des Fensters zu greifen. Die Wanduhrzeit zu vergleichen
/// kostet nichts, braucht keine Windows-API und erkennt zusätzlich Fälle, in
/// denen kein Power-Ereignis kommt — eine angehaltene virtuelle Maschine etwa,
/// oder ein Prozess, der lange keine Rechenzeit bekam.
///
/// Die Schwelle ist bewusst relativ **und** absolut: `expected + max(60s,
/// expected)`. Bei 15 s Intervall wäre eine feste Minute Toleranz zu grob, bei
/// 600 s eine relative Verdopplung zu grosszügig.
pub fn looks_like_wakeup(expected: Duration, actual: Duration) -> bool {
    let tolerance = expected.max(Duration::from_secs(60));
    actual > expected + tolerance
}

/// Zustand der Abrufschleife.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    /// Aufeinanderfolgende Fehlversuche. Wird bei Erfolg auf 0 gesetzt.
    pub failures: u32,
    /// Ob der letzte Fehler durch Wiederholen überhaupt behebbar wäre.
    pub last_retryable: bool,
}

impl Default for Schedule {
    fn default() -> Self {
        Self {
            failures: 0,
            last_retryable: true,
        }
    }
}

impl Schedule {
    pub fn success(&mut self) {
        self.failures = 0;
        self.last_retryable = true;
    }

    /// Zählt einen Fehlversuch. Sättigt statt überzulaufen.
    pub fn failure(&mut self, retryable: bool) {
        self.failures = self.failures.saturating_add(1);
        self.last_retryable = retryable;
    }

    /// Wartezeit bis zum nächsten Versuch.
    ///
    /// ## Warum ein nicht behebbarer Fehler *kein* Backoff bekommt
    ///
    /// Ein falsches Automation-Secret oder ein falscher Site-Name löst sich
    /// nicht dadurch, dass Luchsr länger wartet — aber der Benutzer sitzt
    /// womöglich gerade im Einstellungsdialog und korrigiert es. Dann soll die
    /// nächste Prüfung in einer Minute kommen und nicht in fünf.
    ///
    /// Umgekehrt bei einem Netzaussetzer oder einem 503: da hilft Warten, und
    /// ein Backoff verhindert, dass viele Arbeitsplätze einen ohnehin
    /// angeschlagenen Server weiter belasten.
    pub fn delay(&self, base: Duration, random: f64) -> Duration {
        let steps = if self.last_retryable {
            self.failures
        } else {
            0
        };
        next_delay(base, steps, random)
    }

    /// Ob gerade ein Fehlerzustand vorliegt — steuert das Tray-Icon.
    pub fn is_failing(&self) -> bool {
        self.failures > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SEKUNDE: Duration = Duration::from_secs(1);

    fn secs(n: u64) -> Duration {
        Duration::from_secs(n)
    }

    /* ------------------------------------------------------------- Jitter */

    /// Ohne Fehler ist die Wartezeit das Basisintervall, gestreut um ±10 %.
    #[test]
    fn erfolg_ergibt_basisintervall_mit_jitter() {
        let base = secs(60);
        assert_eq!(next_delay(base, 0, 0.0), secs(54), "Untergrenze = 90 %");
        assert_eq!(next_delay(base, 0, 0.5), secs(60), "Mitte = 100 %");
        assert_eq!(next_delay(base, 0, 1.0), secs(66), "Obergrenze = 110 %");
    }

    /// Über den ganzen Zufallsbereich darf die Streuung 10 % nicht verlassen.
    #[test]
    fn jitter_bleibt_in_den_grenzen() {
        for base in [secs(15), secs(60), secs(300), secs(600)] {
            for step in 0..=100 {
                let random = f64::from(step) / 100.0;
                let delay = next_delay(base, 0, random);
                let untergrenze = base.mul_f64(0.9) - SEKUNDE;
                let obergrenze = base.mul_f64(1.1) + SEKUNDE;
                assert!(
                    delay >= untergrenze && delay <= obergrenze,
                    "base {base:?}, random {random}: {delay:?} liegt aussserhalb \
                     von {untergrenze:?}..{obergrenze:?}"
                );
            }
        }
    }

    /// Ein Zufallswert aus dem erlaubten Bereich heraus darf nicht durchschlagen.
    #[test]
    fn zufallswert_wird_geklemmt() {
        let base = secs(60);
        assert_eq!(next_delay(base, 0, -5.0), next_delay(base, 0, 0.0));
        assert_eq!(next_delay(base, 0, 99.0), next_delay(base, 0, 1.0));
    }

    /// NaN darf nicht auf die Untergrenze durchfallen.
    ///
    /// `f64::clamp` gibt für NaN NaN zurück — jeder Vergleich mit NaN ist
    /// falsch. Ohne Sonderbehandlung ergäbe das eine Wartezeit von MIN_DELAY,
    /// also alle fünf Sekunden eine Anfrage an den Server.
    #[test]
    fn nan_ergibt_das_basisintervall_nicht_die_untergrenze() {
        let base = secs(60);
        assert_eq!(
            next_delay(base, 0, f64::NAN),
            secs(60),
            "NaN muss als neutrale Mitte behandelt werden"
        );
        assert_ne!(next_delay(base, 0, f64::NAN), MIN_DELAY);
        // Auch mit Backoff.
        assert_eq!(next_delay(base, 2, f64::NAN), secs(240));
    }

    /* ------------------------------------------------------------ Backoff */

    #[test]
    fn backoff_verdoppelt_und_deckelt_bei_fuenf_minuten() {
        let base = secs(60);
        // Jitter in der Mitte, damit die Verdopplung sichtbar bleibt.
        let mitte = |failures| next_delay(base, failures, 0.5);

        assert_eq!(mitte(0), secs(60));
        assert_eq!(mitte(1), secs(120));
        assert_eq!(mitte(2), secs(240));
        assert_eq!(mitte(3), secs(300), "480 s wird auf den Deckel gekürzt");
        assert_eq!(mitte(4), secs(300));
        assert_eq!(mitte(50), secs(300), "bleibt gedeckelt");
    }

    /// Der Deckel darf das Intervall nicht *verkürzen*. Bei 600 s Basis wäre
    /// ein Backoff von 300 s häufiger als der Normalbetrieb.
    #[test]
    fn deckel_verkuerzt_ein_langes_intervall_nicht() {
        let base = secs(600);
        let delay = next_delay(base, 3, 0.5);
        assert_eq!(delay, secs(600));
        assert!(
            delay >= base.mul_f64(0.9),
            "Backoff darf nie unter das Basisintervall fallen"
        );
    }

    /// Bei einem Intervall genau am Deckel ändert das Backoff nichts.
    #[test]
    fn intervall_am_deckel_bleibt_stabil() {
        let base = BACKOFF_CAP;
        for failures in 0..10 {
            assert_eq!(next_delay(base, failures, 0.5), BACKOFF_CAP);
        }
    }

    /// Sehr viele Fehlversuche dürfen nicht überlaufen.
    #[test]
    fn viele_fehlversuche_laufen_nicht_ueber() {
        for failures in [20u32, 64, 1000, u32::MAX] {
            let delay = next_delay(secs(600), failures, 1.0);
            assert!(
                delay <= secs(600).mul_f64(1.1) + SEKUNDE,
                "{failures}: {delay:?}"
            );
        }
    }

    /// Ein absurd kleines Intervall darf den Server nicht überziehen.
    #[test]
    fn untergrenze_greift() {
        assert_eq!(next_delay(Duration::from_millis(1), 0, 0.0), MIN_DELAY);
        assert_eq!(next_delay(Duration::ZERO, 0, 0.5), MIN_DELAY);
    }

    /* -------------------------------------------------- Standby-Erkennung */

    #[test]
    fn normale_abweichung_ist_kein_aufwachen() {
        // 60 s geplant, 61 s vergangen — Scheduler-Ungenauigkeit.
        assert!(!looks_like_wakeup(secs(60), secs(61)));
        // Auch eine halbe Minute Verzug ist noch kein Standby.
        assert!(!looks_like_wakeup(secs(60), secs(90)));
        assert!(!looks_like_wakeup(secs(60), secs(119)));
    }

    #[test]
    fn langer_ausreisser_ist_ein_aufwachen() {
        // Über Nacht geschlafen.
        assert!(looks_like_wakeup(secs(60), secs(8 * 3600)));
        assert!(looks_like_wakeup(secs(60), secs(121)));
    }

    /// Bei kurzem Intervall wäre eine feste Minute Toleranz zu grob, bei
    /// langem eine relative Verdopplung zu grosszügig — die Schwelle ist beides.
    #[test]
    fn schwelle_passt_sich_dem_intervall_an() {
        // 15 s Intervall: absolute Toleranz von 60 s greift.
        assert!(!looks_like_wakeup(secs(15), secs(74)));
        assert!(looks_like_wakeup(secs(15), secs(76)));

        // 600 s Intervall: relative Toleranz greift.
        assert!(!looks_like_wakeup(secs(600), secs(1199)));
        assert!(looks_like_wakeup(secs(600), secs(1201)));
    }

    #[test]
    fn kuerzer_als_geplant_ist_kein_aufwachen() {
        assert!(!looks_like_wakeup(secs(60), secs(1)));
        assert!(!looks_like_wakeup(secs(60), Duration::ZERO));
    }

    /* ------------------------------------------------------------ Zustand */

    #[test]
    fn zustand_zaehlt_fehler_und_setzt_bei_erfolg_zurueck() {
        let mut schedule = Schedule::default();
        assert!(!schedule.is_failing());

        schedule.failure(true);
        schedule.failure(true);
        assert_eq!(schedule.failures, 2);
        assert!(schedule.is_failing());
        assert_eq!(schedule.delay(secs(60), 0.5), secs(240));

        schedule.success();
        assert_eq!(schedule.failures, 0);
        assert!(!schedule.is_failing());
        assert_eq!(schedule.delay(secs(60), 0.5), secs(60));
    }

    /// Ein falsches Secret löst sich nicht durch Warten — aber der Benutzer
    /// korrigiert es womöglich gerade. Dann soll die nächste Prüfung in einer
    /// Minute kommen, nicht in fünf.
    #[test]
    fn nicht_behebbarer_fehler_bekommt_kein_backoff() {
        let mut schedule = Schedule::default();
        for _ in 0..10 {
            schedule.failure(false);
        }
        assert_eq!(schedule.failures, 10);
        assert!(schedule.is_failing(), "das Tray-Icon muss trotzdem warnen");
        assert_eq!(
            schedule.delay(secs(60), 0.5),
            secs(60),
            "kein Backoff bei einem Fehler, den Warten nicht behebt"
        );
    }

    /// Ein Netzaussetzer dagegen bekommt Backoff — sonst belasten viele
    /// Arbeitsplätze einen ohnehin angeschlagenen Server weiter.
    #[test]
    fn behebbarer_fehler_bekommt_backoff() {
        let mut schedule = Schedule::default();
        schedule.failure(true);
        assert_eq!(schedule.delay(secs(60), 0.5), secs(120));
        schedule.failure(true);
        assert_eq!(schedule.delay(secs(60), 0.5), secs(240));
    }

    /// Wechselt die Fehlerart, gilt die des letzten Versuchs.
    #[test]
    fn die_art_des_letzten_fehlers_entscheidet() {
        let mut schedule = Schedule::default();
        schedule.failure(true);
        schedule.failure(true);
        schedule.failure(true);
        assert_eq!(schedule.delay(secs(60), 0.5), secs(300), "gedeckelt");

        // Jetzt ein 401 — ab hier wieder im Normalintervall prüfen.
        schedule.failure(false);
        assert_eq!(schedule.delay(secs(60), 0.5), secs(60));

        // Und wieder ein Netzfehler: das Backoff greift mit dem alten Zähler.
        schedule.failure(true);
        assert_eq!(schedule.delay(secs(60), 0.5), secs(300));
    }

    #[test]
    fn fehlerzaehler_saettigt() {
        let mut schedule = Schedule {
            failures: u32::MAX,
            last_retryable: true,
        };
        schedule.failure(true);
        assert_eq!(schedule.failures, u32::MAX, "kein Überlauf");
    }

    #[test]
    fn erfolg_setzt_auch_die_fehlerart_zurueck() {
        let mut schedule = Schedule::default();
        schedule.failure(false);
        assert!(!schedule.last_retryable);
        schedule.success();
        assert!(schedule.last_retryable);
    }
}
