//! Fehlertypen des CheckMK-Clients.
//!
//! Der Auftrag verlangt ausdrücklich konkrete Ursachen statt generischer
//! Meldungen: DNS, TLS-Kette, 401, 404. Der Einstellungsdialog zeigt diese
//! Texte unverändert an, sie sind also Benutzeroberfläche und nicht bloss
//! Entwicklerprotokoll.
//!
//! ## Warum die Klassifizierung eine reine Funktion über Strings ist
//!
//! `reqwest` verpackt Transportfehler in eine Kette aus hyper-, native-tls-
//! und io-Fehlern. Die inneren Typen sind nicht öffentlich, ein `downcast_ref`
//! greift also nicht. Bleibt: die Kette aus `source()` ablaufen und auswerten.
//!
//! Damit das ohne Netzwerk, ohne kaputtes Zertifikat und ohne unerreichbaren
//! Host testbar bleibt, ist die Auswertung in [`classify_transport`] gekapselt —
//! eine reine Funktion über die gesammelten Meldungen. Die Tests unten füttern
//! sie mit echten Fehlerketten, wie sie auf einem deutschen Windows entstehen.
//!
//! ## Warum nach Zahlencodes und nicht nach Text gesucht wird
//!
//! Windows-Fehlermeldungen sind lokalisiert. Auf einem deutschen System heisst
//! es „Der angegebene Host ist unbekannt." statt „No such host is known.".
//! Ein Textvergleich wäre damit sofort kaputt. Stabil sind zwei Dinge:
//!
//! * die Zahl in `(os error N)` — die schreibt Rusts `io::Error` immer selbst
//!   und immer englisch
//! * die Rahmentexte von hyper und reqwest, etwa
//!   `failed to lookup address information` — die kommen aus Rust, nicht aus
//!   Windows, und sind deshalb nicht lokalisiert
//!
//! Deshalb wird primär auf Zahlencodes geprüft und nur ergänzend auf diese
//! Rahmentexte.

use std::error::Error as StdError;
use std::fmt;

/// Hängt die Begründung des Servers an, wenn es eine gibt.
///
/// CheckMK füllt bei jedem Fehler das Feld `detail` in einer
/// `application/problem+json`-Antwort. Diese Zeile ist oft die einzige, die
/// wirklich sagt, was los ist — sie wegzulassen war ein Fehler.
///
/// Der Text ist bewusst „der Server" und nicht „CheckMK": bei einem Rumpf ohne
/// JSON stammt die Antwort mit hoher Wahrscheinlichkeit von einem
/// davorliegenden Apache oder Proxy, und „CheckMK meldet" wäre dann falsch und
/// würde die Suche in die falsche Richtung schicken.
fn detail_suffix(detail: &Option<String>) -> String {
    match detail {
        Some(text) if !text.trim().is_empty() => {
            format!(" Antwort des Servers: „{}“", text.trim())
        }
        _ => String::new(),
    }
}

/// Konkrete Transportursache. Wird in [`CheckmkError`] eingebettet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportCause {
    /// Hostname liess sich nicht auflösen.
    Dns,
    /// Verbindung aktiv abgewiesen — Port zu, Dienst läuft nicht.
    Refused,
    /// Netz nicht erreichbar oder Route fehlt.
    Unreachable,
    /// Zertifikatskette endet nicht in einem vertrauenswürdigen Stamm.
    /// Der häufigste Fall bei interner CA, die nicht im Windows-Speicher liegt.
    TlsUntrustedRoot,
    /// Zertifikat gilt für einen anderen Namen als den aufgerufenen.
    TlsHostnameMismatch,
    /// Zertifikat abgelaufen oder noch nicht gültig.
    TlsExpired,
    /// Zertifikat wurde zurückgezogen.
    TlsRevoked,
    /// TLS-Aushandlung gescheitert, Grund nicht weiter eingrenzbar.
    TlsOther,
    /// Zeitüberschreitung auf Transportebene.
    Timeout,
    /// Verbindung stand, brach dann ab.
    Aborted,
    /// Nicht zuordenbar.
    Unknown,
}

