<#
.SYNOPSIS
    Erzeugt THIRD-PARTY.md aus den echten Abhängigkeitsgraphen.

.DESCRIPTION
    Zwei Quellen, beide maschinell gelesen statt gepflegt:

      Rust   `cargo tree` für das Ziel x86_64-pc-windows-msvc. Bewusst nur
             dieses Ziel — Cargo.lock erfasst alle Plattformen, und eine
             Liste, die Crates nennt, die im Windows-Build gar nicht landen,
             ist keine Auskunft, sondern Lärm.
      npm    die Laufzeitabhängigkeiten (`--omit=dev`). Devabhängigkeiten
             werden nicht ausgeliefert; Vite bündelt nur, was im Code steht.

    Nach jeder Änderung an Abhängigkeiten neu laufen lassen:

        pwsh -File scripts/third-party.ps1

.NOTES
    Die Lizenz-VOLLTEXTE liegen unter licenses/ und werden hier nicht
    erzeugt — sie müssen wortgetreu sein und gehören deshalb nicht in ein
    Skript. Diese Datei nennt nur, was drin ist.
#>

[CmdletBinding()]
param(
    [string]$OutFile
)

$ErrorActionPreference = 'Stop'

# Nicht als Parametervorgabe: in Windows PowerShell 5.1 ist $PSScriptRoot dort
# je nach Aufrufweg leer, und Join-Path scheitert dann mit einer Meldung, die
# nicht auf die Ursache zeigt.
$hier = if ($PSScriptRoot) { $PSScriptRoot } else { Split-Path -Parent $MyInvocation.MyCommand.Path }
$repo = Resolve-Path (Join-Path $hier '..')
if (-not $OutFile) { $OutFile = Join-Path $repo 'THIRD-PARTY.md' }
$tauri = Join-Path $repo 'src-tauri'

# ---------------------------------------------------------------- Rust ----

