//! Zusammenbau der Endpunkt-URLs.
//!
//! Absichtlich von der HTTP-Schicht getrennt: so lässt sich der erzeugte
//! String im Test genau vergleichen, ohne Server und ohne Netzwerk. Genau das
//! braucht es hier, denn der Auftrag stellt eine harte Anforderung — `query`
//! und **jede** `columns`-Angabe müssen URL-encoded übergeben werden.
//!
//! Die Kodierung übernimmt `url::Url::query_pairs_mut`. Es wird nirgends von
//! Hand zusammengeklebt; ein `format!` mit `?columns=…&query=…` wäre genau der
//! Fehler, den diese Datei verhindern soll.

use url::Url;

use super::error::CheckmkError;

/// Pfadanteil, der laut Vertrag zwischen Site und Endpunkt liegt.
const API_PATH: &str = "check_mk/api/1.0";

/// Livestatus-Filter „Zustand ist schlechter als OK".
pub const PROBLEM_QUERY: &str = r#"{"op":">","left":"state","right":"0"}"#;

/// Spalten für den Serviceabruf, in der Reihenfolge des Vertrags.
pub const SERVICE_COLUMNS: &[&str] = &[
    "host_name",
    "description",
    "state",
    "plugin_output",
    "last_state_change",
    "acknowledged",
    "scheduled_downtime_depth",
    "is_flapping",
];

/// Spalten für den Hostabruf. Kein `description`, kein `is_flapping`.
pub const HOST_COLUMNS: &[&str] = &[
    "name",
    "state",
    "plugin_output",
    "last_state_change",
    "acknowledged",
    "scheduled_downtime_depth",
];

/// Basis-URL einer CheckMK-Site.
///
/// Hält Server und Site getrennt, weil beide in Fehlermeldungen einzeln
/// vorkommen — ein 404 zeigt auf den Site-Namen, ein DNS-Fehler auf den Server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SiteUrl {
    /// Immer mit abschliessendem Schrägstrich: `https://host/site/check_mk/api/1.0/`
    api_base: Url,
    /// Wurzel der Weboberfläche: `https://host/site/check_mk/`
    web_base: Url,
    site: String,
}

impl SiteUrl {
    /// Baut die Basis-URL aus Server und Site.
    ///
    /// Toleriert einen fehlenden Schrägstrich am Ende und ein fehlendes Schema
    /// (dann `https`). Verweigert dagegen einen Pfad im Serverfeld: wer dort
    /// `https://server/leosys/check_mk` einträgt, bekommt sonst eine URL mit
    /// doppeltem Pfad und einen 404, dessen Ursache nicht zu erraten ist.
    pub fn new(server: &str, site: &str) -> Result<Self, CheckmkError> {
        let server = server.trim();
        let site = site.trim();

        if server.is_empty() {
            return Err(CheckmkError::InvalidUrl {
                reason: "Es ist keine Server-URL eingetragen.".into(),
            });
        }
        validate_site(site)?;

        // Ohne Schema ergänzt der Parser nichts, er scheitert. Deshalb hier
        // https vorschalten — die realistische Annahme im Firmennetz.
        let with_scheme = if server.contains("://") {
            server.to_string()
        } else {
            format!("https://{server}")
        };

        let parsed = Url::parse(&with_scheme).map_err(|error| CheckmkError::InvalidUrl {
            reason: format!("„{server}“ ist keine gültige Adresse ({error})."),
        })?;

        match parsed.scheme() {
            "https" | "http" => {}
            other => {
                return Err(CheckmkError::InvalidUrl {
                    reason: format!(
                        "Das Protokoll „{other}“ wird nicht unterstützt, erwartet wird https oder http."
                    ),
                })
            }
        }

        if parsed.host_str().is_none() {
            return Err(CheckmkError::InvalidUrl {
                reason: format!("In „{server}“ fehlt der Hostname."),
            });
        }

        let path = parsed.path().trim_end_matches('/');
        if !path.is_empty() {
            return Err(CheckmkError::InvalidUrl {
                reason: format!(
                    "Die Server-URL darf keinen Pfad enthalten, gefunden wurde „{path}“. \
                     Der Site-Name gehört in das eigene Feld."
                ),
            });
        }

        if parsed.query().is_some() || parsed.fragment().is_some() {
            return Err(CheckmkError::InvalidUrl {
                reason: "Die Server-URL darf keine Parameter und kein Fragment enthalten.".into(),
            });
        }

        // Von der bereinigten Wurzel aus aufbauen, damit ein etwaiger Rest im
        // Pfad nicht mitwandert.
        let mut root = parsed.clone();
        root.set_path("/");
        root.set_query(None);
        root.set_fragment(None);

        let web_base =
            root.join(&format!("{site}/check_mk/"))
                .map_err(|error| CheckmkError::InvalidUrl {
                    reason: format!("Site-Name „{site}“ ergibt keine gültige URL ({error})."),
                })?;

        let api_base = web_base
            .join(&format!("{}/", API_PATH.trim_start_matches("check_mk/")))
            .map_err(|error| CheckmkError::InvalidUrl {
                reason: format!("API-Pfad liess sich nicht anhängen ({error})."),
            })?;

        Ok(Self {
            api_base,
            web_base,
            site: site.to_string(),
        })
    }