impl TransportCause {
    /// Deutscher Klartext, direkt anzeigbar.
    pub fn describe(self) -> &'static str {
        match self {
            Self::Dns => "Der Hostname konnte nicht aufgelöst werden. DNS-Eintrag und Schreibweise der Server-URL prüfen.",
            Self::Refused => "Die Verbindung wurde abgewiesen. Der Port ist geschlossen oder auf dem Server läuft kein Webdienst.",
            Self::Unreachable => "Das Netz ist nicht erreichbar. Route, VPN oder Firewall prüfen.",
            Self::TlsUntrustedRoot => "Die Zertifikatskette endet nicht in einem vertrauenswürdigen Stammzertifikat. Das Stammzertifikat der internen CA fehlt im Windows-Zertifikatspeicher.",
            Self::TlsHostnameMismatch => "Das Serverzertifikat gilt für einen anderen Hostnamen als den aufgerufenen.",
            Self::TlsExpired => "Das Serverzertifikat ist abgelaufen oder noch nicht gültig.",
            Self::TlsRevoked => "Das Serverzertifikat wurde zurückgezogen.",
            Self::TlsOther => "Die TLS-Verbindung konnte nicht aufgebaut werden.",
            Self::Timeout => "Der Server hat nicht innerhalb der Zeitgrenze geantwortet.",
            Self::Aborted => "Die Verbindung wurde nach dem Aufbau unterbrochen.",
            Self::Unknown => "Die Verbindung ist aus einem nicht näher bestimmbaren Grund gescheitert.",
        }
    }

    /// Ob es sich um ein Zertifikatsproblem handelt. Das UI zeigt dann den
    /// Hinweis auf die TLS-Prüfungs-Einstellung — mit deutlicher Warnung.
    pub fn is_tls(self) -> bool {
        matches!(
            self,
            Self::TlsUntrustedRoot
                | Self::TlsHostnameMismatch
                | Self::TlsExpired
                | Self::TlsRevoked
                | Self::TlsOther
        )
    }
}

/// Fehler des CheckMK-Clients.
#[derive(Debug, thiserror::Error)]
pub enum CheckmkError {
    /// Die zusammengesetzte Basis-URL ist keine gültige URL.
    #[error("Die Server-URL ist ungültig: {reason}")]
    InvalidUrl { reason: String },

    /// Transportfehler mit eingegrenzter Ursache.
    #[error("{}", cause.describe())]
    Transport {
        cause: TransportCause,
        /// Die vollständige Fehlerkette, für das Detail-Panel und Protokolle.
        /// Enthält nie Zugangsdaten — die stehen nur im Header, nie im Fehler.
        chain: Vec<String>,
    },

    /// 401 — Benutzername oder Automation-Secret falsch.
    ///
    /// CheckMK schreibt die Begründung in den Rumpf, etwa
    /// `Wrong credentials (Bearer header)`. Die wird mitgeführt: sie
    /// unterscheidet „Benutzer unbekannt" von „Secret falsch" von
    /// „Konto gesperrt".
    #[error("Anmeldung abgelehnt (HTTP 401). Benutzername und Automation-Secret prüfen. Beachte: es muss ein Automation-Secret sein, kein normales Kennwort.{}", detail_suffix(detail))]
    Unauthorized { detail: Option<String> },

    /// 403 — angemeldet, aber ohne Recht auf diese Aktion.
    ///
    /// Der wichtigste Hinweis steckt in der Unterscheidung zu 401: bei 403 hat
    /// die **Anmeldung funktioniert**. Zugangsdaten und URL sind also richtig,
    /// und gesucht wird eine Berechtigung in CheckMK. Ohne diesen Satz sucht
    /// man an der falschen Stelle.
    #[error("Keine Berechtigung (HTTP 403). Die Anmeldung war erfolgreich — Benutzername, Secret und URL sind also richtig. CheckMK verweigert aber die Aktion selbst; dem Konto fehlt eine Berechtigung für die REST-API.{}", detail_suffix(detail))]
    Forbidden { detail: Option<String> },

    /// 404 — fast immer ein falscher Site-Name.
    #[error("Endpunkt nicht gefunden (HTTP 404). Meist ist der Site-Name falsch — geprüft wurde: {site}{}", detail_suffix(detail))]
    NotFound {
        site: String,
        detail: Option<String>,
    },