Push-Location $tauri
try {
    $raw = cargo tree --target x86_64-pc-windows-msvc --edges normal `
        --prefix none --format '{p}|{l}' 2>$null
}
finally {
    Pop-Location
}

$crates = $raw |
    Where-Object { $_ -match '\|' } |
    # cargo tree markiert Wiederholungen mit " (*)" — das ist Darstellung,
    # kein Bestandteil des Namens.
    ForEach-Object { ($_ -replace '\s*\(\*\)\s*$', '').Trim() } |
    Sort-Object -Unique |
    ForEach-Object {
        $teile = $_ -split '\|', 2
        [pscustomobject]@{
            Name    = $teile[0].Trim()
            License = if ($teile.Count -gt 1 -and $teile[1].Trim()) { $teile[1].Trim() } else { '(nicht angegeben)' }
        }
    }

# ----------------------------------------------------------------- npm ----

Push-Location $repo
try {
    $pfade = npm ls --omit=dev --all --parseable 2>$null
}
finally {
    Pop-Location
}

$pakete = $pfade |
    Where-Object { $_ -match 'node_modules' } |
    ForEach-Object { $_ -replace '^.*node_modules[\\/]', '' } |
    Sort-Object -Unique |
    ForEach-Object {
        $pj = Join-Path $repo ('node_modules\' + ($_ -replace '/', '\') + '\package.json')
        if (Test-Path $pj) {
            $j = Get-Content $pj -Raw | ConvertFrom-Json
            [pscustomobject]@{
                Name    = "$($j.name)@$($j.version)"
                License = if ($j.license) { $j.license } else { '(nicht angegeben)' }
            }
        }
    } | Where-Object { $_ }

# -------------------------------------------------------------- Ausgabe ----

$zeilen = [System.Collections.Generic.List[string]]::new()
$add = { param($s) $zeilen.Add($s) }

& $add '# Fremdbestandteile'
& $add ''
& $add 'Luchsr selbst steht unter MIT (siehe `LICENSE`). Diese Datei listet, was'
& $add 'mit ausgeliefert wird, und unter welcher Lizenz.'
& $add ''
& $add '**Diese Datei wird erzeugt.** Nicht von Hand pflegen — `scripts/third-party.ps1`'
& $add 'liest die tatsächlichen Abhängigkeitsgraphen. Die Lizenz-Volltexte liegen'
& $add 'unter `licenses/`.'
& $add ''
& $add '## Schriften'
& $add ''
& $add 'Lokal eingebettet (Entscheidung D4: kein Laufzeit-Netzzugriff). Die SIL Open'
& $add 'Font License verlangt, dass ihr Text die Schriftdateien begleitet — deshalb'
& $add 'liegt er unter `licenses/`.'
& $add ''
& $add '| Schrift | Lizenz | Volltext |'
& $add '|---|---|---|'
& $add '| Manrope | SIL Open Font License 1.1 | `licenses/OFL-1.1-Manrope.txt` |'
& $add '| IBM Plex Mono | SIL Open Font License 1.1 | `licenses/OFL-1.1-IBM-Plex.txt` |'
& $add ''
& $add '## Schwaches Copyleft — MPL-2.0'
& $add ''
& $add 'Fünf Crates aus dem Tauri-Unterbau stehen unter MPL-2.0. Das ist mit MIT'
& $add 'vereinbar: die MPL ist dateiweise und erlaubt ausdrücklich die Einbettung in'
& $add 'ein „Larger Work" unter anderer Lizenz (Abschnitt 3.3). Zwei Pflichten'
& $add 'bleiben, und beide sind hier erfüllt: der Lizenztext liegt bei'
& $add '(`licenses/MPL-2.0.txt`), und der Quelltext der betroffenen Dateien ist'
& $add 'verfügbar — sie sind unverändert übernommen und über crates.io abrufbar.'
& $add ''
$mpl = $crates | Where-Object { $_.License -like '*MPL*' }
& $add '| Crate | Quelle |'
& $add '|---|---|'
foreach ($c in $mpl) {
    $name = ($c.Name -split ' ')[0]
    & $add "| ``$($c.Name)`` | https://crates.io/crates/$name |"
}
& $add ''
& $add '## Rust-Crates'
& $add ''
& $add "Im Windows-Build (``x86_64-pc-windows-msvc``) enthalten: **$($crates.Count)** Crates."
& $add 'Cargo.lock erfasst zusätzlich Crates anderer Plattformen; die stehen hier'
& $add 'absichtlich nicht.'
& $add ''
& $add '### Verteilung'
& $add ''
& $add '| Anzahl | Lizenz |'
& $add '|---|---|'
foreach ($g in ($crates | Group-Object License | Sort-Object Count -Descending)) {
    & $add "| $($g.Count) | $($g.Name) |"
}
& $add ''
& $add '### Vollständige Liste'
& $add ''
& $add '| Crate | Lizenz |'
& $add '|---|---|'
foreach ($c in $crates) {
    & $add "| ``$($c.Name)`` | $($c.License) |"
}
& $add ''
& $add '## npm-Laufzeitabhängigkeiten'
& $add ''
& $add "Gebündelt in ``dist/``: **$($pakete.Count)** Pakete. Devabhängigkeiten"
& $add '(Vite, TypeScript, vitest, Tailwind) werden nicht ausgeliefert.'
& $add ''
& $add '| Paket | Lizenz |'
& $add '|---|---|'
foreach ($p in $pakete) {
    & $add "| ``$($p.Name)`` | $($p.License) |"
}
& $add ''

# Ohne BOM schreiben. `Set-Content -Encoding utf8` setzt in Windows
# PowerShell 5.1 eine BOM voran; in einer Markdown-Datei ist die unnötig und
# taucht in Diffs als unsichtbares erstes Zeichen auf.
$ziel = if ([System.IO.Path]::IsPathRooted($OutFile)) { $OutFile } else { Join-Path $repo $OutFile }
[System.IO.File]::WriteAllLines($ziel, $zeilen, [System.Text.UTF8Encoding]::new($false))
Write-Host "THIRD-PARTY.md geschrieben: $($crates.Count) Crates, $($pakete.Count) npm-Pakete, $($mpl.Count) MPL-2.0"
