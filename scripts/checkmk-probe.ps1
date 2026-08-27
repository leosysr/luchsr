<#
.SYNOPSIS
    Prüft die CheckMK-REST-API direkt, unabhängig von Luchsr.

.DESCRIPTION
    Sendet genau die Anfragen, die Luchsr sendet, und zeigt Statuscode und
    vollständigen Antwortkörper. Damit lässt sich trennen, ob ein Problem an
    Luchsr oder an CheckMK liegt.

    Das Automation-Secret wird als SecureString abgefragt. Es landet dadurch
    nicht in der Kommandozeilen-Historie und nicht in einem Skriptparameter,
    der in der Prozessliste sichtbar wäre. Es verlässt diesen Prozess nur im
    Authorization-Header an den angegebenen Server.

.EXAMPLE
    .\scripts\checkmk-probe.ps1 -Server https://checkmk.example.intern -Site meinesite -User automation

    Bewusst ein Platzhalter: der eigene Server gehört nicht in eine Datei, die
    womöglich einmal in ein öffentliches Repository wandert. Interne Adressen
    bleiben auch nach dem Löschen in der Git-Historie stehen.

.NOTES
    Die Statuscodes bedeuten:

      401  Die Anmeldung ist gescheitert. Der Text sagt, woran:
           "Couldn't log in."                  -> kein Authorization-Header
           "Wrong credentials (Bearer header)" -> Benutzer oder Secret falsch
      403  Die Anmeldung war ERFOLGREICH. CheckMK verweigert die Aktion; dem
           Konto fehlt eine Berechtigung. Der Text nennt welche.
      404  Der Pfad stimmt nicht — meist ein falscher Site-Name.
      200  Alles in Ordnung.
#>

[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$Server,
    [Parameter(Mandatory = $true)][string]$Site,
    [Parameter(Mandatory = $true)][string]$User
)

$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Net.Http

Write-Host ""
Write-Host "Automation-Secret fuer '$User' eingeben (die Eingabe ist verborgen):" -ForegroundColor Cyan
$secure = Read-Host -AsSecureString
$bstr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($secure)
try {
    $secret = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($bstr)
} finally {
    [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($bstr)
}

$base = "$($Server.TrimEnd('/'))/$Site/check_mk/api/1.0"

$handler = New-Object System.Net.Http.HttpClientHandler
# Wie Luchsr: Umleitungen werden NICHT gefolgt, damit das Secret nicht an ein
# Umleitungsziel geht.
$handler.AllowAutoRedirect = $false
$handler.AutomaticDecompression = [System.Net.DecompressionMethods]::GZip
$client = New-Object System.Net.Http.HttpClient($handler)
$client.Timeout = [TimeSpan]::FromSeconds(10)

function Invoke-Probe {
    param([string]$Label, [string]$Url)

    Write-Host ""
    Write-Host ("=" * 70)
    Write-Host $Label -ForegroundColor White
    Write-Host "GET $Url" -ForegroundColor DarkGray

    $req = New-Object System.Net.Http.HttpRequestMessage([System.Net.Http.HttpMethod]::Get, $Url)
    [void]$req.Headers.TryAddWithoutValidation("Authorization", "Bearer $User $secret")
    [void]$req.Headers.TryAddWithoutValidation("Accept", "application/json")
    [void]$req.Headers.TryAddWithoutValidation("User-Agent", "Luchsr-Probe/1.0")

    try {
        $resp = $client.SendAsync($req).GetAwaiter().GetResult()
        $code = [int]$resp.StatusCode
        $body = $resp.Content.ReadAsStringAsync().GetAwaiter().GetResult()

        $farbe = if ($code -eq 200) { "Green" } elseif ($code -ge 500) { "Magenta" } else { "Yellow" }
        Write-Host "Status: $code $($resp.StatusCode)" -ForegroundColor $farbe

        if ($resp.Headers.Location) { Write-Host "Location: $($resp.Headers.Location)" -ForegroundColor Yellow }
        Write-Host "Antwort:"
        if ($body.Length -gt 2000) { Write-Host ($body.Substring(0, 2000) + " …") } else { Write-Host $body }

        switch ($code) {
            401 { Write-Host "-> Anmeldung gescheitert. Benutzername und Automation-Secret pruefen." -ForegroundColor Yellow }
            403 { Write-Host "-> Anmeldung war ERFOLGREICH. Es fehlt eine Berechtigung; der Text oben nennt welche." -ForegroundColor Yellow }
            404 { Write-Host "-> Pfad falsch. Site-Name pruefen: '$Site'" -ForegroundColor Yellow }
            200 { Write-Host "-> In Ordnung." -ForegroundColor Green }
        }
    } catch {
        Write-Host "Kein HTTP-Ergebnis: $($_.Exception.Message)" -ForegroundColor Red
        if ($_.Exception.InnerException) {
            Write-Host "  Ursache: $($_.Exception.InnerException.Message)" -ForegroundColor Red
        }
    }
}

Invoke-Probe -Label "1. Versionsauskunft (das, was 'Verbindung testen' macht)" -Url "$base/version"
Invoke-Probe -Label "2. Serviceabruf (das, was die Abrufschleife macht)" `
    -Url "$base/domain-types/service/collections/all?columns=host_name&columns=description&columns=state&query=%7B%22op%22%3A%22%3E%22%2C%22left%22%3A%22state%22%2C%22right%22%3A%220%22%7D"

$client.Dispose()
$secret = $null
[System.GC]::Collect()

Write-Host ""
Write-Host ("=" * 70)
Write-Host "Fertig. Der Antworttext bei 401/403 ist die eigentliche Auskunft." -ForegroundColor Cyan
Write-Host ""