    /// 412/428 — ETag-Vorbedingung nicht erfüllt.
    #[error("Der Server verlangt eine ETag-Vorbedingung (HTTP {status}). Das Objekt wurde zwischenzeitlich geändert; bitte erneut abrufen und die Aktion wiederholen.")]
    PreconditionFailed { status: u16 },

    /// 429 — Rate Limit.
    #[error("Der Server hat die Anfrage wegen zu vieler Zugriffe abgewiesen (HTTP 429).")]
    RateLimited,

    /// 3xx — der Server leitet um.
    ///
    /// Wird bewusst nicht gefolgt: der Authorization-Header trägt das
    /// Automation-Secret, das nicht an ein Umleitungsziel gehen darf. Der
    /// häufigste Fall ist eine http-nach-https-Umleitung, und dann ist der
    /// richtige Hinweis, die Server-URL zu korrigieren.
    #[error("Der Server leitet die Anfrage um{}. Luchsr folgt Umleitungen nicht, weil dabei das Automation-Secret an das Umleitungsziel ginge. Bitte die Server-URL direkt auf das Ziel setzen — meist genügt https statt http.", location.as_deref().map(|l| format!(" auf {l}")).unwrap_or_default())]
    Redirected { location: Option<String> },

    /// Alles andere an HTTP-Fehlern, mit Statuscode und Serverdetail.
    #[error("Der Server antwortete mit HTTP {status}{}", detail.as_deref().map(|d| format!(": {d}")).unwrap_or_default())]
    HttpStatus {
        status: u16,
        /// `detail` bzw. `title` aus der problem+json-Antwort von CheckMK.
        detail: Option<String>,
    },

    /// Antwort war kein verwertbares JSON bzw. hatte eine andere Struktur.
    #[error("Die Antwort des Servers war nicht lesbar: {reason}")]
    Malformed { reason: String },

    /// Der Aufrufer hat abgebrochen — etwa weil manuell aktualisiert wurde.
    #[error("Die Anfrage wurde abgebrochen.")]
    Cancelled,
}

impl CheckmkError {
    /// Baut aus einem `reqwest::Error` den passenden Transportfehler.
    pub fn from_reqwest(error: &reqwest::Error) -> Self {
        let chain = error_chain(error);
        let flags = TransportFlags {
            reqwest_timeout: error.is_timeout(),
            reqwest_connect: error.is_connect(),
        };
        Self::Transport {
            cause: classify_transport(&chain, flags),
            chain,
        }
    }

    /// Ob ein erneuter Versuch überhaupt Sinn hat. Steuert das Backoff in
    /// Slice 5: bei falschem Secret bringt Wiederholen nichts, bei einem
    /// Netzaussetzer schon.
    pub fn is_retryable(&self) -> bool {
        match self {
            Self::Transport { cause, .. } => !cause.is_tls(),
            Self::HttpStatus { status, .. } => *status >= 500,
            Self::RateLimited => true,
            Self::InvalidUrl { .. }
            | Self::Unauthorized { .. }
            | Self::Forbidden { .. }
            | Self::NotFound { .. }
            | Self::PreconditionFailed { .. }
            | Self::Malformed { .. }
            | Self::Redirected { .. }
            | Self::Cancelled => false,
        }
    }
}

/// Hinweise, die `reqwest` selbst über den Fehler gibt.
#[derive(Debug, Clone, Copy, Default)]
pub struct TransportFlags {
    pub reqwest_timeout: bool,
    pub reqwest_connect: bool,
}

/// Sammelt `Display` über die gesamte `source()`-Kette.
pub fn error_chain(error: &dyn StdError) -> Vec<String> {
    let mut out = vec![error.to_string()];
    let mut current = error.source();
    // Obergrenze gegen zyklische Ketten, die es theoretisch geben kann.
    while let Some(inner) = current {
        out.push(inner.to_string());
        if out.len() >= 16 {
            break;
        }
        current = inner.source();
    }
    out
}

/* -------------------------------------------------------------------------- */
/* Klassifizierung                                                            */
/* -------------------------------------------------------------------------- */

// Windows-Sockelfehler (Winsock).
const WSAECONNREFUSED: i64 = 10061;
const WSAETIMEDOUT: i64 = 10060;
const WSAENETUNREACH: i64 = 10051;
const WSAEHOSTUNREACH: i64 = 10065;
const WSAHOST_NOT_FOUND: i64 = 11001;
const WSANO_DATA: i64 = 11004;
const WSAECONNRESET: i64 = 10054;
const WSAECONNABORTED: i64 = 10053;

