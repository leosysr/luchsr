//! HTTP-Client gegen die CheckMK-REST-API.
//!
//! ## Aufteilung
//!
//! Alles, was ohne Netzwerk prüfbar ist, steckt in reinen Funktionen:
//! [`parse_services`], [`parse_hosts`], [`parse_version`] und
//! [`error_from_response`]. Die Tests am Ende der Datei arbeiten gegen die
//! aufgezeichneten JSON-Fixtures in `fixtures/` — kein Server nötig, wie vom
//! Auftrag verlangt. Die `async`-Methoden von [`CheckmkClient`] setzen nur noch
//! HTTP und diese Funktionen zusammen.
//!
//! ## Abbruch
//!
//! Die Methoden sind gewöhnliche `async fn`. In Rust bricht ein Future ab, wenn
//! er verworfen wird — ein laufender Abruf endet also, sobald der Aufrufer die
//! Task abbricht. Slice 5 nutzt das für „Jetzt aktualisieren": laufende Task
//! abbrechen, neue starten. Es braucht dafür keinen eigenen Abbruchmechanismus.
//!
//! ## Weiterleitungen
//!
//! Weiterleitungen werden **nicht** gefolgt. Der Authorization-Header trägt das
//! Automation-Secret; einer Weiterleitung zu folgen würde es an ein Ziel
//! senden, das nicht der konfigurierte Server ist. Stattdessen wird die
//! Umleitung als Fehler gemeldet, der das Ziel nennt — das ist zugleich der
//! nützlichere Hinweis („trage https ein"), weil ein http-zu-https-Redirect der
//! häufigste Fall ist.

use std::time::Duration;

use chrono::Utc;
use reqwest::header::{HeaderMap, HeaderValue, ACCEPT, AUTHORIZATION, IF_MATCH, LOCATION};
use reqwest::{Client, Method, StatusCode};
use serde::Serialize;

use super::error::{CheckmkError, Secret};
use super::model::{
    ApiProblem, Collection, HostExtensions, Problem, ServiceExtensions, Snapshot, VersionInfo,
};
use super::url::SiteUrl;
use super::write::{
    AcknowledgeHostBody, AcknowledgeOptions, AcknowledgeServiceBody, DowntimeHostBody,
    DowntimeServiceBody,
};

/// Vorgabe laut Auftrag.
pub const DEFAULT_TIMEOUT_SECONDS: u64 = 10;

/// Proxy-Verhalten, entspricht der Einstellung „System / keiner / manuell".
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProxyMode {
    /// Proxy-Einstellungen des Systems übernehmen.
    #[default]
    System,
    /// Keinen Proxy verwenden, auch wenn das System einen setzt.
    Disabled,
    /// Fester Proxy, etwa `http://proxy.example.intern:8080`.
    Manual(String),
}

/// Alles, was der Client zum Arbeiten braucht.
///
/// Wird in Slice 4 aus `config.json` und dem Credential Manager gefüllt. Das
/// Secret steckt in [`Secret`] und tritt deshalb nicht über `Debug` aus.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    pub server: String,
    pub site: String,
    pub username: String,
    pub secret: Secret,
    /// TLS-Prüfung. Abschaltbar, aber das UI warnt deutlich davor.
    pub verify_tls: bool,
    pub proxy: ProxyMode,
    pub timeout: Duration,
}

impl ClientConfig {
    pub fn new(
        server: impl Into<String>,
        site: impl Into<String>,
        username: impl Into<String>,
        secret: Secret,
    ) -> Self {
        Self {
            server: server.into(),
            site: site.into(),
            username: username.into(),
            secret,
            verify_tls: true,
            proxy: ProxyMode::System,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECONDS),
        }
    }
}

/// Ergebnis des Verbindungstests.
///
/// Der Auftrag verlangt hier Konkretes: HTTP-Statuscode und erkannte
/// CheckMK-Version.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionReport {
    pub http_status: u16,
    pub checkmk_version: Option<String>,
    pub edition: Option<String>,
    pub edition_label: Option<String>,
    pub site: Option<String>,
    /// Gemessene Antwortzeit in Millisekunden.
    pub elapsed_ms: u64,
    /// Ob die TLS-Prüfung für diesen Test abgeschaltet war.
    pub tls_verification_disabled: bool,
}

/// Client gegen eine CheckMK-Site.
#[derive(Debug, Clone)]
pub struct CheckmkClient {
    http: Client,
    urls: SiteUrl,
    verify_tls: bool,
}

impl CheckmkClient {
    /// Baut den Client. Schlägt fehl, wenn URL, Site oder Zugangsdaten
    /// unbrauchbar sind — also bevor irgendein Netzwerkzugriff passiert.
    pub fn new(config: &ClientConfig) -> Result<Self, CheckmkError> {
        let urls = SiteUrl::new(&config.server, &config.site)?;

        if config.username.trim().is_empty() {
            return Err(CheckmkError::InvalidUrl {
                reason: "Es ist kein Benutzername eingetragen.".into(),
            });
        }
        if config.secret.is_empty() {
            return Err(CheckmkError::InvalidUrl {
                reason: "Es ist kein Automation-Secret eingetragen.".into(),
            });
        }

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            auth_header(&config.username, &config.secret)?,
        );
        headers.insert(ACCEPT, HeaderValue::from_static("application/json"));

        let mut builder = Client::builder()
            .default_headers(headers)
            .timeout(config.timeout)
            // Getrennter Verbindungsaufbau-Timeout: ein nicht erreichbarer Host
            // soll nicht die vollen zehn Sekunden blockieren.
            .connect_timeout(config.timeout.min(Duration::from_secs(5)))
            // Siehe Modulkommentar: kein Folgen von Weiterleitungen.
            .redirect(reqwest::redirect::Policy::none())
            .user_agent(concat!("Luchsr/", env!("CARGO_PKG_VERSION")));

