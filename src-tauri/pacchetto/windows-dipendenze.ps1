# Halo per Windows: installa i programmi che servono alle azioni (con winget, già presente in
# Windows 10/11), whisper.cpp e le librerie Python (fogli Excel, OCR dei PDF, HEIC).
# Lo avvia l'installer di Halo, se scegli «Sì», oppure installa.ps1 con -Silenzioso (senza pause).
# Si può rilanciare quando vuoi: i programmi già installati vengono saltati.
param([switch]$Silenzioso)

$ErrorActionPreference = 'Continue'
$Host.UI.RawUI.WindowTitle = 'Halo - programmi necessari'
Write-Host "Halo: installo i programmi necessari. Alcuni chiedono il permesso di amministratore." -ForegroundColor Cyan
Write-Host ""

$pacchetti = @(
  @('ImageMagick.ImageMagick', 'ImageMagick (immagini)'),
  @('QPDF.QPDF', 'qpdf (PDF)'),
  @('oschwartz10612.Poppler', 'Poppler (PDF)'),
  @('Gyan.FFmpeg', 'FFmpeg (audio e video)'),
  @('7zip.7zip', '7-Zip (archivi)'),
  @('OliverBetz.ExifTool', 'ExifTool (metadati)'),
  @('JohnMacFarlane.Pandoc', 'Pandoc (documenti)'),
  @('TheDocumentFoundation.LibreOffice', 'LibreOffice (Word, Excel, presentazioni)'),
  @('UB-Mannheim.TesseractOCR', 'Tesseract (OCR delle immagini)'),
  @('Python.Python.3.12', 'Python (fogli Excel)')
)

if (-not (Get-Command winget -ErrorAction SilentlyContinue)) {
  Write-Host "Manca winget: installa «Programma di installazione app» dal Microsoft Store, poi rilancia questo script." -ForegroundColor Red
  if (-not $Silenzioso) { Read-Host "Premi Invio per chiudere" }
  exit 1
}

foreach ($p in $pacchetti) {
  Write-Host ("- " + $p[1] + "...") -NoNewline
  $gia = winget list --id $p[0] -e --accept-source-agreements 2>$null | Select-String -SimpleMatch $p[0]
  if ($gia) { Write-Host " già installato" -ForegroundColor Green; continue }
  winget install --id $p[0] -e --silent --accept-package-agreements --accept-source-agreements | Out-Null
  if ($LASTEXITCODE -eq 0) { Write-Host " installato" -ForegroundColor Green } else { Write-Host " NON riuscito (codice $LASTEXITCODE)" -ForegroundColor Yellow }
}

$ProgressPreference = 'SilentlyContinue'
$tmp = Join-Path $env:TEMP 'halo-dipendenze'
New-Item -ItemType Directory -Force $tmp | Out-Null

# Ghostscript non è su winget: l'ultima versione dalla pagina ufficiale di Artifex su GitHub,
# installata solo se l'installer è firmato da Artifex.
Write-Host "- Ghostscript (PDF)..." -NoNewline
if (Get-ChildItem "$env:ProgramFiles\gs\*\bin\gswin64c.exe" -ErrorAction SilentlyContinue) {
  Write-Host " già installato" -ForegroundColor Green
} else {
  try {
    $rel = Invoke-RestMethod 'https://api.github.com/repos/ArtifexSoftware/ghostpdl-downloads/releases/latest' -Headers @{ 'User-Agent' = 'Halo' }
    $asset = $rel.assets | Where-Object name -Match '^gs\d+w64\.exe$' | Select-Object -First 1
    $exe = Join-Path $tmp $asset.name
    Invoke-WebRequest $asset.browser_download_url -OutFile $exe
    $firma = Get-AuthenticodeSignature $exe
    if ($firma.Status -ne 'Valid' -or $firma.SignerCertificate.Subject -notmatch 'Artifex Software') { throw "firma non valida" }
    $p = Start-Process $exe -ArgumentList '/S' -Verb RunAs -Wait -PassThru
    if ($p.ExitCode -eq 0) { Write-Host " installato" -ForegroundColor Green } else { Write-Host " NON riuscito (codice $($p.ExitCode))" -ForegroundColor Yellow }
  } catch {
    Write-Host " NON riuscito ($($_.Exception.Message)): scaricalo da https://ghostscript.com/releases/gsdnld.html" -ForegroundColor Yellow
  }
}

