# Luchsr — Projektkontext für Claude Code

CheckMK-Tray-Client für Windows. Bewusst **nur** CheckMK, kein Multi-Backend —
die Beschränkung ist Absicht und hält das Werkzeug klein. Diese Datei ist der
Einstiegspunkt für jede neue Sitzung.

## Namenskonventionen — durchgängig einhalten

| Kontext | Wert |
|---|---|
| Produktname | Luchsr |
| Hersteller / Autor | Fabian Schatto – leosysr |
| Executable | `luchsr.exe` |
| Installationspfad | `%ProgramFiles%\Luchsr` — Tauris Vorgabe, siehe D77 |
| Benutzerkonfiguration | `%APPDATA%\leosysr\Luchsr\config.json` |
| Maschinenvorgaben | `%ProgramData%\leosysr\Luchsr\defaults.json` |
| Credential-Manager-Service | `leosysr.Luchsr` |
| MSI-Dateiname | `Luchsr_<version>_x64_de-DE.msi` — von Tauri vorgegeben, siehe D77 |
| Rust-Crate / npm-Paket | `luchsr` |

Version steht ausschließlich in `tauri.conf.json` (Single Source of Truth).

## Stack

- **Tauri 2** — Rust-Backend, Web-Frontend
- Frontend: React + TypeScript + Vite, Tailwind CSS
- HTTP: `reqwest` mit Feature `native-tls` — **zwingend**, damit unter Windows Schannel
  und damit der Windows-Zertifikatspeicher genutzt wird. Mit rustls schlägt die TLS-Prüfung
  gegen die interne CA fehl.
- Credentials: `keyring` (Windows Credential Manager)
- Serialisierung: `serde` / `serde_json`
- Plugins: `tauri-plugin-autostart`, `tauri-plugin-single-instance` (beide Slice 9),
  `tauri-plugin-log` (siehe D32; die JS-Seite `@tauri-apps/plugin-log` braucht `log:default`
  in `capabilities/default.json`, damit das Frontend in dieselbe Datei schreiben kann),
  `tauri-plugin-dialog` — wird **aus Rust** aufgerufen und braucht deshalb **keine** Kapazität
- Benachrichtigungen: `tauri-winrt-notification` **direkt**, ohne
  `tauri-plugin-notification` — siehe D91. Dazu `windows-registry` für die
  AppUserModelID. Beide Crates lagen ohnehin im Baum (unter dem Plugin bzw. unter
  reqwest) und kosten als direkte Abhängigkeit keine zusätzliche Übersetzungseinheit
- Signalton: `windows-sys` mit `Win32_Media_Audio` für `PlaySoundW`. Dieselbe
  Hauptversion, die Tauri ohnehin im Baum hat
- Icons: `lucide-react` als npm-Abhängigkeit (nicht der CDN-Fetch aus dem Design-Export)
- Tests: `cargo test` für Rust, `vitest` für die reinen Logikmodule des Frontends

**Zielplattform ist ausschließlich Windows 11 x64.** Plattformspezifische Vereinfachungen sind
erlaubt und erwünscht — es entsteht kein Cross-Platform-Code.

Toolchain: `stable-x86_64-pc-windows-msvc`, VS Build Tools mit C++-Workload, Node.js LTS,
WebView2-Runtime.

## Projektstruktur

```
Luchsr/
├── CLAUDE.md                  diese Datei
├── .github/workflows/
│   └── release.yml            baut auf Tag und veröffentlicht das Release
├── README.md                  Installation, Betrieb, Fehlersuche
├── LICENSE                    MIT
├── THIRD-PARTY.md             ERZEUGT — scripts/third-party.ps1, nicht von Hand
├── licenses/                  Lizenz-Volltexte, wortgetreu aus der Quelle
├── index.html                 Vite-Einstieg
├── package.json               Version ist Platzhalter, siehe tauri.conf.json
├── tsconfig.json              kein baseUrl (TS 7), paths relativ
├── vite.config.ts             React + Tailwind, Port 1420 strict
├── scripts/
│   ├── mark.mjs               DIE BILDMARKE: Geometrie, Rasterizer, PNG/ICO/SVG
│   ├── make-icons.mjs         erzeugt daraus alle ausgelieferten Icon-Dateien
│   ├── mark-studio.mjs        Vergleichsblätter — Entwurfsprotokoll zu D22/D23
│   ├── make-sounds.mjs        DIE HINWEISTÖNE: 6 Familien, Synthese, Selbstprüfung
│   ├── third-party.ps1        erzeugt THIRD-PARTY.md aus den Abhängigkeitsgraphen
│   └── checkmk-probe.ps1      Diagnose gegen die REST-API, ohne Luchsr
├── handover-design/           Design-Export, REFERENZ — nie verändern, NICHT versioniert
├── packaging/
│   └── defaults.example.json  Vorlage für die Maschinenvorgaben
├── src/                       Frontend
│   ├── main.tsx
│   ├── App.tsx                Popup-Rahmen: Ersteinrichtung, Liste, Einstellungen
│   ├── assets/fonts/          8 woff2, latin + latin-ext
│   ├── assets/icons/          luchsr-mark.svg (currentColor), luchsr-tile.svg
│   ├── components/            Primitives nach Export-Spezifikation
│   │   ├── Button.tsx         Button, IconButton
│   │   ├── Field.tsx          Beschriftung, Hinweis, Fehler
│   │   ├── Input.tsx          Input, NumberInput
│   │   ├── Select.tsx
│   │   ├── Toggles.tsx        Switch, Checkbox, Segmented
│   │   └── Surfaces.tsx       Card, Callout, Badge
│   ├── features/
│   │   ├── shell/
│   │   │   ├── PopupChrome.tsx    rahmenlose Titelzeile, Ziehfläche, Statuschip
│   │   │   └── ErrorBoundary.tsx  fängt Renderfehler, protokolliert sie
│   │   ├── problems/          Problemliste (Slice 6)
│   │   │   ├── ProblemsView.tsx  verbindet Filter, Liste und Detail
│   │   │   ├── FilterBar.tsx     Freitext, Statusumschalter, Bearbeitete
│   │   │   ├── ProblemList.tsx   virtualisiert, Gruppen, Tastatur
│   │   │   ├── DetailPanel.tsx   Bodenblatt, volle plugin_output
│   │   │   ├── ActionDialog.tsx   Quittieren und Wartungszeit (Slice 7)
│   │   │   ├── grouping.ts       Filtern, Gruppieren, Zählen — rein, getestet
│   │   │   └── duration.ts       Dauern und Zeitstempel — rein, getestet
│   │   └── settings/          Einstellungen und Ersteinrichtung
│   │       └── SoundPicker.tsx  Klangauswahl je Ereignis, mit Vorhören
│   ├── i18n/                  de.ts + typisierter Zugriff, de.test.ts prüft Hygiene
│   ├── lib/
│   │   ├── api.ts             typisierte Hülle um die Tauri-Befehle
│   │   ├── types.ts           Spiegel des Rust-Schemas
│   │   ├── status.ts          Statusmodell: Symbol, Kürzel, Schwere, Klassen
│   │   └── theme.ts           [data-theme]-Override
│   └── styles/
│       ├── index.css          Einstieg, Importreihenfolge ist bedeutsam
│       ├── tokens.css         DIE EINZIGE WERTEDATEI
│       ├── fonts.css          @font-face, reine Pfade
│       ├── utilities.css      @utility, nur Verpackungen um Tokens
│       └── base.css           Element-Defaults
└── src-tauri/                 Backend
    ├── Cargo.toml             crate-type = ["rlib"]
    ├── build.rs
    ├── tauri.conf.json        VERSION = Single Source of Truth
    ├── capabilities/
    │   └── default.json       Berechtigungen, pro Slice erweitert
    ├── icons/                 App-Icon 32/128/256/512 + icon.ico
    │   ├── tray/              sechs Zustände × 16 und 32 px (Slice 5)
    │   └── toast/             fünf Zustände × 192 px, ins Binary eingebaut
    ├── sounds/                ERZEUGT — 24 WAV, scripts/make-sounds.mjs, ins Binary eingebaut
    └── src/
        ├── main.rs            windows_subsystem = "windows" im Release
        ├── lib.rs             run(), Plugins, Befehlsregistrierung
        ├── commands.rs        Tauri-Befehle, AppState, CommandError
        ├── i18n.rs            Texte der nativen Teile (Tray-Menü)
        ├── tray/              Slice 5
        │   ├── mod.rs         Aufbau, Kontextmenü, Klick-Ereignisse
        │   ├── state.rs       Zustand → Icon → Tooltip, rein und getestet
        │   └── position.rs    Fensterposition, rein und getestet
        ├── poll/              Slice 5
        │   ├── mod.rs         die Abrufschleife
        │   └── schedule.rs    Jitter, Backoff, Standby, rein und getestet
        ├── startup/           Startverhalten (Slice 9)
        │   └── mod.rs         Autostart, Einzelinstanz, Fenster — rein und getestet
        ├── actions/           Schreibaktionen (Slice 7)
        │   └── mod.rs         Berechtigungen, Kommentarvorlage — rein, getestet
        ├── notify/            Benachrichtigungen (Slice 8)
        │   ├── mod.rs         Toasts senden, Deckelung, Anbindung
        │   ├── decide.rs      WAS gemeldet wird — rein und getestet
        │   ├── toast.rs       Toast-Aufbau: Logo, Textzeilen, Auspacken
        │   ├── identity.rs    AppUserModelID in der Registry — Name und Symbol
        │   └── sound.rs       PlaySoundW, nur WAV, Formatprüfung
        ├── export/            CSV-Ausgabe (Slice 6)
        │   └── mod.rs         to_csv() rein und getestet, Trennzeichen/BOM/Formelabwehr
        ├── checkmk/           API-Client (Slice 3)
        │   ├── mod.rs         Re-Exports, Modulübersicht
        │   ├── error.rs       Fehlertypen, Ursachenerkennung, Secret-Hülle
        │   ├── model.rs       Antwortstrukturen + Domänenmodell
        │   ├── url.rs         Endpunkt-URLs samt Kodierung
        │   ├── write.rs       Nutzlasten Quittieren/Wartung, Dauerfenster
        │   ├── client.rs      HTTP-Schicht + reine Auswertungsfunktionen
        │   └── fixtures/      aufgezeichnete API-Antworten für die Tests
        └── config/            Konfiguration und Credentials (Slice 4)
            ├── mod.rs
            ├── schema.rs      Settings, Vorgaben, repair(), validate()
            ├── paths.rs       %APPDATA% und %ProgramData%
            ├── store.rs       Laden, atomares Speichern, Quarantäne
            ├── secrets.rs     Credential Manager, die EINZIGE Secret-Stelle
            └── error.rs       ConfigError, SecretError
```

Alle geplanten Module in `src-tauri/src/` sind vorhanden.

### Prüfbarkeit ohne Server

