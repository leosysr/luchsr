//! Nachsehen, ob es eine neuere Fassung gibt.
//!
//! ## Nur nachsehen, nicht installieren
//!
//! Luchsr kann sich nicht selbst aktualisieren, und das ist keine Bequemlichkeit,
//! sondern eine Folge der Installationsart: sie läuft **per Machine** nach
//! `%ProgramFiles%` (D77) und braucht damit einen erhöhten Kontext. Ein Updater
//! würde entweder eine UAC-Abfrage aus dem Nichts auslösen oder bei einem
//! Benutzer ohne Administratorrechte still scheitern — beides schlechter als
//! ein Hinweis, der sagt, dass es etwas Neues gibt. Ausserdem stünde er in
//! Konkurrenz zum Softwaremanagement, über das die Verteilung läuft: rollte das
//! 1.2.0 aus, während sich Luchsr selbst 1.3.0 zieht, wüsste niemand mehr,
//! welcher Stand wo läuft.
//!
//! ## Nur auf Klick
//!
//! Kein Hintergrundverkehr. Die GitHub-API erlaubt unangemeldet **60 Anfragen
//! je Stunde und IP**, und hinter einem Firmen-NAT teilen sich das alle
//! Rechner: ein automatischer Check bei jeder Anmeldung wäre in einem
//! grösseren Haus schon nach wenigen Minuten am Limit. Wer fragt, bekommt eine
//! Antwort; wer nicht fragt, erzeugt keinen Verkehr.
//!
//! ## Ein eigener HTTP-Client, absichtlich
//!
//! Nicht der aus [`crate::checkmk`], und zwar aus drei Gründen:
//!
//! 1. Der trägt das Automation-Secret im `Authorization`-Header. An GitHub
//!    gehört es unter keinen Umständen.
//! 2. Die Einstellung „TLS-Prüfung aus" gilt der internen CA und **darf nicht
//!    auf eine Anfrage ins öffentliche Netz durchschlagen**. Hier wird immer
//!    geprüft, ohne Schalter.
//! 3. Die Proxy-Einstellung gilt dem internen Server. Für einen Host im
//!    Internet ist der Systemproxy die richtige Quelle — genau umgekehrt zu
//!    D34, wo der Systemproxy für den internen Server das Problem war.
//!
//! Weiterleitungen dürfen hier gefolgt werden, anders als in D10: es geht kein
//! Geheimnis mit, und die GitHub-API leitet auf `api.github.com`-Adressen um.

use std::time::Duration;

use serde::{Deserialize, Serialize};

/// Das Repository, aus dem Luchsr kommt. Die **eine** Stelle mit dieser Angabe.
pub const REPO: &str = "leosysr/luchsr";

/// Zeitgrenze der Anfrage.
///
/// Kürzer als die zehn Sekunden für CheckMK: dort wartet eine Abrufschleife,
/// hier ein Mensch, der gerade auf einen Knopf gedrückt hat.
const TIMEOUT: Duration = Duration::from_secs(8);

/// Die Projektseite im Browser.
pub fn project_url() -> String {
    format!("https://github.com/{REPO}")
}

/// Der Endpunkt für das jüngste Release.
pub fn latest_release_url() -> String {
    format!("https://api.github.com/repos/{REPO}/releases/latest")
}

/* -------------------------------------------------------------------------- */
/* Version                                                                    */
/* -------------------------------------------------------------------------- */

/// Eine Version in drei Zahlen.
///
/// Eigene Zerlegung statt einer Semver-Crate: verglichen werden ausschliesslich
/// Versionen dieses Projekts, und die haben die Form `x.y.z`. Eine Abhängigkeit
/// für drei Zahlen wäre der falsche Tausch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Version {
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}

