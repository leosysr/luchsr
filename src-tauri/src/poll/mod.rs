//! Die Abrufschleife.
//!
//! Ein Hintergrund-Task, der in den vom Auftrag vorgegebenen Abständen abruft:
//! Intervall 15–600 s mit ±10 % Jitter, bei Fehlern exponentielles Backoff bis
//! höchstens 5 Minuten, Zeitgrenze je Abruf, kein Polling im Standby und ein
//! sofortiger Abruf nach dem Aufwachen.
//!
//! Die Rechnung dazu steht in [`schedule`] und ist dort getestet. Hier bleibt
//! die Schleife, die sie anwendet.
//!
//! ## Abbruch bei manueller Aktualisierung
//!
//! Der Auftrag verlangt, dass ein laufender Request bei manuellem Refresh
//! abgebrochen wird. Das braucht keinen eigenen Mechanismus: in Rust bricht ein
//! Future ab, wenn er verworfen wird, und `tokio::select!` verwirft den
//! Verlierer. Der Abruf steht deshalb in einem `select!` gegen das
//! Refresh-Signal — gewinnt das Signal, ist die HTTP-Verbindung weg und es
//! wird sofort neu abgerufen.
//!
//! ## Warum kein Power-Ereignis für den Standby
//!
//! Siehe [`schedule::looks_like_wakeup`]. Kurz: Wanduhrzeit vergleichen kostet
//! nichts und erkennt zusätzlich Fälle ohne Power-Ereignis, etwa eine
//! angehaltene virtuelle Maschine.

pub mod schedule;

use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::Utc;
use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, Runtime};
use tokio::sync::Notify;

use crate::checkmk::{CheckmkClient, ClientConfig, ProxyMode, Snapshot};
use crate::commands::AppState;
use crate::config::{Connection, SecretStore, Settings};
use crate::tray::{self, TrayState};

use schedule::Schedule;

/// Ereignisname, unter dem das Frontend den Zustand bekommt.
pub const STATUS_EVENT: &str = "luchsr://status";

/// Der Zustand, wie das Frontend ihn sieht.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatusPayload {
    pub snapshot: Option<Snapshot>,
    /// Meldung des letzten fehlgeschlagenen Abrufs, deutsch und konkret.
    pub error: Option<String>,
    /// Kürzel des Tray-Zustands, etwa `CRIT`.
    pub tray_state: &'static str,
    pub tooltip: String,
    /// Aufeinanderfolgende Fehlversuche. `0` heisst: letzter Abruf war gut.
    pub failures: u32,
    /// Ob überhaupt eine Verbindung eingerichtet ist.
    pub configured: bool,
}

/// Signal für „jetzt abrufen".
///
/// Eigener Typ statt `Arc<Notify>` direkt, damit der Zweck im Zustand lesbar
/// bleibt und nicht als anonymes Synchronisationsprimitiv herumsteht.
#[derive(Debug, Clone, Default)]
pub struct RefreshSignal(Arc<Notify>);

impl RefreshSignal {
    /// Weckt die Schleife. Verliert nichts, wenn niemand wartet — `Notify`
    /// merkt sich eine Berechtigung.
    pub fn trigger(&self) {
        self.0.notify_one();
    }

    async fn wait(&self) {
        self.0.notified().await;
    }
}

/// Fordert einen sofortigen Abruf an. Bricht einen laufenden ab.
pub fn request_refresh<R: Runtime>(app: &AppHandle<R>) {
    app.state::<AppState>().refresh_signal().trigger();
}

/// Startet die Schleife. Läuft, bis die Anwendung endet.
pub fn spawn<R: Runtime>(app: AppHandle<R>) {
    tauri::async_runtime::spawn(async move {
        run(app).await;
    });
}

