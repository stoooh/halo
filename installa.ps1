# Installa Halo in un colpo solo: scarica l'ultima versione da GitHub, la installa e aggiunge i
# programmi che servono alle azioni (dipendenze.ps1). Uso, in PowerShell:
#   irm https://raw.githubusercontent.com/stoooh/halo/main/installa.ps1 | iex
# Su Linux (PowerShell per Linux) passa da solo a installa.sh.

& {
$ErrorActionPreference = 'Stop'
$repo = 'stoooh/halo'

if ($PSVersionTable.PSEdition -eq 'Core' -and -not $IsWindows) {
    if ($IsLinux) {
        Write-Host 'Sistema: Linux. Uso installa.sh.'
        bash -c "curl -fsSL https://raw.githubusercontent.com/$repo/main/installa.sh | bash"
    } else {
        Write-Host 'Halo funziona solo su Ubuntu (GNOME) e su Windows 10/11.'
    }
    return
}
if (-not [Environment]::Is64BitOperatingSystem) {
    Write-Host 'Halo per Windows richiede la versione a 64 bit.'
    return
}
Write-Host 'Sistema: Windows.'

[Net.ServicePointManager]::SecurityProtocol = [Net.SecurityProtocolType]::Tls12
$release = Invoke-RestMethod "https://api.github.com/repos/$repo/releases/latest" -Headers @{ 'User-Agent' = 'halo-installa' }
$asset = $release.assets | Where-Object { $_.name -like '*_x64-setup.exe' } | Select-Object -First 1
if (-not $asset) { throw "Nella release $($release.tag_name) non c'è l'installer per Windows." }

$dir = Join-Path $env:TEMP ('halo-installa-' + [guid]::NewGuid())
New-Item -ItemType Directory $dir | Out-Null
try {
    $setup = Join-Path $dir $asset.name
    Write-Host "Scarico Halo $($release.tag_name) ($([math]::Round($asset.size / 1MB)) MB)..."
    $ProgressPreference = 'SilentlyContinue'
    Invoke-WebRequest $asset.browser_download_url -OutFile $setup -UseBasicParsing
    # GitHub pubblica l'impronta di ogni file: se c'è, il file scaricato deve corrispondere.
    if ($asset.digest -like 'sha256:*') {
        $atteso = $asset.digest.Substring(7)
        if ((Get-FileHash $setup -Algorithm SHA256).Hash -ne $atteso) { throw 'Il file scaricato è danneggiato: riprova.' }
    }

    Write-Host 'Installo Halo...'
    $p = Start-Process $setup -ArgumentList '/S' -Wait -PassThru
    if ($p.ExitCode -ne 0) { throw "L'installazione non è riuscita (codice $($p.ExitCode))." }
} finally {
    Remove-Item $dir -Recurse -Force -ErrorAction SilentlyContinue
}

$voce = Get-ChildItem 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall' |
    ForEach-Object { Get-ItemProperty $_.PSPath } | Where-Object DisplayName -eq 'Halo' | Select-Object -First 1
$cartella = if ($voce.InstallLocation) { $voce.InstallLocation.Trim('"') } else { Join-Path $env:LOCALAPPDATA 'Halo' }
$dipendenze = Join-Path $cartella 'dipendenze.ps1'
if (-not (Test-Path $dipendenze)) { throw "Non trovo ${dipendenze}: reinstalla Halo." }

Write-Host 'Installo i programmi che servono alle azioni (può volerci qualche minuto)...'
& powershell.exe -NoProfile -ExecutionPolicy Bypass -File $dipendenze -Silenzioso

Write-Host ''
Write-Host "Fatto: Halo $($release.tag_name) è installato."
Write-Host 'Alt+clic su un file in Esplora file o sul desktop per aprire il menu.'
}