impl Version {
    /// Zerlegt `1.2.3`, `v1.2.3` und `1.2.3-beta.1`.
    ///
    /// Das `v` fällt weg, weil der Git-Tag es trägt und `tauri.conf.json` nicht.
    /// Ein Zusatz hinter der dritten Zahl wird **abgeschnitten und ignoriert**:
    /// eine Vorabfassung zu behandeln wäre Code für einen Fall, den dieses
    /// Projekt nicht kennt — und stillschweigend falsch zu vergleichen wäre
    /// schlimmer als sie zu ignorieren.
    pub fn parse(raw: &str) -> Option<Self> {
        let ohne_v = raw.trim().trim_start_matches(['v', 'V']);
        // Alles ab dem ersten Zeichen, das nicht Ziffer oder Punkt ist.
        let kern: &str = ohne_v
            .split(|c: char| !c.is_ascii_digit() && c != '.')
            .next()
            .unwrap_or("");

        let mut teile = kern.split('.');
        let major = teile.next()?.parse().ok()?;
        let minor = teile.next()?.parse().ok()?;
        let patch = teile.next()?.parse().ok()?;
        // Eine vierte Zahl wäre keine Version dieses Projekts.
        if teile.next().is_some() {
            return None;
        }
        Some(Self {
            major,
            minor,
            patch,
        })
    }
}

impl std::fmt::Display for Version {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)
    }
}

/// Das Ergebnis des Vergleichs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Verdict {
    /// Es gibt eine neuere Fassung.
    UpdateAvailable,
    /// Die laufende Fassung ist die jüngste.
    UpToDate,
    /// Die laufende Fassung ist **neuer** als das jüngste Release.
    ///
    /// Kommt bei einem Entwicklungsbau vor. „Aktuell" zu melden wäre bequem und
    /// falsch — der Unterschied ist gerade der interessante.
    Ahead,
}

/// Vergleicht die laufende Fassung mit der veröffentlichten.
pub fn compare(current: Version, latest: Version) -> Verdict {
    match latest.cmp(&current) {
        std::cmp::Ordering::Greater => Verdict::UpdateAvailable,
        std::cmp::Ordering::Equal => Verdict::UpToDate,
        std::cmp::Ordering::Less => Verdict::Ahead,
    }
}

/* -------------------------------------------------------------------------- */
/* Antwort von GitHub                                                         */
/* -------------------------------------------------------------------------- */

/// Die Felder der GitHub-Antwort, die gebraucht werden.
///
/// Bewusst wenige: jedes zusätzliche Feld ist eine Annahme über eine fremde
/// Schnittstelle, die sich ändern kann.
#[derive(Debug, Clone, Deserialize)]
struct GithubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    draft: bool,
    #[serde(default)]
    prerelease: bool,
}

/// Was der Befehl zurückgibt.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateReport {
    pub verdict: Verdict,
    /// Die laufende Fassung, wie sie im Fenster steht.
    pub current: String,
    /// Die veröffentlichte Fassung.
    pub latest: String,
    /// Die Seite des Releases — dort liegen MSI und Prüfsummen.
    pub release_url: String,
}

/// Wertet eine Antwort aus, ohne HTTP.
///
/// Rein, damit die Auswertung gegen aufgezeichnete Antworten geprüft werden
/// kann — dieselbe Aufteilung wie im `checkmk`-Modul.
pub fn evaluate(current_raw: &str, body: &str) -> Result<UpdateReport, UpdateError> {
    let current = Version::parse(current_raw).ok_or_else(|| UpdateError::OwnVersion {
        raw: current_raw.to_owned(),
    })?;

    let release: GithubRelease = serde_json::from_str(body).map_err(|error| UpdateError::Body {
        reason: error.to_string(),
        excerpt: excerpt(body),
    })?;

    // `releases/latest` liefert laut Dokumentation weder Entwürfe noch
    // Vorabfassungen. Geprüft wird es trotzdem: verlässt man sich darauf und es
    // ändert sich, empfiehlt Luchsr einen Stand, den niemand veröffentlicht hat.
    if release.draft || release.prerelease {
        return Err(UpdateError::NoRelease);
    }

    let latest = Version::parse(&release.tag_name).ok_or_else(|| UpdateError::TagName {
        raw: release.tag_name.clone(),
    })?;

    Ok(UpdateReport {
        verdict: compare(current, latest),
        current: current.to_string(),
        latest: latest.to_string(),
        release_url: release.html_url,
    })
}

