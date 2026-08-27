/**
 * Deutsche Texte. Aktuell die einzige ausgelieferte Sprache.
 *
 * Struktur ist bewusst flach mit punktierten Schlüsseln: eine weitere Sprache
 * ist damit eine Datei mit denselben Schlüsseln, kein Umbau. Der Typ dieser
 * Datei ist der Vertrag (siehe ./index.ts).
 *
 * Ton: sachlich. Technische Werte bleiben unübersetzt. Fehlermeldungen kommen
 * aus dem Backend und stehen deshalb nicht hier — sie sind dort schon deutsch
 * und konkret, siehe `checkmk/error.rs` und `config/error.rs`.
 */

export const de = {
  "app.name": "Luchsr",
  "app.startFailed": "Luchsr konnte nicht starten",
  "app.renderFailed": "Die Anzeige ist abgestürzt",
  "app.renderFailedHint":
    "Der Fehler steht im Protokoll unter %LOCALAPPDATA%\\de.leosysr.luchsr\\logs.",

  // ------------------------------------------------ Über das Programm --
  "about.author": "Fabian Schatto – leosysr",
  "about.license": "MIT-Lizenz",
  "about.ai":
    "Entwickelt mit Claude von Anthropic. Entwürfe, Quelltext und Tests entstanden im Dialog mit einem KI-Modell; Auftrag, Entscheidungen und Prüfung liegen beim Autor.",
  "update.check": "Nach Updates suchen",
  "update.checking": "Wird nachgesehen …",
  "update.upToDate": "Diese Fassung ist die aktuelle.",
  // Ohne Platzhalter: die Version wird als eigenes Element in Mono angehängt,
  // wie alle technischen Werte.
  "update.available": "Neuere Fassung verfügbar:",
  "update.ahead": "Diese Fassung ist neuer als das jüngste Release:",
  "update.openRelease": "Zum Release",
  "update.hint":
    "Installiert wird nicht automatisch: Luchsr liegt unter %ProgramFiles% und ein Update braucht Administratorrechte.",

  // ---------------------------------------------------------------- Status --
  "status.ok": "OK",
  "status.warn": "Warnung",
  "status.crit": "Kritisch",
  "status.unknown": "Unbekannt",
  "status.down": "Host nicht erreichbar",
  "status.stale": "Veraltet",
  "status.waiting": "Warte auf den ersten Abruf …",
  "status.lastError": "Letzter Abruf fehlgeschlagen",
  "status.consecutiveFailures": "Fehlversuche hintereinander",
  "status.showingStale": "Angezeigt wird der letzte erfolgreiche Abruf.",

  // ---------------------------------------------------------------- Popup --
  "popup.pin": "Fenster anheften",
  "popup.unpin": "Anheftung lösen",
  "popup.close": "Fenster schließen",
  "popup.backToList": "Zurück zur Liste",

  // ------------------------------------------------------------ Tray-Menü --
  "tray.open": "Öffnen",
  "tray.refresh": "Jetzt aktualisieren",
  "tray.openInBrowser": "CheckMK im Browser öffnen",
  "tray.settings": "Einstellungen",
  "tray.quit": "Beenden",

  // ---------------------------------------------------------------- Liste --
  "list.column.status": "Status",
  "list.column.host": "Host",
  "list.column.service": "Service",
  "list.column.duration": "Dauer",
  "list.column.output": "Ausgabe",
  "list.empty": "Keine offenen Probleme",
  "list.filterPlaceholder": "Host oder Service filtern",
  "list.showHandled": "Quittierte und Wartung anzeigen",
  "list.clearFilter": "Filter löschen",
  "list.filterStates": "Nach Status filtern",
  "list.expand": "Services einblenden",
  "list.collapse": "Services einklappen",
  "list.groupedServices": "{n} Services betroffen",
  "list.flapping": "flattert",

  // ------------------------------------------------------------- Detail --
  "detail.state": "Status",
  "detail.since": "Seit",
  "detail.duration": "Dauer",
  "detail.output": "Ausgabe",
  "detail.acknowledged": "Quittiert",
  "detail.downtime": "Wartungszeit",
  "detail.flapping": "Flattert",
  "detail.region": "Details zum Problem",
  "detail.hostProblem": "Hostproblem",
  "detail.alreadyAcknowledged": "Ist bereits quittiert.",
  "detail.alreadyDowntime": "Steht bereits in einer Wartungszeit.",
  "detail.close": "Detail schließen",
  "detail.downtimeDepth": "Tiefe {n}",

  // -------------------------------------------------------------- Aktionen --
  "action.openInCheckmk": "In CheckMK öffnen",
  "action.acknowledge": "Quittieren",
  "action.downtime": "Wartungszeit setzen",
  "action.cancel": "Abbrechen",
  "action.running": "Läuft …",
  "action.comment": "Kommentar",
  "action.commentHint":
    "Landet in der CheckMK-Historie. Die Vorlage steht in den Einstellungen. Strg+Enter führt aus.",
  "action.save": "Speichern",
  "action.saved": "Gespeichert",
  "action.discard": "Änderungen verwerfen",
  "action.continue": "Weiter",
  "action.details": "Technische Details",
  "action.exportCsv": "Liste als CSV exportieren",
  "action.failed": "Aktion fehlgeschlagen",
  "action.doneHint": "Der Stand wird gerade neu abgerufen.",
  "action.reload": "Anzeige neu laden",

  // ---------------------------------------------------------------- Export --
  "export.done": "CSV geschrieben",

  // ------------------------------------------------------------ Wartungszeit --
  "downtime.duration": "Dauer",
  "downtime.minutes15": "15 Minuten",
  "downtime.hour1": "1 Stunde",
  "downtime.hours4": "4 Stunden",
  "downtime.untilMorning": "Bis morgen früh",
  "downtime.custom": "Frei",
  "downtime.minutes": "Minuten",

  // ================================================== Ersteinrichtung ======
  "setup.title": "Ersteinrichtung",
  "setup.intro":
    "Luchsr braucht Zugang zu einer CheckMK-Instanz. Server, Site und ein Automation-Secret genügen; alles Weitere lässt sich später in den Einstellungen ändern.",
  "setup.secretHint":
    "Das Automation-Secret findest du in CheckMK unter „Benutzer“ beim jeweiligen Konto. Es ist nicht dasselbe wie das Anmeldekennwort.",
  "setup.testFirst": "Bitte zuerst die Verbindung prüfen.",

  // ==================================================== Einstellungen ======
  "settings.title": "Einstellungen",
  "settings.unsaved": "Es gibt ungespeicherte Änderungen.",
  "settings.notice": "Hinweis beim Laden",

  // --- Verbindung -----------------------------------------------------------
  "settings.connection.section": "Verbindung",
  "settings.connection.kicker": "CheckMK",

  "settings.server.label": "Server-URL",
  "settings.server.hint": "Ohne Pfad, zum Beispiel https://checkmk.example.intern",
  "settings.server.placeholder": "https://checkmk.example.intern",

  "settings.site.label": "Site-Name",
  "settings.site.hint": "Der Name der CheckMK-Site, ergibt zusammen mit der URL den API-Pfad.",
  "settings.site.placeholder": "meinesite",

  "settings.username.label": "Benutzername",
  "settings.username.hint": "Das Konto, zu dem das Automation-Secret gehört.",
  "settings.username.placeholder": "m.mustermann",

  "settings.secret.label": "Automation-Secret",
  "settings.secret.stored": "Gespeichert",
  "settings.secret.missing": "Nicht gesetzt",
  "settings.secret.placeholderStored": "Unverändert lassen",
  "settings.secret.placeholderEmpty": "Automation-Secret eingeben",
  "settings.secret.hint":
    "Wird ausschliesslich im Windows Credential Manager abgelegt, nie in der Konfigurationsdatei. Leer lassen behält das gespeicherte Secret.",
  "settings.secret.delete": "Secret löschen",
  "settings.secret.movedUser":
    "Der Benutzername hat sich geändert. Für den neuen Namen ist noch kein Secret gespeichert.",
  "settings.secret.storeUnavailable":
    "Der Windows Credential Manager ist nicht verfügbar. Ohne ihn kann das Automation-Secret nicht gespeichert werden.",

  "settings.verifyTls.label": "TLS-Prüfung",
  "settings.verifyTls.hint": "Prüft das Serverzertifikat gegen den Windows-Zertifikatspeicher.",
  "settings.verifyTls.warningTitle": "TLS-Prüfung ist abgeschaltet",

  "settings.proxy.label": "Proxy",
  "settings.proxy.system": "System",
  "settings.proxy.none": "Keiner",
  "settings.proxy.manual": "Manuell",
  "settings.proxy.warningTitle": "Ein Proxy der Umgebung würde greifen",
  "settings.proxy.useNone": "Auf „Keiner“ umstellen",
  "settings.proxy.urlLabel": "Proxy-Adresse",
  "settings.proxy.urlHint": "Mit Protokoll, zum Beispiel http://proxy.example.intern:8080",
  "settings.proxy.urlPlaceholder": "http://proxy.example.intern:8080",

  "settings.test.button": "Verbindung testen",
  "settings.test.running": "Verbindung wird geprüft …",
  "settings.test.successTitle": "Verbindung in Ordnung",
  "settings.test.failureTitle": "Verbindung fehlgeschlagen",
  "settings.test.tlsHint":
    "Der saubere Weg ist, das Stammzertifikat der internen CA in den Windows-Zertifikatspeicher aufzunehmen. Die TLS-Prüfung abzuschalten ist eine Notlösung und macht die Verbindung angreifbar.",

  // --- Abruf ----------------------------------------------------------------
  "settings.polling.section": "Abruf",
  "settings.polling.kicker": "Aktualisierung",
  "settings.interval.label": "Abrufintervall",
  "settings.interval.hint": "15 bis 600 Sekunden. Luchsr streut zusätzlich ±10 %, damit nicht alle Clients gleichzeitig abfragen.",
  "settings.timeout.label": "Zeitgrenze je Abruf",
  "settings.timeout.hint": "Bricht einen Abruf ab, der nicht antwortet.",
  "settings.unit.seconds": "s",

  // --- Darstellung ----------------------------------------------------------
  "settings.appearance.section": "Darstellung",
  "settings.appearance.kicker": "Oberfläche",
  "settings.theme.label": "Farbmodus",
  "settings.theme.system": "System",
  "settings.theme.light": "Hell",
  "settings.theme.dark": "Dunkel",
  "settings.language.label": "Sprache",
  "settings.language.hint": "Derzeit wird nur Deutsch ausgeliefert.",

  // --- Verhalten ------------------------------------------------------------
  "settings.behaviour.section": "Verhalten",
  "settings.behaviour.kicker": "Start und Fenster",
  "settings.autostart.label": "Mit Windows starten",
  "settings.autostart.hint": "Legt eine Autostart-Verknüpfung für den aktuellen Benutzer an.",
  "settings.startMinimised.label": "Auch von Hand minimiert starten",
  "settings.startMinimised.hint":
    "Beim Autostart bleibt das Fenster ohnehin zu. Diese Einstellung gilt für einen Start per Doppelklick oder Verknüpfung.",
  "settings.pinPopup.label": "Fenster angeheftet",
  "settings.pinPopup.hint": "Angeheftet bleibt das Fenster offen, bis du es schliesst. Sonst verschwindet es, sobald es den Fokus verliert.",
  "settings.hideHandled.label": "Quittierte und Wartung ausblenden",
  "settings.hideHandled.hint": "Blendet Zustände aus, die bereits bearbeitet sind. Per Umschalter in der Liste sichtbar.",

  // --- Benachrichtigungen ---------------------------------------------------
  "settings.notifications.section": "Benachrichtigungen",
  "settings.notifications.kicker": "Meldungen",
  "settings.notificationLevel.label": "Wann benachrichtigen",
  "settings.notificationLevel.off": "Aus",
  "settings.notificationLevel.criticalOnly": "Nur CRIT und Host DOWN",
  "settings.notificationLevel.allChanges": "Alle Statusänderungen",
  "settings.sound.intro":
    "Ein Klang je Ereignis, jeder einzeln abschaltbar. Die eingebauten sind kurze Hinweistöne von zwei bis vier Tönen; das Symbol daneben spielt sie vor.",
  "settings.sound.none": "Kein Ton",
  "settings.sound.ownFile": "Eigene Datei",
  "settings.sound.preview": "Vorhören",
  "settings.sound.critical": "Kritisches Problem",
  "settings.sound.criticalHint": "Neues CRIT, DOWN oder UNREACHABLE.",
  "settings.sound.warning": "Warnung",
  "settings.sound.warningHint":
    "Neues WARN oder UNKNOWN. Kommt nur, wenn oben „Jede Statusänderung“ gewählt ist.",
  "settings.sound.recovery": "Entwarnung",
  "settings.sound.acknowledged": "Quittieren erfolgreich",
  "settings.sound.downtime": "Wartungszeit gesetzt",

  // --- Aktionen -------------------------------------------------------------
  "settings.permissions.section": "Schreibaktionen",
  "settings.permissions.kicker": "Freigaben",
  "settings.permissions.intro":
    "Beide Aktionen sind standardmässig gesperrt. Freigegeben erscheinen sie im Detail-Panel und verändern den Zustand in CheckMK.",
  "settings.allowAcknowledge.label": "Quittieren erlauben",
  "settings.allowAcknowledge.hint": "Erlaubt, Probleme in CheckMK als quittiert zu markieren.",
  "settings.allowDowntime.label": "Wartungszeit erlauben",
  "settings.allowDowntime.hint": "Erlaubt, in CheckMK Wartungszeiten zu setzen.",
  "settings.comment.intro":
    "Vorlage für den Kommentar, der in CheckMK landet. Der Dialog belegt das Feld damit vor und lässt es überschreiben. Platzhalter:",
  "settings.acknowledgeComment.label": "Vorlage: Quittieren",
  "settings.acknowledgeComment.hint": "Leer lassen setzt die Vorgabe zurück.",
  "settings.downtimeComment.label": "Vorlage: Wartungszeit",
  "settings.downtimeComment.hint": "Leer lassen setzt die Vorgabe zurück.",
} as const;
