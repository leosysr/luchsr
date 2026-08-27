# Lizenz-Volltexte

Hier liegen die Lizenztexte von Bestandteilen, die **mit ausgeliefert** werden
und deren Lizenz verlangt, dass ihr Text dabei ist. Welche Bestandteile das
sind, steht in `../THIRD-PARTY.md`.

Diese Texte werden **nicht** erzeugt und nicht aus dem Gedächtnis geschrieben.
Ein Lizenztext muss wortgetreu sein; ein falsches Wort darin ist kein
Schönheitsfehler, sondern hebt die Bedingung auf, die er festhält.

## Was hier liegt

| Datei | Für | Bezogen von |
|---|---|---|
| `OFL-1.1-Manrope.txt` | Schrift Manrope (`src/assets/fonts/manrope-*.woff2`) | `google/fonts/ofl/manrope/OFL.txt` |
| `OFL-1.1-IBM-Plex.txt` | Schrift IBM Plex Mono (`src/assets/fonts/ibm-plex-mono-*.woff2`) | `IBM/plex/LICENSE.txt` |
| `MPL-2.0.txt` | fünf Crates aus dem Tauri-Unterbau, siehe `../THIRD-PARTY.md` | `mozilla.org/media/MPL/2.0/index.txt` |

### Zu Manrope

Bezogen von **Google Fonts**, nicht vom Repository des Urhebers: `sharanda/manrope`
antwortet inzwischen mit 404, das Projekt ist offenbar verschoben. Google Fonts ist
hier ohnehin die passendere Quelle — die ausgelieferten `.woff2`-Dateien sind die
dort erzeugten Teilmengen (latin, latin-ext), nicht die Originaldateien des Urhebers.
Der Copyright-Vermerk im Text nennt weiterhin „The Manrope Project Authors".

### Zu IBM Plex

Die Fassungsfrage ist damit geklärt: die bezogene `LICENSE.txt` ist die **SIL Open
Font License 1.1**. Plex stand bis Version 1 unter Apache-2.0; die hier eingebetteten
Dateien fallen unter die OFL.

## Warum das nicht optional ist

Die **SIL Open Font License** verlangt in Abschnitt 2, dass jede Kopie der
Schrift eine Kopie der Lizenz mitführt. Die Schriften sind hier eingebettet
(Entscheidung D4 in `../CLAUDE.md`: kein Laufzeit-Netzzugriff), werden also
mitkopiert — in das Git-Repository und in die MSI.

Die **MPL-2.0** verlangt in Abschnitt 3.1, dass der Lizenztext den betroffenen
Dateien beiliegt, und in 3.2, dass ihr Quelltext verfügbar ist. Beides ist
erfüllt: die Crates sind unverändert übernommen und über crates.io abrufbar.

## Beim Aktualisieren

Ändert sich eine Abhängigkeit, `scripts/third-party.ps1` neu laufen lassen. Kommt
eine Lizenz dazu, die einen Volltext verlangt (OFL, MPL, EPL, CDDL, jede Form von
Copyleft), gehört er hierher — und diese Tabelle mit der Quelle dazu.