/// Ein Auszug aus einem Rumpf, der kein JSON war.
///
/// Dieselbe Begründung wie D33: ein Rumpf, der kein JSON ist, kommt in der
/// Regel **nicht** von GitHub, sondern von einem Proxy davor — und
/// `<title>403 Forbidden</title>` beantwortet die Frage „wer sagt hier nein"
/// sofort.
fn excerpt(body: &str) -> String {
    let eine_zeile: String = body.split_whitespace().collect::<Vec<_>>().join(" ");
    if eine_zeile.chars().count() <= 200 {
        return eine_zeile;
    }
    eine_zeile.chars().take(200).collect::<String>() + " …"
}

/* -------------------------------------------------------------------------- */
/* Fehler                                                                     */
/* -------------------------------------------------------------------------- */

/// Was beim Nachsehen schiefgehen kann.
///
/// Jede Variante nennt eine **Ursache**, nicht „Fehler beim Update-Check".
#[derive(Debug, Clone)]
pub enum UpdateError {
    /// GitHub war nicht erreichbar.
    Unreachable { detail: String },
    /// Das Anfragelimit ist erreicht.
    RateLimited,
    /// Es gibt (noch) kein veröffentlichtes Release.
    NoRelease,
    /// Ein anderer HTTP-Status.
    Status { code: u16, excerpt: String },
    /// Der Rumpf war nicht auswertbar.
    Body { reason: String, excerpt: String },
    /// Der Tag des Releases hatte nicht die Form `x.y.z`.
    TagName { raw: String },
    /// Die eigene Version war nicht lesbar. Das wäre ein Fehler in Luchsr.
    OwnVersion { raw: String },
}

impl std::fmt::Display for UpdateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable { detail } => write!(
                f,
                "GitHub ist nicht erreichbar ({detail}). Im Firmennetz führt der Weg \
                 nach aussen womöglich über einen Proxy, der hier nicht gesetzt ist."
            ),
            Self::RateLimited => write!(
                f,
                "Das Anfragelimit von GitHub ist erreicht (60 pro Stunde und IP-Adresse, \
                 geteilt mit allen Rechnern hinter derselben Adresse). Später erneut versuchen."
            ),
            Self::NoRelease => write!(f, "Für {REPO} ist noch kein Release veröffentlicht."),
            // Der Fremdtext steht in Anführungszeichen und der Satz endet
            // dahinter. Ohne die Klammer läuft die Meldung ungetrennt in einen
            // fremden Text hinein, und man sieht nicht, wo Luchsr aufhört zu
            // sprechen — bei einer HTML-Seite als Rumpf ist das genau die
            // Stelle, auf die es ankommt.
            Self::Status { code, excerpt } if excerpt.is_empty() => {
                write!(f, "GitHub antwortete mit HTTP {code}.")
            }
            Self::Status { code, excerpt } => {
                write!(f, "GitHub antwortete mit HTTP {code}: „{excerpt}“.")
            }
            Self::Body { reason, excerpt } => write!(
                f,
                "Die Antwort war nicht auswertbar ({reason}). Anfang der Antwort: „{excerpt}“."
            ),
            Self::TagName { raw } => write!(
                f,
                "Der Tag „{raw}“ des jüngsten Releases hat nicht die Form x.y.z."
            ),
            Self::OwnVersion { raw } => write!(
                f,
                "Die eigene Version „{raw}“ ist nicht lesbar. Das ist ein Fehler in Luchsr."
            ),
        }
    }
}

/* -------------------------------------------------------------------------- */
/* HTTP                                                                       */
/* -------------------------------------------------------------------------- */

