// Kein Konsolenfenster im Release-Build. Luchsr ist eine Tray-Anwendung —
// ein aufblitzendes schwarzes Fenster beim Autostart wäre ein Fehler.
// Im Debug-Build bleibt die Konsole, damit Logausgaben sichtbar sind.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    luchsr_lib::run()
}