// Zertifikatsfehler von Schannel. HRESULT als i32 gelesen, so schreibt
// Rusts io::Error sie in "(os error N)".
const CERT_E_EXPIRED: i64 = -2146762495; // 0x800B0101
const CERT_E_UNTRUSTEDROOT: i64 = -2146762487; // 0x800B0109
const CERT_E_CN_NO_MATCH: i64 = -2146762481; // 0x800B010F
const CERT_E_REVOKED: i64 = -2146762484; // 0x800B010C
const CERT_E_CHAINING: i64 = -2146762486; // 0x800B010A
const SEC_E_UNTRUSTED_ROOT: i64 = -2146893019; // 0x80090325
const SEC_E_CERT_EXPIRED: i64 = -2146893016; // 0x80090328
const SEC_E_WRONG_PRINCIPAL: i64 = -2146893022; // 0x80090322

/// Grenzt die Ursache aus einer Fehlerkette ein.
///
/// Reine Funktion, damit sie ohne Netzwerk testbar ist. Reihenfolge der
/// Prüfungen ist Absicht: spezifische Zertifikatscodes vor allgemeinem TLS,
/// TLS vor Zeitüberschreitung, weil ein hängender Handshake beides melden kann.
pub fn classify_transport(chain: &[String], flags: TransportFlags) -> TransportCause {
    let codes = os_error_codes(chain);
    let haystack = chain.join(" | ").to_ascii_lowercase();

    // 1 — Zertifikatsfehler über den Zahlencode. Am verlässlichsten.
    for code in &codes {
        match *code {
            CERT_E_UNTRUSTEDROOT | CERT_E_CHAINING | SEC_E_UNTRUSTED_ROOT => {
                return TransportCause::TlsUntrustedRoot
            }
            CERT_E_CN_NO_MATCH | SEC_E_WRONG_PRINCIPAL => {
                return TransportCause::TlsHostnameMismatch
            }
            CERT_E_EXPIRED | SEC_E_CERT_EXPIRED => return TransportCause::TlsExpired,
            CERT_E_REVOKED => return TransportCause::TlsRevoked,
            _ => {}
        }
    }

    // 2 — DNS. Der Rahmentext kommt aus hyper und ist nicht lokalisiert.
    if codes
        .iter()
        .any(|c| *c == WSAHOST_NOT_FOUND || *c == WSANO_DATA)
        || haystack.contains("failed to lookup address information")
        || haystack.contains("dns error")
        || haystack.contains("name or service not known")
    {
        return TransportCause::Dns;
    }

    // 3 — Verbindung abgewiesen.
    if codes.contains(&WSAECONNREFUSED)
        || haystack.contains("connection refused")
        || haystack.contains("actively refused")
    {
        return TransportCause::Refused;
    }

    // 4 — Netz oder Host nicht erreichbar.
    if codes
        .iter()
        .any(|c| *c == WSAENETUNREACH || *c == WSAEHOSTUNREACH)
        || haystack.contains("network is unreachable")
        || haystack.contains("no route to host")
    {
        return TransportCause::Unreachable;
    }

    // 5 — TLS ohne konkreten Zertifikatscode. Erst NACH den Netzprüfungen,
    // damit ein abgewiesener Port nicht als TLS-Fehler durchgeht.
    if haystack.contains("certificate")
        || haystack.contains("zertifikat")
        || haystack.contains("tls")
        || haystack.contains("ssl")
        || haystack.contains("handshake")
    {
        // "untrusted" bzw. "not trusted" auch als Text, falls kein Code kam.
        if haystack.contains("not trusted")
            || haystack.contains("untrusted")
            || haystack.contains("self signed")
            || haystack.contains("self-signed")
            || haystack.contains("unable to get local issuer")
        {
            return TransportCause::TlsUntrustedRoot;
        }
        if haystack.contains("expired") {
            return TransportCause::TlsExpired;
        }
        if haystack.contains("hostname") || haystack.contains("cn name") {
            return TransportCause::TlsHostnameMismatch;
        }
        return TransportCause::TlsOther;
    }

    // 6 — Zeitüberschreitung.
    if flags.reqwest_timeout
        || codes.contains(&WSAETIMEDOUT)
        || haystack.contains("timed out")
        || haystack.contains("timeout")
    {
        return TransportCause::Timeout;
    }

    // 7 — Abbruch einer stehenden Verbindung.
    if codes
        .iter()
        .any(|c| *c == WSAECONNRESET || *c == WSAECONNABORTED)
        || haystack.contains("connection reset")
        || haystack.contains("connection aborted")
        || haystack.contains("closed connection")
    {
        return TransportCause::Aborted;
    }

    // 8 — reqwest sagt "connect", ohne dass wir mehr wissen.
    if flags.reqwest_connect {
        return TransportCause::Unreachable;
    }

    TransportCause::Unknown
}