        if !config.verify_tls {
            // Bewusst und nur auf ausdrückliche Einstellung hin.
            builder = builder.danger_accept_invalid_certs(true);
        }

        builder = match &config.proxy {
            ProxyMode::System => builder,
            ProxyMode::Disabled => builder.no_proxy(),
            ProxyMode::Manual(raw) => {
                // Erst selbst prüfen: reqwest::Proxy::all ist zu großzügig und
                // macht aus „proxy.intern:8080" stillschweigend
                // http://proxy.intern:8080/ — aus einem Tippfehler wie
                // „kein-proxy-url" wird ein gültig aussehender Proxy, und jeder
                // Abruf scheitert danach mit einer Meldung, die nicht auf die
                // Ursache zeigt.
                let checked = validate_proxy(raw)?;
                let proxy = reqwest::Proxy::all(checked.as_str()).map_err(|error| {
                    CheckmkError::InvalidUrl {
                        reason: format!("Die Proxy-Adresse „{raw}“ ist ungültig ({error})."),
                    }
                })?;
                builder.proxy(proxy)
            }
        };

        let http = builder.build().map_err(|error| CheckmkError::InvalidUrl {
            reason: format!("Der HTTP-Client liess sich nicht anlegen ({error})."),
        })?;

        Ok(Self {
            http,
            urls,
            verify_tls: config.verify_tls,
        })
    }

    pub fn urls(&self) -> &SiteUrl {
        &self.urls
    }

    /* ------------------------------------------------------------------ */
    /* Lesen                                                              */
    /* ------------------------------------------------------------------ */

    /// Prüft die Verbindung und liest die CheckMK-Version.
    pub async fn test_connection(&self) -> Result<ConnectionReport, CheckmkError> {
        let started = std::time::Instant::now();
        let url = self.urls.version()?;
        let (status, body) = self.send(Method::GET, url, None::<&()>).await?;
        let elapsed_ms = started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64;

        let version = parse_version(&body)?;
        Ok(ConnectionReport {
            http_status: status.as_u16(),
            checkmk_version: version.checkmk_version(),
            edition: version.edition.clone(),
            edition_label: version.edition_label().map(str::to_string),
            site: version.site.clone(),
            elapsed_ms,
            tls_verification_disabled: !self.verify_tls,
        })
    }

    /// Holt Host- und Serviceprobleme und setzt sie zu einem Abzug zusammen.
    ///
    /// Beide Abrufe laufen gleichzeitig. Scheitert einer, scheitert der ganze
    /// Abzug — ein halber Abzug wäre schlimmer als keiner, weil das Tray-Icon
    /// dann „alles gut" zeigen könnte, obwohl ein Host ausgefallen ist.
    pub async fn fetch_snapshot(&self) -> Result<Snapshot, CheckmkError> {
        let services_url = self.urls.services()?;
        let hosts_url = self.urls.hosts()?;

        let (services, hosts) =
            tokio::try_join!(self.fetch_text(services_url), self.fetch_text(hosts_url))?;

        let mut problems = parse_hosts(&hosts)?;
        problems.extend(parse_services(&services)?);
        Ok(Snapshot::new(problems, Utc::now()))
    }

    async fn fetch_text(&self, url: url::Url) -> Result<String, CheckmkError> {
        let (_, body) = self.send(Method::GET, url, None::<&()>).await?;
        Ok(body)
    }

    /* ------------------------------------------------------------------ */
    /* Schreiben                                                          */
    /* ------------------------------------------------------------------ */

    pub async fn acknowledge_service(
        &self,
        host: &str,
        service: &str,
        options: &AcknowledgeOptions,
    ) -> Result<(), CheckmkError> {
        let url = self.urls.acknowledge_service()?;
        let body = AcknowledgeServiceBody::new(host, service, options);
        self.send(Method::POST, url, Some(&body)).await.map(drop)
    }

    pub async fn acknowledge_host(
        &self,
        host: &str,
        options: &AcknowledgeOptions,
    ) -> Result<(), CheckmkError> {
        let url = self.urls.acknowledge_host()?;
        let body = AcknowledgeHostBody::new(host, options);
        self.send(Method::POST, url, Some(&body)).await.map(drop)
    }

    pub async fn downtime_service(&self, body: &DowntimeServiceBody) -> Result<(), CheckmkError> {
        let url = self.urls.downtime_service()?;
        self.send(Method::POST, url, Some(body)).await.map(drop)
    }

    pub async fn downtime_host(&self, body: &DowntimeHostBody) -> Result<(), CheckmkError> {
        let url = self.urls.downtime_host()?;
        self.send(Method::POST, url, Some(body)).await.map(drop)
    }

    /* ------------------------------------------------------------------ */
    /* Transport                                                          */
    /* ------------------------------------------------------------------ */

    /// Führt eine Anfrage aus und wertet den Status aus.
    async fn send<B: Serialize + ?Sized>(
        &self,
        method: Method,
        url: url::Url,
        body: Option<&B>,
    ) -> Result<(StatusCode, String), CheckmkError> {
        let mut request = self.http.request(method.clone(), url);

        if let Some(payload) = body {
            // Siehe Modulkommentar zum ETag: If-Match wird grundsätzlich
            // mitgesendet, damit Versionen, die es verlangen, bedient sind.
            request = request
                .header(IF_MATCH, HeaderValue::from_static("*"))
                .json(payload);
        }

        let response = request
            .send()
            .await
            .map_err(|error| CheckmkError::from_reqwest(&error))?;

        let status = response.status();

        if status.is_redirection() {
            let location = response
                .headers()
                .get(LOCATION)
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            return Err(CheckmkError::Redirected { location });
        }

        // Fehlertext vor dem Statuscheck lesen: CheckMK legt die eigentliche
        // Begründung in den Rumpf, nicht in den Status.
        let text = response
            .text()
            .await
            .map_err(|error| CheckmkError::from_reqwest(&error))?;

        if !status.is_success() {
            return Err(error_from_response(status, &text, self.urls.site()));
        }
        Ok((status, text))
    }
}

