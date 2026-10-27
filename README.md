# Halo — menu radiale per file (Ubuntu)

Clic destro su un file in **File (Nautilus)** → **Azioni rapide…** → compare il menu
radiale sotto il puntatore. Ogni azione apre le sue sottoazioni; il risultato viene
salvato **accanto all'originale con un nome nuovo**: l'originale non viene mai toccato.

```
ui/                 interfaccia (HTML/CSS/JS): la stessa del prototipo
  index.html          aprilo in un browser per l'anteprima, con le schede per tipo di file
  actions.js          azioni e sottoazioni per tipo di file  <- qui si modifica il menu
  menu.js             disegno, animazioni, tastiera, chiamate al backend
src-tauri/          app desktop (Rust + Tauri 2)
  src/actions.rs      cosa fa ogni sottoazione (ffmpeg, ImageMagick, Ghostscript…)
  src/main.rs         comandi esposti all'interfaccia
nautilus/           voce "Azioni rapide…" nel menu del tasto destro
```

## 0. Installazione automatica (consigliata)

Dalla pagina **Releases** del repository scarica, per il tuo sistema:

- **Linux (Ubuntu/GNOME):** `Halo_…_amd64.deb` e `installa-halo.sh` nella stessa cartella, poi
  `bash installa-halo.sh`. Installa i programmi necessari, la voce del tasto destro di Nautilus e
  l'Alt+clic (attivo dal prossimo accesso).
- **Windows 10/11:** `Halo_…_x64-setup.exe`. Alla fine chiede se installare i programmi necessari
  (con winget, circa 1 GB): rispondi **Sì**.

Gli installer li crea GitHub (`.github/workflows/release.yml`) a ogni tag `v…`.

## 1. Preparare Ubuntu (24.04 o successivo)

```bash
# strumenti di compilazione e librerie di Tauri
sudo apt update
sudo apt install -y build-essential curl wget file libssl-dev libxdo-dev \
  libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev

# Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"
cargo install tauri-cli --version "^2" --locked

# programmi che eseguono le azioni sui file
sudo apt install -y ffmpeg imagemagick ghostscript qpdf poppler-utils 7zip \
  libimage-exiftool-perl pandoc libreoffice-writer libreoffice-calc libreoffice-impress ocrmypdf tesseract-ocr-ita whisper.cpp python3-openpyxl \
  python3-msoffcrypto-tool libheif-examples libheif-plugin-x265 wl-clipboard curl
```

## 2. Provarlo

```bash
cd halo/src-tauri
cargo tauri dev -- -- ~/Documenti/un_file.pdf
```

Solo l'aspetto, senza compilare nulla: apri `ui/index.html` in Chrome o Firefox.

## 3. Installarlo

```bash
cd halo/src-tauri
cargo tauri build            # crea il pacchetto .deb in target/release/bundle/deb/
sudo apt install --reinstall ./target/release/bundle/deb/Halo_*_amd64.deb   # --reinstall: anche a parità di versione
```