    pub fn site(&self) -> &str {
        &self.site
    }

    /// `https://host/site/check_mk/api/1.0/`
    pub fn api_base(&self) -> &Url {
        &self.api_base
    }

    /// Hostname für Fehlermeldungen.
    pub fn host(&self) -> &str {
        self.api_base.host_str().unwrap_or("")
    }

    /// Beliebiger Endpunkt unterhalb der API-Basis.
    fn endpoint(&self, relative: &str) -> Result<Url, CheckmkError> {
        self.api_base
            .join(relative)
            .map_err(|error| CheckmkError::InvalidUrl {
                reason: format!("Endpunkt „{relative}“ liess sich nicht bilden ({error})."),
            })
    }

    /// `GET /version` — für den Verbindungstest.
    pub fn version(&self) -> Result<Url, CheckmkError> {
        self.endpoint("version")
    }

    /// Serviceabruf, gefiltert auf Probleme.
    pub fn services(&self) -> Result<Url, CheckmkError> {
        let mut url = self.endpoint("domain-types/service/collections/all")?;
        append_columns_and_query(&mut url, SERVICE_COLUMNS);
        Ok(url)
    }

    /// Hostabruf, gefiltert auf Probleme.
    pub fn hosts(&self) -> Result<Url, CheckmkError> {
        let mut url = self.endpoint("domain-types/host/collections/all")?;
        append_columns_and_query(&mut url, HOST_COLUMNS);
        Ok(url)
    }

    /// Endpunkt zum Quittieren eines Services.
    pub fn acknowledge_service(&self) -> Result<Url, CheckmkError> {
        self.endpoint("domain-types/acknowledge/collections/service")
    }

    /// Endpunkt zum Quittieren eines Hosts.
    pub fn acknowledge_host(&self) -> Result<Url, CheckmkError> {
        self.endpoint("domain-types/acknowledge/collections/host")
    }

    /// Endpunkt für Service-Wartungszeiten.
    pub fn downtime_service(&self) -> Result<Url, CheckmkError> {
        self.endpoint("domain-types/downtime/collections/service")
    }

    /// Endpunkt für Host-Wartungszeiten.
    pub fn downtime_host(&self) -> Result<Url, CheckmkError> {
        self.endpoint("domain-types/downtime/collections/host")
    }

    // -----------------------------------------------------------------------
    // Seiten der Weboberfläche — für "In CheckMK öffnen"
    // -----------------------------------------------------------------------

    /// Detailansicht eines Services in der Weboberfläche.
    pub fn service_page(&self, host: &str, service: &str) -> Result<Url, CheckmkError> {
        let mut url = self
            .web_base
            .join("view.py")
            .map_err(|error| CheckmkError::InvalidUrl {
                reason: format!("Ansichts-URL liess sich nicht bilden ({error})."),
            })?;
        url.query_pairs_mut()
            .append_pair("view_name", "service")
            .append_pair("host", host)
            .append_pair("service", service);
        Ok(url)
    }

    /// Detailansicht eines Hosts in der Weboberfläche.
    pub fn host_page(&self, host: &str) -> Result<Url, CheckmkError> {
        let mut url = self
            .web_base
            .join("view.py")
            .map_err(|error| CheckmkError::InvalidUrl {
                reason: format!("Ansichts-URL liess sich nicht bilden ({error})."),
            })?;
        url.query_pairs_mut()
            .append_pair("view_name", "host")
            .append_pair("host", host);
        Ok(url)
    }