/// Liest alle Zahlen aus `(os error N)`-Vorkommen. `N` kann negativ sein.
fn os_error_codes(chain: &[String]) -> Vec<i64> {
    const MARKER: &str = "os error ";
    let mut out = Vec::new();
    for line in chain {
        let mut rest = line.as_str();
        while let Some(at) = rest.find(MARKER) {
            rest = &rest[at + MARKER.len()..];
            let mut end = 0;
            let bytes = rest.as_bytes();
            if end < bytes.len() && bytes[end] == b'-' {
                end += 1;
            }
            while end < bytes.len() && bytes[end].is_ascii_digit() {
                end += 1;
            }
            if let Ok(value) = rest[..end].parse::<i64>() {
                out.push(value);
            }
        }
    }
    out
}

/* -------------------------------------------------------------------------- */
/* Secret-Wrapper                                                             */
/* -------------------------------------------------------------------------- */

/// Hülle um das Automation-Secret.
///
/// Der Auftrag ist hier eindeutig: das Secret landet nie in der Config-Datei,
/// nie in Logs, nie im Frontend-State. `Debug` und `Display` geben deshalb
/// niemals den Inhalt heraus. Wer den Klartext braucht, muss [`Self::expose`]
/// aufrufen — und das fällt beim Lesen auf.
#[derive(Clone, PartialEq, Eq)]
pub struct Secret(String);

impl Secret {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Gibt den Klartext heraus. Nur für den Authorization-Header verwenden.
    pub fn expose(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Debug for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Länge ist unkritisch und hilft bei "ich habe nichts eingetragen".
        write!(f, "Secret(<{} Zeichen verborgen>)", self.0.len())
    }
}

impl fmt::Display for Secret {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<verborgen>")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Echte Fehlerkette eines deutschen Windows: interne CA nicht im
    /// Zertifikatspeicher. Genau der Fall, den native-tls/Schannel lösen soll.
    #[test]
    fn erkennt_nicht_vertrauenswuerdige_ca_auf_deutschem_windows() {
        let chain = vec![
            "error sending request for url (https://checkmk.example.intern/leosys/check_mk/api/1.0/version)".to_string(),
            "client error (Connect)".to_string(),
            "Die Zertifikatkette wurde von einer nicht vertrauenswürdigen Zertifizierungsstelle ausgestellt. (os error -2146762487)".to_string(),
        ];
        assert_eq!(
            classify_transport(
                &chain,
                TransportFlags {
                    reqwest_connect: true,
                    ..Default::default()
                }
            ),
            TransportCause::TlsUntrustedRoot
        );
    }

    /// Dieselbe Ursache auf englischem Windows — muss identisch erkannt
    /// werden, weil die Erkennung am Zahlencode hängt, nicht am Text.
    #[test]
    fn erkennt_nicht_vertrauenswuerdige_ca_sprachunabhaengig() {
        let deutsch = vec!["Die Zertifikatkette wurde von einer nicht vertrauenswürdigen Zertifizierungsstelle ausgestellt. (os error -2146762487)".to_string()];
        let englisch = vec!["The certificate chain was issued by an authority that is not trusted. (os error -2146762487)".to_string()];
        let flags = TransportFlags::default();
        assert_eq!(
            classify_transport(&deutsch, flags),
            classify_transport(&englisch, flags),
            "Die Klassifizierung darf nicht von der Windows-Sprache abhängen"
        );
        assert_eq!(
            classify_transport(&deutsch, flags),
            TransportCause::TlsUntrustedRoot
        );
    }

