//! CheckMK-REST-API — Client, Datenstrukturen, Fehlertypen.
//!
//! Basis-URL: `{server}/{site}/check_mk/api/1.0`
//! Auth-Header: `Authorization: Bearer {username} {secret}`
//!
//! ## Aufbau
//!
//! | Datei       | Inhalt                                                        |
//! |-------------|---------------------------------------------------------------|
//! | [`error`]   | Fehlertypen, Ursachenerkennung, [`Secret`]-Hülle              |
//! | [`model`]   | Antwortstrukturen und das Domänenmodell darüber               |
//! | [`url`]     | Endpunkt-URLs samt Kodierung von `columns` und `query`         |
//! | [`write`]   | Nutzlasten für Quittieren und Wartungszeit                     |
//! | [`client`]  | HTTP-Schicht und die reinen Auswertungsfunktionen              |
//!
//! ## Prüfbarkeit ohne Server
//!
//! Der Auftrag verlangt Unit-Tests gegen aufgezeichnete JSON-Fixtures, ohne
//! Live-Server. Deshalb ist alles Auswertende in reine Funktionen gezogen:
//! URL-Bau, Antwortauswertung, Fehlerklassifizierung, Nutzlastbau und die
//! Zeitfenster der Wartungsdauern sind einzeln testbar. Übrig bleibt in
//! `client.rs` nur das Zusammensetzen — der Teil, für den es einen Server
//! bräuchte, und der deshalb dünn gehalten ist.
//!
//! Die Fixtures liegen in `fixtures/` und werden mit `include_str!` eingebettet.

pub mod client;
pub mod error;
pub mod model;
pub mod proxy;
pub mod url;
pub mod write;

pub use client::{
    parse_services, CheckmkClient, ClientConfig, ConnectionReport, ProxyMode,
    DEFAULT_TIMEOUT_SECONDS,
};
pub use error::{CheckmkError, Secret, TransportCause};
pub use model::{Problem, ProblemState, Snapshot, StateCounts, VersionInfo};
pub use proxy::{proxy_for_host, ProxyEnv};
pub use url::SiteUrl;
pub use write::{
    AcknowledgeOptions, DowntimeDuration, DowntimeHostBody, DowntimeServiceBody, MORNING_HOUR,
};
