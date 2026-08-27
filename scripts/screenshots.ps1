<#
.SYNOPSIS
  Nimmt die README-Bilder der Oberfläche auf.

.DESCRIPTION
  Gerendert wird die echte Anwendung mit vorbereiteten Backend-Antworten —
  siehe src/dev/mockup.tsx. Aufgenommen wird headless über Edge, damit eine
  Datei entsteht und nicht nur ein Bild auf dem Schirm.

  Voraussetzung: `npm run dev` läuft (Port 1420).

  Die Fenstergrösse ist die der Anwendung (780 × 600 aus tauri.conf.json), der
  Skalierungsfaktor 2 — auf einem hochauflösenden Bildschirm wäre ein 1x-Bild
  unscharf, und GitHub skaliert ohnehin herunter.

.EXAMPLE
  pwsh -File scripts/screenshots.ps1
#>

[CmdletBinding()]
param(
  [string]$Url = "http://localhost:1420/mockup.html",
  [int]$Breite = 780,
  [int]$Hoehe = 600
)

$ErrorActionPreference = "Stop"

$wurzel = Split-Path -Parent $PSScriptRoot
$ziel = Join-Path $wurzel "docs\bilder"
New-Item -ItemType Directory -Force $ziel | Out-Null

# Edge bringt Windows mit; ein eigener Chrome ist nicht vorauszusetzen.
$kandidaten = @(
  "C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe",
  "C:\Program Files\Microsoft\Edge\Application\msedge.exe",
  "C:\Program Files\Google\Chrome\Application\chrome.exe"
)
$browser = $kandidaten | Where-Object { Test-Path $_ } | Select-Object -First 1
if (-not $browser) { throw "Kein Edge und kein Chrome gefunden." }

# Erreichbarkeit zuerst: sonst entstehen leere Bilder, und das fällt erst im
# README auf.
try { Invoke-WebRequest -Uri $Url -UseBasicParsing -TimeoutSec 5 | Out-Null }
catch { throw "$Url antwortet nicht. Laeuft ``npm run dev``?" }

# Eigenes Profil: ohne eines greift Edge das laufende Benutzerprofil an und
# weigert sich, headless zu starten.
$profil = Join-Path $env:TEMP "luchsr-screenshots-profil"

$aufnahmen = @(
  @{ Datei = "popup-problemliste.png";  Frage = "" }
  @{ Datei = "popup-einstellungen.png"; Frage = "?view=settings&scroll=2180" }
)

foreach ($a in $aufnahmen) {
  $pfad = Join-Path $ziel $a.Datei
  $argumente = @(
    "--headless=new"
    "--disable-gpu"
    "--hide-scrollbars"
    "--no-first-run"
    "--no-default-browser-check"
    "--force-device-scale-factor=2"
    "--window-size=$Breite,$Hoehe"
    # Zeit für Schriften, React und das Rollen. Ohne Budget entsteht ein
    # halb gezeichnetes Bild.
    "--virtual-time-budget=8000"
    "--user-data-dir=$profil"
    "--screenshot=$pfad"
    ($Url + $a.Frage)
  )
  Start-Process -FilePath $browser -ArgumentList $argumente -Wait -NoNewWindow | Out-Null

  if (-not (Test-Path $pfad)) { throw "$($a.Datei) wurde nicht geschrieben." }
  $groesse = (Get-Item $pfad).Length
  "{0,-28} {1,8:N0} B" -f $a.Datei, $groesse
}

Write-Output ""
Write-Output "Fertig. Bilder ansehen, bevor sie eingecheckt werden — ein Bild,"
Write-Output "das falsch aussieht, besteht keinen Test."