    /// Übersicht aller offenen Probleme — das Ziel von „CheckMK im Browser öffnen".
    pub fn overview_page(&self) -> Result<Url, CheckmkError> {
        let mut url = self
            .web_base
            .join("view.py")
            .map_err(|error| CheckmkError::InvalidUrl {
                reason: format!("Ansichts-URL liess sich nicht bilden ({error})."),
            })?;
        url.query_pairs_mut()
            .append_pair("view_name", "svcproblems");
        Ok(url)
    }
}

/// Hängt alle `columns` einzeln und dann `query` an — jeweils kodiert.
fn append_columns_and_query(url: &mut Url, columns: &[&str]) {
    let mut pairs = url.query_pairs_mut();
    for column in columns {
        pairs.append_pair("columns", column);
    }
    pairs.append_pair("query", PROBLEM_QUERY);
}

/// Prüft den Site-Namen.
///
/// CheckMK erlaubt Buchstaben, Ziffern und Unterstrich, maximal 16 Zeichen.
/// Streng zu prüfen ist hier besser als eine 404-Meldung später: ein
/// Schrägstrich im Site-Feld würde die Pfadstruktur zerlegen.
fn validate_site(site: &str) -> Result<(), CheckmkError> {
    if site.is_empty() {
        return Err(CheckmkError::InvalidUrl {
            reason: "Es ist kein Site-Name eingetragen.".into(),
        });
    }
    if site.len() > 16 {
        return Err(CheckmkError::InvalidUrl {
            reason: format!(
                "Der Site-Name „{site}“ ist zu lang, CheckMK erlaubt höchstens 16 Zeichen."
            ),
        });
    }
    if let Some(bad) = site
        .chars()
        .find(|c| !c.is_ascii_alphanumeric() && *c != '_')
    {
        return Err(CheckmkError::InvalidUrl {
            reason: format!(
                "Der Site-Name „{site}“ enthält das unerlaubte Zeichen „{bad}“. \
                 Erlaubt sind Buchstaben, Ziffern und Unterstrich."
            ),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn site() -> SiteUrl {
        SiteUrl::new("https://checkmk.example.intern", "leosys").unwrap()
    }

    /* ------------------------------------------------------- Basis-URL -- */

    #[test]
    fn baut_die_basis_url_nach_vertrag() {
        assert_eq!(
            site().api_base().as_str(),
            "https://checkmk.example.intern/leosys/check_mk/api/1.0/"
        );
    }

    #[test]
    fn abschliessender_schraegstrich_ist_egal() {
        let ohne = SiteUrl::new("https://checkmk.example.intern", "leosys").unwrap();
        let mit = SiteUrl::new("https://checkmk.example.intern/", "leosys").unwrap();
        assert_eq!(ohne, mit);
    }

    #[test]
    fn umgebende_leerzeichen_werden_entfernt() {
        let getrimmt = SiteUrl::new("  https://checkmk.example.intern  ", " leosys ").unwrap();
        assert_eq!(getrimmt, site());
    }

    #[test]
    fn fehlendes_schema_wird_zu_https() {
        let url = SiteUrl::new("checkmk.example.intern", "leosys").unwrap();
        assert_eq!(
            url.api_base().as_str(),
            "https://checkmk.example.intern/leosys/check_mk/api/1.0/"
        );
    }

    #[test]
    fn http_ist_erlaubt() {
        let url = SiteUrl::new("http://checkmk.example.intern", "leosys").unwrap();
        assert!(url.api_base().as_str().starts_with("http://"));
    }

    #[test]
    fn nicht_standard_port_bleibt_erhalten() {
        let url = SiteUrl::new("https://checkmk.example.intern:8443", "leosys").unwrap();
        assert_eq!(
            url.api_base().as_str(),
            "https://checkmk.example.intern:8443/leosys/check_mk/api/1.0/"
        );
    }

    /* ------------------------------------------------------ Ablehnungen -- */

    /// Der häufigste Bedienfehler: die komplette API-URL ins Serverfeld
    /// kopieren. Muss mit einer Meldung scheitern, die den Pfad nennt.
    #[test]
    fn pfad_im_serverfeld_wird_abgelehnt() {
        let error =
            SiteUrl::new("https://checkmk.example.intern/leosys/check_mk", "leosys").unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains("Pfad"),
            "Meldung nennt den Pfad nicht: {text}"
        );
        assert!(
            text.contains("/leosys/check_mk"),
            "Meldung zeigt den gefundenen Pfad nicht: {text}"
        );
    }

    #[test]
    fn leere_eingaben_werden_abgelehnt() {
        assert!(SiteUrl::new("", "leosys").is_err());
        assert!(SiteUrl::new("   ", "leosys").is_err());
        assert!(SiteUrl::new("https://checkmk.example.intern", "").is_err());
        assert!(SiteUrl::new("https://checkmk.example.intern", "  ").is_err());
    }

    #[test]
    fn fremdes_protokoll_wird_abgelehnt() {
        let error = SiteUrl::new("ftp://checkmk.example.intern", "leosys").unwrap_err();
        assert!(error.to_string().contains("ftp"));
    }

    /// Ein Schrägstrich im Site-Feld würde die Pfadstruktur zerlegen.
    #[test]
    fn schraegstrich_im_sitenamen_wird_abgelehnt() {
        let error = SiteUrl::new("https://checkmk.example.intern", "leosys/x").unwrap_err();
        let text = error.to_string();
        assert!(
            text.contains('/'),
            "Meldung nennt das Zeichen nicht: {text}"
        );
    }

    #[test]
    fn ungueltige_zeichen_im_sitenamen_werden_abgelehnt() {
        for kaputt in [
            "leosys site",
            "leosys.site",
            "leosys:1",
            "../etc",
            "leosys%2f",
        ] {
            assert!(
                SiteUrl::new("https://checkmk.example.intern", kaputt).is_err(),
                "„{kaputt}“ hätte abgelehnt werden müssen"
            );
        }
    }

    #[test]
    fn zu_langer_sitename_wird_abgelehnt() {
        let lang = "a".repeat(17);
        let error = SiteUrl::new("https://checkmk.example.intern", &lang).unwrap_err();
        assert!(error.to_string().contains("16"));
    }

    #[test]
    fn parameter_im_serverfeld_werden_abgelehnt() {
        assert!(SiteUrl::new("https://checkmk.example.intern/?a=b", "leosys").is_err());
        assert!(SiteUrl::new("https://checkmk.example.intern/#x", "leosys").is_err());
    }

    /* ----------------------------------------------------- Serviceabruf -- */

    /// Die zentrale Zusicherung: alle acht Spalten einzeln, dann der Filter,
    /// und alles kodiert.
    #[test]
    fn serviceabruf_entspricht_dem_vertrag() {
        let url = site().services().unwrap();
        assert_eq!(
            url.as_str(),
            "https://checkmk.example.intern/leosys/check_mk/api/1.0/\
             domain-types/service/collections/all\
             ?columns=host_name&columns=description&columns=state\
             &columns=plugin_output&columns=last_state_change&columns=acknowledged\
             &columns=scheduled_downtime_depth&columns=is_flapping\
             &query=%7B%22op%22%3A%22%3E%22%2C%22left%22%3A%22state%22%2C%22right%22%3A%220%22%7D"
        );
    }

    #[test]
    fn hostabruf_entspricht_dem_vertrag() {
        let url = site().hosts().unwrap();
        assert_eq!(
            url.as_str(),
            "https://checkmk.example.intern/leosys/check_mk/api/1.0/\
             domain-types/host/collections/all\
             ?columns=name&columns=state&columns=plugin_output\
             &columns=last_state_change&columns=acknowledged\
             &columns=scheduled_downtime_depth\
             &query=%7B%22op%22%3A%22%3E%22%2C%22left%22%3A%22state%22%2C%22right%22%3A%220%22%7D"
        );
    }

    /// Der Filter darf nicht rohes JSON in der URL hinterlassen.
    #[test]
    fn der_filter_ist_kodiert_nicht_roh() {
        let raw = site().services().unwrap().to_string();
        assert!(!raw.contains('{'), "geschweifte Klammer unkodiert: {raw}");
        assert!(!raw.contains('"'), "Anführungszeichen unkodiert: {raw}");
        // Aber dekodiert muss genau der Vertragsfilter herauskommen.
        let filter = site()
            .services()
            .unwrap()
            .query_pairs()
            .find(|(k, _)| k == "query")
            .map(|(_, v)| v.to_string())
            .unwrap();
        assert_eq!(filter, PROBLEM_QUERY);
    }

    /// Jede Spalte muss ein eigenes Paar sein, nicht eine kommaseparierte Liste.
    #[test]
    fn jede_spalte_ist_ein_eigenes_paar() {
        let url = site().services().unwrap();
        let columns: Vec<String> = url
            .query_pairs()
            .filter(|(k, _)| k == "columns")
            .map(|(_, v)| v.to_string())
            .collect();
        assert_eq!(columns, SERVICE_COLUMNS);
        assert!(
            columns.iter().all(|c| !c.contains(',')),
            "Spalten dürfen nicht zusammengefasst werden: {columns:?}"
        );
    }

    #[test]
    fn version_endpunkt() {
        assert_eq!(
            site().version().unwrap().as_str(),
            "https://checkmk.example.intern/leosys/check_mk/api/1.0/version"
        );
    }

    /* ---------------------------------------------------- Schreibpfade -- */

    #[test]
    fn schreibendpunkte_entsprechen_dem_vertrag() {
        let s = site();
        let base = "https://checkmk.example.intern/leosys/check_mk/api/1.0/domain-types";
        assert_eq!(
            s.acknowledge_service().unwrap().as_str(),
            format!("{base}/acknowledge/collections/service")
        );
        assert_eq!(
            s.acknowledge_host().unwrap().as_str(),
            format!("{base}/acknowledge/collections/host")
        );
        assert_eq!(
            s.downtime_service().unwrap().as_str(),
            format!("{base}/downtime/collections/service")
        );
        assert_eq!(
            s.downtime_host().unwrap().as_str(),
            format!("{base}/downtime/collections/host")
        );
    }

    /* -------------------------------------------------- Weboberfläche -- */

    #[test]
    fn serviceseite_fuer_in_checkmk_oeffnen() {
        let url = site()
            .service_page("leosys-sql-01", "Filesystem /var")
            .unwrap();
        assert_eq!(
            url.as_str(),
            "https://checkmk.example.intern/leosys/check_mk/view.py\
             ?view_name=service&host=leosys-sql-01&service=Filesystem+%2Fvar"
        );
    }

    /// Sonderzeichen in Service- und Hostnamen sind normal (Klammern,
    /// Leerzeichen, Umlaute) und müssen kodiert werden.
    #[test]
    fn sonderzeichen_in_namen_werden_kodiert() {
        let url = site()
            .service_page("host mit leer", "Interface ge-0/0/3 (Uplink) Größe")
            .unwrap();
        let raw = url.as_str();
        assert!(!raw.contains(' '), "Leerzeichen unkodiert: {raw}");
        assert!(!raw.contains('ö'), "Umlaut unkodiert: {raw}");

        // Und dekodiert wieder identisch.
        let service = url
            .query_pairs()
            .find(|(k, _)| k == "service")
            .map(|(_, v)| v.to_string())
            .unwrap();
        assert_eq!(service, "Interface ge-0/0/3 (Uplink) Größe");
    }

    /// Ein Ampersand im Servicenamen darf keinen zusätzlichen Parameter
    /// erzeugen — das wäre eine Parameterinjektion.
    #[test]
    fn ampersand_im_namen_erzeugt_keinen_zusatzparameter() {
        let url = site().service_page("h", "A&view_name=edit&x=1").unwrap();
        let views: Vec<String> = url
            .query_pairs()
            .filter(|(k, _)| k == "view_name")
            .map(|(_, v)| v.to_string())
            .collect();
        assert_eq!(views, vec!["service"], "view_name wurde überschrieben");
        assert!(url.query_pairs().all(|(k, _)| k != "x"));
    }

    #[test]
    fn hostseite_und_uebersicht() {
        let s = site();
        assert_eq!(
            s.host_page("leosys-esxi-03").unwrap().as_str(),
            "https://checkmk.example.intern/leosys/check_mk/view.py?view_name=host&host=leosys-esxi-03"
        );
        assert_eq!(
            s.overview_page().unwrap().as_str(),
            "https://checkmk.example.intern/leosys/check_mk/view.py?view_name=svcproblems"
        );
    }

    #[test]
    fn host_und_site_sind_einzeln_abfragbar() {
        let s = site();
        assert_eq!(s.site(), "leosys");
        assert_eq!(s.host(), "checkmk.example.intern");
    }
}
