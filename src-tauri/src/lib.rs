//! Luchsr — CheckMK-Statusmeldungen im Windows-Infobereich.
//!
//! Zielplattform ist ausschliesslich Windows 11 x64. Plattformspezifische
//! Vereinfachungen sind erwünscht; es entsteht kein Cross-Platform-Code.
//!
//! Aufbau der Module (entsteht slice-weise, siehe CLAUDE.md):
//!
//! | Modul       | Slice | Aufgabe                                           |
//! |-------------|-------|---------------------------------------------------|
//! | `checkmk`   | 3     | API-Client, Datenstrukturen, Fehlertypen           |
//! | `config`    | 4     | config.json, defaults.json, Credential Manager     |
//! | `export`    | 6     | CSV-Ausgabe der Problemliste                       |
//! | `commands`  | 4     | Tauri-Befehle als Brücke zum Frontend              |
//! | `i18n`      | 5     | Texte der nativen Oberflächenteile                 |
//! | `tray`      | 5     | Icon-Zustände, Kontextmenü, Fensterposition        |
//! | `poll`      | 5     | Abrufschleife: Jitter, Backoff, Standby            |
//! | `actions`   | 7     | Berechtigungen, Kommentarvorlage                   |
//! | `notify`    | 8     | Toasts, Signalton, Entscheidung was gemeldet wird  |
//! | `startup`   | 9     | Autostart, Einzelinstanz, Fenster beim Start       |

pub mod actions;
pub mod checkmk;
pub mod commands;
pub mod config;
pub mod export;
pub mod i18n;
pub mod notify;
pub mod poll;
pub mod startup;
pub mod tray;

use commands::AppState;
use tauri::Manager;

/// Startet die Anwendung.
pub fn run() {
    // Einstellungen vor dem Fenster laden. Scheitert das, liegt es an der
    // Umgebung (fehlendes %APPDATA%) und nicht an der Konfiguration — dann ist
    // ein Abbruch mit Meldung richtig, weil nichts davon reparierbar ist.
    let (state, outcome) =
        AppState::initialise().expect("Die Einstellungen liessen sich nicht laden");

    for notice in &outcome.notices {
        eprintln!("Luchsr: {notice}");
    }

    // Wie gestartet wurde, entscheidet über das Fenster — siehe `startup`.
    let per_autostart = startup::launched_by_autostart(std::env::args());
    let fenster = startup::window_on_start(
        per_autostart,
        outcome.settings.behaviour.start_minimised,
        outcome.needs_setup,
    );

    tauri::Builder::default()
        // Einzelinstanz **zuerst**: der Rückruf muss greifen, bevor irgendein
        // anderes Plugin oder das Fenster aufgebaut wird. Sonst startet die
        // zweite Instanz halb, bevor sie sich beendet — im ungünstigen Fall mit
        // einem zweiten Tray-Icon, das gleich wieder verschwindet.
        .plugin(tauri_plugin_single_instance::init(|app, argv, _cwd| {
            match startup::second_instance_action(&argv) {
                startup::SecondInstance::Focus => {
                    log::info!("zweiter Start — vorhandenes Fenster nach vorne");
                    tray::show_popup(app, None);
                }
                startup::SecondInstance::Ignore => {
                    log::info!("zweiter Start trug die Autostartmarke — nichts zu tun");
                }
            }
        }))
        // Autostart: die Marke landet im Registrierungseintrag und macht einen
        // Start durch die Anmeldung später unterscheidbar.
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![startup::AUTOSTART_FLAG]),
        ))
        // Protokoll: ohne Logger verschwindet jedes log::-Makro still,
        // und eine Tray-Anwendung hat keine Konsole, in die man schauen könnte.
        // Ziel ist %LOCALAPPDATA%\de.leosysr.luchsr\logs.
        .plugin(
            tauri_plugin_log::Builder::new()
                .level(log::LevelFilter::Info)
                // Der Abrufzyklus protokolliert auf debug; im Fehlerfall lässt
                // sich das gezielt anheben, ohne den Rest zu überschwemmen.
                .level_for("luchsr_lib::poll", log::LevelFilter::Debug)
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("luchsr".into()),
                    }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                ])
                .max_file_size(1_000_000)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .build(),
        )
        // Nur für den Speicherdialog des CSV-Exports. Der Öffner-Plugin wird
        // nicht registriert, weil `open_url` ohne Zustand arbeitet.
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(state)
        .setup(move |app| {
            let handle = app.handle().clone();

            // Tray zuerst: von hier aus kann der Benutzer alles erreichen,
            // auch wenn das Fenster verborgen bleibt.
            tray::setup(&handle)?;

            // Das Fenster steht in tauri.conf.json auf `visible: false` — beim
            // Autostart soll nichts aufblitzen. Ob es gezeigt wird, hat
            // `startup::window_on_start` oben entschieden.
            if let Some(window) = app.get_webview_window(tray::POPUP_LABEL) {
                if fenster == startup::StartupWindow::Show {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
                tray::watch_focus(&window);
            }

            // Autostart erst hier: ein Fehlschlag beim Registrierungszugriff
            // darf den Start nicht verhindern, und das Fenster steht schon.
            startup::initialise_autostart(&handle);

            log::info!(
                "gestartet {} — Fenster: {}",
                if per_autostart {
                    "durch den Autostart"
                } else {
                    "von Hand"
                },
                if fenster == startup::StartupWindow::Show {
                    "sichtbar"
                } else {
                    "verborgen"
                }
            );

            // Abrufschleife starten. Sie ruft sofort einmal ab und meldet den
            // Zustand an Tray und Fenster.
            poll::spawn(handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::settings_load,
            commands::settings_current,
            commands::settings_save,
            commands::settings_validate,
            commands::credential_store_available,
            commands::secret_set,
            commands::secret_exists,
            commands::secret_delete,
            commands::connection_test,
            commands::status_current,
            commands::refresh_now,
            commands::open_in_checkmk,
            commands::set_pin_popup,
            commands::export_csv,
            commands::action_comment,
            commands::acknowledge,
            commands::set_downtime,
            commands::builtin_sounds,
            commands::play_sound,
            commands::about_info,
            commands::open_project_page,
        ])
        .run(tauri::generate_context!())
        .expect("Luchsr konnte nicht gestartet werden");
}
