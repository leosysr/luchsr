//! Erkennung von Proxy-Einstellungen der Umgebung.
//!
//! ## Warum das nötig ist
//!
//! `reqwest` liest bei aktivem Proxy-Feature die Umgebungsvariablen
//! `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY` und `NO_PROXY`. Windows-Programme
//! wie der Browser oder .NET nehmen dagegen die Einstellungen aus WinINET oder
//! WinHTTP. Diese Quellen können auseinanderlaufen — und tun es in Firmennetzen
//! regelmässig.
//!
//! Der beobachtete Fall: `HTTP_PROXY` zeigt auf einen Firmenproxy, `NO_PROXY`
//! führt nur `localhost` und `.local`, in WinINET ist der Proxy **abgeschaltet**.
//! Der Browser erreicht den internen CheckMK-Server direkt, Luchsr schickt die
//! Anfrage an den Proxy, und der antwortet mit `403 Forbidden` und einer
//! HTML-Seite. Für den Benutzer sieht das aus wie ein Berechtigungsproblem in
//! CheckMK — es ist aber keins, und ohne Hinweis sucht man an der falschen
//! Stelle.
//!
//! Deshalb wird die Umgebung ausgelesen und im Einstellungsdialog gemeldet,
//! wenn Anfragen an den konfigurierten Server über einen Proxy laufen würden.
//!
//! ## Aufbau
//!
//! [`ProxyEnv`] hält die Variablen als **Daten**. Das Lesen der Umgebung ist
//! ein einziger Aufruf, alles andere sind reine Funktionen — sonst wären die
//! Umgehungsregeln nicht testbar, ohne Umgebungsvariablen zu verbiegen, was in
//! parallel laufenden Tests unzuverlässig ist.

/// Die proxyrelevanten Umgebungsvariablen.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProxyEnv {
    pub http_proxy: Option<String>,
    pub https_proxy: Option<String>,
    pub all_proxy: Option<String>,
    pub no_proxy: Option<String>,
}

impl ProxyEnv {
    /// Liest die Umgebung. Gross- und Kleinschreibung wird beide geprüft, weil
    /// beides gebräuchlich ist und Windows sie nicht unterscheidet.
    pub fn from_environment() -> Self {
        Self {
            http_proxy: first_var(&["HTTP_PROXY", "http_proxy"]),
            https_proxy: first_var(&["HTTPS_PROXY", "https_proxy"]),
            all_proxy: first_var(&["ALL_PROXY", "all_proxy"]),
            no_proxy: first_var(&["NO_PROXY", "no_proxy"]),
        }
    }

    /// Der Proxy, der für dieses Schema gelten würde.
    fn proxy_for_scheme(&self, scheme: &str) -> Option<&str> {
        let specific = if scheme.eq_ignore_ascii_case("https") {
            self.https_proxy.as_deref()
        } else {
            self.http_proxy.as_deref()
        };
        specific
            .or(self.all_proxy.as_deref())
            .map(str::trim)
            .filter(|value| !value.is_empty())
    }
}

fn first_var(names: &[&str]) -> Option<String> {
    names
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .map(|value| value.trim().to_string())
        .find(|value| !value.is_empty())
}

/// Würde eine Anfrage an `host` über einen Proxy laufen? Wenn ja, über welchen.
///
/// Gibt `None` zurück, wenn kein Proxy gesetzt ist **oder** `NO_PROXY` den Host
/// umfasst.
pub fn proxy_for_host(host: &str, scheme: &str, env: &ProxyEnv) -> Option<String> {
    let proxy = env.proxy_for_scheme(scheme)?;
    if bypassed(host, env.no_proxy.as_deref()) {
        return None;
    }
    Some(proxy.to_string())
}