/// Fragt GitHub nach dem jüngsten Release.
///
/// Die einzige unreine Funktion des Moduls; alles Auswertende steckt in
/// [`evaluate`].
pub async fn check(current_raw: &str) -> Result<UpdateReport, UpdateError> {
    let client = reqwest::Client::builder()
        .timeout(TIMEOUT)
        .connect_timeout(TIMEOUT.min(Duration::from_secs(4)))
        // GitHub lehnt Anfragen ohne User-Agent mit 403 ab. Die Version kommt
        // von aussen: `CARGO_PKG_VERSION` ist in diesem Projekt der
        // Platzhalter 0.0.0, die verbindliche Version steht in
        // tauri.conf.json.
        .user_agent(format!("Luchsr/{current_raw} (+https://github.com/{REPO})"))
        .build()
        .map_err(|error| UpdateError::Unreachable {
            detail: error.to_string(),
        })?;

    let antwort = client
        .get(latest_release_url())
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await
        .map_err(|error| UpdateError::Unreachable {
            detail: error.to_string(),
        })?;

    let status = antwort.status();

    // Das Limit erkennt man am Zähler, nicht am Statuscode: 403 kommt auch von
    // einem Proxy davor, und dann wäre „später erneut versuchen" ein falscher
    // Rat, der jemanden eine Stunde warten lässt.
    let limit_erschoepft = antwort
        .headers()
        .get("x-ratelimit-remaining")
        .and_then(|wert| wert.to_str().ok())
        .map(|wert| wert.trim() == "0")
        .unwrap_or(false);

    let body = antwort.text().await.unwrap_or_default();

    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(UpdateError::NoRelease);
    }
    if !status.is_success() {
        if limit_erschoepft {
            return Err(UpdateError::RateLimited);
        }
        return Err(UpdateError::Status {
            code: status.as_u16(),
            excerpt: excerpt(&body),
        });
    }

    evaluate(current_raw, &body)
}

#[cfg(test)]
mod tests {
    use super::*;

    /* ------------------------------------------------------- Version ---- */

    #[test]
    fn liest_die_uebliche_form() {
        assert_eq!(
            Version::parse("1.2.3"),
            Some(Version {
                major: 1,
                minor: 2,
                patch: 3
            })
        );
    }

    #[test]
    fn das_v_des_tags_faellt_weg() {
        assert_eq!(Version::parse("v1.2.3"), Version::parse("1.2.3"));
    }

    #[test]
    fn ein_zusatz_wird_abgeschnitten() {
        assert_eq!(Version::parse("1.2.3-beta.1"), Version::parse("1.2.3"));
        assert_eq!(Version::parse("1.2.3+build7"), Version::parse("1.2.3"));
    }

    #[test]
    fn unfug_ergibt_nichts() {
        for raw in ["", "v", "1.2", "1", "abc", "1.2.3.4", "1.2.x", "..", "1..3"] {
            assert!(Version::parse(raw).is_none(), "„{raw}“ wurde gelesen");
        }
    }

    /// Der Fall, an dem ein Textvergleich scheitert.
    #[test]
    fn zehn_ist_groesser_als_neun() {
        let neun = Version::parse("1.9.0").expect("1.9.0");
        let zehn = Version::parse("1.10.0").expect("1.10.0");
        assert!(zehn > neun, "1.10.0 muss neuer sein als 1.9.0");
        assert_eq!(compare(neun, zehn), Verdict::UpdateAvailable);
    }

    #[test]
    fn gleiche_version_ist_aktuell() {
        let v = Version::parse("1.2.0").expect("1.2.0");
        assert_eq!(compare(v, v), Verdict::UpToDate);
    }

    /// Ein Entwicklungsbau ist nicht „aktuell", er ist voraus. Das zu
    /// verschweigen wäre bequem und falsch.
    #[test]
    fn ein_entwicklungsbau_ist_voraus() {
        let laufend = Version::parse("1.3.0").expect("1.3.0");
        let veroeffentlicht = Version::parse("1.2.0").expect("1.2.0");
        assert_eq!(compare(laufend, veroeffentlicht), Verdict::Ahead);
    }