async fn run<R: Runtime>(app: AppHandle<R>) {
    let refresh = app.state::<AppState>().refresh_signal();
    let mut cached: Option<(String, CheckmkClient)> = None;
    let mut plan = Schedule::default();

    // Erster Abruf sofort — das Tray-Icon steht bis dahin auf „getrennt",
    // und das soll nicht länger dauern als nötig.
    let mut delay = Duration::ZERO;
    let mut last_tick = Utc::now();

    // Der zuletzt protokollierte Zustand.
    //
    // Protokolliert werden **Wechsel**, nicht Durchläufe. Ein Log, das jede
    // Minute „unverändert" schreibt, ist nach einem Tag unlesbar; ein Log, das
    // nur die Übergänge zeigt, ist genau das, was man im Supportfall braucht.
    let mut logged: Option<TrayState> = None;

    log::info!(
        "Abrufschleife gestartet (Intervall {} s, Zeitgrenze {} s)",
        app.state::<AppState>().settings().polling.interval_seconds,
        app.state::<AppState>().settings().polling.timeout_seconds
    );

    loop {
        // ---------------------------------------------------------- warten --
        tokio::select! {
            _ = tokio::time::sleep(delay) => {}
            _ = refresh.wait() => {
                log::debug!("Abruf manuell angefordert");
            }
        }

        // ------------------------------------------------ Standby erkennen --
        let now = Utc::now();
        let elapsed = (now - last_tick).to_std().unwrap_or_default();
        last_tick = now;
        if schedule::looks_like_wakeup(delay, elapsed) {
            log::info!(
                "Aufwachen erkannt: {} s vergangen, {} s geplant — Backoff wird zurückgesetzt",
                elapsed.as_secs(),
                delay.as_secs()
            );
            // Nach dem Aufwachen ist die Lage eine andere. Ein Backoff, der
            // aus der Zeit vor dem Standby stammt, hilft niemandem.
            plan.success();
        }

        // ----------------------------------------------- Konfiguration ----
        let settings = app.state::<AppState>().settings();
        let connection = settings.active().clone();
        let interval = Duration::from_secs(u64::from(settings.polling.interval_seconds));

        if !connection.is_complete() {
            let state = publish(
                &app,
                None,
                Some(crate::i18n::status::NOT_CONFIGURED),
                &plan,
                false,
            );
            log_transition(
                &mut logged,
                state,
                Some(crate::i18n::status::NOT_CONFIGURED),
            );
            delay = interval;
            continue;
        }

        // ------------------------------------------------------- Client ----
        let print = fingerprint(&connection, settings.polling.timeout_seconds);
        if cached.as_ref().map(|(p, _)| p.as_str()) != Some(print.as_str()) {
            match build_client(&connection, &settings) {
                Ok(client) => cached = Some((print, client)),
                Err(message) => {
                    // Ein Client, der sich nicht bauen lässt, liegt an der
                    // Konfiguration oder am fehlenden Secret — Warten hilft
                    // nicht, also kein Backoff.
                    plan.failure(false);
                    let state = publish(&app, None, Some(&message), &plan, true);
                    log_transition(&mut logged, state, Some(&message));
                    delay = plan.delay(interval, jitter_seed());
                    continue;
                }
            }
        }
        let client = &cached.as_ref().expect("gerade gesetzt").1;

        // -------------------------------------------------------- abrufen --
        let outcome = tokio::select! {
            result = client.fetch_snapshot() => Some(result),
            _ = refresh.wait() => None,
        };

        match outcome {
            // Abgebrochen: sofort neu, ohne den Fehlerzähler zu erhöhen.
            None => {
                log::debug!("laufender Abruf zugunsten einer manuellen Anforderung abgebrochen");
                delay = Duration::ZERO;
                continue;
            }
            Some(Ok(snapshot)) => {
                plan.success();
                // Melden, bevor veröffentlicht wird: `publish` verbraucht den
                // Abzug, und die Entscheidung braucht ihn noch. Reihenfolge
                // sonst gleichgültig — beides ist schnell und ohne Netz.
                crate::notify::announce(&app, &snapshot);
                let state = publish(&app, Some(snapshot), None, &plan, true);
                log_transition(&mut logged, state, None);
            }
            Some(Err(error)) => {
                // Die Art des Fehlers entscheidet über das Backoff — siehe
                // Schedule::delay. Ein falsches Secret löst sich nicht durch
                // Warten, ein Netzaussetzer schon.
                plan.failure(error.is_retryable());
                let message = error.to_string();
                log::warn!(
                    "Abruf fehlgeschlagen ({}. Versuch): {message}",
                    plan.failures
                );
                let state = publish(&app, keep_last_snapshot(&app), Some(&message), &plan, true);
                log_transition(&mut logged, state, Some(&message));
            }
        }

        delay = plan.delay(interval, jitter_seed());
        log::debug!("nächster Abruf in {} s", delay.as_secs());
    }
}