Im `checkmk`-Modul ist alles Auswertende in **reine Funktionen** gezogen: URL-Bau,
Antwortauswertung, Fehlerklassifizierung, Nutzlastbau, Zeitfenster. Nur das
Zusammensetzen von HTTP bleibt in `async`-Methoden. Deshalb laufen die 125 Unit-Tests
gegen `fixtures/*.json` ohne Netzwerk.

**Diese Aufteilung bitte beibehalten.** Wer Logik in eine `async fn` schreibt, macht
sie untestbar.

## Design-Regeln

### Herkunft

`.\handover-design` ist ein **Export aus Claude Design für die Marke "schattutor"** (IT-Bildungs-
kanal), nicht für Luchsr. Er enthält keine Luchsr-Screens. Verwendet wird daraus die
**Token- und Interaktionsgrundlage**; Logo, Wordmark und Avatare bleiben ungenutzt
(Entscheidung D2 unten).

**`.\handover-design` wird nie verändert oder überschrieben.** Es bleibt als Referenz liegen.

### Die eiserne Regel

`src/styles/tokens.css` ist die **einzige** Stelle im Projekt mit konkreten Farb-, Abstands-,
Radius-, Schatten-, Schrift- und Dauerwerten. In Komponenten stehen ausschließlich Tokens
bzw. darauf gemappte Tailwind-Klassen — **niemals** ein Hex-Wert, eine px-Zahl für Abstände
oder ein Font-Name. Das Design muss austauschbar bleiben.

Präfixe in `tokens.css`:

- `--st-*` — Rohwerte **wörtlich** aus `handover-design/tokens/*.css` übernommen
- `--lx-*` — Rohwerte, die Luchsr **ergänzt** (nur die Statusfarben, siehe D3)
- semantische Aliase behalten die Namen des Exports (`--text-body`, `--surface-card`,
  `--accent-solid`, `--border-subtle`, …), damit der Komponenten-Vertrag deckungsgleich bleibt

Damit ist jederzeit auditierbar, was aus dem Export kommt und was dazugekommen ist.

### Statusfarben

Eigene, klar benannte Gruppe, getrennt von den übrigen Farbtokens:
`--state-ok`, `--state-warn`, `--state-crit`, `--state-unknown`, `--state-down`, `--state-stale`
(jeweils plus `-soft` für Zeilenhintergründe).

**Status wird nie allein über Farbe kodiert** — immer zusätzlich ein Symbol. Zwingend, nicht
optional.

Für das Tray-Icon gilt das nicht: dort gibt es nur eine 16-px-Farbfläche und kein Symbol
daneben. Deshalb sind die **Farbtonabstände** dort die tragende Eigenschaft, nicht die
Helligkeit — siehe D23 und die Tabelle in `tokens.css`.

### Bildmarke

Ein **Luchskopf** in Silhouette: lange Ohrpinsel, gezackter Backenbart der tiefer hängt als
das Kinn, kurzes breites Gesicht, kleine schräg gestellte Augen als Aussparung.

Die Geometrie steht **einmal** in `scripts/mark.mjs`. `make-icons.mjs` erzeugt daraus alle
ausgelieferten Dateien, `mark-studio.mjs` die Vergleichsblätter. Nach einer Änderung an der
Geometrie:

```bash
node scripts/make-icons.mjs
```

Regeln:

- Die Augen erscheinen **erst ab 24 px**. Darunter wären sie unter 2 px und würden matschen.
  Dass ein Icon bei 16 px anders aufgebaut ist als bei 32 px, ist die richtige Anpassung.
- `luchsr-mark.svg` trägt `fill="currentColor"` und enthält **keinen** Farbwert — die Farbe
  kommt aus den Tokens.
- Die Augen sind eine Aussparung über `fill-rule="evenodd"`, nicht eine überlagerte Form in
  der Hintergrundfarbe. Sonst bricht die Marke auf jeder anderen Fläche.
- Die Farbtabelle in `mark.mjs` ist die **eine erlaubte Ausnahme** von der Token-Regel: ein
  Skript, das Binärdateien erzeugt, kann kein CSS lesen. Bei einer Palettenänderung muss sie
  mitgeführt werden.

### Weiteres

- Hell und Dunkel über `prefers-color-scheme` plus manuellem Override (`[data-theme]`).
  Der Export deckt beide ab: hell ist Default, `[data-theme="dark"]` ist ein vollständiger
  Scope-Flip der semantischen Aliase.
- Dichte Tabellendarstellung. Das UI muss bei **80 gleichzeitigen Problemen** brauchbar
  bleiben → virtualisierte Liste.
- Interaktionen aus dem Export: Hover 120 ms, Controls 180 ms, Panels 280 ms,
  Easing `cubic-bezier(.2,.8,.2,1)`. Press = `translateY(1px)`, **nie** `scale`.
  Focus = 2 px Ring mit 2 px Surface-Offset. Disabled = 42 % Opazität ohne Farbwechsel.
- Dunkle Flächen tragen **Border, keinen Schatten** ("shadow on ink reads as mud").
- Fonts: Manrope (Display/Body) + IBM Plex Mono (alles Technische). Mono sitzt immer
  eine Stufe kleiner als der Prosatext daneben. Lokal eingebettet, kein Laufzeit-Netzzugriff.
- Alle technischen Werte — Hostnamen, Services, Zeitstempel, `plugin_output` — in Mono.

## CheckMK-API-Vertrag

Basis-URL: `{server}/{site}/check_mk/api/1.0`

Auth-Header: `Authorization: Bearer {username} {secret}`

`query` und **jede** `columns`-Angabe müssen URL-encoded übergeben werden.
Nutzdaten liegen jeweils unter `value[].extensions`.

### Services

```
GET /domain-types/service/collections/all
  ?columns=host_name&columns=description&columns=state&columns=plugin_output
  &columns=last_state_change&columns=acknowledged
  &columns=scheduled_downtime_depth&columns=is_flapping
  &query={"op":">","left":"state","right":"0"}
```

### Hosts

```
GET /domain-types/host/collections/all
  ?columns=name&columns=state&columns=plugin_output&columns=last_state_change
  &columns=acknowledged&columns=scheduled_downtime_depth
  &query={"op":">","left":"state","right":"0"}
```

### Quittieren

```
POST /domain-types/acknowledge/collections/service
{
  "acknowledge_type": "service",
  "sticky": true, "persistent": false, "notify": true,
  "comment": "...", "host_name": "...", "service_description": "..."
}
```

### Wartungszeit

```
POST /domain-types/downtime/collections/service
{
  "downtime_type": "service",
  "start_time": "<ISO8601>", "end_time": "<ISO8601>",
  "comment": "...", "host_name": "...", "service_descriptions": ["..."]
}
```

Schreiboperationen brauchen `ETag`-Handling bzw. `If-Match: *` — gegen die Doku der
eingesetzten CheckMK-Version prüfen und robust implementieren.

Beide Schreibaktionen laufen nur, wenn in den Einstellungen aktiviert
(`allow_acknowledge` / `allow_downtime`, beide Default **aus**).

### Polling

- Intervall 15–600 s, Default 60 s, mit **Jitter ±10 %** damit viele Clients nicht synchron feuern
- Fehler → exponentielles Backoff bis max. 5 Minuten, Tray-Icon in Verbindungsfehler-Zustand
- Timeout 10 s; ein laufender Request wird bei manuellem Refresh abgebrochen
- Kein Polling im Standby; nach dem Aufwachen einmalig sofort abrufen

## Konfiguration

Alles variabel, **keine hartkodierten Werte** — jeder Parameter ist im Einstellungsdialog pflegbar.

- Einstellungen: JSON in `%APPDATA%\leosysr\Luchsr\config.json`
- **Secret ausschließlich im Credential Manager**, Service `leosysr.Luchsr`, Account = Benutzername.
  Das Automation-Secret landet **nie** in der Config-Datei, nie in Logs, nie im Frontend-State.
- Optionale Vorbelegung: existiert beim ersten Start `%ProgramData%\leosysr\Luchsr\defaults.json`,
  werden daraus Vorgabewerte übernommen (typisch Server-URL und Site). Danach vom Benutzer
  überschreibbar. Fehlt die Datei → Ersteinrichtungs-Assistent.
- Datenmodell so anlegen, dass **mehrere CheckMK-Instanzen später nachrüstbar** wären
  (aktuell nur eine aktiv).
- "Verbindung testen" muss konkret werden: HTTP-Statuscode, erkannte CheckMK-Version, und im
  Fehlerfall die **echte Ursache** — DNS, TLS-Kette, 401, 404. Keine generischen Meldungen.

## Autostart

- Default **an**, beim allerersten Start automatisch aktiviert **und in der Config vermerkt**,
  damit eine spätere Deaktivierung durch den Benutzer nicht bei jedem Start überschrieben wird.
- Beim Autostart startet Luchsr minimiert in den Tray, ohne Fenster.
- `tauri-plugin-single-instance`: ein zweiter Start bringt das vorhandene Fenster nach vorne.

## Packaging

- Bundle-Target **MSI** (WiX), NSIS zusätzlich als Fallback
- Per-Machine-Installation nach `%ProgramFiles%\leosysr\Luchsr`
- **Stabile Upgrade-GUID** — Updates laufen in-place
- `msiexec /i Luchsr_<version>_x64_de-DE.msi /qn` muss silent durchlaufen — braucht
  einen erhöhten Kontext, siehe D77
- Codesignatur über `bundle.windows.certificateThumbprint`, Thumbprint als Umgebungsvariable,
  plus `timestampUrl` (öffentlicher Zeitstempeldienst funktioniert auch mit intern
  ausgestelltem Zertifikat)
- Kein Auto-Update — Verteilung läuft über Softwaremanagement

## Ausdrücklich nicht Teil des Projekts

- Andere Monitoring-Backends als CheckMK
- Konfigurationsänderungen an CheckMK (Hosts anlegen etc.)
- Graphen, Performance-Daten, Historie
- Mehrere CheckMK-Instanzen gleichzeitig (nur nachrüstbar vorbereiten)
- Auto-Update-Mechanismus

## Entscheidungslog