    #[test]
    fn erkennt_dns_fehler() {
        let chain = vec![
            "error sending request for url (https://tippfehler.example.intern/leosys/check_mk/api/1.0/version)".to_string(),
            "dns error".to_string(),
            "failed to lookup address information: Der angegebene Host ist unbekannt. (os error 11001)".to_string(),
        ];
        assert_eq!(
            classify_transport(
                &chain,
                TransportFlags {
                    reqwest_connect: true,
                    ..Default::default()
                }
            ),
            TransportCause::Dns
        );
    }

    #[test]
    fn erkennt_abgewiesene_verbindung() {
        let chain = vec![
            "error sending request for url (https://checkmk.example.intern:8443/)".to_string(),
            "Es konnte keine Verbindung hergestellt werden, da der Zielcomputer die Verbindung verweigerte. (os error 10061)".to_string(),
        ];
        assert_eq!(
            classify_transport(
                &chain,
                TransportFlags {
                    reqwest_connect: true,
                    ..Default::default()
                }
            ),
            TransportCause::Refused
        );
    }

    /// Ein geschlossener Port darf NICHT als TLS-Fehler durchgehen, obwohl in
    /// der URL "https" steht und der Rahmentext womöglich "tls" enthält.
    #[test]
    fn geschlossener_port_ist_kein_tls_fehler() {
        let chain = vec![
            "error sending request for url (https://checkmk.example.intern/leosys/)".to_string(),
            "tls connect error".to_string(),
            "Es konnte keine Verbindung hergestellt werden, da der Zielcomputer die Verbindung verweigerte. (os error 10061)".to_string(),
        ];
        let cause = classify_transport(
            &chain,
            TransportFlags {
                reqwest_connect: true,
                ..Default::default()
            },
        );
        assert_eq!(cause, TransportCause::Refused);
        assert!(!cause.is_tls());
    }

    #[test]
    fn erkennt_hostnamen_abweichung() {
        let chain = vec!["Der Zielprinzipalname ist falsch. (os error -2146893022)".to_string()];
        assert_eq!(
            classify_transport(&chain, TransportFlags::default()),
            TransportCause::TlsHostnameMismatch
        );
    }

    #[test]
    fn erkennt_abgelaufenes_zertifikat() {
        let chain =
            vec!["Das empfangene Zertifikat ist abgelaufen. (os error -2146762495)".to_string()];
        assert_eq!(
            classify_transport(&chain, TransportFlags::default()),
            TransportCause::TlsExpired
        );
    }

    #[test]
    fn erkennt_zeitueberschreitung_ueber_reqwest_flag() {
        let chain = vec!["operation timed out".to_string()];
        assert_eq!(
            classify_transport(
                &chain,
                TransportFlags {
                    reqwest_timeout: true,
                    ..Default::default()
                }
            ),
            TransportCause::Timeout
        );
    }

    #[test]
    fn erkennt_selbstsigniertes_zertifikat_ohne_code() {
        let chain = vec![
            "tls handshake eof".to_string(),
            "self signed certificate".to_string(),
        ];
        assert_eq!(
            classify_transport(&chain, TransportFlags::default()),
            TransportCause::TlsUntrustedRoot
        );
    }

    #[test]
    fn unbekanntes_bleibt_unbekannt() {
        let chain = vec!["irgendwas ganz anderes".to_string()];
        assert_eq!(
            classify_transport(&chain, TransportFlags::default()),
            TransportCause::Unknown
        );
    }

    #[test]
    fn liest_negative_und_positive_os_codes() {
        let chain = vec![
            "a (os error -2146762487) b (os error 10061)".to_string(),
            "c (os error 11001)".to_string(),
        ];
        assert_eq!(
            os_error_codes(&chain),
            vec![CERT_E_UNTRUSTEDROOT, WSAECONNREFUSED, WSAHOST_NOT_FOUND]
        );
    }

    #[test]
    fn os_code_leser_stolpert_nicht_ueber_muell() {
        assert!(os_error_codes(&["os error ".to_string()]).is_empty());
        assert!(os_error_codes(&["os error -".to_string()]).is_empty());
        assert!(os_error_codes(&["kein marker".to_string()]).is_empty());
        assert_eq!(os_error_codes(&["(os error 5)".to_string()]), vec![5]);
    }