/// Prüft eine manuell eingetragene Proxy-Adresse.
///
/// Verlangt ausdrücklich ein Schema und einen Hostnamen. Ein bloßes
/// `proxy.example.intern:8080` wird abgelehnt statt stillschweigend als
/// `http://` interpretiert — im Einstellungsdialog ist eine klare Meldung
/// wertvoller als eine geratene Vorgabe.
fn validate_proxy(raw: &str) -> Result<url::Url, CheckmkError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(CheckmkError::InvalidUrl {
            reason: "Es ist keine Proxy-Adresse eingetragen.".into(),
        });
    }

    let parsed = url::Url::parse(trimmed).map_err(|_| CheckmkError::InvalidUrl {
        reason: format!(
            "Die Proxy-Adresse „{trimmed}“ ist unvollständig. Erwartet wird eine \
             vollständige Adresse mit Protokoll, etwa http://proxy.example.intern:8080"
        ),
    })?;

    match parsed.scheme() {
        "http" | "https" | "socks5" | "socks5h" => {}
        other => {
            return Err(CheckmkError::InvalidUrl {
                reason: format!(
                    "Das Proxy-Protokoll „{other}“ wird nicht unterstützt. \
                     Erlaubt sind http, https, socks5 und socks5h."
                ),
            })
        }
    }

    match parsed.host_str() {
        None | Some("") => Err(CheckmkError::InvalidUrl {
            reason: format!("In der Proxy-Adresse „{trimmed}“ fehlt der Hostname."),
        }),
        Some(_) => Ok(parsed),
    }
}

/// Baut den Authorization-Header nach Vertrag: `Bearer {username} {secret}`.
fn auth_header(username: &str, secret: &Secret) -> Result<HeaderValue, CheckmkError> {
    let raw = format!("Bearer {} {}", username.trim(), secret.expose());
    let mut value = HeaderValue::from_str(&raw).map_err(|_| CheckmkError::InvalidUrl {
        reason: "Benutzername oder Automation-Secret enthält Zeichen, die in einem \
                 HTTP-Header nicht erlaubt sind (etwa Zeilenumbrüche)."
            .into(),
    })?;
    // Der Header trägt das Secret. Als sensibel markieren, damit er nicht in
    // Debug-Ausgaben von reqwest oder hyper auftaucht.
    value.set_sensitive(true);
    Ok(value)
}

/* -------------------------------------------------------------------------- */
/* Reine Funktionen — hier hängen die Tests                                   */
/* -------------------------------------------------------------------------- */

/// Übersetzt eine Fehlerantwort in einen konkreten Fehler.
pub fn error_from_response(status: StatusCode, body: &str, site: &str) -> CheckmkError {
    let detail = server_detail(body);

    match status.as_u16() {
        // Die Begründung des Servers wird bei jedem Fall mitgeführt. CheckMK
        // schreibt sie in `detail`, und sie ist oft die einzige Zeile, die
        // wirklich sagt, was los ist — etwa „Wrong credentials (Bearer header)"
        // gegenüber „Wrong credentials (Basic header)".
        401 => CheckmkError::Unauthorized { detail },
        403 => CheckmkError::Forbidden { detail },
        404 => CheckmkError::NotFound {
            site: site.to_string(),
            detail,
        },
        412 | 428 => CheckmkError::PreconditionFailed {
            status: status.as_u16(),
        },
        429 => CheckmkError::RateLimited,
        other => CheckmkError::HttpStatus {
            status: other,
            detail,
        },
    }
}

/// Zieht die Begründung aus einer Fehlerantwort.
///
/// Zwei Wege, und die Unterscheidung ist diagnostisch wertvoll:
///
/// 1. **Gültiges JSON** — CheckMK antwortet in `application/problem+json` und
///    schreibt die Begründung nach `detail` (ersatzweise `title`). Nur dieses
///    Feld wird genommen; aus einem strukturierten, aber leeren Rumpf wird
///    nichts erfunden.
/// 2. **Kein JSON** — dann kommt die Antwort mit hoher Wahrscheinlichkeit
///    **nicht von CheckMK**, sondern von einem davorliegenden Apache, einem
///    Reverse Proxy oder einer Firewall. Ein Auszug aus dem Rohtext macht das
///    sichtbar: `<title>403 Forbidden</title>` beantwortet die Frage „wer sagt
///    hier nein" sofort, und ohne ihn sucht man an der falschen Stelle.
fn server_detail(body: &str) -> Option<String> {
    match serde_json::from_str::<serde_json::Value>(body) {
        Ok(_) => serde_json::from_str::<ApiProblem>(body)
            .ok()
            .and_then(|problem| problem.best_detail()),
        Err(_) => {
            // Zeilenumbrüche und Einrückung einer HTML-Seite zusammenziehen,
            // sonst ist die Meldung im Dialog unlesbar.
            let collapsed = body.split_whitespace().collect::<Vec<_>>().join(" ");
            if collapsed.is_empty() {
                None
            } else {
                Some(truncate_for_message(&collapsed, RAW_EXCERPT_CHARS))
            }
        }
    }
}

/// Wie viel Rohtext in eine Fehlermeldung darf.
///
/// Genug für den `<title>` einer Apache-Fehlerseite, zu wenig für eine ganze
/// Seite im Dialog.
const RAW_EXCERPT_CHARS: usize = 240;