| # | Entscheidung | Begründung |
|---|---|---|
| D1 | Toolchain per winget installiert: VS Build Tools 2022 (VCTools), Node.js LTS, Rustup | Auf dem Rechner war nur WebView2, Git und winget vorhanden |
| D2 | Aus dem Design-Export werden **Tokens und Interaktionsregeln** übernommen, **nicht** die Marke | Der Export gestaltet den Kanal "schattutor", nicht Luchsr. Luchsr ist ein leosysr-Produkt und bekommt eine eigene Bildmarke (Luchs) |
| D3 | Die Markenregel "kein dritter Farbton" wird für die Statusgruppe **gebrochen** | In einem Monitoring-Tool ist Farbcodierung Funktion, nicht Dekoration. Bei 80 dichten Zeilen sind Helligkeitsstufen eines Farbtons nicht zuverlässig unterscheidbar, erst recht nicht bei Rot-Grün-Schwäche |
| D4 | Fonts werden **lokal eingebettet** statt von Google Fonts geladen | Desktop-App im Firmennetz: kein Laufzeit-Netzzugriff, funktioniert offline und hinter Proxy |
| D5 | Statusbelegung: OK = Marken-Grün, WARN = Amber (neu), CRIT = Zinnober (neu), DOWN = Karmin (neu), UNKNOWN = Marken-Pink, STALE = Slate | Grün/Gelb/Rot ist die Konvention, die CheckMK selbst nutzt. Marken-Pink bleibt für UNKNOWN, weil "der Check funktioniert nicht" ein anderer Fehlertyp ist als "der Dienst ist kaputt" — und Magenta von Grün/Amber/Rot maximal weit weg liegt. **Korrigiert in D23:** CRIT und DOWN waren zunächst zwei Rottöne derselben Familie und im Tray-Icon nicht unterscheidbar |
| D6 | `tokens.css` ist **eine** Datei, kein Paar aus tokens + theme | Die Token-Namen des Exports (`--radius-md`, `--shadow-card`, `--ease-out`, `--font-mono`, `--leading-*`, `--tracking-*`) sind identisch mit Tailwind-4-Namensräumen. Ein Alias auf sich selbst wäre zirkulär, also stehen diese Werte direkt im `@theme`-Block derselben Datei |
| D7 | ~~App-Icon ist ein Platzhalter~~ — **erledigt in D22**, die Bildmarke liegt vor | `tauri-build` bricht ohne `icons/icon.ico` ab, deshalb gab es bis Slice 4 eine mintfarbene Kachel mit "L" |
| D8 | `crate-type = ["rlib"]` statt der Tauri-Vorlage `["staticlib", "cdylib", "rlib"]` | staticlib und cdylib braucht nur der Mobile-Build. Auf Windows erzeugt cdylib eine unnötige DLL samt Importbibliothek und eine Linker-Warnung bei jedem Build |
| D9 | Transparente Varianten werden als `color-mix(in srgb, var(--token) N%, transparent)` geschrieben, nie als `rgba()` mit ausgeschriebenen Kanälen | Der Export schreibt sie inline aus. Damit bleibt beim Palettentausch die Soft-Variante auf dem alten Farbton stehen — genau der stille Designbruch, den die Token-Regel verhindern soll. Nachgemessen: die Umstellung ist bitgenau identisch, `color-mix` serialisiert nur als `color(srgb …)` statt `rgba()` |