    /* ---------------------------------------------------- Auswertung ---- */

    fn antwort(tag: &str) -> String {
        format!(
            r#"{{"tag_name":"{tag}","html_url":"https://github.com/leosysr/luchsr/releases/tag/{tag}","draft":false,"prerelease":false}}"#
        )
    }

    #[test]
    fn eine_neuere_fassung_wird_gemeldet() {
        let bericht = evaluate("1.2.0", &antwort("v1.3.0")).expect("auswertbar");
        assert_eq!(bericht.verdict, Verdict::UpdateAvailable);
        assert_eq!(bericht.current, "1.2.0");
        assert_eq!(bericht.latest, "1.3.0");
        assert!(bericht.release_url.contains("releases/tag/v1.3.0"));
    }

    #[test]
    fn ein_entwurf_gilt_nicht_als_release() {
        let entwurf = r#"{"tag_name":"v9.9.9","html_url":"x","draft":true,"prerelease":false}"#;
        assert!(matches!(
            evaluate("1.2.0", entwurf),
            Err(UpdateError::NoRelease)
        ));
    }

    #[test]
    fn eine_vorabfassung_gilt_nicht_als_release() {
        let vorab = r#"{"tag_name":"v9.9.9","html_url":"x","draft":false,"prerelease":true}"#;
        assert!(matches!(
            evaluate("1.2.0", vorab),
            Err(UpdateError::NoRelease)
        ));
    }

    #[test]
    fn ein_unbrauchbarer_tag_wird_benannt() {
        let seltsam = r#"{"tag_name":"release-herbst","html_url":"x"}"#;
        match evaluate("1.2.0", seltsam) {
            Err(UpdateError::TagName { raw }) => assert_eq!(raw, "release-herbst"),
            other => panic!("erwartet TagName, war {other:?}"),
        }
    }

    /// Eine HTML-Seite statt JSON heisst: die Antwort kam nicht von GitHub.
    /// Der Auszug muss das zeigen, sonst rät man (D33).
    #[test]
    fn kein_json_ergibt_einen_auszug() {
        let html = "<html><head><title>403 Forbidden</title></head><body>Proxy</body></html>";
        match evaluate("1.2.0", html) {
            Err(UpdateError::Body { excerpt, .. }) => {
                assert!(excerpt.contains("403 Forbidden"), "Auszug: {excerpt}")
            }
            other => panic!("erwartet Body, war {other:?}"),
        }
    }

    #[test]
    fn ein_langer_rumpf_wird_gekuerzt() {
        let lang = "x".repeat(5000);
        let gekuerzt = excerpt(&lang);
        assert!(gekuerzt.chars().count() <= 202, "{}", gekuerzt.len());
        assert!(gekuerzt.ends_with('…'));
    }

    #[test]
    fn eine_unlesbare_eigene_version_wird_benannt() {
        match evaluate("kaputt", &antwort("v1.3.0")) {
            Err(UpdateError::OwnVersion { raw }) => assert_eq!(raw, "kaputt"),
            other => panic!("erwartet OwnVersion, war {other:?}"),
        }
    }

    /* ------------------------------------------------------- Drahtform --- */

    /// Die Namen, auf die das Frontend vergleicht.
    ///
    /// Steht in `src/lib/types.ts` als `"updateAvailable" | "upToDate" |
    /// "ahead"` und in `Colophon.tsx` als Vergleich. Weicht die
    /// Serialisierung davon ab, greift kein Vergleich mehr — und weil
    /// TypeScript den Rust-Typ nicht kennt, fällt es nicht beim Übersetzen auf,
    /// sondern als falscher Satz im Fenster. Deshalb hier festgenagelt.
    #[test]
    fn die_urteile_serialisieren_wie_das_frontend_erwartet() {
        let paare = [
            (Verdict::UpdateAvailable, "\"updateAvailable\""),
            (Verdict::UpToDate, "\"upToDate\""),
            (Verdict::Ahead, "\"ahead\""),
        ];
        for (urteil, erwartet) in paare {
            assert_eq!(
                serde_json::to_string(&urteil).expect("serialisierbar"),
                erwartet
            );
        }
    }