/// Prüft `host` gegen eine `NO_PROXY`-Liste.
///
/// Umgesetzt sind die Regeln, die in der Praxis vorkommen:
///
/// * `*` umfasst alles
/// * ein Eintrag mit führendem Punkt trifft die Domäne und alle Unterdomänen
/// * ein Eintrag ohne Punkt trifft genau diesen Namen — und ebenfalls seine
///   Unterdomänen, so verhalten sich curl und reqwest
/// * ein angehängter Port wird abgeschnitten
///
/// **Nicht** umgesetzt sind CIDR-Bereiche wie `10.0.0.0/8`. Die kommen in
/// `NO_PROXY` selten vor und werden auch von reqwest nur teilweise
/// unterstützt. Folge: die Prüfung meldet in so einem Fall einen Proxy, den es
/// praktisch nicht gibt. Das ist die richtige Richtung für einen Irrtum — ein
/// Hinweis zu viel kostet einen Blick, ein fehlender Hinweis eine halbe Stunde.
fn bypassed(host: &str, no_proxy: Option<&str>) -> bool {
    let Some(list) = no_proxy else {
        return false;
    };
    let host = host.trim().trim_end_matches('.').to_ascii_lowercase();
    if host.is_empty() {
        return false;
    }

    for raw in list.split(',') {
        let entry = raw.trim().to_ascii_lowercase();
        if entry.is_empty() {
            continue;
        }
        if entry == "*" {
            return true;
        }
        // Port abschneiden, aber nicht bei einer IPv6-Adresse in Klammern.
        let entry = match entry.rsplit_once(':') {
            Some((left, right))
                if right.chars().all(|c| c.is_ascii_digit()) && !left.is_empty() =>
            {
                left.to_string()
            }
            _ => entry,
        };
        let entry = entry.trim_start_matches('.').trim_end_matches('.');
        if entry.is_empty() {
            continue;
        }
        if host == entry || host.ends_with(&format!(".{entry}")) {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env(http: Option<&str>, no: Option<&str>) -> ProxyEnv {
        ProxyEnv {
            http_proxy: http.map(str::to_string),
            https_proxy: None,
            all_proxy: None,
            no_proxy: no.map(str::to_string),
        }
    }

    /* ------------------------------------------------- Der beobachtete Fall */

    /// Genau die Umgebung, die den 403 verursacht hat: Firmenproxy gesetzt,
    /// `NO_PROXY` deckt nur localhost, Ziel ist ein interner Server.
    #[test]
    fn erkennt_den_beobachteten_fall() {
        let umgebung = env(
            Some("http://10.0.0.53:8080"),
            Some("localhost,127.0.0.1,::1,.local"),
        );
        assert_eq!(
            proxy_for_host("10.0.0.26", "http", &umgebung),
            Some("http://10.0.0.53:8080".to_string()),
            "der interne Server steht nicht in NO_PROXY und würde proxied"
        );
    }

    #[test]
    fn ohne_proxy_variablen_kein_hinweis() {
        assert_eq!(
            proxy_for_host("10.0.0.26", "http", &ProxyEnv::default()),
            None
        );
        assert_eq!(
            proxy_for_host("host", "http", &env(Some("   "), None)),
            None
        );
    }

    /* -------------------------------------------------------- NO_PROXY ---- */

    #[test]
    fn exakter_name_wird_umgangen() {
        let umgebung = env(Some("http://proxy:8080"), Some("checkmk.intern"));
        assert_eq!(proxy_for_host("checkmk.intern", "http", &umgebung), None);
        assert!(proxy_for_host("checkmk.example.com", "http", &umgebung).is_some());
    }

    #[test]
    fn fuehrender_punkt_umfasst_unterdomaenen() {
        let umgebung = env(Some("http://proxy:8080"), Some(".intern"));
        assert_eq!(proxy_for_host("checkmk.intern", "http", &umgebung), None);
        assert_eq!(proxy_for_host("a.b.intern", "http", &umgebung), None);
        assert!(proxy_for_host("intern.example.com", "http", &umgebung).is_some());
    }

    /// curl und reqwest behandeln einen Eintrag ohne Punkt ebenfalls als Suffix.
    #[test]
    fn eintrag_ohne_punkt_umfasst_auch_unterdomaenen() {
        let umgebung = env(Some("http://proxy:8080"), Some("intern"));
        assert_eq!(proxy_for_host("checkmk.intern", "http", &umgebung), None);
        assert_eq!(proxy_for_host("intern", "http", &umgebung), None);
    }

    #[test]
    fn stern_umfasst_alles() {
        let umgebung = env(Some("http://proxy:8080"), Some("*"));
        assert_eq!(proxy_for_host("egal.example.com", "http", &umgebung), None);
    }

    #[test]
    fn port_im_eintrag_wird_abgeschnitten() {
        let umgebung = env(Some("http://proxy:8080"), Some("checkmk.intern:80"));
        assert_eq!(proxy_for_host("checkmk.intern", "http", &umgebung), None);
    }

    #[test]
    fn gross_und_kleinschreibung_ist_egal() {
        let umgebung = env(Some("http://proxy:8080"), Some("CheckMK.INTERN"));
        assert_eq!(proxy_for_host("checkmk.intern", "http", &umgebung), None);
        assert_eq!(proxy_for_host("CHECKMK.Intern", "http", &umgebung), None);
    }

    #[test]
    fn leere_und_verrutschte_eintraege_stoeren_nicht() {
        let umgebung = env(Some("http://proxy:8080"), Some(",, . ,checkmk.intern , "));
        assert_eq!(proxy_for_host("checkmk.intern", "http", &umgebung), None);
        assert!(proxy_for_host("andere.intern", "http", &umgebung).is_some());
    }

    #[test]
    fn abschliessender_punkt_im_hostnamen_stoert_nicht() {
        let umgebung = env(Some("http://proxy:8080"), Some("checkmk.intern"));
        assert_eq!(proxy_for_host("checkmk.intern.", "http", &umgebung), None);
    }

    /// Eine Teilzeichenkette darf nicht treffen: `mk.intern` ist nicht
    /// `checkmk.intern`.
    #[test]
    fn teilzeichenkette_trifft_nicht() {
        let umgebung = env(Some("http://proxy:8080"), Some("mk.intern"));
        assert!(
            proxy_for_host("checkmk.intern", "http", &umgebung).is_some(),
            "checkmk.intern endet nicht auf .mk.intern"
        );
    }

    /* ----------------------------------------------------------- Schemata */

    #[test]
    fn https_nutzt_die_eigene_variable() {
        let umgebung = ProxyEnv {
            http_proxy: Some("http://nur-fuer-http:8080".into()),
            https_proxy: Some("http://nur-fuer-https:8080".into()),
            all_proxy: None,
            no_proxy: None,
        };
        assert_eq!(
            proxy_for_host("h", "https", &umgebung).as_deref(),
            Some("http://nur-fuer-https:8080")
        );
        assert_eq!(
            proxy_for_host("h", "http", &umgebung).as_deref(),
            Some("http://nur-fuer-http:8080")
        );
        assert_eq!(
            proxy_for_host("h", "HTTPS", &umgebung).as_deref(),
            Some("http://nur-fuer-https:8080"),
            "Schema wird ohne Rücksicht auf Gross- und Kleinschreibung geprüft"
        );
    }

    #[test]
    fn all_proxy_greift_als_rueckfall() {
        let umgebung = ProxyEnv {
            http_proxy: None,
            https_proxy: None,
            all_proxy: Some("http://fuer-alles:8080".into()),
            no_proxy: None,
        };
        assert_eq!(
            proxy_for_host("h", "http", &umgebung).as_deref(),
            Some("http://fuer-alles:8080")
        );
        assert_eq!(
            proxy_for_host("h", "https", &umgebung).as_deref(),
            Some("http://fuer-alles:8080")
        );
    }

    /// Die schemaspezifische Variable hat Vorrang vor ALL_PROXY.
    #[test]
    fn schemavariable_schlaegt_all_proxy() {
        let umgebung = ProxyEnv {
            http_proxy: Some("http://spezifisch:8080".into()),
            https_proxy: None,
            all_proxy: Some("http://allgemein:8080".into()),
            no_proxy: None,
        };
        assert_eq!(
            proxy_for_host("h", "http", &umgebung).as_deref(),
            Some("http://spezifisch:8080")
        );
    }

    /* --------------------------------------------------------- Grenzfälle */

    #[test]
    fn leerer_hostname_wird_nicht_umgangen() {
        let umgebung = env(Some("http://proxy:8080"), Some("*"));
        // Bei leerem Host greift die Umgehungsprüfung nicht — der Aufrufer hat
        // dann ohnehin ein grösseres Problem als den Proxy.
        assert!(proxy_for_host("", "http", &umgebung).is_some());
    }

    /// CIDR wird bewusst nicht unterstützt. Der Test hält die Folge fest,
    /// damit sie nicht als Fehler missverstanden wird.
    #[test]
    fn cidr_wird_nicht_unterstuetzt_und_meldet_deshalb_einen_proxy() {
        let umgebung = env(Some("http://proxy:8080"), Some("10.0.0.0/8"));
        assert!(
            proxy_for_host("10.0.0.26", "http", &umgebung).is_some(),
            "ein Hinweis zu viel ist besser als ein fehlender"
        );
    }
}