| D10 | Weiterleitungen werden **nicht** gefolgt (`redirect::Policy::none()`), 3xx wird als Fehler gemeldet | Der Authorization-Header trägt das Automation-Secret. Einer Umleitung zu folgen würde es an ein Ziel senden, das nicht der konfigurierte Server ist. Der häufigste Fall ist http→https, und dann ist „korrigiere die Server-URL" der nützlichere Hinweis als eine stille Umleitung |
| D11 | Transportfehler werden über **OS-Fehlercodes** klassifiziert, nicht über Meldungstexte | Windows-Meldungen sind lokalisiert („Der angegebene Host ist unbekannt." statt „No such host is known."). Ein Textvergleich wäre auf deutschem Windows sofort kaputt. Stabil ist die Zahl in `(os error N)` — die schreibt Rusts `io::Error` immer englisch |
| D12 | Der Benachrichtigungsschlüssel ist **längenpräfigiert**, nicht mit Trennzeichen verkettet | `("host", "a\|b")` und `("host\|a", "b")` ergaben denselben Schlüssel. Eine Kollision hier unterdrückt eine Benachrichtigung — ein Fehler, der niemandem auffällt. Ein Test hat das nachgewiesen |
| D13 | Manuelle Proxy-Adressen werden **selbst** geprüft, bevor sie an reqwest gehen | `reqwest::Proxy::all("kein-proxy-url")` schluckt den Tippfehler und baut daraus `http://kein-proxy-url/`. Jeder Abruf scheitert danach mit einer Meldung, die nicht auf die Ursache zeigt |
| D14 | `If-Match: *` wird bei **jeder** Schreiboperation mitgesendet | Die genutzten Endpunkte sind Collection-POSTs, die es nicht zwingend verlangen — das unterscheidet sich aber zwischen CheckMK-Versionen. Wo der Server den Header nicht auswertet, ignoriert er ihn; wo er ihn verlangt, ist er erfüllt. 412 und 428 werden trotzdem eigens gemeldet |
| D15 | `Settings` hat **kein Feld** für das Automation-Secret, auch kein optionales | Damit ist es strukturell unmöglich, dass es in `config.json` landet — keine Frage der Disziplin, sondern des Typs. Zwei Tests prüfen die serialisierte Form und die geschriebene Datei gegen verdächtige Schlüsselnamen |
| D16 | Es gibt **keinen** Befehl, der das Secret ausliest | `secret_exists` gibt nur einen Wahrheitswert zurück. Ein Wächtertest prüft, dass kein lesender Befehl dazukommt. Beim Verbindungstest darf das Frontend ein ungespeichertes Secret **hinein**geben, sonst müsste man erst speichern, um testen zu können |
| D17 | `config.json` wird **atomar** geschrieben (Nebendatei, dann verschieben) | Ein Absturz mitten im Schreiben hinterlässt sonst eine halbe Datei, die beim nächsten Start als beschädigt gilt — und die Einstellungen des Benutzers kostet |
| D18 | Eine beschädigte `config.json` wird nach `config.json.beschaedigt-N` **zur Seite gelegt**, die App startet mit Vorgaben | Hart scheitern lähmt die App, stillschweigend überschreiben vernichtet Beweise. Der Vorfall erscheint als Hinweis im Dialog |
| D19 | `LoadOutcome.needs_setup` ist ein **Feld**, keine Methode | serde serialisiert nur Felder, und das Frontend braucht die Antwort. Die Bedingung im Frontend nachzubauen wären zwei Wahrheiten für dieselbe Frage |
| D20 | Prüfung gibt **alle** Probleme zurück, getrennt in Fehler und Warnungen | Der Benutzer soll nicht Feld für Feld durch Meldungen geführt werden. Warnungen blockieren das Speichern nicht — sonst liesse sich die TLS-Prüfung nie abschalten |
| D21 | Manuelle Proxy-Adressen und Feldprüfungen laufen im **Backend**, nicht im Frontend | Die URL-Prüfung steckt im `checkmk`-Modul und ist dort getestet. Sie im Dialog nachzubauen hiesse, zwei Wahrheiten zu pflegen. Der Aufruf ist entprellt (250 ms) |
| D22 | Bildmarke: **Luchskopf-Silhouette** mit langen Ohrpinseln, gezacktem Backenbart und kleinen schrägen Augen als Aussparung. Geometrie in `scripts/mark.mjs` | Drei Merkmale überleben 16 px: die Ohrpinsel, der Backenbart der tiefer hängt als das Kinn, und das kurze breite Gesicht. Verworfen wurden ein langes spitzes Kinn (las sich als Fuchs), ein einzelner breiter Backenbart (generisch), eine Reduktion auf nur die Ohren (las sich als Krone) und eine reine Kontur (zerfiel bei 16 px). Jede Runde wurde gerastert und angesehen, nicht beschrieben |
| D23 | CRIT ist **Zinnober**, DOWN ist **Karmin** — zwei getrennte Farbtöne, 27° auseinander, beide mit runder Kachel | Der erste Entwurf hatte CRIT `#ff6b5e` und DOWN `#ff4436`: im Tray bei 16 px zwei nicht unterscheidbare rote Blöcke. Eine eckige Kachel für DOWN hätte die Form zur Unterscheidung genutzt und funktioniert, wurde aber verworfen — die Ursache zu beheben ist besser, als sie zu umgehen. Ein dunkleres Rot war ebenfalls keine Lösung: bei 16 px verschmilzt es mit der Taskleiste |
| D24 | Tray-Icons: **gefüllte Kachel** in der Zustandsfarbe, Luchs in Ink ausgestanzt — nicht umgekehrt | Bei 16 px schlägt Farbfläche jedes Glyphendetail: man sieht aus dem Augenwinkel, dass die Leiste rot geworden ist. Die invertierte Variante (dunkle Kachel, farbiger Luchs) ist bei 32 px schöner, verschwindet bei 16 px aber fast — besonders im Zustand „getrennt" |
| D25 | Je Zustand werden **16 px und 32 px einzeln gerendert**, nicht eine Grösse skaliert | Windows fragt im Infobereich 16 px bei 100 % und 32 px bei 200 % Skalierung. Beides einzeln zu rendern kostet nichts und ist die einzige Variante, die auf beiden scharf ist — zumal die Augen erst ab 24 px gezeichnet werden |
| D26 | Ein **Verbindungsfehler schlägt jeden Abzug**: das Tray-Icon geht auf „getrennt", nie auf grün. Auch „noch nichts abgerufen" ist nicht OK | Ein grünes Icon, während der Server nicht antwortet, ist die schlimmste Fehlinformation, die dieses Programm liefern kann — es sieht aus wie „alles in Ordnung". Ein Test hält das fest |
| D27 | Ein Fehler, den Wiederholen **nicht** behebt (401, 404, kaputte Konfiguration), bekommt **kein Backoff** — nur behebbare (Netz, 5xx) | Ein falsches Secret löst sich nicht durch Warten, aber der Benutzer korrigiert es womöglich gerade im Dialog. Dann soll die nächste Prüfung in einer Minute kommen und nicht in fünf. Umgekehrt schützt das Backoff einen angeschlagenen Server |
| D28 | Standby wird über die **Wanduhrzeit** erkannt, nicht über `WM_POWERBROADCAST` | Das Power-Ereignis abzufangen heisst, in die Nachrichtenschleife des Fensters zu greifen. Zeiten zu vergleichen kostet nichts und erkennt zusätzlich Fälle ohne Power-Ereignis — eine angehaltene VM etwa. Schwelle ist relativ **und** absolut: `erwartet + max(60 s, erwartet)` |
| D29 | Der Abbruch eines laufenden Abrufs braucht **keinen eigenen Mechanismus** — `tokio::select!` gegen das Refresh-Signal genügt | In Rust bricht ein Future ab, wenn er verworfen wird. `select!` verwirft den Verlierer, und damit ist die HTTP-Verbindung weg. Ein Abbruchtoken wäre zusätzlicher Zustand ohne Gewinn |
| D30 | Der Jitter kommt aus den **Nanosekunden der Systemuhr**, nicht aus `rand` | Für ±10 % auf ein Intervall von Minuten ist das mehr als genug Entropie und spart eine Abhängigkeit. Die Rechnung bleibt testbar, weil der Zufallswert ein Parameter ist |
| D31 | Das Fenster wird gegen den **Arbeitsbereich** des Monitors geklemmt und **verlässt seinen Monitor nicht** | `Monitor::work_area()` schliesst die Taskleiste aus — gegen die Bildschirmgrösse zu klemmen legte das Fenster darunter. Und ein Popup über zwei Bildschirme mit womöglich verschiedener DPI sieht kaputt aus |
| D33 | Fehlerantworten führen die **Begründung des Servers** mit — bei 401, 403 und 404, nicht nur im Sammelfall. Ist der Rumpf kein JSON, kommt ein **Rohauszug** in die Meldung | Ursprünglich wurden 401/403/404 nur auf den Statuscode abgebildet und das `detail`-Feld verworfen. Ein 403 sah damit für jede Ursache gleich aus. Der Rohauszug ist genauso wichtig: ein Statuscode ohne `problem+json` kommt in der Regel **nicht von CheckMK**, sondern von einem Apache oder Proxy davor, und `<title>403 Forbidden</title>` beantwortet die Frage „wer sagt hier nein" sofort |
| D34 | Bei „Proxy: System" wird die **Umgebung geprüft** und gewarnt, wenn Anfragen an den konfigurierten Server über einen Proxy laufen würden | Einmal passiert und eine halbe Stunde gekostet: `HTTP_PROXY` zeigte auf einen Firmenproxy, `NO_PROXY` führte nur `localhost` und `.local`, in WinINET war der Proxy **abgeschaltet**. Browser und PowerShell erreichten den internen CheckMK-Server direkt, `reqwest` schickte an den Proxy, und der antwortete `403 Forbidden` mit HTML. Das sah aus wie ein Berechtigungsproblem in CheckMK und war keins |
| D32 | `tauri-plugin-log` wird eingebunden, obwohl der Auftrag kein Logging verlangt | Ohne installierten Logger verschwindet jedes `log::`-Makro still. Eine Tray-Anwendung hat keine Konsole, in die man schauen könnte; Diagnose, die niemand sieht, ist toter Code. Ziel ist `%LOCALAPPDATA%\de.leosysr.luchsr\logs` |
| D35 | Bei aktiver **Textsuche wird die Gruppierung abgeschaltet** — Treffer erscheinen flach. Statusfilter lassen die Gruppierung stehen | Wer tippt, sucht eine bestimmte Zeile und will sie sehen, nicht einen zugeklappten Host, der sie enthält. Automatisch alle Gruppen aufzuklappen wäre die Alternative, hinterlässt aber nach dem Löschen des Filters einen anderen Zustand als vorher |
| D36 | Eine **leere Statusauswahl heisst „alle"**, nicht „keine" | Beim Öffnen ist nichts angeklickt. Eine dann leere Liste sähe wie „keine Probleme" aus — genau die Fehlinformation, die D26 für das Tray-Icon verbietet |
| D37 | Nach dem Ausblenden durch Fokusverlust gilt eine **Gnadenfrist von 300 ms**, in der ein Tray-Klick nicht als „öffnen" zählt | Windows nimmt dem Fenster den Fokus, *bevor* das Klickereignis des Trays ankommt. Ohne die Frist blendet der Klick aus und sofort wieder ein — das Fenster liesse sich durch Anklicken des Icons nie schliessen |
| D38 | Die **Wörterbuchhygiene wird mechanisch geprüft** (`src/i18n/de.test.ts`): kein verwaister Schlüssel, keine leeren Werte, keine ASCII-Ersatzzeichen. `src-tauri/src/i18n.rs` gilt als zweiter Konsument | Beim Aufräumen von Slice 6 lagen 16 Schlüssel verwaist herum, und zwei davon waren kein toter Text, sondern ein **fehlender Hinweis im Dialog** — die Erläuterungen zu „Quittieren erlauben" und „Wartungszeit erlauben" wurden nie angezeigt. Ein verwaister Schlüssel ist also nicht nur Ballast, er ist ein Verdachtsmoment |
| D39 | Das **Token-Musterblatt aus Slice 2 ist gelöscht** (`features/dev/`, `features/status/`) | Es war ein Gerüst, um Tokens und Primitives sichtbar zu machen, bevor es echte Screens gab. Mit der Problemliste gibt es sie; ein zweiter Ort, an dem Komponenten vorkommen, läuft sonst auseinander. Wiederherstellbar ist es aus der Historie |
| D40 | Zukünftige Texte werden **nicht vorab** ins Wörterbuch gelegt | Die Schlüssel für Quittieren und Wartungszeit (Slice 7) wurden wieder entfernt, statt sie in D38 als Ausnahme zu führen. Eine Ausnahmeliste, die Wünsche aufnimmt, prüft am Ende nichts mehr |
| D41 | Eine **`ErrorBoundary` unter der Titelzeile**, die den Fehler anzeigt *und* ins Protokoll schreibt | React hängt bei einem Renderfehler den ganzen Baum ab. Ohne Netz heisst das: rahmenloses Fenster, weiss, ohne Schliessknopf — man wird es nicht mehr los, und es gibt keine Konsole, in der die Ursache stünde. Unter der Titelzeile bleiben Ziehfläche und Schliessknopf am Leben. `key={view}` setzt sie beim Wechsel zurück, sonst blockiert ein Fehler in der Liste auch die Einstellungen |
| D42 | Der CSV-Export schreibt **den ganzen Abzug**, nicht die gefilterte Ansicht. Trennzeichen `;`, UTF-8 **mit BOM**, CRLF, Zeitstempel in **Ortszeit** als `TT.MM.JJJJ HH:MM:SS`, und führende `= + - @` werden mit Apostroph entschärft | Eine Datei, die man weitergibt, soll den Stand vollständig zeigen und nicht davon abhängen, was beim Klicken gerade eingestellt war — quittiert und Wartung stehen als eigene Spalten drin, in Excel lässt sich nachfiltern. Die Formatentscheidungen sind je ein eigener Fallstrick: mit `,` landet auf deutschem Windows alles in Spalte A, ohne BOM wird „Wärmefühler" zu „WÃ¤rmefÃ¼hler", und `plugin_output` ist Fremdtext — ein Feld, das mit `=` beginnt, ist in Excel eine Formel. Der Zeitstempel stand zuerst als ISO-8601 in UTC drin: das wich um zwei Stunden von der Liste ab, aus der die Datei kommt, und Excel liest es als Text statt als Datum. Die Zone geht dabei verloren — für einen Bericht im eigenen Haus der bessere Tausch |
| D43 | Während eines Systemdialogs sperrt ein **RAII-Wächter** das Ausblenden bei Fokusverlust | Der Speicherdialog nimmt dem Popup den Fokus, der Fokusverlust-Behandler hätte es verborgen — nach dem Speichern wäre das Fenster weg samt der Meldung, wohin geschrieben wurde. Wächter statt Setzerpaar, weil eine stehengebliebene Sperre bedeutet, dass das Fenster nie wieder von selbst ausblendet; zwei Tests halten das fest, einer davon über eine Panik |
| D44 | Der Umschalter „quittierte und Wartung anzeigen" ist ein **Symbolknopf in der Filterzeile**, kein Kontrollkästchen mit Fliesstext | Als Kästchen mit vollem Text brauchte er eine zweite Zeile. Bei 600 px Fensterhöhe ist das Platz, den die Liste braucht, und optisch wog die Nebensache schwerer als die Statusfilter daneben. Der ausgeschriebene Text steht im Tooltip und in `aria-label` |
| D45 | Die Freigabe einer Schreibaktion wird **im Backend** geprüft (`actions::ensure_allowed`), nicht nur über einen ausgeblendeten Knopf | Ein Knopf, der nicht erscheint, ist Anzeige und keine Sicherung — wer den Befehl direkt aufruft, käme daran vorbei. Bei einer Aktion, die in ein Produktionsmonitoring schreibt, ist das der Unterschied zwischen Vorsicht und Zufall. Die Meldung nennt Aktion *und* Einstellungspfad, damit der Benutzer nicht suchen muss |
| D46 | Der Aktionsdialog ist eine **Fläche im Popup**, kein Systemfenster | Ein eigenes Fenster nähme dem Popup den Fokus, und der Fokusverlust-Behandler blendet es aus (D37). Dafür bräuchte es die Sperre aus D43 — für zwei Eingabefelder der falsche Aufwand. Ausserdem bleibt die Titelzeile bedienbar, weil die Fläche unter ihr liegt |
| D47 | Der Kommentar wird **vorbelegt und bleibt editierbar**, die Vorlage steht in den Einstellungen. Zwei getrennte Vorlagen für Quittieren und Wartung | Ein fester Text ergibt in CheckMK bei jedem Eintrag dieselbe Zeile und macht die Historie wertlos; ein leeres Feld kostet bei jedem Klick Tipparbeit. Getrennt, weil die Sätze verschiedene Dinge sagen: „ist bekannt, wird bearbeitet" gegen „ist geplant, nicht alarmieren". Die Platzhalterersetzung liegt im Backend — sie im Frontend nachzubauen wären zwei Wahrheiten für denselben Text |
| D48 | Ein **unbekannter Platzhalter bleibt stehen** statt entfernt zu werden | `{hosst}` stillschweigend zu löschen macht einen Tippfehler in der Vorlage unsichtbar. So sieht der Benutzer ihn im vorbelegten Feld sofort. Ein Test prüft ausserdem, dass kein Platzhalter Teil eines anderen ist — sonst hinge das Ergebnis an der Ersetzungsreihenfolge |
| D49 | Nach einer Schreibaktion löst das Backend einen **sofortigen Abruf** aus; das Frontend nimmt den neuen Zustand **nicht** vorweg | Ohne den Abruf stünde die Zeile bis zum nächsten Intervall unverändert da, und es sähe aus, als hätte die Aktion nichts getan. „Quittiert" anzuzeigen, bevor CheckMK es bestätigt hat, wäre die andere Richtung desselben Fehlers — man würde sich auf etwas verlassen, das vielleicht nicht ankam |
| D50 | Der Herstellername ist **`leosysr`, durchgängig klein** — auch dort, wo er sichtbar ist. Produktname und Bildmarke bleiben `Luchsr` und Luchs | Umbenannt wurde das Entwicklerstudio, nicht das Produkt. Klein geschrieben, weil der Name so geschrieben wird; eine Groß-/Kleinschreibung, die je nach Ort wechselt, ist keine Konvention, sondern eine Fehlerquelle. Der Bundle-Identifier heißt jetzt `de.leosysr.luchsr` — davon hängt das Protokollverzeichnis ab |
| D51 | Für die Umbenennung gibt es **keinen Migrationspfad im Code** | Vor dem Umzug hieß der Konfigurationspfad `%APPDATA%SimbaITLuchsr` und der Credential-Dienst `SimbaIT.Luchsr`. Eine Migration einzubauen wäre Code für einen Fall, der genau einmal auftritt und nur auf einem Rechner — die App ist noch nicht verteilt. Die Datei wurde von Hand kopiert; das Secret **muss neu eingegeben werden**, weil es nach D16 keinen lesenden Befehl gibt. Das ist der Preis dieser Entscheidung, und er ist hier richtig bezahlt |
| D52 | `handover-design/` bleibt **lokal** liegen, wird aber **nicht versioniert** | Der Export gestaltet die Marke „schattutor" und enthält deren Logo, Wordmark und Avatare. Übernommen sind daraus nur Tokens und Interaktionsregeln (D2), und die stehen in `tokens.css`. Als Referenz auf der Platte ist er nützlich, in einem öffentlichen Repository wären es fremde Markenmittel ohne Zweck. Die frühere Notiz in `.gitignore` sagte das Gegenteil und ist ersetzt |
| D53 | Lizenz ist **MIT**. `THIRD-PARTY.md` wird **erzeugt**, nicht gepflegt | MIT, weil das Werkzeug ohne Rechtsprüfung weitergegeben werden soll; es ist **keine Zeile fremden Quelltexts** übernommen, insbesondere nichts unter Copyleft — es besteht also keine entsprechende Pflicht. Die Fremdliste zu pflegen hiesse, sie beim nächsten `cargo update` falsch zu haben — `scripts/third-party.ps1` liest `cargo tree` für **nur das Windows-Ziel** und die npm-Laufzeitabhängigkeiten. Cargo.lock erfasst alle Plattformen; Crates zu nennen, die im Build nicht landen, wäre Lärm statt Auskunft |
| D54 | Die Lizenz-**Volltexte** unter `licenses/` werden aus der Quelle übernommen, nie aus dem Gedächtnis geschrieben oder erzeugt | Ein falsches Wort in einem Lizenztext hebt die Bedingung auf, die er festhält. Betroffen sind zwei Schriften (SIL OFL 1.1 verlangt in Abschnitt 2 die Lizenz bei jeder Kopie — und die Schriften sind nach D4 eingebettet) und fünf MPL-2.0-Crates aus dem Tauri-Unterbau. MPL und MIT sind vereinbar: die MPL ist dateiweise und erlaubt das „Larger Work" ausdrücklich (3.3) |
| D55 | Der **erste Abzug meldet nichts** und füllt nur das Gedächtnis | Er enthält alle offenen Probleme — bei 40 Zeilen wären das 40 Toasts, und weil Luchsr im Autostart läuft, bei jeder Anmeldung. Ein Test hält den Fall mit 40 Problemen fest |
| D56 | Das Gedächtnis ist eine **Zuordnung Gegenstand → gemeldeter Zustand**, nicht eine Menge aus `(host, service, state)` | Der Auftrag beschreibt eine Menge. Die hätte einen stillen Fehler: nach `CRIT → OK → CRIT` wäre der Schlüssel noch enthalten und das zweite CRIT käme nie an. Die Zuordnung beantwortet zusätzlich die Frage, die eine Menge nicht beantworten kann — *ist dieser Gegenstand inzwischen weg?* Sie hält auch `WARN → CRIT` richtig: eine Meldung, **keine** Entwarnung für WARN dazu. `BTreeMap`, damit die Reihenfolge der Toasts nicht von der Hash-Streuung abhängt |
| D57 | **Entwarnungen sagen, warum** nicht mehr gemeldet wird: behoben, quittiert, in Wartung, oder nur noch weniger schlimm | „Wieder in Ordnung" für alle vier zu schreiben wäre falsch — quittiert ist nicht behoben, und wer sich darauf verlässt, hat ein Problem übersehen. Dass eine Entwarnung überhaupt kommt, ist an die vorherige Meldung gebunden: nur worüber gemeldet wurde, wird entwarnt. Damit ist sie nie Lärm |
| D58 | Bearbeitete Probleme (quittiert, Wartung) werden **auf jeder Stufe** übergangen, auch bei „jede Statusänderung" | Quittiert heisst „jemand weiss davon", Wartung heisst „ist geplant". Sie zu melden untergräbt genau das, wofür die Kennzeichen da sind |
| D59 | Höchstens **fünf** Einzeltoasts je Runde, danach eine Sammelmeldung. Der Signalton kommt **einmal** je Runde und nur bei Problemen | Kommt ein Server nach einem Ausfall zurück, sind dreissig neue Probleme im ersten Abzug — dreissig Toasts sind keine Information, sondern eine Sperre für den Bildschirm. Fünfmal zu klingeln wäre dasselbe akustisch. Eine Entwarnung ist eine gute Nachricht und soll nicht wie ein Alarm klingen. Die Grenzfallrechnung steckt in `split_for_toasts` und ist geprüft — genau bei fünf darf noch keine Sammelmeldung kommen |
| D60 | Bei „aus" wird das Gedächtnis **geleert**, nicht nur das Melden übersprungen | Sonst käme beim Wiedereinschalten eine Nachmeldung für alles, was in der Zwischenzeit passiert ist |
| D61 | Der Signalton läuft über `PlaySoundW` und kann damit **nur WAV**. Die Einstellungsprüfung warnt bei jedem anderen Format | Eine MP3 ergibt keinen Fehler, sondern **Stille** — die schlimmste Variante, weil niemand merkt, woran es liegt. `SND_NODEFAULT` verhindert zusätzlich, dass Windows bei einer kaputten Datei seinen Standardklang spielt und es klingt, als hätte es funktioniert. Eine Audiobibliothek mit MP3-Decoder wöge mehr als der halbe Rest der Anwendung |
| D62 | Nach einem **Fehlversuch** wird nicht gemeldet | Der Abzug ist dann der alte, daraus lässt sich keine Änderung ableiten. Ein Verbindungsfehler ist am Tray-Icon zu sehen (D26); ein Toast dafür wäre bei einem längeren Ausfall eine Meldung pro Minute |
| D63 | Die Hinweistöne sind **erzeugt** (`scripts/make-sounds.mjs`), nicht besorgt: zwei bis vier Töne, **unter 350 ms** | Dieselbe Begründung wie bei der Bildmarke: eine heruntergeladene Datei hat eine Herkunft und eine Lizenz, die mitzuführen sind — ein Skript, das sie erzeugt, hat beides nicht. Kurz, weil die Klänge in `C:\Windows\Media` halbe bis ganze Sekunden lang sind: wer zwanzig Meldungen am Tag bekommt, hört sie zwanzigmal und schaltet sie ab. Das Skript **liest die erzeugten Dateien zurück** und prüft Kopf, Länge, Spitzenwert und Ränder — ein falscher WAV-Kopf ergibt sonst Stille ohne Fehlermeldung. **Erweitert in D81–D84** |
| D64 | Die Klänge sind per `include_bytes!` **ins Programm eingebaut** und werden mit `SND_MEMORY` aus dem Speicher gespielt, nicht als Ressourcendateien | Eine Datei, die es zur Laufzeit gibt, kann fehlen — nach einer halben Installation, nach einem Virenscanner, nach einem Benutzer, der aufgeräumt hat. Ein `&'static [u8]` kann das nicht. 67 KB für sechs Klänge sind dafür ein niedriger Preis |
| D65 | **Ein Klang je Ereignis**, jeder einzeln abschaltbar. Fünf Felder im Typ statt einer Zuordnung | Fünf Felder halten im Typ fest, welche Ereignisse es gibt: ein Tippfehler im Namen ist ein Compilerfehler statt eines Ereignisses, das stillschweigend nie klingt. Der **Verbindungsfehler bekommt keinen** — er wiederholt sich bei einem Ausfall jede Minute und wäre nach zehn Minuten der Grund, alle Töne abzuschalten. Vorgabe: nur „kritisch" klingt; alles klingen zu lassen wäre der schnellste Weg dazu, dass der Benutzer alles abschaltet |
| D66 | Je Runde klingt **einer**, und zwar der **dringlichste**. Ist für dessen Stufe „kein Ton" gewählt, bleibt es still — es wird **nicht** auf eine niedrigere ausgewichen | Fünfmal zu klingeln wäre Lärm, und der Reihe nach zu spielen ginge nicht: `PlaySoundW` bricht den laufenden Klang ab, man hörte nur den letzten. Eine Entwarnung zwischen zwei kritischen Meldungen darf den Alarm nicht ersetzen. Und „für Kritisches keinen Ton" muss still bedeuten, nicht „dann eben den Warnton" — sonst wäre das Abschalten wirkungslos. Drei Tests halten das fest |
| D67 | Der Einstellungsdialog kann **vorhören** (`play_sound`), und die Klangliste kommt aus dem Backend (`builtin_sounds`) | Ohne Vorhören müsste man blind wählen, speichern und auf ein Ereignis warten — bei einem Klang für kritische Probleme dauert das Stunden. Die Liste im Frontend nachzuschreiben wären zwei Wahrheiten; eine Kennung, die auseinanderläuft, ergibt eine Auswahl, die stumm bleibt. Ein Test prüft, dass die Vorgabe auf einen vorhandenen Klang zeigt |
| D68 | Schemaversion **2**: der alte einzelne `soundPath` wird beim Laden nach `sounds.critical` überführt und geleert | Eine bestehende Konfiguration soll ihre Einstellung nicht stillschweigend verlieren. `repair()` läuft bei jedem Laden, die Migration ist deshalb **idempotent** und überschreibt keine schon getroffene Wahl. Drei Tests: Übernahme, Wiederholbarkeit, Vorrang der bestehenden Wahl |
| D69 | Das **Kontextmenü von WebView2 ist im Auslieferungsbau abgeschaltet** | Ein Rechtsklick zeigte Chromes Menü: „Zurück", „Speichern unter", „Drucken", „Untersuchen". In einer Desktop-Anwendung ist das falsch — es verrät den Browser darunter, und die Einträge tun teils Unsinn (ein Popup „drucken", das Fenster „zurück" navigieren). In der Entwicklung bleibt es, weil „Untersuchen" dort das einzige Werkzeug ist |
| D70 | Ein Start durch den Autostart wird an der **Marke `--autostart`** im Registrierungseintrag erkannt | Windows gibt keine Auskunft darüber, ob ein Prozess aus dem `Run`-Schlüssel kam. Unterscheiden muss man aber, weil beides Verschiedenes bedeutet: bei der Anmeldung soll nichts aufgehen, bei einem Doppelklick will man das Fenster sehen. Verglichen wird auf **Gleichheit**, nicht mit `contains` — sonst wäre `C:\--autostart\luchsr.exe` ein Autostart |
| D71 | Beim Start schlägt die **fehlende Einrichtung alles** — auch den Autostart | Ein unkonfiguriertes Programm, das verborgen startet, ist unsichtbar und tut nichts; man sähe nur ein Tray-Icon im Zustand „getrennt" und wüsste nicht, warum. Ein Test prüft alle vier Kombinationen |
| D72 | `start_minimised` wirkt nur noch auf den **manuellen** Start. Vorgabe bleibt **an**, wie in der Parametertabelle | Der Autostart verbirgt das Fenster jetzt unabhängig von der Einstellung, also bleibt ihr nur der Doppelklick. Die Vorgabe `true` bedeutet dort: ein Doppelklick legt nur das Tray-Icon an, ohne Fenster. Ich hatte sie auf `false` gestellt — und zurückgenommen, weil die Tabelle des Auftrags `an` sagt und ein Vorgabewert dem Auftraggeber gehört. Der Nebeneffekt ist im Feldkommentar vermerkt |
| D73 | Ein zweiter Start **mit** Autostartmarke tut nichts | Der Fall tritt auf, wenn beim Anmelden schon eine Instanz läuft — nach einem Benutzerwechsel oder bei doppeltem `Run`-Eintrag. Ein aufspringendes Fenster wäre dann genau das, was der Autostart nicht tun soll. Beide Wege sind gegen den echten Prozess nachgewiesen |
| D74 | `tauri-plugin-single-instance` wird als **erstes** Plugin registriert | Der Rückruf muss greifen, bevor ein anderes Plugin oder das Fenster aufgebaut wird. Sonst startet die zweite Instanz halb, bevor sie sich beendet — im ungünstigen Fall mit einem zweiten Tray-Icon, das gleich wieder verschwindet |
| D75 | Bei jedem Start wird der Registrierungseintrag gegen die Einstellung **abgeglichen**, nicht nur beim ersten Mal gesetzt — **verschärft in D80**, weil dieser Abgleich nur den Schalter verglich und nicht den Pfad | Beides kann auseinanderlaufen: jemand räumt den `Run`-Schlüssel auf, eine Gruppenrichtlinie greift ein, das Programm wird verschoben. Der Abgleich schreibt den Grund ins Protokoll — sonst fällt eine Abweichung nie auf. Der **Vermerk** `autostart_initialised` bleibt trotzdem nötig: ohne ihn wäre eine Abschaltung durch den Benutzer nicht möglich, weil sie bei jedem Start überschrieben würde |
| D76 | Der Autostart wird **nach** dem Fenster gesetzt, und `settings_save` stellt ihn mit um | Ein Fehlschlag beim Registrierungszugriff darf den Start nicht verhindern. Und ohne den Aufruf in `settings_save` wäre der Schalter im Dialog eine Notiz ohne Wirkung: gespeichert, aber nichts täte sich. Scheitert er dort, sind die übrigen Einstellungen trotzdem gespeichert und die Meldung sagt das — ein Rückrollen wäre schlimmer |
| D77 | **Installationspfad und MSI-Dateiname folgen Tauris Vorgabe** statt der ursprünglichen Namenskonvention: `%ProgramFiles%\Luchsr` und `Luchsr_<version>_x64_de-DE.msi` | Die Konvention wollte `%ProgramFiles%\leosysr\Luchsr` und `Luchsr-<version>-x64.msi`. Beides ist nur über eine eigene WiX-Vorlage erreichbar, die bei jedem Tauri-Update gegenzulesen wäre — für einen Herstellerordner und drei Trennzeichen ist das der falsche Tausch. Nach Absprache übernommen und die Konvention an die Wirklichkeit angepasst, statt eine Wunschangabe stehen zu lassen, die niemand einhält. Nachgeprüft **an der gebauten MSI**, nicht an der Dokumentation: `ALLUSERS=1` (per Machine), Upgrade-GUID stabil, Startmenüeintrag vorhanden |
| D78 | Die Lizenz-Volltexte werden **mit installiert** (`bundle.resources`), nicht nur ins Repository gelegt | Die Schriften sind in `luchsr.exe` eingebettet (D4), und die SIL Open Font License verlangt in Abschnitt 2, dass ihr Text jede Kopie begleitet — eine MSI ist eine Kopie. Dasselbe für MPL-2.0, Abschnitt 3.1. Der erste Build enthielt **nur** `luchsr.exe`; aufgefallen ist das, weil ich die Dateiliste der MSI ausgelesen habe statt anzunehmen, dass es passt |
| D79 | `bundle.resources` steht als **Zuordnung** mit ausdrücklichem Ziel, nicht als Liste | Die Listenform übersetzt das `..` im Quellpfad in einen Verzeichnisnamen `_up_`: die Lizenztexte landeten unter `C:\Program Files\Luchsr\_up_\LICENSE`. Aufgefallen ist das erst beim **Installieren** — in der Dateiliste der MSI sieht man nur Namen, nicht Pfade. Die Zuordnungsform setzt das Ziel und behebt es; den **Dateinamen** ändert sie allerdings nicht, `LICENSE` bleibt ohne Endung. Ausserdem lässt `tauri.conf.json` **keine Kommentarschlüssel** zu (`//resources` bricht die Schemaprüfung ab); Begründungen gehören deshalb hierher |
| D80 | Solange Autostart „an" ist, wird der Registrierungseintrag bei **jedem** Start neu geschrieben — nicht nur, wenn keiner da ist | Der Eintrag enthält den **Pfad** der ausführbaren Datei. Dass einer existiert, sagt nichts darüber, ob er auf *diese* Datei zeigt. Genau das ist passiert: nach der MSI-Installation zeigte er weiter auf `target\debug\luchsr.exe`, weil die Prüfung aus D75 nur „ist ein Eintrag da?" verglich. Verschwindet der alte Pfad, startet nichts mehr — und niemand merkt es, weil ein fehlender Autostart keine Meldung erzeugt. Jetzt heilt sich der Eintrag selbst; nachgewiesen an der Registry vor und nach dem Start. Die Entscheidung steckt in `autostart_action` und ist geprüft |