/// Protokolliert einen **Zustandswechsel**, keinen Durchlauf.
///
/// Ein Log, das jede Minute „unverändert" schreibt, ist nach einem Tag
/// unlesbar. Ein Log, das nur die Übergänge zeigt, ist genau die Spur, die man
/// im Supportfall braucht: wann ging es auf CRIT, wann war die Verbindung weg.
fn log_transition(logged: &mut Option<TrayState>, state: TrayState, error: Option<&str>) {
    if *logged == Some(state) {
        return;
    }
    let von = logged.map_or("—", TrayState::label);
    match error {
        Some(message) => log::info!("Zustand {von} → {} ({message})", state.label()),
        None => log::info!("Zustand {von} → {}", state.label()),
    }
    *logged = Some(state);
}

/* -------------------------------------------------------------------------- */
/* Hilfsfunktionen                                                            */
/* -------------------------------------------------------------------------- */

/// Kennzeichnet die verbindungsrelevanten Einstellungen.
///
/// Ändert sich der Abdruck, wird der HTTP-Client neu gebaut. Ihn bei jedem
/// Durchlauf neu zu bauen würde jede Minute einen TLS-Aufbau kosten.
fn fingerprint(connection: &Connection, timeout_seconds: u32) -> String {
    format!(
        "{}|{}|{}|{}|{:?}|{}",
        connection.server,
        connection.site,
        connection.username,
        connection.verify_tls,
        connection.proxy,
        timeout_seconds
    )
}

fn build_client(connection: &Connection, settings: &Settings) -> Result<CheckmkClient, String> {
    let secret = SecretStore::load(&connection.username).map_err(|error| match error {
        crate::config::SecretError::NotFound { .. } => crate::i18n::status::NO_SECRET.to_string(),
        other => other.to_string(),
    })?;

    let config = ClientConfig {
        server: connection.server.clone(),
        site: connection.site.clone(),
        username: connection.username.clone(),
        secret,
        verify_tls: connection.verify_tls,
        proxy: ProxyMode::from(&connection.proxy),
        timeout: Duration::from_secs(u64::from(settings.polling.timeout_seconds)),
    };
    CheckmkClient::new(&config).map_err(|error| error.to_string())
}

/// Der letzte erfolgreiche Abzug.
///
/// Bei einem Fehler bleibt er stehen: die Liste soll nicht leer werden, nur
/// weil der Server kurz nicht antwortet. Dass die Daten alt sind, sagt das
/// Tray-Icon und der Tooltip.
fn keep_last_snapshot<R: Runtime>(app: &AppHandle<R>) -> Option<Snapshot> {
    app.state::<AppState>().snapshot()
}

/// Schreibt den Zustand in den AppState, aktualisiert das Tray und meldet ihn
/// ans Frontend.
fn publish<R: Runtime>(
    app: &AppHandle<R>,
    snapshot: Option<Snapshot>,
    error: Option<&str>,
    plan: &Schedule,
    configured: bool,
) -> TrayState {
    let state = app.state::<AppState>();
    let include_handled = !state.settings().behaviour.hide_handled;

    let tray_state = if configured {
        tray::tray_state(snapshot.as_ref(), plan.is_failing(), include_handled)
    } else {
        TrayState::Disconnected
    };
    let tip = tray::tooltip(snapshot.as_ref(), error, include_handled);

    state.set_status(
        snapshot.clone(),
        error.map(str::to_string),
        plan.failures,
        configured,
    );
    tray::update(app, tray_state, &tip);

    let payload = StatusPayload {
        snapshot,
        error: error.map(str::to_string),
        tray_state: tray_state.label(),
        tooltip: tip,
        failures: plan.failures,
        configured,
    };
    if let Err(error) = app.emit(STATUS_EVENT, payload) {
        log::warn!("Zustand liess sich nicht ans Fenster melden: {error}");
    }
    tray_state
}