    /// Ebenso die Feldnamen des Berichts.
    #[test]
    fn der_bericht_traegt_camelcase_feldnamen() {
        let bericht = evaluate("1.2.0", &antwort("v1.3.0")).expect("auswertbar");
        let json = serde_json::to_string(&bericht).expect("serialisierbar");
        for feld in ["\"verdict\"", "\"current\"", "\"latest\"", "\"releaseUrl\""] {
            assert!(json.contains(feld), "{feld} fehlt in {json}");
        }
        assert!(
            !json.contains("release_url"),
            "Schlangenschrift im Drahtformat: {json}"
        );
    }

    /* --------------------------------------------------------- Adressen -- */

    #[test]
    fn die_adressen_kommen_aus_einer_quelle() {
        assert_eq!(project_url(), "https://github.com/leosysr/luchsr");
        assert_eq!(
            latest_release_url(),
            "https://api.github.com/repos/leosysr/luchsr/releases/latest"
        );
    }

    /* ------------------------------------------------ gegen die echte API -- */

    /// Fragt wirklich bei GitHub nach.
    ///
    /// Absichtlich `#[ignore]`: ein Test, der bei jedem Durchlauf ins Internet
    /// greift, ist kein Test der eigenen Logik — er scheitert an einem Proxy,
    /// an einem Anfragelimit oder an einer Wartung bei GitHub, und dann steht
    /// eine rote Zeile, die nichts über dieses Projekt aussagt. Die Auswertung
    /// prüfen die Tests darüber gegen aufgezeichnete Antworten.
    ///
    /// Wozu er dann gut ist: er belegt, dass die **Anfrage** stimmt — User-Agent
    /// gesetzt (ohne lehnt GitHub mit 403 ab), Header richtig, Feldnamen wie
    /// erwartet. Das kann keine Aufzeichnung beweisen.
    ///
    /// ```text
    /// cargo test --manifest-path src-tauri/Cargo.toml --lib -- --ignored update_gegen_github --nocapture
    /// ```
    #[test]
    #[ignore = "greift ins Internet"]
    fn update_gegen_github() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Laufzeit");

        match rt.block_on(check("1.0.0")) {
            Ok(bericht) => {
                println!("Urteil:      {:?}", bericht.verdict);
                println!("laufend:     {}", bericht.current);
                println!("veröffentl.: {}", bericht.latest);
                println!("Release:     {}", bericht.release_url);
                assert_eq!(bericht.verdict, Verdict::UpdateAvailable, "1.0.0 ist alt");
                assert!(bericht
                    .release_url
                    .starts_with(&format!("https://github.com/{REPO}/releases/")));
            }
            // Kein `panic!`: ein Proxy oder ein Anfragelimit ist keine Aussage
            // über den Quelltext. Die Meldung soll man aber lesen können.
            Err(fehler) => println!("nicht erreichbar — {fehler}"),
        }
    }

    /// Die Fehlermeldungen müssen eine Ursache nennen, nicht „Fehler".
    #[test]
    fn jede_fehlermeldung_sagt_etwas() {
        let faelle = [
            UpdateError::Unreachable {
                detail: "dns error".into(),
            },
            UpdateError::RateLimited,
            UpdateError::NoRelease,
            UpdateError::Status {
                code: 500,
                excerpt: String::new(),
            },
            UpdateError::Body {
                reason: "expected value".into(),
                excerpt: "<html>".into(),
            },
            UpdateError::TagName { raw: "x".into() },
            UpdateError::OwnVersion { raw: "y".into() },
        ];
        for fall in faelle {
            let text = fall.to_string();
            assert!(text.len() > 20, "zu knapp: {text}");
            assert!(text.ends_with('.'), "kein Satz: {text}");
        }
    }
}