| D81 | **24 Klänge in sechs Familien** — Sinus, Marimba, Glocke, Blip, Tropfen, Akkord. Variiert wird die **Klangfarbe**, nicht die Länge | Auf Wunsch nach mehr Auswahl. Die Grenze von zwei bis vier Tönen unter 350 ms gilt weiter für jede Familie: mehr Auswahl soll nicht heissen, dass die Klänge länger werden — der Grund aus D63 bleibt. Innerhalb jeder Familie ist die Tonrichtung gleich belegt (aufwärts = Hinweis/Entwarnung, abwärts = Warnung/Kritisch), damit die Bedeutung erkennbar bleibt, egal welche Klangfarbe gewählt ist. Die ersten sechs Kennungen sind unverändert, damit eine gespeicherte Auswahl weiter gilt |
| D82 | Der **Dateiname wird aus der Kennung abgeleitet** (`klang!`-Makro), und `BUILTIN` ist ein Slice statt eines Arrays mit fester Länge | Vorher stand `id` und `include_bytes!("…/id.wav")` nebeneinander — ein Eintrag, der eine andere Datei einbindet als er behauptet, war formulierbar und wäre erst zur Laufzeit als falscher Klang aufgefallen. Mit `concat!` aus der Kennung ist das nicht mehr möglich, und eine Kennung ohne Datei ist ein Compilerfehler. Das Slice nimmt die Länge aus dem Typ: eine Zahl, die bei jedem neuen Klang mitzuführen wäre, ist eine zweite Wahrheit |
| D83 | Die Knackprüfung misst die **Ränder**, nicht die Flankensteilheit | Der erste Versuch prüfte den Sprung zwischen aufeinanderfolgenden Abtastwerten und beanstandete acht Klänge — zu Unrecht: eine Pulswelle hat von Natur aus steile Flanken, das ist ihr Timbre. Die Prüfung hätte die ganze `blip`-Familie unmöglich gemacht, obwohl an ihr nichts falsch ist. Ein Test, der die falsche Eigenschaft misst, verbietet richtige Entwürfe. Gemessen wird jetzt, ob die Datei bei nahezu null beginnt und endet — das ist die Bedingung, unter der es knackt |
| D84 | Jeder Klang wird auf denselben **Spitzenwert normiert** | Die Familien haben von Natur aus verschiedene Amplituden: ein Dreiklang summiert drei Stimmen, eine Pulswelle trägt mehr Energie als ein Sinus. Ohne Normierung wäre das Durchhören der Auswahl ein Lautstärkeritt, und die Wahl fiele nach Lautstärke statt nach Charakter |
| D85 | Das Auswahlfeld gruppiert nach Familie (`<optgroup>`), und zwar über **aufeinanderfolgende** Einträge | 25 flache Einträge liest niemand durch. Gruppiert wird nur, was benachbart ist: dann bestimmt die Reihenfolge des Aufrufers die Reihenfolge der Gruppen, und es gibt keine zweite, verborgene Sortierregel im Primitive. `Kein Ton` bleibt ohne Gruppe und deshalb oben |