/// Zufallswert in `[0, 1)` für den Jitter.
///
/// Aus den Nanosekunden der Systemuhr statt aus `rand`. Für eine Streuung von
/// ±10 % auf ein Intervall von Minuten ist das mehr als genug Entropie, und es
/// spart eine Abhängigkeit — die Rechnung selbst bleibt über den Parameter
/// testbar.
fn jitter_seed() -> f64 {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.subsec_nanos());
    f64::from(nanos) / 1_000_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ProxyConfig;

    fn connection() -> Connection {
        Connection {
            id: "default".into(),
            name: String::new(),
            server: "https://checkmk.example.intern".into(),
            site: "leosys".into(),
            username: "m.mustermann".into(),
            verify_tls: true,
            proxy: ProxyConfig::System,
        }
    }

    /* --------------------------------------------------------- Fingerprint */

    #[test]
    fn abdruck_aendert_sich_bei_jedem_relevanten_feld() {
        let basis = fingerprint(&connection(), 10);

        let mut anders = connection();
        anders.server = "https://andere.intern".into();
        assert_ne!(fingerprint(&anders, 10), basis, "Server");

        let mut anders = connection();
        anders.site = "andere".into();
        assert_ne!(fingerprint(&anders, 10), basis, "Site");

        let mut anders = connection();
        anders.username = "andere".into();
        assert_ne!(fingerprint(&anders, 10), basis, "Benutzername");

        let mut anders = connection();
        anders.verify_tls = false;
        assert_ne!(fingerprint(&anders, 10), basis, "TLS-Prüfung");

        let mut anders = connection();
        anders.proxy = ProxyConfig::None;
        assert_ne!(fingerprint(&anders, 10), basis, "Proxy-Modus");

        let mut anders = connection();
        anders.proxy = ProxyConfig::Manual {
            url: "http://p:8080".into(),
        };
        assert_ne!(fingerprint(&anders, 10), basis, "Proxy-Adresse");

        assert_ne!(fingerprint(&connection(), 30), basis, "Zeitgrenze");
    }

    /// Felder, die den HTTP-Client nicht betreffen, dürfen ihn nicht neu bauen.
    #[test]
    fn abdruck_ignoriert_belanglose_felder() {
        let basis = fingerprint(&connection(), 10);
        let mut anders = connection();
        anders.name = "Anzeigename".into();
        anders.id = "andere-id".into();
        assert_eq!(fingerprint(&anders, 10), basis);
    }

    /// Der Abdruck darf das Secret nicht enthalten — er landet in Protokollen.
    #[test]
    fn abdruck_enthaelt_kein_secret() {
        let text = fingerprint(&connection(), 10).to_lowercase();
        for verdaechtig in ["secret", "password", "passwort", "token"] {
            assert!(!text.contains(verdaechtig), "„{verdaechtig}“ im Abdruck");
        }
    }

    /* ------------------------------------------------------------- Jitter */

    #[test]
    fn jitter_liegt_im_erlaubten_bereich() {
        for _ in 0..200 {
            let value = jitter_seed();
            assert!(
                (0.0..1.0).contains(&value),
                "{value} liegt aussserhalb [0,1)"
            );
        }
    }

    /* ------------------------------------------------------------- Signal */

    #[tokio::test]
    async fn signal_geht_nicht_verloren_wenn_niemand_wartet() {
        let signal = RefreshSignal::default();
        // Auslösen, bevor gewartet wird.
        signal.trigger();
        // Muss sofort zurückkommen, nicht hängen.
        tokio::time::timeout(Duration::from_millis(200), signal.wait())
            .await
            .expect("die gespeicherte Berechtigung muss den Wartenden sofort wecken");
    }

    #[tokio::test]
    async fn signal_weckt_einen_wartenden() {
        let signal = RefreshSignal::default();
        let kopie = signal.clone();
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(20)).await;
            kopie.trigger();
        });
        tokio::time::timeout(Duration::from_millis(500), signal.wait())
            .await
            .expect("Signal kam nicht an");
    }

    /* ------------------------------------------------------ Ereignisform */

    /// Das Frontend liest camelCase.
    #[test]
    fn ereignisform_ist_camelcase() {
        let payload = StatusPayload {
            snapshot: None,
            error: None,
            tray_state: "OK",
            tooltip: "Luchsr — Keine offenen Probleme".into(),
            failures: 0,
            configured: true,
        };
        let json = serde_json::to_value(&payload).unwrap();
        let object = json.as_object().unwrap();
        for key in [
            "snapshot",
            "error",
            "trayState",
            "tooltip",
            "failures",
            "configured",
        ] {
            assert!(object.contains_key(key), "{key} fehlt in {json}");
        }
        assert_eq!(object.len(), 6);
    }
}