# L'OCR legge italiano e inglese: Tesseract installa solo l'inglese.
$tessdata = "$env:ProgramFiles\Tesseract-OCR\tessdata"
if ((Test-Path $tessdata) -and -not (Test-Path "$tessdata\ita.traineddata")) {
  Write-Host "- Italiano per l'OCR..." -NoNewline
  try {
    $ita = Join-Path $tmp 'ita.traineddata'
    Invoke-WebRequest 'https://github.com/tesseract-ocr/tessdata_fast/raw/main/ita.traineddata' -OutFile $ita
    $p = Start-Process powershell -ArgumentList '-NoProfile', '-Command', "Copy-Item -LiteralPath '$ita' -Destination '$tessdata' -Force" -Verb RunAs -Wait -PassThru
    if (Test-Path "$tessdata\ita.traineddata") { Write-Host " installato" -ForegroundColor Green } else { Write-Host " NON riuscito" -ForegroundColor Yellow }
  } catch {
    Write-Host " NON riuscito ($($_.Exception.Message))" -ForegroundColor Yellow
  }
}

# Sottotitoli: whisper.cpp ufficiale per Windows (le versioni con i programmi sono quelle «bNNNN»),
# nella cartella dei dati di Halo, accanto ai modelli che Halo scarica al primo uso.
$whisper = Join-Path $env:LOCALAPPDATA 'Halo\whisper\bin'
Write-Host "- whisper.cpp (sottotitoli)..." -NoNewline
if (Test-Path "$whisper\whisper-cli.exe") {
  Write-Host " già installato" -ForegroundColor Green
} else {
  try {
    $rels = Invoke-RestMethod 'https://api.github.com/repos/ggml-org/whisper.cpp/releases?per_page=15' -Headers @{ 'User-Agent' = 'Halo' }
    $asset = $rels | ForEach-Object { $_.assets } | Where-Object name -EQ 'whisper-bin-x64.zip' | Select-Object -First 1
    if (-not $asset) { throw "versione per Windows non trovata" }
    $zip = Join-Path $tmp $asset.name
    Invoke-WebRequest $asset.browser_download_url -OutFile $zip
    Expand-Archive $zip (Join-Path $tmp 'whisper') -Force
    $cli = Get-ChildItem (Join-Path $tmp 'whisper') -Recurse -Filter 'whisper-cli.exe' | Select-Object -First 1
    if (-not $cli) { throw "whisper-cli.exe non trovato nell'archivio" }
    New-Item -ItemType Directory -Force $whisper | Out-Null
    Copy-Item "$($cli.DirectoryName)\*" $whisper -Recurse -Force
    Write-Host " installato" -ForegroundColor Green
  } catch {
    Write-Host " NON riuscito ($($_.Exception.Message))" -ForegroundColor Yellow
  }
}
Remove-Item -Recurse -Force $tmp -ErrorAction SilentlyContinue

# Librerie Python: fogli Excel (openpyxl, msoffcrypto-tool), OCR dei PDF (ocrmypdf), HEIC (pillow-heif).
$py = Get-Command py -ErrorAction SilentlyContinue
if (-not $py) { $env:Path = [Environment]::GetEnvironmentVariable('Path', 'User') + ';' + [Environment]::GetEnvironmentVariable('Path', 'Machine'); $py = Get-Command py -ErrorAction SilentlyContinue }
if ($py) {
  Write-Host "- Librerie Python (Excel, OCR dei PDF, HEIC)..." -NoNewline
  & py -3 -m pip install --user --quiet --disable-pip-version-check openpyxl msoffcrypto-tool ocrmypdf pillow-heif 2>$null
  if ($LASTEXITCODE -eq 0) { Write-Host " installate" -ForegroundColor Green } else { Write-Host " NON riuscite" -ForegroundColor Yellow }
}

Write-Host ""
Write-Host "Fatto. Halo è pronto: Alt+clic su un file, oppure tasto destro > Mostra altre opzioni > Azioni rapide (Halo)." -ForegroundColor Cyan
if (-not $Silenzioso) { Read-Host "Premi Invio per chiudere" }