| D86 | **Andere Monitoringwerkzeuge werden namentlich nicht erwähnt** — nicht im README, nicht hier, nicht in Kommentaren. Luchsr wird aus sich heraus beschrieben, nicht als Abgrenzung | Auf ausdrücklichen Wunsch des Auftraggebers. Betroffen waren sechs Stellen, zwei davon Begründungen, die inhaltlich an einem Vergleich hingen: die Lizenzwahl und die Schwere-Sortierung. Beide sind jetzt aus der Sache begründet statt aus einer fremden Konvention — bei der Sortierung etwa: ist der Host weg, ist jede Aussage über seine Services wertlos, und „der Check funktioniert nicht" ist weniger dringend als „der Dienst ist kaputt". Das ist die bessere Begründung, weil sie erklärt statt zu verweisen. **Diese Regel gilt auch für diesen Eintrag selbst** — ein Verbot, das den verbotenen Namen nennt, hebt sich auf. In der Nachricht von Commit `6d1b4ce` steht der Name noch; ihn dort zu entfernen erfordert einen Force-Push |
| D87 | Die Pakete werden **in CI gebaut und als Release veröffentlicht**, nicht von Hand hochgeladen | Eine MSI, die auf einem Entwicklungsrechner entsteht, ist nur so nachvollziehbar wie dieser Rechner. Der Workflow läuft auf einem frischen System und durchläuft vorher die vollständige Prüfkette — ein Release entsteht nicht aus einem Stand, der die Tests nicht besteht. Er prüft ausserdem, dass der **Tag zur Version** in `tauri.conf.json` passt: sonst entstünde ein Release, dessen Dateiname etwas anderes sagt als sein Name, und das fällt erst beim Installieren auf |
| D88 | Zum Release gehören **SHA256-Prüfsummen** | Die Pakete sind nicht codesigniert (nach Absprache). Eine veröffentlichte Prüfsumme ist der Ersatz, der wenigstens die Frage beantwortet, ob die geladene Datei die gebaute ist. Ohne sie hat ein Herunterladender **keine** Möglichkeit, das zu prüfen |
| D89 | Für „Datei an ein Release hängen" wird das vorinstallierte `gh` benutzt, keine Fremd-Action | Eine Action, die Schreibrechte auf das Repository bekommt, ist eine Abhängigkeit in der Lieferkette. Für einen Aufruf, den `gh` mit einem Befehl erledigt, ist das der falsche Tausch. Die Argumentliste wird als Array übergeben und mit dem Aufrufoperator ausgeführt — `@`-Splatting an ein natives Programm verhält sich zwischen den PowerShell-Fassungen unterschiedlich |
| D90 | Ein **manueller** Workflow-Lauf legt kein Release an, sondern nur ein Artefakt | Sonst füllt sich die Release-Liste mit Testbauten, und die Frage „welches ist die richtige Fassung" ist genau die, die ein Release beantworten soll |
| D91 | Der Toast wird **direkt** über `tauri-winrt-notification` gebaut; `tauri-plugin-notification` ist entfernt | Über das Plugin sind weder Name noch Logo erreichbar, und beides ist im Quelltext der Abhängigkeiten nachgelesen, nicht vermutet. Erstens setzt das Plugin die AppUserModelID **nur**, wenn die ausführbare Datei nicht unter `target\debug` oder `target\release` liegt — im Entwicklungsbau bleibt sie ungesetzt, und `notify-rust` fällt dann auf `Toast::POWERSHELL_APP_ID` zurück. Genau daher stand „Windows PowerShell" in der Kopfzeile. Zweitens liest die Windows-Umsetzung von `notify-rust` für das Bild nur `path_to_image`, während das Plugin `icon` setzt: das Symbol wird verworfen, egal was man angibt. Die Crate lag ohnehin im Baum, unter dem Plugin — der Tausch entfernt eine Abhängigkeit, statt eine hinzuzufügen |
| D92 | Die AppUserModelID wird **selbst** unter `HKCU\Software\Classes\AppUserModelId\<AUMID>` eingetragen und bei **jedem** Start abgeglichen | Ohne Eintrag hat Windows keine Quelle für Name und Symbol in der Kopfzeile. Der Schlüssel liegt im Benutzerzweig und braucht keine erhöhten Rechte. Abgeglichen statt einmalig gesetzt, aus derselben Lehre wie D80: `IconUri` ist ein **Pfad**, und dass ein Eintrag existiert, sagt nichts darüber, ob er auf eine vorhandene Datei zeigt. `ShowInSettings` steht auf 1, sonst fände der Benutzer in den Windows-Benachrichtigungseinstellungen keinen Eintrag — und hätte keinen Weg, die Toasts dort zu regeln |
| D93 | Die Toast-Logos sind per `include_bytes!` eingebaut, werden aber beim Start **auf die Platte ausgepackt** | Anders als der Klang, der mit `SND_MEMORY` aus dem Speicher läuft (D64), lädt Windows das Bild eines Toasts ausschliesslich über eine `file:///`-Adresse — aus dem Speicher geht es nicht. Eingebaut bleiben sie trotzdem, damit dieselbe Begründung greift: eine Datei, die es zur Laufzeit gibt, kann fehlen. Geschrieben wird nur, was fehlt oder abweicht; damit heilt sich der Bestand selbst, ohne bei jedem Start sinnlos 16 KB zu schreiben. Ziel ist `%LOCALAPPDATA%`, nicht der Ordner neben der Exe: dort wäre es im Entwicklungsbau `target\debug` und nach der Installation ein Verzeichnis unter `%ProgramFiles%`, in das ein gewöhnlicher Benutzer nicht schreiben darf |
| D94 | Das Logo unterscheidet **fünf** Zustände, obwohl `EventKind` nur drei Stufen kennt — dafür führt `NotifyEvent` den Zustand als eigenes Feld mit | Die Stufe bestimmt den Klang und fasst CRIT, DOWN und UNREACHABLE zusammen. Die Farbe soll das nicht: CRIT und DOWN sind zwei getrennte Farbtöne (D23), und im Toast ist genug Platz, den Unterschied zu zeigen. Den Zustand aus dem Titel zurückzulesen wäre die Alternative gewesen — der Titel ist aber Text für Menschen und keine Schnittstelle. Eine Entwarnung ist grün, aus welchem Zustand sie auch kommt |
| D95 | **Gefundener Fehler:** `problem_event` setzte ausnahmslos `EventKind::Critical`. `EventKind::Warning` wurde von `decide` **nie** erzeugt | Eine neue WARN oder UNKNOWN bekam damit den Klang für Kritisches, und die Auswahl „Warnung" in den Einstellungen war ohne jede Wirkung. Die Zuordnung stand seit Slice 8 im Doc-Kommentar von `EventKind` und nie im Code. Unentdeckt blieb es, weil die Klangtests ihre Ereignisse von Hand bauen: die Naht zwischen `decide` und `loudest` war nie durchlaufen. Aufgefallen ist es erst, als das Logo dieselbe Stufe brauchte. Jetzt steht die Zuordnung in `kind_of`, und ein Test prüft sie **durch `decide` hindurch** statt an einem selbstgebauten Ereignis |
| D96 | Die Sammelmeldung trägt Stufe und Farbe des **dringlichsten** übergangenen Ereignisses | Sie hatte vorher gar keine Stufe — als Titel „Luchsr" und einen Rumpf. Mit einem farbigen Logo wäre sie ohne Zustand grün geworden: eine Entwarnung im Aussehen, hinter der fünfundzwanzig kritische Probleme stehen. Das ist die Fehlinformation aus D26 in anderer Gestalt |
| D97 | Ein `#[ignore]`-Test schickt **echte** Toasts zum Ansehen | Ob ein Toast richtig *aussieht*, kann keine Zusicherung beantworten — das entscheidet das Auge, und zwar an einem echten Toast. Ein Test, der ungefragt Benachrichtigungen auf den Bildschirm wirft, gehört aber nicht in einen Durchlauf, den jemand nebenbei startet, und auf einem Bauläufer sieht ohnehin niemand hin. Also ein Werkzeug, kein Wächter: `cargo test -- --ignored toast_augenschein`. Dieselbe Haltung wie bei der Bildmarke — jede Runde wurde gerastert und angesehen, nicht beschrieben |
| D98 | Die Toast-**Kopfzeile trägt kein Symbol**. `IconUri` wird nicht gesetzt — und ein bestehender Eintrag wird **entfernt** | Sie trug eines in der Markenfarbe Mint. Nebeneinander war das verwirrend: ein grünes Symbol neben einer roten Zustandsfläche, und Grün heisst in diesem Programm „OK". Genau **ein** farbiges Element, und das bedeutet eine Sache. Das Symbol der Zustandsfarbe folgen zu lassen wäre die andere Richtung gewesen und ist **falsch**: es kommt aus einer Datei für die ganze Anwendung, und das Info-Center zeichnet alte Toasts daraus neu — alle vergangenen Meldungen trügen die aktuelle Farbe, ein CRIT von vor zehn Minuten stünde nach der Entwarnung in Grün. Das ist D26 in anderer Gestalt. Fünf Kandidaten wurden gerastert und auf hellem **und** dunklem Toast-Grund angesehen, in 16, 24 und 64 px; die Wahl fiel nach dem Bild, nicht nach der Beschreibung |
| D99 | `identity::reconcile` **entfernt** verwaltete Werte, die nicht mehr gewollt sind; die Liste `MANAGED` ist getrennt von `desired` | Ein Abgleich, der nur schreibt, hinterlässt Reste. `IconUri` stand nach D92 in der Registry; hätte D98 ihn bloss nicht mehr geschrieben, zeigte Windows weiter ein Symbol, das niemand mehr angefordert hat — auf jedem Rechner, auf dem die alte Fassung einmal lief. `MANAGED` führt **alle** je verwalteten Namen, damit das auch für die nächste Änderung gilt. Nicht verwaltete Werte bleiben unangetastet: der Schlüssel gehört Windows, nicht diesem Modul, und ein Test hält das fest |
| D100 | `toast::extract` räumt **verwaiste Bilder** im Zielverzeichnis weg, statt eine Liste veralteter Dateinamen zu pflegen | Dasselbe Vorgehen wie `make-sounds.mjs` bei den Klängen. Eine Namensliste wäre nach der zweiten Änderung ein Friedhof; „alles, was kein aktuelles Logo ist" bleibt richtig, ohne gepflegt zu werden. Angefasst werden nur `.png` in einem Verzeichnis, das das Modul selbst anlegt — eine fremde Datei daneben bleibt liegen, und ein Test prüft das. Fehler beim Löschen werden übergangen: das Auslegen ist gelungen, und ein Rest ist kein Grund, den Start zu behelligen |
| D101 | **Gefundener Fehler:** der Schliessknopf tat nichts. `core:window:default` enthält **ausschliesslich lesende** Fensterbefehle — `allow-hide` ist nicht dabei | `getCurrentWindow().hide()` wurde von der Berechtigungsprüfung abgelehnt. Die Ablehnung kam als abgewiesenes Promise zurück, und das vorangestellte `void` warf sie weg: der Knopf tat nichts und hinterliess **keine Spur**, nicht einmal im Protokoll. Behoben als Befehl `hide_popup` im Backend, nicht durch Öffnen der Kapazität: das Fenster wird an drei Stellen verborgen — Tray-Klick, Fokusverlust, Schliessknopf — und die gehören in eine Hand. Der Fehler wird jetzt angezeigt. Die Lehre ist allgemeiner als der Fall: `void` auf einem Promise ist eine Entscheidung, Fehler nicht zu sehen |
| D102 | Der Fokusverlust-Behandler prüft, ob das Fenster **überhaupt noch sichtbar** ist, bevor er die Gnadenfrist setzt | Das Verbergen löst selbst einen Fokusverlust aus. Ohne die Abfrage setzte der Behandler danach die Frist aus D37, und ein sofortiger Klick auf das Tray-Icon nach dem Schliessknopf wäre wirkungslos geblieben — man hätte zweimal klicken müssen, um das Fenster wiederzubekommen. Aufgefallen beim Lesen, nicht beim Ausprobieren: der Fall braucht zwei Klicks in 300 ms |