Il pacchetto installa tutto: l'eseguibile `halo`, la voce «Azioni rapide» nel tasto destro di
Nautilus (`/usr/share/nautilus-python/extensions/`), l'estensione di GNOME Shell per l'Alt+clic
(`/usr/share/gnome-shell/extensions/halo@halo.dev/`) e un avvio automatico
(`/etc/xdg/autostart/halo-configura.desktop`) che al primo accesso attiva l'estensione con
`halo --configura`. Dopo l'installazione basta **uscire e rientrare nella sessione**
(su Wayland GNOME carica le nuove estensioni solo all'accesso).

## 4. Sviluppo: estensioni senza pacchetto

Per provare le estensioni modificate senza rifare il pacchetto, copiale nella home
(queste copie hanno la precedenza su quelle del pacchetto; toglile quando hai finito):

```bash
mkdir -p ~/.local/share/nautilus-python/extensions ~/.local/share/gnome-shell/extensions
cp nautilus/halo_nautilus.py ~/.local/share/nautilus-python/extensions/ && nautilus -q
cp -r gnome-shell/halo@halo.dev ~/.local/share/gnome-shell/extensions/
# esci e rientra nella sessione, poi: gnome-extensions enable halo@halo.dev
```

## 5. Alt+clic su un file (Nautilus e desktop)

L'estensione di GNOME Shell apre il menu con **Alt+clic** su un file in Nautilus o sul desktop;
su uno spazio vuoto lo apre sulla cartella. Il menu compare dove si trova il puntatore
(l'estensione passa la posizione a Halo con `--pos=X,Y`).

Su Wayland i clic sulle finestre delle app non passano da GNOME Shell: l'estensione legge lo
stato di mouse e tasti ogni 35 ms e, all'Alt+clic, lancia `halo-pick`, che trova il file sotto
il puntatore tramite l'accessibilità (AT-SPI) e apre `halo` su quel file.

## Come funziona la posizione del menu su Ubuntu

Ubuntu usa GNOME su Wayland, dove un'app non può decidere dove mettere una sua finestra.
Halo quindi apre una finestra **trasparente e massimizzata** (non a tutto schermo: su Wayland
una finestra trasparente a tutto schermo appare nera) e disegna il menu nel punto
in cui si trova il puntatore, spostandolo se toccherebbe i bordi. Clic fuori dal menu o
`Esc` chiudono la finestra.

## Cosa fa davvero e cosa no

Le sottoazioni implementate sono elencate in `SUPPORTED` dentro `src-tauri/src/actions.rs`
(tutte le 214). *Traduci* (Documento e PDF) è uno spicchio senza sottomenu, con chiave
`tipo/traduci`: apre subito un pannello dove scegli lingua e strumento di IA (Claude, ChatGPT,
Gemini, DeepSeek). Halo non traduce da sé: copia negli appunti un prompt pronto, apre lo strumento
nel browser e mostra il file nella cartella, da trascinare nella chat.

- **Dove salva:** ogni risultato va accanto al file di partenza, con un nome nuovo
  (`out_path()` / `out_dir()`); l'originale non viene mai modificato. Se nella cartella del
  file non si può scrivere, il risultato va sulla Scrivania e il messaggio lo dice.
- **Pannelli:** le opzioni che chiedono dati (password, orari, testo, file da scegliere,
  elenco del contenuto di un archivio, editor sull'anteprima) aprono un pannello di vetro
  accanto al menu; il tipo di pannello di ogni chiave è in `FORMS`.
- **Condividi (cartelle):** Email e Altro creano lo ZIP, lo copiano negli appunti e
  propongono le app installate. **Link** carica lo ZIP su [Gofile](https://gofile.io) solo
  dopo la conferma «Carica»: chiunque abbia il link può scaricarlo.

Per aggiungerne una:

1. scegli la chiave: `tipo/azione/sottoazione` scritte in minuscolo, senza accenti, con
   i trattini (es. *Pagine › Dividi* di un PDF → `pdf/pagine/dividi`);
2. aggiungila a `SUPPORTED` e scrivi il relativo ramo nel `match` di `run()`
   (o nel modulo della sua famiglia); se chiede dati, aggiungila a `FORMS`;
3. `cargo test` controlla, tra l'altro, che i nomi dei file in uscita non sovrascrivano niente;
   `cargo test -- --include-ignored --test-threads=1` esegue anche le prove su file veri
   (in `/tmp/halo-azioni` e `/tmp/halo-test-*`, servono i programmi del punto 1).
   Le prove che caricano online o usano Upscayl partono solo con
   `HALO_PROVA_ONLINE=1` e `HALO_UPSCAYL=/percorso/upscayl.AppImage`.

## Upscayl (Real-ESRGAN)

*Immagine › Ridimensiona › Upscayl* (2×, 4×, 6×, 8×) usa Real-ESRGAN (`realesrgan-ncnn-vulkan`, BSD-3/MIT) incluso nel
pacchetto in `/usr/lib/halo/realesrgan` (sorgente: `src-tauri/risorse/realesrgan`, con
`PROVENIENZA.txt` e le licenze). Funziona sul computer, senza Internet, con la scheda video (Vulkan);
è intelligenza artificiale, aggiunta su richiesta. Prova reale:
`cargo test ingrandisci::tests::file_veri -- --ignored`.

## OCR (Tesseract)

*PDF › Converti › OCR* rende ricercabile un PDF scansionato (ocrmypdf aggiunge il testo sotto le
pagine, che restano uguali); *Immagine › Converti › Testo* estrae il testo da una foto di un documento
in un `.txt`. Lavora sul computer, in italiano e inglese; è intelligenza artificiale, aggiunta su richiesta.
Servono `ocrmypdf` e `tesseract-ocr-ita`. Prova reale: `cargo test ocr::tests::file_veri -- --ignored`.

## Sottotitoli (Whisper)

*Video › Sottotitoli* trascrive l'audio con whisper.cpp, sul computer: **Crea .srt** (file accanto
al video), **Nel video** (traccia attivabile, video e audio copiati così come sono) o **Impressi**
(scritti nell'immagine, il video viene ricodificato). Lingua: automatica, italiano o inglese;
modello Veloce (142 MB) o Preciso (466 MB), scaricato la prima volta da Hugging Face in
`~/.cache/halo/whisper`. È intelligenza artificiale, aggiunta su richiesta. Serve `whisper.cpp`.

## Altro

- *PDF › Pagine › Estrai*: un PDF nuovo solo con le pagine scritte (es. «2, 5-7»).
- *Più immagini › Insieme › Unisci in PDF*: una pagina per immagine, nell'ordine della selezione.
- **Codici QR**: in *Condividi › Link* compare il QR del link (si apre col telefono); *Testo › Converti
  › QR code* crea un'immagine QR del testo del file (link, password del Wi-Fi, brevi note).

## Presentazioni e altri file

- **Presentazioni** (`.pptx`, `.ppt`, `.odp`…): Converti (PDF, Immagini, PowerPoint, ODP) con
  LibreOffice Impress, Comprimi (immagini interne, ZIP), Metadati (mostra, rimuovi tutti), Condividi.
- **Tutti gli altri file** (tipi che Halo non conosce): Comprimi (ZIP, ZIP protetto, 7z),
  Verifica (impronta SHA-256, confronta con un altro file, metadati), Condividi (il file così com'è).

## Più file insieme

Con più file selezionati (tasto destro › Azioni rapide, oppure Alt+clic su uno dei file
selezionati) il menu mostra lo spicchio **Insieme**: ZIP con tutti, **Per email** (ogni file
alleggerito e poi un unico ZIP, con l'avviso se supera 25 MB), Rinomina in serie e **Data di scatto**
(copie in una cartella nuova), Unisci (solo PDF o solo video, nell'ordine della selezione) e
Percorsi (copiati negli appunti). Filigrana e firma si possono mettere su più PDF o Word insieme. Se i file sono dello stesso tipo ci sono anche le azioni normali,
eseguite su ogni file con l'avanzamento «File 3 di 10»; quelle che richiedono un lavoro
diverso per ogni file (editor, elenchi, orari) restano «un file alla volta». Il codice è in
`src-tauri/src/multi.rs`.

## Impostazioni e scorciatoie

Le impostazioni si aprono **tenendo premuto il cerchio centrale per 1,5 secondi** (un anello si riempie) oppure con **Ctrl+virgola**; si salvano in
`~/.config/halo/impostazioni.json`:

- **Aspetto:** colore principale (14 colori pronti o uno a scelta), tema (automatico, chiaro, scuro),
  vetro, grandezza del menu, ingrandimento dello spicchio sotto il cursore, animazioni (normali o
  ridotte) e chiusura dopo un'operazione (3, 5, 10 secondi o mai).
- **Scorciatoie:** «Le mie app», uno spicchio in più nei tipi di file scelti. Ogni scorciatoia
  apre il file con un'app, esegue un comando (`{file}`, `{cartella}`, `{nome}`) oppure
  ingrandisce un'immagine 4× con il programma contenuto in Upscayl (estratto una volta in
  `~/.cache/halo/upscayl`).
- **Cronologia:** le ultime 50 operazioni che hanno creato file (in `~/.local/share/halo/cronologia.json`),
  con «Mostra» e «Cestino» (sposta nel cestino solo i file creati, mai quelli di partenza).

## Comodità

- **Risultato:** il messaggio dice di quanto è cambiato il file (es. «4,2 MB → 1,1 MB (−74%)»,
  e le dimensioni per le immagini) e ha il pulsante **Mostra nella cartella**.
- **Ripeti:** sotto il nome del file, al centro del menu, c'è l'ultima azione usata per quel tipo
  di file; un clic la ripete.
- **Lavori lunghi:** al centro del menu compaiono l'avanzamento (per video, audio e Upscayl, letto
  da ffmpeg e Real-ESRGAN) e il tempo trascorso, con **Annulla** (ferma il programma e toglie i file
  lasciati a metà) e, dopo 3 secondi, **In sottofondo** (nasconde la finestra; a lavoro finito arriva
  una notifica con «Mostra nella cartella»).
- **Cerca scrivendo:** con il menu aperto scrivi (es. «comp») e compaiono le azioni che
  corrispondono; frecce e Invio per scegliere, Esc per annullare la ricerca.

## Tastiera

Frecce: spostarsi · Invio: scegliere · Esc / Backspace: indietro · Esc al primo livello: chiudere. · Ctrl+virgola: impostazioni.

Il menu si chiude con un clic fuori o con Esc. Dopo un'operazione resta aperto 5 secondi; se nel frattempo lo usi, resta aperto.

## Windows (in sviluppo)

Lo stesso codice si compila anche per **Windows 10/11**: le differenze (dove stanno i programmi,
appunti, cestino, notifiche, «Mostra nella cartella») sono tutte in `src-tauri/src/sistema.rs`,
la configurazione in `src-tauri/tauri.windows.conf.json`.

- **Installer:** `Halo_…_x64-setup.exe` dalla pagina Releases (vedi sopra). Si installa solo per
  l'utente; alla fine propone di installare i programmi necessari con winget.
- **Menu del tasto destro:** l'installazione aggiunge «Azioni rapide (Halo)» su file, cartelle e
  sfondo delle cartelle (registro, `HKCU\Software\Classes`). Su Windows 11 è in «Mostra altre opzioni».
  Con più file selezionati, Halo li raccoglie in un solo menu.
- **Programmi necessari** (una volta, da PowerShell):

```powershell
winget install ImageMagick.ImageMagick ArtifexSoftware.GhostScript QPDF.QPDF oschwartz10612.Poppler Gyan.FFmpeg 7zip.7zip OliverBetz.ExifTool JohnMacFarlane.Pandoc TheDocumentFoundation.LibreOffice UB-Mannheim.TesseractOCR Python.Python.3.12
py -m pip install openpyxl msoffcrypto-tool
```

- **Non ancora disponibili su Windows:** Alt+clic, Upscayl (Real-ESRGAN), l'elenco delle app in
  Condividi (restano Gmail, Link e «Mostra nella cartella»), OCR dei PDF (ocrmypdf).