fn truncate_for_message(value: &str, max: usize) -> String {
    if value.chars().count() <= max {
        return value.to_string();
    }
    let mut out: String = value.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Liest eine Service-Collection.
pub fn parse_services(json: &str) -> Result<Vec<Problem>, CheckmkError> {
    let parsed: Collection<ServiceExtensions> =
        serde_json::from_str(json).map_err(|error| CheckmkError::Malformed {
            reason: format!("Serviceliste liess sich nicht lesen: {error}"),
        })?;
    Ok(parsed
        .value
        .into_iter()
        .map(|entry| Problem::from(entry.extensions))
        .collect())
}

/// Liest eine Host-Collection.
pub fn parse_hosts(json: &str) -> Result<Vec<Problem>, CheckmkError> {
    let parsed: Collection<HostExtensions> =
        serde_json::from_str(json).map_err(|error| CheckmkError::Malformed {
            reason: format!("Hostliste liess sich nicht lesen: {error}"),
        })?;
    Ok(parsed
        .value
        .into_iter()
        .map(|entry| Problem::from(entry.extensions))
        .collect())
}

/// Liest die Versionsauskunft.
pub fn parse_version(json: &str) -> Result<VersionInfo, CheckmkError> {
    serde_json::from_str(json).map_err(|error| CheckmkError::Malformed {
        reason: format!("Versionsauskunft liess sich nicht lesen: {error}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checkmk::model::ProblemState;

    const SERVICES: &str = include_str!("fixtures/services_problems.json");
    const HOSTS: &str = include_str!("fixtures/hosts_problems.json");
    const VERSION: &str = include_str!("fixtures/version.json");
    const EMPTY: &str = include_str!("fixtures/services_empty.json");
    const VARIANTS: &str = include_str!("fixtures/services_typvarianten.json");
    const ERROR_401: &str = include_str!("fixtures/error_401.json");
    const ERROR_403: &str = include_str!("fixtures/error_403.json");
    const ERROR_404: &str = include_str!("fixtures/error_404.json");
    const ERROR_400: &str = include_str!("fixtures/error_400_downtime.json");

    fn find<'a>(problems: &'a [Problem], host: &str) -> &'a Problem {
        problems
            .iter()
            .find(|p| p.host == host)
            .unwrap_or_else(|| panic!("Host {host} nicht in der Liste"))
    }

    /* --------------------------------------------------- Services lesen -- */

    #[test]
    fn liest_die_service_fixture_vollstaendig() {
        let problems = parse_services(SERVICES).unwrap();
        assert_eq!(problems.len(), 8);
        assert!(problems.iter().all(|p| !p.is_host_problem()));
    }

    #[test]
    fn liest_die_felder_eines_services_richtig() {
        let problems = parse_services(SERVICES).unwrap();
        let sql = problems
            .iter()
            .find(|p| p.host == "leosys-sql-01" && p.service.as_deref() == Some("Filesystem /var"))
            .unwrap();

        assert_eq!(sql.state, ProblemState::Crit);
        assert!(sql.output.starts_with("CRIT - Used: 96.41%"));
        assert_eq!(
            sql.last_state_change.unwrap().timestamp(),
            1_723_449_713,
            "Zeitstempel muss aus dem Epoch-Wert kommen"
        );
        assert!(!sql.acknowledged);
        assert_eq!(sql.downtime_depth, 0);
        assert!(!sql.flapping);
    }

    #[test]
    fn erkennt_flattern() {
        let problems = parse_services(SERVICES).unwrap();
        assert!(find(&problems, "leosys-dc-02").flapping);
        assert!(!find(&problems, "leosys-sql-01").flapping);
    }

    #[test]
    fn erkennt_quittiert_und_wartung_aus_der_fixture() {
        let problems = parse_services(SERVICES).unwrap();
        let quittiert = find(&problems, "leosys-web-02");
        assert!(quittiert.acknowledged);
        assert!(quittiert.is_handled());

        let wartung = find(&problems, "leosys-nas-02");
        assert!(!wartung.acknowledged);
        assert_eq!(wartung.downtime_depth, 2);
        assert!(wartung.is_handled());

        let offen = find(&problems, "leosys-fw-01");
        assert!(!offen.is_handled());
    }

    #[test]
    fn unknown_zustand_wird_erkannt() {
        let problems = parse_services(SERVICES).unwrap();
        assert_eq!(
            find(&problems, "leosys-print-01").state,
            ProblemState::Unknown
        );
    }

    /// Umlaute und Anführungszeichen im plugin_output müssen unverändert
    /// durchkommen — das landet später in Mono im Detail-Panel.
    #[test]
    fn umlaute_und_sonderzeichen_bleiben_erhalten() {
        let problems = parse_services(SERVICES).unwrap();
        let esxi = find(&problems, "leosys-esxi-03");
        assert_eq!(esxi.service.as_deref(), Some("Multipath Größe (Uplink)"));
        assert!(esxi.output.contains("äöüß"));
        assert!(esxi.output.contains('"'));
        assert!(esxi.output.contains('\\'));
    }

    #[test]
    fn leere_collection_ergibt_leere_liste() {
        assert!(parse_services(EMPTY).unwrap().is_empty());
    }

    /* ------------------------------------------------------ Hosts lesen -- */

    #[test]
    fn liest_hosts_und_bildet_zustaende_ab() {
        let problems = parse_hosts(HOSTS).unwrap();
        assert_eq!(problems.len(), 3);
        assert!(problems.iter().all(|p| p.is_host_problem()));

        assert_eq!(find(&problems, "leosys-esxi-03").state, ProblemState::Down);
        assert_eq!(
            find(&problems, "leosys-vm-77").state,
            ProblemState::Unreachable,
            "state 2 bedeutet beim Host UNREACHABLE, nicht CRIT"
        );
    }

    #[test]
    fn hostprobleme_haben_keinen_servicenamen() {
        let problems = parse_hosts(HOSTS).unwrap();
        assert!(problems.iter().all(|p| p.service.is_none()));
    }

    #[test]
    fn host_in_wartung_wird_als_bearbeitet_erkannt() {
        let problems = parse_hosts(HOSTS).unwrap();
        assert!(find(&problems, "leosys-test-09").is_handled());
    }

    /* ------------------------------------------------- Typvarianten ------ */

    /// Der eigentliche Grund für die eigenen Deserialisierer.
    #[test]
    fn haelt_alle_beobachteten_typvarianten_aus() {
        let problems = parse_services(VARIANTS).unwrap();
        assert_eq!(problems.len(), 8, "keine Variante darf verloren gehen");
    }

    #[test]
    fn echte_booleans_werden_gelesen() {
        let problems = parse_services(VARIANTS).unwrap();
        let p = find(&problems, "variante-bool");
        assert!(p.acknowledged, "true muss als true ankommen");
        assert!(!p.flapping, "false muss als false ankommen");
    }

    #[test]
    fn zahlen_und_wahrheitswerte_als_text_werden_gelesen() {
        let problems = parse_services(VARIANTS).unwrap();
        let p = find(&problems, "variante-text");
        assert_eq!(p.state, ProblemState::Warn, "\"1\" muss WARN ergeben");
        assert!(!p.acknowledged, "\"0\" muss false ergeben");
        assert_eq!(p.downtime_depth, 3, "\"3\" muss 3 ergeben");
        assert!(p.flapping, "\"true\" muss true ergeben");
    }

    #[test]
    fn zeitstempel_mit_nachkommastellen_wird_gelesen() {
        let problems = parse_services(VARIANTS).unwrap();
        let p = find(&problems, "variante-float");
        assert_eq!(p.last_state_change.unwrap().timestamp(), 1_723_449_713);
        assert_eq!(p.state, ProblemState::Crit, "2.0 muss CRIT ergeben");
    }

    /// Der wichtigste Sonderfall: `0` heisst „noch nie gewechselt", nicht
    /// „1. Januar 1970". Sonst zeigt die Liste 56 Jahre Dauer an.
    #[test]
    fn zeitstempel_null_bedeutet_nie_nicht_1970() {
        let problems = parse_services(VARIANTS).unwrap();
        assert!(
            find(&problems, "variante-null").last_state_change.is_none(),
            "0 darf nicht zu 1970 werden"
        );
        assert!(find(&problems, "variante-explizit-null")
            .last_state_change
            .is_none());
    }

    #[test]
    fn fehlende_felder_bekommen_vernuenftige_vorgaben() {
        let problems = parse_services(VARIANTS).unwrap();
        let p = find(&problems, "variante-luecken");
        assert_eq!(p.state, ProblemState::Unknown);
        assert_eq!(p.service.as_deref(), Some(""));
        assert_eq!(p.output, "");
        assert!(p.last_state_change.is_none());
        assert!(!p.acknowledged);
        assert_eq!(p.downtime_depth, 0);
    }

    #[test]
    fn explizite_nullwerte_stuerzen_nicht_ab() {
        let problems = parse_services(VARIANTS).unwrap();
        let p = find(&problems, "variante-explizit-null");
        assert_eq!(p.output, "");
        assert!(!p.acknowledged);
        assert_eq!(p.downtime_depth, 0);
        assert!(!p.flapping);
    }

    #[test]
    fn iso_zeitstempel_wird_alternativ_akzeptiert() {
        let problems = parse_services(VARIANTS).unwrap();
        let p = find(&problems, "variante-iso");
        assert_eq!(p.last_state_change.unwrap().timestamp(), 1_723_454_513);
    }

    /// Mehrzeiliger plugin_output wird nur an den Rändern getrimmt, innere
    /// Umbrüche bleiben — das Detail-Panel braucht den vollen Text.
    #[test]
    fn mehrzeilige_ausgabe_wird_nur_am_rand_getrimmt() {
        let problems = parse_services(VARIANTS).unwrap();
        let p = find(&problems, "variante-mehrzeilig");
        assert!(p.output.starts_with("CRIT - erste Zeile"));
        assert!(p.output.ends_with("dritte Zeile"));
        assert_eq!(p.output.lines().count(), 3, "innere Umbrüche bleiben");
    }

    /* ----------------------------------------------------- Version ------- */

    #[test]
    fn liest_version_und_edition() {
        let version = parse_version(VERSION).unwrap();
        assert_eq!(version.checkmk_version().as_deref(), Some("2.3.0p23"));
        assert_eq!(version.site.as_deref(), Some("leosys"));
        assert_eq!(version.edition.as_deref(), Some("cre"));
        assert_eq!(version.edition_label(), Some("Raw Edition"));
    }

    #[test]
    fn unbekannte_edition_liefert_keine_beschriftung_aber_das_kuerzel() {
        let version = parse_version(r#"{"edition":"xyz","versions":{}}"#).unwrap();
        assert_eq!(version.edition.as_deref(), Some("xyz"));
        assert_eq!(version.edition_label(), None);
    }

    #[test]
    fn version_ohne_checkmk_feld_gibt_none_statt_fehler() {
        let version = parse_version(r#"{"versions":{"python":"3.12"}}"#).unwrap();
        assert!(version.checkmk_version().is_none());
    }

    /* ------------------------------------------------ Fehlerantworten ---- */

    #[test]
    fn vierhunderteins_wird_zu_unauthorized() {
        let error = error_from_response(StatusCode::UNAUTHORIZED, ERROR_401, "leosys");
        assert!(matches!(error, CheckmkError::Unauthorized { .. }));
        assert!(!error.is_retryable(), "falsches Secret nicht wiederholen");
    }

    /// Ein 404 zeigt fast immer auf einen falschen Site-Namen. Die Meldung
    /// muss den geprüften Namen nennen, sonst rät der Benutzer.
    #[test]
    fn vierhundertvier_nennt_den_geprueften_sitenamen() {
        let error = error_from_response(StatusCode::NOT_FOUND, ERROR_404, "falsche-site");
        match &error {
            CheckmkError::NotFound { site, .. } => assert_eq!(site, "falsche-site"),
            other => panic!("erwartet wurde NotFound, gelesen wurde {other:?}"),
        }
        assert!(error.to_string().contains("falsche-site"));
    }

    #[test]
    fn dreihundertdrei_wird_zu_forbidden() {
        let error = error_from_response(StatusCode::FORBIDDEN, "{}", "leosys");
        assert!(matches!(error, CheckmkError::Forbidden { .. }));
    }

    /// Der eigentliche Wert dieses Moduls: die Begründung des Servers muss in
    /// der Meldung ankommen.
    ///
    /// Diese Zeile stand vorher nicht drin — 401, 403 und 404 warfen das
    /// `detail`-Feld weg. Damit sah ein 403 wegen fehlender REST-API-Berechtigung
    /// genauso aus wie jeder andere 403, und man suchte an der falschen Stelle.
    #[test]
    fn serverbegruendung_kommt_bei_jedem_statuscode_durch() {
        let faelle: [(StatusCode, &str, &str); 3] = [
            (
                StatusCode::UNAUTHORIZED,
                ERROR_401,
                "Wrong credentials (Bearer header)",
            ),
            (
                StatusCode::FORBIDDEN,
                ERROR_403,
                "You do not have the permission for general.use.",
            ),
            (
                StatusCode::NOT_FOUND,
                ERROR_404,
                "The requested URL was not found on the server",
            ),
        ];

        for (status, body, erwartet) in faelle {
            let text = error_from_response(status, body, "leosys").to_string();
            assert!(
                text.contains(erwartet),
                "die Begründung des Servers fehlt bei {status}: {text}"
            );
            assert!(
                text.contains("Antwort des Servers"),
                "die Begründung ist nicht als solche gekennzeichnet: {text}"
            );
        }
    }

    /// Ein 403 muss sagen, dass die Anmeldung **funktioniert hat**.
    ///
    /// Das ist der entscheidende Unterschied zu 401 und der Satz, der beim
    /// Suchen die Richtung vorgibt: Zugangsdaten und URL sind richtig, gesucht
    /// wird eine Berechtigung.
    #[test]
    fn forbidden_sagt_dass_die_anmeldung_erfolgreich_war() {
        let text = error_from_response(StatusCode::FORBIDDEN, ERROR_403, "leosys").to_string();
        assert!(
            text.contains("Anmeldung war erfolgreich"),
            "der entscheidende Hinweis fehlt: {text}"
        );
        assert!(text.contains("403"), "{text}");
        assert!(text.contains("Berechtigung"), "{text}");
    }

    /// Fehlt die Begründung, darf kein leerer Anhang entstehen.
    ///
    /// Nur bei gültigem JSON ohne Inhalt — aus einem Rumpf ohne JSON wird
    /// bewusst ein Auszug gezeigt, siehe Test unten.
    #[test]
    fn ohne_serverbegruendung_bleibt_die_meldung_glatt() {
        for body in ["{}", "", r#"{"detail":"   "}"#] {
            let text = error_from_response(StatusCode::FORBIDDEN, body, "leosys").to_string();
            assert!(
                !text.contains("Antwort des Servers"),
                "leerer Anhang bei Rumpf {body:?}: {text}"
            );
            assert!(!text.ends_with(' '), "Leerzeichen am Ende: {text:?}");
        }
    }

    /// Der Server-Text muss durchkommen — bei einem 400 steckt dort die
    /// eigentliche Begründung.
    #[test]
    fn vierhundert_uebernimmt_das_serverdetail() {
        let error = error_from_response(StatusCode::BAD_REQUEST, ERROR_400, "leosys");
        match &error {
            CheckmkError::HttpStatus { status, detail } => {
                assert_eq!(*status, 400);
                assert_eq!(
                    detail.as_deref(),
                    Some("These fields have problems: end_time")
                );
            }
            other => panic!("erwartet wurde HttpStatus, gelesen wurde {other:?}"),
        }
        assert!(error.to_string().contains("end_time"));
    }

    #[test]
    fn etag_vorbedingung_wird_erkannt() {
        for code in [412u16, 428] {
            let status = StatusCode::from_u16(code).unwrap();
            let error = error_from_response(status, "{}", "leosys");
            match error {
                CheckmkError::PreconditionFailed { status } => assert_eq!(status, code),
                other => panic!("{code} ergab {other:?}"),
            }
        }
    }

    #[test]
    fn rate_limit_ist_wiederholbar() {
        let error = error_from_response(StatusCode::TOO_MANY_REQUESTS, "{}", "leosys");
        assert!(matches!(error, CheckmkError::RateLimited));
        assert!(error.is_retryable());
    }

    #[test]
    fn serverfehler_sind_wiederholbar_clientfehler_nicht() {
        assert!(
            error_from_response(StatusCode::SERVICE_UNAVAILABLE, "{}", "leosys").is_retryable()
        );
        assert!(error_from_response(StatusCode::BAD_GATEWAY, "{}", "leosys").is_retryable());
        assert!(!error_from_response(StatusCode::BAD_REQUEST, "{}", "leosys").is_retryable());
    }

    /// Ein Rumpf ohne JSON gehört in die Meldung, nicht in den Müll.
    ///
    /// Die frühere Fassung dieses Tests verlangte das Gegenteil — „aus HTML
    /// darf kein Detail erfunden werden". Das war der falsche Instinkt: einen
    /// Auszug aus der Antwort zu zeigen ist nichts Erfundenes, sondern die
    /// einzige Auskunft, die es gibt. Und sie ist entscheidend, denn ein
    /// Statuscode ohne problem+json kommt in der Regel **nicht von CheckMK**,
    /// sondern von einem davorliegenden Apache oder Proxy. Der `<title>` sagt
    /// das in drei Wörtern.
    #[test]
    fn html_fehlerseite_kommt_als_auszug_in_die_meldung() {
        let html = "<html>\n  <head>\n    <title>403 Forbidden</title>\n  </head>\n</html>";
        let error = error_from_response(StatusCode::FORBIDDEN, html, "leosys");

        let detail = match &error {
            CheckmkError::Forbidden { detail } => detail.clone(),
            other => panic!("erwartet wurde Forbidden, gelesen wurde {other:?}"),
        };
        let detail = detail.expect("der Rohauszug fehlt");
        assert!(detail.contains("403 Forbidden"), "{detail}");
        assert!(
            !detail.contains('\n'),
            "Zeilenumbrüche müssen zusammengezogen sein: {detail:?}"
        );
        assert!(error.to_string().contains("403 Forbidden"));
    }

    /// Ein sehr langer Rumpf darf die Meldung nicht sprengen.
    #[test]
    fn langer_rohtext_wird_gekuerzt() {
        let lang = format!("<html>{}</html>", "x ".repeat(5000));
        let error = error_from_response(StatusCode::BAD_GATEWAY, &lang, "leosys");
        let detail = match &error {
            CheckmkError::HttpStatus { detail, .. } => detail.clone().expect("Auszug fehlt"),
            other => panic!("erwartet wurde HttpStatus, gelesen wurde {other:?}"),
        };
        assert_eq!(detail.chars().count(), RAW_EXCERPT_CHARS);
        assert!(detail.ends_with('…'));
    }

    /// Gültiges JSON ohne `detail` darf keinen Rohauszug erzeugen — daraus
    /// würde sonst `{}` als Begründung.
    #[test]
    fn leeres_json_erzeugt_keinen_rohauszug() {
        assert_eq!(server_detail("{}"), None);
        assert_eq!(server_detail(r#"{"foo":1}"#), None);
        assert_eq!(server_detail(""), None);
        assert_eq!(server_detail("   \n  "), None);
    }

    /* ------------------------------------------------- Kaputtes JSON ----- */

    #[test]
    fn kaputtes_json_ergibt_malformed_mit_begruendung() {
        let error = parse_services("{ das ist kein json").unwrap_err();
        match &error {
            CheckmkError::Malformed { reason } => {
                assert!(reason.contains("Serviceliste"), "Kontext fehlt: {reason}");
            }
            other => panic!("erwartet wurde Malformed, gelesen wurde {other:?}"),
        }
        assert!(!error.is_retryable());
    }

    /// Fehlt der Umschlag ganz, ist das ein Malformed — kein Absturz.
    #[test]
    fn fehlender_umschlag_ergibt_malformed_oder_leer() {
        // value fehlt -> leere Liste, weil #[serde(default)]
        assert!(parse_services(r#"{"domainType":"service"}"#)
            .unwrap()
            .is_empty());
        // host_name fehlt -> das Feld ist Pflicht, also Malformed
        let error = parse_services(r#"{"value":[{"extensions":{"state":2}}]}"#).unwrap_err();
        assert!(matches!(error, CheckmkError::Malformed { .. }));
    }

    /* ------------------------------------------------ Client-Aufbau ------ */

    #[test]
    fn client_verweigert_fehlende_zugangsdaten_vor_dem_netzzugriff() {
        let mut config = ClientConfig::new(
            "https://checkmk.example.intern",
            "leosys",
            "",
            Secret::new("s"),
        );
        assert!(CheckmkClient::new(&config).is_err(), "leerer Benutzername");

        config.username = "m.mustermann".into();
        config.secret = Secret::new("");
        assert!(CheckmkClient::new(&config).is_err(), "leeres Secret");

        config.secret = Secret::new("geheim");
        assert!(CheckmkClient::new(&config).is_ok());
    }

    #[test]
    fn client_verweigert_ungueltige_server_url() {
        let config = ClientConfig::new("nicht::gültig", "leosys", "u", Secret::new("s"));
        assert!(CheckmkClient::new(&config).is_err());
    }

    fn config_mit_proxy(proxy: ProxyMode) -> ClientConfig {
        let mut config = ClientConfig::new(
            "https://checkmk.example.intern",
            "leosys",
            "u",
            Secret::new("s"),
        );
        config.proxy = proxy;
        config
    }

    #[test]
    fn client_verweigert_ungueltigen_proxy() {
        let config = config_mit_proxy(ProxyMode::Manual("kein-proxy-url".into()));
        let error = CheckmkClient::new(&config).unwrap_err();
        assert!(error.to_string().contains("Proxy"), "{error}");
    }

    /// Ohne diese Prüfung macht reqwest aus einem Tippfehler stillschweigend
    /// einen http-Proxy. Jede dieser Eingaben muss abgelehnt werden.
    #[test]
    fn proxy_ohne_schema_wird_abgelehnt_nicht_geraten() {
        for kaputt in [
            "kein-proxy-url",
            "proxy.example.intern:8080",
            "",
            "   ",
            "://proxy",
            "http://",
        ] {
            let error = validate_proxy(kaputt).unwrap_err();
            let text = error.to_string();
            assert!(
                text.contains("Proxy") || text.contains("Protokoll"),
                "„{kaputt}“ ergab eine unbrauchbare Meldung: {text}"
            );
        }
    }

    #[test]
    fn proxy_mit_fremdem_protokoll_wird_abgelehnt() {
        let error = validate_proxy("ftp://proxy.example.intern:2121").unwrap_err();
        assert!(error.to_string().contains("ftp"), "{error}");
    }

    #[test]
    fn gueltige_proxy_adressen_werden_angenommen() {
        for gut in [
            "http://proxy.example.intern:8080",
            "https://proxy.example.intern:3128",
            "socks5://10.42.0.9:1080",
            "socks5h://proxy.intern",
            "  http://proxy.intern:8080  ",
        ] {
            assert!(
                validate_proxy(gut).is_ok(),
                "„{gut}“ hätte angenommen werden müssen"
            );
            assert!(CheckmkClient::new(&config_mit_proxy(ProxyMode::Manual(gut.into()))).is_ok());
        }
    }

    #[test]
    fn proxy_modi_ohne_adresse_bauen_durch() {
        assert!(CheckmkClient::new(&config_mit_proxy(ProxyMode::System)).is_ok());
        assert!(CheckmkClient::new(&config_mit_proxy(ProxyMode::Disabled)).is_ok());
    }

    /* ----------------------------------------------- Authorization ------- */

    #[test]
    fn authorization_header_entspricht_dem_vertrag() {
        let value = auth_header("m.mustermann", &Secret::new("GEHEIM123")).unwrap();
        assert_eq!(value.to_str().unwrap(), "Bearer m.mustermann GEHEIM123");
    }

    /// Der Header muss als sensibel markiert sein, sonst kann das Secret in
    /// Debug-Ausgaben von reqwest oder hyper landen.
    #[test]
    fn authorization_header_ist_als_sensibel_markiert() {
        let value = auth_header("u", &Secret::new("GEHEIM123")).unwrap();
        assert!(value.is_sensitive());
        assert!(
            !format!("{value:?}").contains("GEHEIM123"),
            "Secret tritt über Debug aus: {value:?}"
        );
    }

    /// Ein Zeilenumbruch im Secret wäre eine Header-Injektion. Muss scheitern,
    /// nicht abgeschnitten werden.
    #[test]
    fn zeilenumbruch_in_zugangsdaten_wird_abgelehnt() {
        let error = auth_header("u", &Secret::new("geheim\r\nX-Evil: 1")).unwrap_err();
        assert!(error.to_string().contains("Header"), "{error}");
        assert!(auth_header("u\nx", &Secret::new("geheim")).is_err());
    }

    #[test]
    fn benutzername_wird_getrimmt() {
        let value = auth_header("  m.mustermann  ", &Secret::new("s")).unwrap();
        assert_eq!(value.to_str().unwrap(), "Bearer m.mustermann s");
    }

    /* ---------------------------------------- Zusammenspiel im Abzug ----- */

    /// Der Abzug aus beiden Fixtures muss die Gruppierung und die Zähler
    /// richtig ergeben — das ist die Nahtstelle zu Slice 5 und 6.
    #[test]
    fn abzug_aus_beiden_fixtures_ergibt_die_erwarteten_zaehler() {
        let mut problems = parse_hosts(HOSTS).unwrap();
        problems.extend(parse_services(SERVICES).unwrap());
        let snapshot = Snapshot::new(problems, Utc::now());

        // 3 Hosts + 8 Services = 11 Einträge.
        assert_eq!(snapshot.problems.len(), 11);

        // Sichtbar: ohne leosys-test-09 (Wartung), leosys-web-02 (quittiert),
        // leosys-nas-02 (Wartung) bleiben 8.
        let sichtbar = snapshot.counts(false);
        assert_eq!(sichtbar.total(), 8);
        assert_eq!(sichtbar.down, 1, "leosys-esxi-03");
        assert_eq!(sichtbar.unreachable, 1, "leosys-vm-77");
        assert_eq!(
            sichtbar.crit, 3,
            "sql-01 Filesystem, dc-02 NTP, esxi-03 Multipath — web-02 ist quittiert"
        );
        assert_eq!(sichtbar.unknown, 1, "print-01");
        assert_eq!(
            sichtbar.warn, 2,
            "sql-01 Memory, fw-01 Interface — nas-02 ist in Wartung"
        );

        // Der schlimmste Zustand steuert das Tray-Icon.
        assert_eq!(snapshot.worst(false), Some(ProblemState::Down));

        // Ausgefallene Hosts für die Gruppierung in Slice 6. leosys-test-09 ist
        // ebenfalls down, liegt aber in Wartungszeit und ist damit in der
        // Standardansicht nicht sichtbar — also auch keine Gruppe.
        let mut hosts = snapshot.failed_hosts(false);
        hosts.sort_unstable();
        assert_eq!(hosts, vec!["leosys-esxi-03", "leosys-vm-77"]);

        let mut mit_bearbeiteten = snapshot.failed_hosts(true);
        mit_bearbeiteten.sort_unstable();
        assert_eq!(
            mit_bearbeiteten,
            vec!["leosys-esxi-03", "leosys-test-09", "leosys-vm-77"]
        );
    }

    #[test]
    fn abzug_ist_absteigend_nach_schwere_sortiert() {
        let mut problems = parse_hosts(HOSTS).unwrap();
        problems.extend(parse_services(SERVICES).unwrap());
        let snapshot = Snapshot::new(problems, Utc::now());

        let schweren: Vec<u8> = snapshot
            .problems
            .iter()
            .map(|p| p.state.severity())
            .collect();
        assert!(
            schweren.windows(2).all(|w| w[0] >= w[1]),
            "nicht absteigend sortiert: {schweren:?}"
        );
        // Erste Zeile ist der ausgefallene Host, nicht ein CRIT-Service.
        assert!(snapshot.problems[0].is_host_problem());
        assert_eq!(snapshot.problems[0].state, ProblemState::Down);
    }
}