### Konsequenz aus D9 — prüfbare Invariante

**In `tokens.css` steht jeder konkrete Farbwert genau einmal**, und zwar in einem der
beiden Palettenblöcke (`--st-*` oder `--lx-*`). Alles darunter — semantische Aliase,
Statusgruppe, Schatten — referenziert nur. Das ist mit einem Blick prüfbar: außerhalb
der Palettenblöcke darf keine Zeile ein `#rrggbb` oder ein `rgb(`/`rgba(` enthalten.

## Fallstricke der Toolchain

Alles hier ist einmal aufgetreten und hat Zeit gekostet. Bitte nicht neu entdecken.

**PATH nach der Installation.** Wurde die Sitzung vor der Toolchain-Installation
gestartet, kennt sie `cargo` und `npm` nicht — die Umgebung ist beim Start eingefroren.
In PowerShell vorher den PATH aus der Registry neu aufbauen:

```powershell
$m = [Environment]::GetEnvironmentVariable("Path","Machine"); $u = [Environment]::GetEnvironmentVariable("Path","User"); $env:Path = "$m;$u"
```

**Ein `.ps1` mit Umlauten braucht eine UTF-8-BOM.** Windows PowerShell 5.1 liest
eine Skriptdatei **ohne** BOM als ANSI-Codepage. Die Umlaute in den Zeichenketten
sind dann schon beim Parsen kaputt, lange bevor irgendetwas geschrieben wird — aus
`Abhängigkeiten` wird `AbhÃ¤ngigkeiten`, und die Ausgabekodierung zu ändern hilft
nicht, weil der Fehler vorher passiert. Aufgetreten bei `scripts/third-party.ps1`.
Nachprüfen lässt sich es an den ersten drei Bytes:

```powershell
[System.IO.File]::ReadAllBytes("scripts\third-party.ps1")[0..2]   # 239 187 191
```

Setzen, falls sie fehlen — auch **nach jeder Bearbeitung mit einem Werkzeug, das
ohne BOM zurückschreibt** (`sed`, `perl -i`):

```powershell
$p = "scripts\third-party.ps1"; $c = [System.IO.File]::ReadAllText($p, [System.Text.UTF8Encoding]::new($false)); [System.IO.File]::WriteAllText($p, $c, [System.Text.UTF8Encoding]::new($true))
```

**Das Backtick ist in PowerShell das Escape-Zeichen.** In einer doppelt
zitierten Zeichenkette verschwindet ein einzelnes `` ` `` still — genau das, was
man beim Erzeugen von Markdown mit Codespans braucht. Ein wörtliches Backtick
schreibt man doppelt: ``` `` ```.

**Ein Dateiwächter mag es nicht, wenn im Projektwurzelverzeichnis geschrieben
wird, während er läuft.** `scripts/third-party.ps1` hat `tauri dev` einmal
abgeschossen: Vites Chokidar bekam `EBUSY: resource busy or locked, watch
'THIRD-PARTY.md'` und beendete den Prozess. Erzeugte Dateien deshalb bei
gestopptem Dev-Server schreiben.

**`core:default` erlaubt keine mutierenden Fensterbefehle.** In `core:window:default`
stehen ausschliesslich **lesende**: `allow-is-visible`, `allow-inner-size`,
`allow-theme` und so weiter. `allow-hide`, `allow-show` und `allow-set-focus`
sind **nicht** dabei. Ein `getCurrentWindow().hide()` aus dem Frontend wird
abgelehnt — und zwar als abgewiesenes Promise, nicht als Ausnahme. Steht davor
ein `void`, verschwindet die Ablehnung, und der Knopf tut still nichts. Genau
das ist beim Schliessknopf passiert, siehe D101.

Nachsehen lässt sich der Inhalt eines Berechtigungssatzes im erzeugten
Manifest:

```bash
node -e "const m=require('./src-tauri/gen/schemas/acl-manifests.json'); console.log(m['core:window'].default_permission.permissions.join('\n'))"
```

**`void` auf einem Promise ist eine Entscheidung, Fehler nicht zu sehen.** In
dieser Anwendung gibt es keine Konsole, in die eine unbehandelte Ablehnung
fallen könnte. Jeder Aufruf ins Backend gehört deshalb mit `.catch` versehen —
lieber eine Fehlermeldung im Fenster als ein Knopf, der nichts tut.

**Vite 8 liefert esbuild nicht mehr mit.** `build.minify: "esbuild"` bricht ab
(„Cannot find package 'esbuild'"). Vite 8 minifiziert über Rolldown/oxc — die
Einstellung einfach weglassen.

**TypeScript 7 hat `baseUrl` entfernt.** Fehler TS5102. `paths` funktioniert ohne,
die Pfade sind dann relativ zur `tsconfig.json`.

**Tailwind 4, Namensräume.** Zwei Kollisionen, die als stille Fehlfunktion auftreten:

- `--color-body` und `--text-body` erzeugen **beide** die Klasse `text-body`.
  Deshalb heißt die 16-px-Stufe `--text-base`, nicht `--text-body`.
- `--font-x` (Familie) und `--font-weight-x` (Gewicht) erzeugen **beide** `font-x`.
  Deshalb werden gar keine eigenen Gewichtstokens definiert — Tailwinds
  `font-medium` / `font-bold` / `font-extrabold` treffen die Skala des Exports exakt.

**Tailwind 4 hat keinen `--duration-*`-Namensraum.** Die Variablen landen in `:root`,
aber `duration-fast` als Klasse entsteht nicht. `--ease-*` dagegen ist ein
Namensraum und funktioniert. Die fehlenden Utilities stehen in
`src/styles/utilities.css` als `@utility`.

**keyring 4 ist gegenüber 3 umgebaut.** Es ist jetzt eine Fassade über
`keyring-core` mit austauschbaren Stores. Zwei Fallen:

- Das Feature `v1` ist Pflicht, sonst bricht der Build mit `compile_error!` ab.
- Das Windows-Backend heißt `windows-native-keyring-store`, nicht `windows-native`.

**Korrektur zu einer früheren Notiz:** der Default-Store muss **nicht** von Hand
registriert werden. Mit `v1` erledigt `Entry::new()` das beim ersten Aufruf über
einen `LazyLock`. Nützlich ist aber `Entry::store_status()` — das stößt die
Initialisierung an und gibt ihr Ergebnis zurück, ohne einen Eintrag anzulegen.
Genau dafür gibt es `SecretStore::availability()`: fehlt der Credential Manager,
sieht der Benutzer eine klare Meldung beim Öffnen des Dialogs und nicht erst beim
Speichern seines Secrets.

**Selbsttests gegen den echten Credential Manager.** `config/secrets.rs` testet
Speichern, Lesen und Löschen gegen den echten Windows-Store — das ist die einzige
Stelle, an der die keyring-API überhaupt geprüft wird. Die Testkonten heißen
`luchsr-selbsttest-*` und werden über einen `Drop`-Wächter aufgeräumt, auch wenn
ein Test panisch wird. Rückstände findet man mit:

```bash
cmdkey /list | findstr leosysr.Luchsr
```

**reqwest 0.13 liefert `default-tls = [rustls]`.** Die Defaults würden rustls
hereinziehen, deshalb ist `default-features = false` Pflicht. Dadurch müssen aber
auch die harmlosen Defaults einzeln nachgezogen werden: `query`, `system-proxy`,
`http2`, `charset`.

**`reqwest` und Windows lesen den Proxy aus verschiedenen Quellen.** Das ist der
Fallstrick, der am meisten Zeit gekostet hat.

| Programm | Quelle |
|---|---|
| `reqwest` (Feature `system-proxy`) | Umgebungsvariablen `HTTP_PROXY`, `HTTPS_PROXY`, `ALL_PROXY`, `NO_PROXY` |
| Browser | WinINET, meist über eine PAC-Datei |
| .NET / PowerShell | WinINET |
| `netsh winhttp` | WinHTTP, davon unabhängig |

Diese Quellen laufen in Firmennetzen auseinander. Beobachtet: `HTTP_PROXY` auf
einen Firmenproxy gesetzt, `NO_PROXY` nur mit `localhost,127.0.0.1,::1,.local`,
in WinINET `ProxyEnable = 0`. Browser und PowerShell erreichten den internen
CheckMK-Server direkt, Luchsr schickte an den Proxy — und bekam **403 Forbidden
mit einer HTML-Seite**. Das ist von einem CheckMK-Berechtigungsproblem nicht zu
unterscheiden, wenn man den Rumpf nicht anschaut.

Zur Diagnose gibt es `scripts/checkmk-probe.ps1`. Es fragt das Secret verborgen
ab und zeigt Status, `Content-Type` und Rumpf. Die Unterscheidung:

| Antwort | Herkunft |
|---|---|
| `application/problem+json` mit `detail` | CheckMK |
| `text/html`, kein `Server`-Header | Proxy oder Apache davor |

**rustls in `Cargo.lock` ist kein Fehler.** Die Lockdatei erfasst alle Plattformen.
Ob rustls im Windows-Build landet, sagt nur:

```bash
cargo tree --invert rustls --edges normal
```

Antwortet das mit „nothing to print", ist alles in Ordnung. Die Gegenprobe
`cargo tree --invert schannel` muss die Kette
`schannel ← native-tls ← reqwest ← luchsr` zeigen — das ist der Nachweis, dass TLS
über den Windows-Zertifikatspeicher läuft.

## Vorgehen — Slices

Nach jedem Slice anhalten und dem Benutzer das Ergebnis zeigen, bevor es weitergeht.

1. ✅ **Bestandsaufnahme** — Voraussetzungen geprüft, `handover-design` ausgelesen
2. ✅ **Gerüst** — Tauri-2-Projekt, Abhängigkeiten, Ordnerstruktur, `tokens.css`, `tauri dev` läuft
3. ✅ **API-Client** — Modul `checkmk`, 125 Unit-Tests gegen JSON-Fixtures, clippy `-D warnings` sauber
4. ✅ **Konfiguration und Credentials** — `config`-Modul, Credential Manager, Einstellungsdialog, Ersteinrichtung, Verbindungstest. 194 Tests gesamt
5. ✅ **Tray und Polling-Loop** — sechs Icon-Zustände, Kontextmenü, Abrufschleife mit Jitter/Backoff/Standby, Fensterpositionierung. 257 Tests gesamt
6. ✅ **Popup-UI** — rahmenloses Fenster am Tray, virtualisierte Liste, Host-Gruppierung, Filter, Detail-Panel, CSV-Export. 297 Rust-Tests, 47 Frontend-Tests
7. ✅ **Aktionen** — Quittieren und Wartungszeit, Kommentarvorlagen, Berechtigungsprüfung im Backend. 316 Rust-Tests, 47 Frontend-Tests
8. ✅ **Benachrichtigungen** — Toasts bei Änderung, Entwarnungen, Deckelung, 24 erzeugte Hinweistöne in sechs Klangfamilien mit Auswahl je Ereignis. 383 Rust-Tests, 47 Frontend-Tests
9. ✅ **Autostart, Single-Instance, Startverhalten** — Registry-Eintrag mit Marke, zweiter Start holt das Fenster vor, Fensterentscheidung rein und getestet. 378 Rust-Tests, 47 Frontend-Tests
10. ✅ **Packaging und README** — MSI und NSIS gebaut, Silent-Install gegen den echten Rechner belegt, Lizenztexte im Paket, unsigniert. 380 Rust-Tests, 47 Frontend-Tests

Nach Slice 10:

- **Release über CI** — Tag löst Bau und Veröffentlichung aus, mit Prüfsummen (D87–D90)
- **Toasts mit eigener Identität** — Name, Symbol und farbiges Zustandslogo statt „Windows PowerShell"; dabei ein Fehler in der Stufenzuordnung gefunden und behoben (D91–D97). 407 Rust-Tests, 47 Frontend-Tests
- **Kopfsymbol und Schliessknopf** — Toast-Kopfzeile ohne Symbol, damit nur ein
  Element Farbe trägt; dabei fiel auf, dass das × nie funktioniert hat (D98–D102).
  412 Rust-Tests, 47 Frontend-Tests

## Bei Unklarheit

Nachfragen statt raten. Das gilt besonders für Designentscheidungen, die über die Tokens
hinausgehen, und für alles, was den Rahmen des Auftrags verlässt.