    /// Jede Ursache muss einen deutschen Text haben — kein leerer String,
    /// kein englischer Rest.
    #[test]
    fn jede_ursache_hat_einen_klartext() {
        for cause in [
            TransportCause::Dns,
            TransportCause::Refused,
            TransportCause::Unreachable,
            TransportCause::TlsUntrustedRoot,
            TransportCause::TlsHostnameMismatch,
            TransportCause::TlsExpired,
            TransportCause::TlsRevoked,
            TransportCause::TlsOther,
            TransportCause::Timeout,
            TransportCause::Aborted,
            TransportCause::Unknown,
        ] {
            let text = cause.describe();
            assert!(!text.is_empty(), "{cause:?} hat keinen Text");
            assert!(
                text.ends_with('.'),
                "{cause:?}: Text sollte ein ganzer Satz sein, ist: {text}"
            );
        }
    }

    #[test]
    fn tls_erkennung_deckt_genau_die_tls_faelle_ab() {
        assert!(TransportCause::TlsUntrustedRoot.is_tls());
        assert!(TransportCause::TlsHostnameMismatch.is_tls());
        assert!(TransportCause::TlsExpired.is_tls());
        assert!(TransportCause::TlsRevoked.is_tls());
        assert!(TransportCause::TlsOther.is_tls());
        assert!(!TransportCause::Dns.is_tls());
        assert!(!TransportCause::Refused.is_tls());
        assert!(!TransportCause::Timeout.is_tls());
        assert!(!TransportCause::Unknown.is_tls());
    }

    /// Ein falsches Secret darf das Backoff nicht endlos beschäftigen.
    #[test]
    fn wiederholung_nur_wo_sie_sinn_hat() {
        assert!(!CheckmkError::Unauthorized { detail: None }.is_retryable());
        assert!(!CheckmkError::Forbidden { detail: None }.is_retryable());
        assert!(!CheckmkError::NotFound {
            site: "leosys".into(),
            detail: None
        }
        .is_retryable());
        assert!(!CheckmkError::Cancelled.is_retryable());
        assert!(CheckmkError::RateLimited.is_retryable());
        assert!(CheckmkError::HttpStatus {
            status: 503,
            detail: None
        }
        .is_retryable());
        assert!(!CheckmkError::HttpStatus {
            status: 400,
            detail: None
        }
        .is_retryable());

        // Netzaussetzer: wiederholen. Kaputtes Zertifikat: sinnlos.
        assert!(CheckmkError::Transport {
            cause: TransportCause::Dns,
            chain: vec![]
        }
        .is_retryable());
        assert!(!CheckmkError::Transport {
            cause: TransportCause::TlsUntrustedRoot,
            chain: vec![]
        }
        .is_retryable());
    }

    /// Das Secret darf über keinen der beiden Formatierungswege austreten.
    #[test]
    fn secret_tritt_nicht_aus() {
        let secret = Secret::new("SEHR-GEHEIM-1234567890");
        let via_debug = format!("{secret:?}");
        let via_display = format!("{secret}");
        let verschachtelt = format!("{:?}", vec![secret.clone()]);

        for rendered in [&via_debug, &via_display, &verschachtelt] {
            assert!(
                !rendered.contains("SEHR-GEHEIM"),
                "Secret ist ausgetreten: {rendered}"
            );
        }
        assert_eq!(secret.expose(), "SEHR-GEHEIM-1234567890");
        assert!(
            via_debug.contains("22"),
            "Länge darf sichtbar sein: {via_debug}"
        );
    }

    #[test]
    fn fehlermeldungen_sind_konkret_nicht_generisch() {
        // Der Auftrag verbietet generische Meldungen. Stichprobe: die
        // 404-Meldung muss den geprüften Site-Namen nennen.
        let error = CheckmkError::NotFound {
            site: "falsche-site".into(),
            detail: None,
        };
        let text = error.to_string();
        assert!(text.contains("falsche-site"), "Site fehlt in: {text}");
        assert!(text.contains("404"), "Statuscode fehlt in: {text}");

        // Die 401-Meldung muss den Automation-Secret-Stolperstein erwähnen.
        let text = CheckmkError::Unauthorized { detail: None }.to_string();
        assert!(
            text.contains("Automation-Secret"),
            "Hinweis auf Automation-Secret fehlt in: {text}"
        );
    }
}
