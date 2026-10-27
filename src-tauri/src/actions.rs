//! Esecuzione delle azioni sui file.
//!
//! Ogni azione è identificata da una chiave "tipo/azione/sottoazione"
//! (slug delle etichette mostrate nel menu, es. "pdf/comprimi/per-email").
//! Regola d'oro: il file originale non viene MAI modificato o sovrascritto;
//! il risultato viene scritto accanto, con un nome libero.

use std::collections::HashMap;
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::process::Stdio;

/// Chiavi effettivamente implementate. Il menu mostra le altre come "presto".
pub const SUPPORTED: &[&str] = &[
    // PDF
    "pdf/comprimi/alta-qualita", "pdf/comprimi/bilanciata", "pdf/comprimi/massima", "pdf/comprimi/per-email",
    "pdf/pagine/ruota",
    "pdf/converti/word", "pdf/converti/immagini", "pdf/converti/testo", "pdf/converti/pdf-a",
    "pdf/proteggi/password", "pdf/proteggi/rimuovi-password", "pdf/proteggi/filigrana", "pdf/proteggi/limita-stampa",
    "pdf/annota/testo", "pdf/annota/evidenzia", "image/annota/testo",
    // Rapide (modulo rapide.rs)
    "pdf/pagine/dividi", "pdf/pagine/riordina", "pdf/pagine/estrai",
    "archive/anteprima/dimensioni", "archive/anteprima/cerca", "archive/proteggi/verifica-integrita",
    "archive/ricomprimi/massima", "archive/ricomprimi/rapida", "archive/ricomprimi/per-email",
    "video/fotogrammi/estrai-foto", "folder/condividi/copia-percorso",
    // Condividi (modulo condividi.rs)
    "folder/condividi/email", "folder/condividi/altro", "folder/condividi/link",
    "other/condividi/email", "other/condividi/altro", "other/condividi/link",
    "slides/condividi/email", "slides/condividi/altro", "slides/condividi/link",
    // Presentazioni e altri file (modulo altri.rs, più comprimi e metadati in documenti.rs)
    "other/comprimi/zip", "other/comprimi/zip-protetto", "other/comprimi/7z", "slides/comprimi/zip",
    "other/verifica/impronta", "other/verifica/confronta", "other/verifica/metadati", "other/condividi/copia-percorso",
    "slides/converti/pdf", "slides/converti/immagini", "slides/converti/powerpoint", "slides/converti/odp",
    "slides/metadati/mostra", "slides/condividi/copia-percorso",
    "slides/comprimi/immagini-interne", "slides/metadati/rimuovi-tutti",
    // Filigrana e firma nei .docx (modulo word.rs)
    "document/proteggi/filigrana", "document/proteggi/firma",
    // Ingrandisci con Real-ESRGAN (modulo ingrandisci.rs)
    "image/ridimensiona/upscayl",
    // OCR con Tesseract (modulo ocr.rs)
    "pdf/converti/ocr", "image/converti/testo",
    // Codice QR di un testo breve (modulo qr.rs)
    "text/converti/qr-code",
    // Sottotitoli con whisper.cpp (modulo sottotitoli.rs)
    "video/sottotitoli/crea-srt", "video/sottotitoli/nel-video", "video/sottotitoli/impressi",
    // Traduci: azione diretta, apre uno strumento di IA con il prompt pronto (modulo traduci.rs)
    "document/traduci", "pdf/traduci",
    // Editor sull'anteprima (modulo editor.rs)
    "image/ritaglia/libero", "image/annota/freccia", "image/annota/forma", "image/annota/evidenzia",
    "image/sfondo/sfocato", "image/sfondo/colore", "image/sfondo/rimuovi", "pdf/annota/forma", "video/ruota/ritaglia",
    "pdf/metadati/cambia-autore", "pdf/metadati/cambia-data",
    "pdf/metadati/mostra", "pdf/metadati/rimuovi-tutti",
    // Immagini
    "image/modifica/ruota", "image/modifica/capovolgi", "image/modifica/luminosita", "image/modifica/bianco-e-nero",
    "image/ritaglia/1-1", "image/ritaglia/16-9", "image/ritaglia/automatico",
    "image/sfondo/bianco",
    "image/ridimensiona/meta", "image/ridimensiona/per-il-web", "image/ridimensiona/per-email", "image/ridimensiona/su-misura",
    "image/converti/png", "image/converti/webp", "image/converti/pdf",
    "image/metadati/mostra", "image/metadati/rimuovi-posizione", "image/metadati/rimuovi-tutti", "image/metadati/cambia-data",
    // Audio
    "audio/taglia/inizio", "audio/taglia/fine", "audio/taglia/rimuovi-silenzi", "audio/taglia/dividi",
    "audio/volume/normalizza", "audio/volume/aumenta", "audio/volume/riduci-rumore", "audio/volume/dissolvenza",
    "audio/converti/wav", "audio/converti/aac", "audio/converti/m4a", "audio/converti/flac",
    "audio/tag/titolo", "audio/tag/artista", "audio/tag/album",
    "audio/comprimi/128-kbps", "audio/comprimi/192-kbps", "audio/comprimi/320-kbps", "audio/comprimi/mono",
    // Video
    "video/comprimi/1080p", "video/comprimi/720p", "video/comprimi/per-email", "video/comprimi/per-il-web",
    "video/converti/mov", "video/converti/gif", "video/converti/solo-audio", "video/converti/webm",
    "video/audio/rimuovi-audio", "video/audio/normalizza", "video/audio/dissolvenza",
    "video/ruota/90-a-destra", "video/ruota/90-a-sinistra", "video/ruota/capovolgi",
    "video/taglia/inizio", "video/taglia/fine", "video/taglia/dividi",
    // Con la finestra «Apri file» (modulo scegli.rs)
    "pdf/pagine/unisci", "pdf/annota/firma", "video/taglia/unisci", "video/audio/sostituisci",
    "audio/tag/copertina", "archive/contenuto/aggiungi-file", "archive/estrai/scegli-dove",
    "image/converti/heic",
    // Word ed Excel modificando l'XML (modulo documenti.rs)
    "document/comprimi/immagini-interne", "document/comprimi/bilanciata", "document/comprimi/massima", "document/comprimi/per-email",
    "document/metadati/rimuovi-tutti", "document/metadati/revisioni", "sheet/metadati/rimuovi-tutti",
    "document/proteggi/sola-lettura", "sheet/proteggi/sola-lettura",
    // Fogli con openpyxl e password Office con msoffcrypto (modulo fogli.rs)
    "sheet/converti/json",
    "sheet/pulisci/duplicati", "sheet/pulisci/righe-vuote", "sheet/pulisci/spazi-in-piu", "sheet/pulisci/formato-date",
    "sheet/grafico/barre", "sheet/grafico/linee", "sheet/grafico/torta", "sheet/grafico/pivot",
    "sheet/ordina/a-z", "sheet/ordina/z-a", "sheet/ordina/per-data", "sheet/ordina/filtra",
    "sheet/proteggi/password", "sheet/proteggi/blocca-celle", "sheet/proteggi/nascondi-formule",
    "document/proteggi/password",
    // Archivi con l'elenco del contenuto (modulo archivi.rs)
    "archive/estrai/solo-alcuni", "archive/contenuto/rimuovi-file", "archive/contenuto/sostituisci", "archive/contenuto/rinomina",
    "video/fotogrammi/copertina", "video/fotogrammi/anteprima-gif", "video/fotogrammi/velocita",
    // Documenti e fogli
    "document/converti/pdf", "document/converti/testo", "document/converti/markdown", "document/converti/html",
    "document/metadati/mostra", "document/metadati/cambia-autore",
    "sheet/converti/csv", "sheet/converti/pdf", "sheet/converti/ods",
    "sheet/metadati/mostra", "sheet/metadati/cambia-autore", "sheet/metadati/cambia-data",
    // Cartelle e archivi
    "folder/comprimi/zip", "folder/comprimi/zip-protetto", "folder/comprimi/7z", "folder/comprimi/dividi-in-parti",
    "folder/analizza/dimensione", "folder/analizza/tipi-di-file", "folder/analizza/modifiche-recenti", "folder/analizza/cerca",
    "folder/rinomina/in-serie", "folder/rinomina/data-di-scatto", "folder/rinomina/aggiungi-data", "folder/rinomina/numera", "folder/rinomina/minuscole",
    "folder/organizza/per-tipo", "folder/organizza/per-data", "folder/organizza/per-dimensione", "folder/organizza/appiattisci",
    "folder/pulisci/duplicati", "folder/pulisci/file-vuoti", "folder/pulisci/temporanei", "folder/pulisci/file-grandi",
    "archive/estrai/qui", "archive/estrai/in-una-cartella",
    "archive/anteprima/contenuto", "archive/anteprima/verifica",
    "archive/proteggi/password", "archive/proteggi/cifra-aes", "archive/proteggi/rimuovi-password",
    "archive/converti/7z", "archive/converti/tar", "archive/converti/cartella", "archive/converti/zip-protetto",
    "archive/ricomprimi/dividi-in-parti",
    // Testo
    "text/converti/pdf", "text/converti/word", "text/converti/markdown", "text/converti/html",
    "text/cerca/trova", "text/cerca/sostituisci", "text/cerca/conta-parole", "text/cerca/righe-doppie",
    // Oscura dati (modulo oscura.rs)
    "pdf/oscura-dati/email", "pdf/oscura-dati/telefoni", "pdf/oscura-dati/iban", "pdf/oscura-dati/manuale",
    "document/oscura-dati/email", "document/oscura-dati/telefoni", "document/oscura-dati/iban", "document/oscura-dati/manuale",
    "text/oscura-dati/email", "text/oscura-dati/telefoni", "text/oscura-dati/iban", "text/oscura-dati/manuale",
    "text/codifica/utf-8", "text/codifica/fine-riga-lf", "text/codifica/fine-riga-crlf", "text/codifica/rileva",
    "text/formatta/maiuscole", "text/formatta/minuscole", "text/formatta/ordina-righe", "text/formatta/togli-spazi",
];

/// Opzioni che prima di eseguire chiedono dati nel pannello: chiave → tipo di modulo.
/// "nuova-password" chiede password e conferma, "password-attuale" la password già usata;
/// "taglia-inizio", "taglia-fine" e "dividi" chiedono un orario;
/// "autore", "titolo", "artista", "album" un testo; "data" una data (gg/mm/aaaa ore:minuti);
/// "dimensioni" larghezza×altezza o percentuale, "velocita" un fattore, "parti" i MB di ogni parte;
/// "nome-serie" il nome di base per Rinomina › In serie, "cerca" il testo da cercare nei nomi,
/// "trova" il testo da cercare in un file, "sostituisci" il testo da cercare e quello nuovo;
/// "oscura" mostra cosa verrà oscurato e chiede conferma, "oscura-manuale" chiede anche le parole;
/// "filigrana" e "evidenzia" un testo, "annota-pdf" testo + posizione + pagine, "annota-img" testo + posizione;
/// "ordine-pagine" il nuovo ordine (es. 3, 1, 2), "intervallo" ogni quanti secondi estrarre una foto;
/// "scegli" fa scegliere altri file con la finestra «Apri file», "firma" anche posizione e pagine;
/// "elenco-*" mostra l'elenco dei file dell'archivio da spuntare (più file, oppure uno solo).
pub const FORMS: &[(&str, &str)] = &[
    ("pdf/proteggi/password", "nuova-password"),
    ("pdf/proteggi/rimuovi-password", "password-attuale"),
    ("folder/comprimi/zip-protetto", "nuova-password"),
    ("archive/proteggi/password", "nuova-password"),
    ("archive/proteggi/cifra-aes", "nuova-password"),
    ("archive/proteggi/rimuovi-password", "password-attuale"),
    ("archive/converti/zip-protetto", "nuova-password"),
    ("audio/taglia/inizio", "taglia-inizio"),
    ("audio/taglia/fine", "taglia-fine"),
    ("audio/taglia/dividi", "dividi"),
    ("video/taglia/inizio", "taglia-inizio"),
    ("video/taglia/fine", "taglia-fine"),
    ("video/taglia/dividi", "dividi"),
    ("pdf/metadati/cambia-autore", "autore"),
    ("pdf/metadati/cambia-data", "data"),
    ("image/metadati/cambia-data", "data"),
    ("audio/tag/titolo", "titolo"),
    ("audio/tag/artista", "artista"),
    ("audio/tag/album", "album"),
    ("document/metadati/cambia-autore", "autore"),
    ("sheet/metadati/cambia-autore", "autore"),
    ("sheet/metadati/cambia-data", "data"),
    ("image/ridimensiona/su-misura", "dimensioni"),
    ("video/fotogrammi/velocita", "velocita"),
    ("archive/ricomprimi/dividi-in-parti", "parti"),
    ("folder/rinomina/in-serie", "nome-serie"),
    ("folder/analizza/cerca", "cerca"),
    ("text/cerca/trova", "trova"),
    ("text/cerca/sostituisci", "sostituisci"),
    ("pdf/oscura-dati/email", "oscura"), ("pdf/oscura-dati/telefoni", "oscura"), ("pdf/oscura-dati/iban", "oscura"),
    ("pdf/oscura-dati/manuale", "oscura-manuale"),
    ("document/oscura-dati/email", "oscura"), ("document/oscura-dati/telefoni", "oscura"), ("document/oscura-dati/iban", "oscura"),
    ("document/oscura-dati/manuale", "oscura-manuale"),
    ("text/oscura-dati/email", "oscura"), ("text/oscura-dati/telefoni", "oscura"), ("text/oscura-dati/iban", "oscura"),
    ("text/oscura-dati/manuale", "oscura-manuale"),
    ("pdf/proteggi/filigrana", "filigrana"),
    ("pdf/annota/testo", "annota-pdf"),
    ("pdf/annota/evidenzia", "evidenzia"),
    ("image/annota/testo", "annota-img"),
    ("pdf/pagine/riordina", "ordine-pagine"),
    ("pdf/pagine/estrai", "estrai-pagine"),
    ("video/sottotitoli/crea-srt", "sottotitoli"), ("video/sottotitoli/nel-video", "sottotitoli"), ("video/sottotitoli/impressi", "sottotitoli"),
    ("archive/anteprima/cerca", "cerca"),
    ("video/fotogrammi/estrai-foto", "intervallo"),
    ("pdf/pagine/unisci", "scegli"),
    ("pdf/annota/firma", "firma"),
    ("video/taglia/unisci", "scegli"),
    ("video/audio/sostituisci", "scegli"),
    ("audio/tag/copertina", "scegli"),
    ("archive/contenuto/aggiungi-file", "scegli"),
    ("archive/estrai/scegli-dove", "scegli"),
    ("sheet/proteggi/password", "nuova-password"),
    ("document/proteggi/password", "nuova-password"),
    ("archive/estrai/solo-alcuni", "elenco-estrai"),
    ("archive/contenuto/rimuovi-file", "elenco-rimuovi"),
    ("archive/contenuto/sostituisci", "elenco-sostituisci"),
    ("archive/contenuto/rinomina", "elenco-rinomina"),
    ("folder/condividi/email", "condividi"),
    ("folder/condividi/altro", "condividi"),
    ("folder/condividi/link", "condividi-link"),
    ("other/condividi/email", "condividi"), ("other/condividi/altro", "condividi"), ("other/condividi/link", "condividi-link"),
    ("slides/condividi/email", "condividi"), ("slides/condividi/altro", "condividi"), ("slides/condividi/link", "condividi-link"),
    ("other/comprimi/zip-protetto", "nuova-password"),
    ("other/verifica/confronta", "scegli"),
    ("document/proteggi/filigrana", "filigrana"),
    ("document/proteggi/firma", "firma-doc"),
    ("image/ridimensiona/upscayl", "ingrandisci"),
    ("document/traduci", "traduci"), ("pdf/traduci", "traduci"),
    ("image/ritaglia/libero", "ed-ritaglia"), ("image/annota/freccia", "ed-freccia"), ("image/annota/forma", "ed-forma"),
    ("image/annota/evidenzia", "ed-evidenzia"), ("image/sfondo/sfocato", "ed-sfocato"), ("image/sfondo/colore", "ed-colore"),
    ("image/sfondo/rimuovi", "ed-rimuovi"), ("pdf/annota/forma", "ed-forma-pdf"), ("video/ruota/ritaglia", "ed-video"),
];

/// Il file che un'opzione con pannello creerà, da mostrare prima di eseguire.
pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    Some(match key {
        "pdf/proteggi/password" => out_path(input, "_protetto", "pdf"),
        "pdf/proteggi/rimuovi-password" => out_path(input, "_sbloccato", "pdf"),
        "folder/comprimi/zip-protetto" => out_path(input, "", "zip"),
        "archive/proteggi/password" | "archive/converti/zip-protetto" => out_path(input, "_protetto", "zip"),
        "archive/proteggi/cifra-aes" => out_path(input, "_cifrato", "7z"),
        "archive/proteggi/rimuovi-password" => out_path(input, "_sbloccato", if ext == "7z" { "7z" } else { "zip" }),
        "audio/taglia/inizio" | "audio/taglia/fine" => out_path(input, "_taglio", &ext),
        "video/taglia/inizio" | "video/taglia/fine" => out_path(input, "_taglio", video_ext(&ext)),
        "audio/taglia/dividi" => out_path(input, "_parte1", &ext),
        "video/taglia/dividi" => out_path(input, "_parte1", video_ext(&ext)),
        k if k.ends_with("/cambia-autore") => out_path(input, "_autore", &ext),
        k if k.ends_with("/cambia-data") => out_path(input, "_data", &ext),
        k if k.starts_with("audio/tag/") => out_path(input, "_tag", &ext),
        "image/ridimensiona/su-misura" => out_path(input, "_ridimensionata", &ext),
        "video/fotogrammi/velocita" => out_path(input, "_velocita", video_ext(&ext)),
        "archive/ricomprimi/dividi-in-parti" => out_path(input, "_parti", "zip"),
        k if crate::condividi::KEYS.contains(&k) => return crate::condividi::planned_out(input, k),
        k if crate::editor::KEYS.contains(&k) => return crate::editor::planned_out(input, k),
        k if crate::word::KEYS.contains(&k) => return crate::word::planned_out(input, k),
        k if crate::altri::KEYS.contains(&k) => return crate::altri::planned_out(input, k),
        k if crate::ocr::KEYS.contains(&k) => return crate::ocr::planned_out(input, k),
        k if crate::qr::KEYS.contains(&k) => return crate::qr::planned_out(input, k),
        k if crate::sottotitoli::KEYS.contains(&k) => return crate::sottotitoli::planned_out(input, k),
        k if crate::ingrandisci::KEYS.contains(&k) => return crate::ingrandisci::planned_out(input, k, &HashMap::new()),
        k if k.starts_with("folder/") => return crate::cartelle::planned_out(input, k),
        "text/cerca/sostituisci" => out_path(input, "_sostituito", &ext),
        k if k.contains("/oscura-dati/") => return crate::oscura::planned_out(input, k),
        k if crate::annota::KEYS.contains(&k) => return crate::annota::planned_out(input, k),
        k if crate::rapide::KEYS.contains(&k) => return crate::rapide::planned_out(input, k),
        k if crate::scegli::KEYS.contains(&k) => return crate::scegli::planned_out(input, k),
        k if crate::documenti::KEYS.contains(&k) => return crate::documenti::planned_out(input, k),
        k if crate::fogli::KEYS.contains(&k) => return crate::fogli::planned_out(input, k),
        k if crate::archivi::KEYS.contains(&k) => return crate::archivi::planned_out(input, k),
        "image/converti/heic" => out_path(input, "", "heic"),
        _ => return None,
    })
}

/// Riconosce il tipo di file dall'estensione (o cartella).
pub fn kind_of(path: &Path) -> &'static str {
    if path.is_dir() {
        return "folder";
    }
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    match ext.as_str() {
        "pdf" => "pdf",
        "jpg" | "jpeg" | "png" | "webp" | "heic" | "gif" | "tif" | "tiff" | "bmp" => "image",
        "mp3" | "wav" | "m4a" | "aac" | "flac" | "ogg" | "opus" => "audio",
        "mp4" | "mov" | "mkv" | "webm" | "avi" | "m4v" => "video",
        "docx" | "doc" | "odt" | "rtf" => "document",
        "xlsx" | "xls" | "ods" | "csv" => "sheet",
        "zip" | "7z" | "tar" | "gz" | "tgz" | "xz" | "rar" => "archive",
        "txt" | "md" | "log" => "text",
        "pptx" | "ppt" | "odp" | "ppsx" | "pps" | "key" => "slides",
        _ => "other",
    }
}

/// Esegue un'azione. Ritorna un messaggio da mostrare all'utente.
#[cfg_attr(not(test), allow(dead_code))] // l'app usa run_with; run resta per i test
pub fn run(path: &str, key: &str) -> Result<String, String> {
    run_with(path, key, &HashMap::new())
}

/// Come run(), con i dati inseriti nel pannello (per ora solo "password").
pub fn run_with(path: &str, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let msg = run_inner(path, key, data)?;
    let here = Path::new(path).parent().is_some_and(writable);
    Ok(if !here && (msg.starts_with("Creato") || msg.starts_with("Estratto")) {
        format!("{}\nSalvato sulla Scrivania: nella cartella del file non si può scrivere.", msg)
    } else { msg })
}

fn run_inner(path: &str, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let input = PathBuf::from(path);
    if !input.exists() {
        return Err(format!("Il file non esiste più: {}", path));
    }
    if !SUPPORTED.contains(&key) {
        return Err("Questa opzione non è ancora disponibile.".into());
    }
    let i = s(&input);
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    precheck(&input, key, &ext)?;
    let password = || -> Result<&str, String> {
        let p = data.get("password").map(|p| p.as_str()).unwrap_or("");
        if p.is_empty() { return Err("Serve una password.".into()); }
        if p.contains('\n') || p.contains('\r') { return Err("La password non può contenere a capo.".into()); }
        Ok(p)
    };
    // Testo del pannello (autore, titolo…): una riga, non vuota.
    let testo = || -> Result<String, String> {
        let v = data.get("valore").map(|v| v.trim()).unwrap_or("");
        if v.is_empty() { return Err("Scrivi un valore.".into()); }
        if v.contains('\n') || v.chars().count() > 200 { return Err("Il valore deve stare su una riga (massimo 200 caratteri).".into()); }
        Ok(v.to_string())
    };
    let data_ora = || data.get("valore").and_then(|v| parse_date(v)).ok_or_else(|| "Scrivi la data come 24/10/2026 o 24/10/2026 15:30.".to_string());
    let planned = || planned_out(&input, key).ok_or_else(|| "Opzione senza file di uscita.".to_string());
    // Orario del pannello, in secondi, controllato rispetto alla durata del file.
    let orario = || -> Result<String, String> {
        let t = data.get("orario").and_then(|v| parse_time(v)).ok_or("Scrivi un orario, per esempio 1:30.")?;
        let dur = duration(&input)?;
        if t <= 0.0 || t >= dur {
            return Err(format!("L'orario deve essere tra 0:00 e {}.", fmt_time(dur)));
        }
        Ok(format!("{:.3}", t))
    };

    match key {
        // ---------------- PDF ----------------
        "pdf/comprimi/alta-qualita" => gs_compress(&input, "/prepress"),
        "pdf/comprimi/bilanciata" => gs_compress(&input, "/ebook"),
        "pdf/comprimi/massima" | "pdf/comprimi/per-email" => gs_compress(&input, "/screen"),
        "pdf/pagine/ruota" => {
            let out = out_path(&input, "_ruotato", "pdf");
            run_cmd("qpdf", &[&i, &s(&out), "--rotate=+90"])?;
            Ok(format!("Creato: {} (tutte le pagine ruotate di 90° in senso orario)", name(&out)))
        }
        "pdf/converti/word" => soffice(&input, "docx", Some("writer_pdf_import")),
        "pdf/converti/testo" => {
            let out = out_path(&input, "", "txt");
            run_cmd("pdftotext", &["-layout", &i, &s(&out)])?;
            done(&out)
        }
        "pdf/converti/immagini" => {
            let dir = out_dir(&input, "_pagine");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            run_cmd("pdftoppm", &["-png", "-r", "150", &i, &s(&dir.join("pagina"))])?;
            done(&dir)
        }
        "pdf/converti/pdf-a" => {
            let out = out_path(&input, "_PDFA", "pdf");
            run_cmd("gs", &["-dPDFA=2", "-dBATCH", "-dNOPAUSE", "-dQUIET", "-sColorConversionStrategy=RGB",
                "-dPDFACompatibilityPolicy=1", "-sDEVICE=pdfwrite", &format!("-sOutputFile={}", s(&out)), &i])?;
            done(&out)
        }
        "pdf/proteggi/password" => {
            let pw = password()?;
            if qpdf_encrypted(&input) { return Err("Questo PDF ha già una password.".into()); }
            let out = planned()?;
            qpdf_job(serde_json::json!({"inputFile": i, "outputFile": s(&out),
                "encrypt": {"userPassword": pw, "ownerPassword": pw, "256bit": {}}}))?;
            done(&out)
        }
        "pdf/proteggi/rimuovi-password" => {
            let pw = password()?;
            if !qpdf_encrypted(&input) { return Err("Questo PDF non ha una password.".into()); }
            let out = planned()?;
            qpdf_job(serde_json::json!({"inputFile": i, "password": pw, "decrypt": "", "outputFile": s(&out)}))?;
            done(&out)
        }
        "pdf/metadati/mostra" | "image/metadati/mostra" | "document/metadati/mostra" | "sheet/metadati/mostra" => {
            let txt = run_cmd("exiftool", &["-S", "-Title", "-Author", "-Creator", "-Producer", "-CreateDate",
                "-ModifyDate", "-Make", "-Model", "-GPSPosition", "-ImageSize", "-PageCount", &i])?;
            // exiftool -S stampa anche i campi vuoti ("Title: "): li togliamo.
            let txt: Vec<&str> = txt.lines().filter(|l| !l.trim_end().ends_with(':')).collect();
            Ok(if txt.is_empty() { "Nessun metadato rilevante.".into() } else { txt.join("\n") })
        }
        "pdf/metadati/cambia-autore" => {
            let v = testo()?;
            let out = planned()?;
            // L'autore sta sia nel dizionario Info sia nell'XMP (dc:creator): li cambiamo entrambi.
            pdf_exif(&input, &out, &[&format!("-PDF:Author={}", v), &format!("-XMP-dc:Creator={}", v)])?;
            done(&out)
        }
        "pdf/metadati/cambia-data" => {
            let d = data_ora()?;
            let out = planned()?;
            let e = d.exif();
            pdf_exif(&input, &out, &[&format!("-PDF:CreateDate={}", e), &format!("-PDF:ModifyDate={}", e),
                &format!("-XMP-xmp:CreateDate={}", e), &format!("-XMP-xmp:ModifyDate={}", e)])?;
            done(&out)
        }
        "image/metadati/cambia-data" => {
            // AllDates = data di scatto, di creazione e di modifica.
            let d = data_ora()?;
            let out = planned()?;
            run_cmd("exiftool", &[&format!("-AllDates={}", d.exif()), "-o", &s(&out), &i])?;
            done(&out)
        }
        "audio/tag/titolo" | "audio/tag/artista" | "audio/tag/album" => {
            let v = testo()?;
            let field = match key { "audio/tag/titolo" => "title", "audio/tag/artista" => "artist", _ => "album" };
            let meta = format!("{}={}", field, v);
            let mut ops = vec!["-map", "0", "-c", "copy", "-metadata", meta.as_str()];
            if ext == "mp3" { ops.extend_from_slice(&["-id3v2_version", "3"]); }
            ff_to(&input, &planned()?, &[], &ops)
        }
        "document/metadati/cambia-autore" | "sheet/metadati/cambia-autore" => {
            let v = xml_escape(&testo()?);
            let out = planned()?;
            ooxml_core(&input, &out, |x| {
                let x = set_xml(x, "dc:creator", "", &v);
                set_xml(&x, "cp:lastModifiedBy", "", &v)
            })?;
            done(&out)
        }
        "sheet/metadati/cambia-data" => {
            let d = data_ora()?;
            let out = planned()?;
            let w3c = d.w3c();
            ooxml_core(&input, &out, |x| {
                let x = set_xml(x, "dcterms:created", " xsi:type=\"dcterms:W3CDTF\"", &w3c);
                set_xml(&x, "dcterms:modified", " xsi:type=\"dcterms:W3CDTF\"", &w3c)
            })?;
            done(&out)
        }
        "pdf/metadati/rimuovi-tutti" => {
            // Sui PDF exiftool aggiunge una revisione e lascia i vecchi dati nel file:
            // qpdf riscrive il PDF tenendo solo gli oggetti ancora usati.
            let out = out_path(&input, "_senza_metadati", "pdf");
            let tmp = std::env::temp_dir().join(format!("halo-meta-{}.pdf", std::process::id()));
            let _ = fs::remove_file(&tmp);
            let r = run_cmd("exiftool", &["-all:all=", "-o", &s(&tmp), &i])
                .and_then(|_| run_cmd("qpdf", &[&s(&tmp), &s(&out)]));
            let _ = fs::remove_file(&tmp);
            r?;
            done(&out)
        }
        "image/metadati/rimuovi-tutti" => {
            let out = out_path(&input, "_senza_metadati", &ext);
            run_cmd("exiftool", &["-all:all=", "-o", &s(&out), &i])?;
            done(&out)
        }

        // ---------------- Immagini (ImageMagick 6: "convert") ----------------
        "image/modifica/ruota" => im_note(&input, "_ruotata", &ext, &["-rotate", "90"], "ruotata di 90° in senso orario"),
        "image/modifica/capovolgi" => im_note(&input, "_capovolta", &ext, &["-flop"], "specchiata da sinistra a destra"),
        "image/modifica/luminosita" => im_note(&input, "_luminosa", &ext, &["-modulate", "115"], "luminosità +15%"),
        "image/modifica/bianco-e-nero" => im_note(&input, "_bn", &ext, &["-colorspace", "Gray"], "in bianco e nero"),
        "image/ritaglia/automatico" => {
            // Toglie i bordi di colore uniforme (bande bianche o nere, cornici, margini delle scansioni).
            let out = out_path(&input, "_ritagliata", &ext);
            let r = im_note(&input, "_ritagliata", &ext, &["-fuzz", "8%", "-trim", "+repage"], "tolti i bordi di colore uniforme")?;
            let dims = |p: &Path| run_cmd("identify", &["-format", "%wx%h", &format!("{}[0]", s(p))]).unwrap_or_default();
            let before = run_cmd("convert", &[&i, "-auto-orient", "-format", "%wx%h", "info:"]).unwrap_or_default();
            if dims(&out) == before {
                let _ = fs::remove_file(&out);
                return Err("Nessun bordo uniforme da togliere: l'immagine resta uguale, nessun file creato. «Automatico» toglie bande e cornici di un solo colore attorno all'immagine.".into());
            }
            Ok(r)
        }
        "image/sfondo/bianco" => im_note(&input, "_sfondo_bianco", &ext, &["-background", "white", "-alpha", "remove", "-alpha", "off"], "parti trasparenti riempite di bianco"),
        "image/ritaglia/1-1" => im_note(&input, "_1x1", &ext, &["-gravity", "center", "-crop", "1:1", "+repage"], "quadrato preso dal centro"),
        "image/ritaglia/16-9" => im_note(&input, "_16x9", &ext, &["-gravity", "center", "-crop", "16:9", "+repage"], "formato 16:9 preso dal centro"),
        "image/ridimensiona/su-misura" => {
            let g = data.get("valore").and_then(|v| parse_size(v))
                .ok_or("Scrivi la larghezza (1200), larghezza×altezza (1200x800) o una percentuale (50%).")?;
            let out = planned()?;
            run_cmd("convert", &[&i, "-resize", &g, &s(&out)])?;
            done(&out)
        }
        "image/ridimensiona/meta" => im_note(&input, "_50", &ext, &["-resize", "50%"], "metà larghezza e altezza"),
        "image/ridimensiona/per-il-web" => im_note(&input, "_web", "jpg", &["-resize", "1920x1920>", "-strip", "-quality", "82"], "JPEG, lato lungo al massimo 1920 px, senza metadati"),
        "image/ridimensiona/per-email" => im_note(&input, "_email", "jpg", &["-resize", "1280x1280>", "-strip", "-quality", "75"], "JPEG, lato lungo al massimo 1280 px, senza metadati"),
        "image/converti/png" => im(&input, "", "png", &[]),
        "image/converti/webp" => im(&input, "", "webp", &["-quality", "85"]),
        "image/converti/pdf" => im(&input, "", "pdf", &[]),
        "image/metadati/rimuovi-posizione" => {
            let out = out_path(&input, "_senza_gps", &ext);
            run_cmd("exiftool", &["-gps:all=", "-o", &s(&out), &i])?;
            done(&out)
        }

        // ---------------- Audio (FFmpeg) ----------------
        // Audio: copia diretta dei dati, il taglio è preciso. Video: ricodifica, preciso al fotogramma.
        "audio/taglia/inizio" => { let t = orario()?; ff_to(&input, &planned()?, &["-ss", &t], &["-c", "copy"]) }
        "audio/taglia/fine" => { let t = orario()?; ff_to(&input, &planned()?, &[], &["-t", &t, "-c", "copy"]) }
        "video/taglia/inizio" => { let t = orario()?; ff_to(&input, &planned()?, &["-ss", &t], VIDEO_HQ) }
        "video/taglia/fine" => {
            let t = orario()?;
            ff_to(&input, &planned()?, &[], &[&["-t", t.as_str()][..], VIDEO_HQ].concat())
        }
        "audio/taglia/dividi" | "video/taglia/dividi" => {
            let t = orario()?;
            let video = key.starts_with("video");
            let codec: &[&str] = if video { VIDEO_HQ } else { &["-c", "copy"] };
            let p1 = planned()?;
            let p2 = out_path(&input, "_parte2", if video { video_ext(&ext) } else { &ext });
            ff_to(&input, &p1, &[], &[&["-t", t.as_str()][..], codec].concat())?;
            ff_to(&input, &p2, &["-ss", &t], codec)?;
            Ok(format!("Creato: {} e {}", name(&p1), name(&p2)))
        }
        "audio/taglia/rimuovi-silenzi" => ff(&input, "_senza_silenzi", &ext,
            &["-af", "silenceremove=stop_periods=-1:stop_duration=1:stop_threshold=-40dB"]).map(|m| m + " (tolte le pause più lunghe di 1 secondo)"),
        "audio/volume/normalizza" => ff(&input, "_normalizzato", &ext, &["-af", "loudnorm"]),
        "audio/volume/aumenta" => ff(&input, "_piu_forte", &ext, &["-af", "volume=6dB"]).map(|m| m + " (+6 dB, circa il doppio del volume percepito)"),
        "audio/volume/riduci-rumore" => ff(&input, "_pulito", &ext, &["-af", "afftdn"]),
        "audio/volume/dissolvenza" => {
            let f = fade_filter(&input)?;
            ff(&input, "_dissolvenza", &ext, &["-af", &f]).map(|m| m + " (entrata e uscita graduali)")
        }
        "audio/converti/wav" => ff(&input, "", "wav", &[]),
        "audio/converti/aac" => ff(&input, "", "aac", &["-c:a", "aac", "-b:a", "192k"]),
        "audio/converti/m4a" => ff(&input, "", "m4a", &["-vn", "-c:a", "aac", "-b:a", "192k"]),
        "audio/converti/flac" => ff(&input, "", "flac", &[]),
        "audio/comprimi/128-kbps" => ff(&input, "_128k", "mp3", &["-b:a", "128k"]),
        "audio/comprimi/192-kbps" => ff(&input, "_192k", "mp3", &["-b:a", "192k"]),
        "audio/comprimi/320-kbps" => ff(&input, "_320k", "mp3", &["-b:a", "320k"]),
        "audio/comprimi/mono" => ff(&input, "_mono", &ext, &["-ac", "1"]).map(|m| m + " (un solo canale: adatto alla voce)"),

        // ---------------- Video (FFmpeg) ----------------
        "video/comprimi/1080p" => ff(&input, "_1080p", "mp4", &["-vf", "scale=-2:'min(1080,ih)'", "-c:v", "libx264", "-crf", "23", "-preset", "medium", "-c:a", "aac", "-b:a", "160k"]).map(|m| m + " (al massimo 1080p, MP4)"),
        "video/comprimi/720p" => ff(&input, "_720p", "mp4", &["-vf", "scale=-2:'min(720,ih)'", "-c:v", "libx264", "-crf", "24", "-preset", "medium", "-c:a", "aac", "-b:a", "128k"]).map(|m| m + " (al massimo 720p, MP4)"),
        "video/comprimi/per-email" => ff(&input, "_email", "mp4", &["-vf", "scale=-2:'min(720,ih)'", "-c:v", "libx264", "-crf", "30", "-preset", "slow", "-c:a", "aac", "-b:a", "96k"]).map(|m| m + " (al massimo 720p, compressione forte)"),
        "video/comprimi/per-il-web" => ff(&input, "_web", "mp4", &["-c:v", "libx264", "-crf", "26", "-preset", "medium", "-movflags", "+faststart", "-c:a", "aac", "-b:a", "128k"]).map(|m| m + " (MP4 che parte subito nel browser)"),
        "video/converti/mov" => {
            // Prima si prova a copiare i flussi così come sono (veloce, senza perdita); se il formato
            // non li accetta (es. un WebM), si ricodifica.
            ff(&input, "", "mov", &["-c", "copy"]).or_else(|_| {
                let _ = fs::remove_file(out_path(&input, "", "mov"));
                ff(&input, "", "mov", VIDEO_HQ)
            })
        }
        "video/converti/gif" => {
            // Una GIF lunga diventa enorme: al massimo i primi 30 secondi.
            let long = duration(&input).map(|d| d > 30.0).unwrap_or(false);
            let pre: &[&str] = if long { &["-t", "30"] } else { &[] };
            ff_pre(&input, "", "gif", &[], &[pre, &["-vf", "fps=12,scale=480:-1:flags=lanczos"][..]].concat())
                .map(|m| if long { m + " (solo i primi 30 secondi: una GIF più lunga sarebbe enorme)" } else { m })
        }
        "video/converti/solo-audio" => ff(&input, "_audio", "m4a", &["-vn", "-c:a", "aac", "-b:a", "192k"]),
        "video/converti/webm" => ff(&input, "", "webm", &["-c:v", "libvpx-vp9", "-crf", "34", "-b:v", "0", "-c:a", "libopus"]),
        "video/audio/rimuovi-audio" => ff(&input, "_muto", &ext, &["-an", "-c:v", "copy"]),
        "video/audio/normalizza" => ff(&input, "_normalizzato", &ext, &["-c:v", "copy", "-af", "loudnorm"]),
        "video/audio/dissolvenza" => {
            let f = fade_filter(&input)?;
            ff(&input, "_dissolvenza", &ext, &["-c:v", "copy", "-af", &f])
        }
        "video/ruota/90-a-destra" => ff(&input, "_ruotato", &ext, &["-vf", "transpose=1", "-c:a", "copy"]),
        "video/ruota/90-a-sinistra" => ff(&input, "_ruotato", &ext, &["-vf", "transpose=2", "-c:a", "copy"]),
        "video/ruota/capovolgi" => ff(&input, "_capovolto", &ext, &["-vf", "hflip", "-c:a", "copy"]),
        "video/fotogrammi/velocita" => {
            let f = data.get("valore").and_then(|v| parse_speed(v))
                .ok_or("Scrivi la velocità tra 0,25 e 4 (per esempio 2 per il doppio, 0,5 per la metà).")?;
            let vf = format!("setpts=PTS/{}", f);
            let af = atempo(f);
            let mut ops = vec!["-filter:v", vf.as_str()];
            if has_audio(&input) { ops.extend_from_slice(&["-filter:a", af.as_str()]); }
            ops.extend_from_slice(VIDEO_HQ);
            ff_to(&input, &planned()?, &[], &ops)
        }
        "video/fotogrammi/copertina" => {
            // Il fotogramma a 1 secondo (o a metà, per i video più corti di 2 secondi).
            let at = duration(&input).map(|d| if d < 2.0 { d / 2.0 } else { 1.0 }).unwrap_or(1.0);
            ff_pre(&input, "_copertina", "jpg", &["-ss", &format!("{:.2}", at)], &["-frames:v", "1", "-q:v", "2"])
        }
        "video/fotogrammi/anteprima-gif" => ff(&input, "_anteprima", "gif", &["-t", "5", "-vf", "fps=10,scale=360:-1:flags=lanczos"]).map(|m| m + " (i primi 5 secondi)"),

        // ---------------- Documenti e fogli (LibreOffice / Pandoc) ----------------
        "document/converti/pdf" | "sheet/converti/pdf" | "text/converti/pdf" => soffice(&input, "pdf", None),
        "document/converti/testo" => soffice(&input, "txt:Text", None),
        "document/converti/markdown" => pandoc(&input, "md", &["-t", "gfm"]),
        "document/converti/html" => pandoc(&input, "html", &["-s"]),
        "sheet/converti/csv" => soffice(&input, "csv", None),
        "sheet/converti/ods" => soffice(&input, "ods", None),

        // ---------------- Cartelle (7-Zip) ----------------
        "folder/comprimi/zip" => {
            let out = out_path(&input, "", "zip");
            run_cmd("7z", &["a", "-tzip", &s(&out), &i])?;
            done(&out)
        }
        "folder/comprimi/zip-protetto" => {
            let pw = zip_password(password()?)?;
            let out = planned()?;
            seven_in(&["a", "-tzip", "-mem=AES256", "-p", &s(&out), &i], &format!("{0}\n{0}\n", pw))?;
            done(&out)
        }
        "folder/comprimi/7z" => {
            let out = out_path(&input, "", "7z");
            run_cmd("7z", &["a", &s(&out), &i])?;
            done(&out)
        }
        "folder/comprimi/dividi-in-parti" => {
            let out = out_path(&input, "", "zip");
            run_cmd("7z", &["a", "-tzip", "-v100m", &s(&out), &i])?;
            Ok(format!("Creato in parti da 100 MB: {}.001, .002…", name(&out)))
        }
        "folder/analizza/dimensione" => {
            let (bytes, files) = crate::sistema::size_and_count(&input);
            Ok(format!("Dimensione totale: {} in {} file", human(bytes), files))
        }

        // ---------------- Archivi (7-Zip) ----------------
        "archive/estrai/qui" => {
            let parent = parent(&input);
            run_cmd("7z", &["x", "-aou", &format!("-o{}", s(&parent)), &i])?;
            Ok(format!("Estratto in {}", s(&parent)))
        }
        "archive/estrai/in-una-cartella" | "archive/converti/cartella" => {
            let dir = out_dir(&input, "");
            run_cmd("7z", &["x", &format!("-o{}", s(&dir)), &i])?;
            done(&dir)
        }
        "archive/anteprima/contenuto" => {
            let txt = run_cmd("7z", &["l", "-ba", &i])?;
            let lines: Vec<&str> = txt.lines().collect();
            let mut msg = format!("{} elementi", lines.len());
            for l in lines.iter().take(8) {
                if let Some(n) = l.get(53..) { msg.push_str(&format!("\n{}", n.trim())); }
            }
            Ok(msg)
        }
        "archive/anteprima/verifica" => {
            run_cmd("7z", &["t", &i])?;
            Ok("Archivio integro: nessun errore trovato.".into())
        }
        "archive/converti/7z" | "archive/converti/tar" => {
            let target = if key.ends_with("7z") { "7z" } else { "tar" };
            let tmp = std::env::temp_dir().join(format!("halo-{}", std::process::id()));
            let _ = fs::remove_dir_all(&tmp);
            run_cmd("7z", &["x", &format!("-o{}", s(&tmp)), &i])?;
            let out = out_path(&input, "", target);
            let mut args = vec!["a".to_string()];
            if target == "tar" { args.push("-ttar".into()); }
            args.push(s(&out));
            args.push(format!("{}/*", s(&tmp)));
            let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
            let r = run_cmd("7z", &refs);
            let _ = fs::remove_dir_all(&tmp);
            r?;
            done(&out)
        }

        "archive/proteggi/password" | "archive/converti/zip-protetto" | "archive/proteggi/cifra-aes" => {
            let out = planned()?;
            let pw = if key.ends_with("cifra-aes") { password()? } else { zip_password(password()?)? };
            repack(&input, &out, None, Some(pw), &[])?;
            done(&out)
        }
        "archive/proteggi/rimuovi-password" => {
            let pw = password()?;
            let out = planned()?;
            repack(&input, &out, Some(pw), None, &[])?;
            done(&out)
        }
        "archive/ricomprimi/dividi-in-parti" => {
            let mb: u32 = data.get("valore").and_then(|v| v.trim().trim_end_matches("MB").trim().parse().ok())
                .filter(|n| (1..=100_000).contains(n))
                .ok_or("Scrivi quanti MB deve avere ogni parte (un numero, per esempio 25).")?;
            let out = planned()?;
            repack(&input, &out, None, None, &[&format!("-v{}m", mb)])?;
            let n = fs::read_dir(parent(&out)).map(|d| d.filter_map(|e| e.ok())
                .filter(|e| name(&e.path()).starts_with(&format!("{}.", name(&out)))).count()).unwrap_or(0);
            Ok(format!("Creato: {}.001 ({} {} da {} MB al massimo)", name(&out), n, if n == 1 { "parte" } else { "parti" }, mb))
        }

        // ---------------- Testo (in Rust, senza dipendenze) ----------------
        "text/converti/word" => soffice(&input, "docx", None),
        "text/converti/markdown" => {
            let out = out_path(&input, "", "md");
            fs::copy(&input, &out).map_err(|e| e.to_string())?;
            done(&out)
        }
        "text/converti/html" => pandoc(&input, "html", &["-f", "markdown", "-s"]),
        "text/cerca/trova" => {
            let q = data.get("valore").filter(|v| !v.is_empty()).ok_or("Scrivi cosa cercare.")?;
            let t = read_text(&input)?;
            let ql = q.to_lowercase();
            let hits: Vec<String> = t.lines().enumerate().filter(|(_, l)| l.to_lowercase().contains(&ql))
                .map(|(n, l)| format!("riga {}: {}", n + 1, short(l.trim(), 60))).collect();
            if hits.is_empty() { return Ok(format!("«{}» non compare nel testo.", q)); }
            let total = t.to_lowercase().matches(&ql).count();
            let more = if hits.len() > 6 { format!("\n…e altre {} righe", hits.len() - 6) } else { String::new() };
            Ok(format!("{} {}:\n{}{}", total, if total == 1 { "risultato" } else { "risultati" }, hits[..hits.len().min(6)].join("\n"), more))
        }
        "text/cerca/sostituisci" => {
            // Testo esatto, maiuscole comprese; il secondo campo può essere vuoto (= togli).
            let q = data.get("valore").filter(|v| !v.is_empty()).ok_or("Scrivi il testo da cercare.")?;
            let new = data.get("sostituto").map(|s| s.as_str()).unwrap_or("");
            let t = read_utf8(&input)?;
            let n = t.matches(q.as_str()).count();
            if n == 0 { return Err(format!("«{}» non compare nel testo (contano anche le maiuscole).", q)); }
            let out = planned()?;
            fs::write(&out, t.replace(q.as_str(), new)).map_err(|e| e.to_string())?;
            Ok(format!("Creato: {} ({} {})", name(&out), n, if n == 1 { "sostituzione" } else { "sostituzioni" }))
        }
        "text/codifica/rileva" => {
            let b = fs::read(&input).map_err(|e| e.to_string())?;
            let (crlf, lf, cr) = line_ends(&b);
            let fine = match (crlf > 0, lf > 0, cr > 0) {
                (false, false, false) => "nessun a capo",
                (true, false, false) => "CRLF (Windows)",
                (false, true, false) => "LF (Linux, macOS)",
                (false, false, true) => "CR (vecchi Mac)",
                _ => "misti",
            };
            Ok(format!("Codifica: {}\nFine riga: {}", encoding_name(&b), fine))
        }
        "text/codifica/utf-8" => {
            let b = fs::read(&input).map_err(|e| e.to_string())?;
            let text = match detect(&b) {
                Enc::Utf8 => return Err("Il file è già in UTF-8.".into()),
                Enc::Utf8Bom => String::from_utf8_lossy(&b[3..]).into_owned(),
                Enc::Utf16(le) => decode_utf16(&b[2..], le),
                Enc::Win1252 => b.iter().map(|&c| cp1252(c)).collect(),
            };
            let out = out_path(&input, "_utf8", &ext);
            fs::write(&out, text).map_err(|e| e.to_string())?;
            done(&out)
        }
        "text/codifica/fine-riga-lf" | "text/codifica/fine-riga-crlf" => {
            let b = fs::read(&input).map_err(|e| e.to_string())?;
            let crlf = key.ends_with("crlf");
            let mut out_b = Vec::with_capacity(b.len() + b.len() / 20);
            let mut k = 0;
            while k < b.len() {
                let nl = match b[k] { b'\r' if b.get(k + 1) == Some(&b'\n') => { k += 1; true } b'\r' | b'\n' => true, _ => false };
                if nl { if crlf { out_b.push(b'\r'); } out_b.push(b'\n'); } else { out_b.push(b[k]); }
                k += 1;
            }
            if out_b == b { return Err(format!("Il file usa già {}.", if crlf { "CRLF" } else { "LF" })); }
            let out = out_path(&input, if crlf { "_crlf" } else { "_lf" }, &ext);
            fs::write(&out, out_b).map_err(|e| e.to_string())?;
            done(&out)
        }
        "text/cerca/conta-parole" => {
            let t = read_text(&input)?;
            Ok(format!("{} parole · {} righe · {} caratteri", t.split_whitespace().count(), t.lines().count(), t.chars().count()))
        }
        "text/cerca/righe-doppie" => transform_text(&input, "_senza_doppie", |t| {
            let mut seen = std::collections::HashSet::new();
            t.lines().filter(|l| seen.insert(l.to_string())).collect::<Vec<_>>().join("\n")
        }),
        "text/formatta/maiuscole" => transform_text(&input, "_maiuscolo", |t| t.to_uppercase()),
        "text/formatta/minuscole" => transform_text(&input, "_minuscolo", |t| t.to_lowercase()),
        "text/formatta/ordina-righe" => transform_text(&input, "_ordinato", |t| {
            let mut v: Vec<&str> = t.lines().collect();
            v.sort_by_key(|a| a.to_lowercase());
            v.join("\n")
        }),
        "text/formatta/togli-spazi" => transform_text(&input, "_pulito", |t| {
            t.lines().map(|l| l.split_whitespace().collect::<Vec<_>>().join(" ")).collect::<Vec<_>>().join("\n")
        }),

        // Rinomina, Organizza, Pulisci (su una copia) e Analizza.
        k if crate::cartelle::KEYS.contains(&k) => crate::cartelle::run(&input, k, data),
        k if crate::oscura::KEYS.contains(&k) => crate::oscura::run(&input, k, data),
        k if crate::annota::KEYS.contains(&k) => crate::annota::run(&input, k, data),
        k if crate::rapide::KEYS.contains(&k) => crate::rapide::run(&input, k, data),
        k if crate::scegli::KEYS.contains(&k) => crate::scegli::run(&input, k, data),
        k if crate::documenti::KEYS.contains(&k) => crate::documenti::run(&input, k),
        k if crate::fogli::KEYS.contains(&k) => crate::fogli::run(&input, k, data),
        k if crate::archivi::KEYS.contains(&k) => crate::archivi::run(&input, k, data),
        k if crate::condividi::KEYS.contains(&k) => crate::condividi::run(&input, k),
        k if crate::editor::KEYS.contains(&k) => crate::editor::run(&input, k, data),
        k if crate::word::KEYS.contains(&k) => crate::word::run(&input, k, data),
        k if crate::altri::KEYS.contains(&k) => crate::altri::run(&input, k, data),
        k if crate::ocr::KEYS.contains(&k) => crate::ocr::run(&input, k),
        k if crate::qr::KEYS.contains(&k) => crate::qr::run(&input, k),
        k if crate::sottotitoli::KEYS.contains(&k) => crate::sottotitoli::run(&input, k, data),
        k if crate::ingrandisci::KEYS.contains(&k) => crate::ingrandisci::run(&input, k, data),
        k if crate::traduci::KEYS.contains(&k) => Err("Traduci si usa dal pannello: scegli lo strumento e premi Apri.".into()),
        "image/converti/heic" => {
            // heif-enc legge JPEG e PNG; gli altri formati passano prima da un PNG temporaneo.
            let out = planned()?;
            let tmp = std::env::temp_dir().join(format!("halo-heic-{}.png", std::process::id()));
            let src = if ["jpg", "jpeg", "png"].contains(&ext.as_str()) { input.clone() } else {
                run_cmd("convert", &[&format!("{}[0]", i), &s(&tmp)])?;
                tmp.clone()
            };
            let r = run_cmd("heif-enc", &["-q", "85", &s(&src), "-o", &s(&out)]);
            let _ = fs::remove_file(&tmp);
            r.map_err(|e| if e.contains("No HEVC encoder") {
                "Manca il codificatore HEIC. Installa con: sudo apt install libheif-plugin-x265".to_string()
            } else { e })?;
            done(&out)
        }

        _ => Err("Questa opzione non è ancora disponibile.".into()),
    }
}

// ---------------- helper ----------------

pub(crate) fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }
pub(crate) fn name(p: &Path) -> String { p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default() }
/// Dove salvare i risultati: la cartella del file (quella in cui ti trovi in Nautilus o sulla
/// Scrivania). Se lì non si può scrivere (disco in sola lettura, cartella di sistema) si usa la Scrivania.
pub(crate) fn parent(p: &Path) -> PathBuf {
    let dir = p.parent().map(|x| x.to_path_buf()).unwrap_or_else(|| PathBuf::from("."));
    if writable(&dir) { dir } else { desktop_dir() }
}

fn writable(dir: &Path) -> bool {
    crate::sistema::writable(dir)
}

/// La Scrivania dell'utente (~/Scrivania in italiano), o la home se non c'è.
pub(crate) fn desktop_dir() -> PathBuf {
    crate::sistema::desktop_dir()
}
fn stem(p: &Path) -> String { p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| "file".into()) }
fn done(out: &Path) -> Result<String, String> { Ok(format!("Creato: {}", name(out))) }

/// "cartella/nome_suffisso.ext", aggiungendo " (2)", " (3)"… se esiste già.
pub fn out_path(input: &Path, suffix: &str, ext: &str) -> PathBuf {
    let dir = parent(input);
    let base = format!("{}{}", stem(input), suffix);
    let mut candidate = dir.join(format!("{}.{}", base, ext));
    let mut n = 2;
    while candidate.exists() || candidate == input {
        candidate = dir.join(format!("{} ({}).{}", base, n, ext));
        n += 1;
    }
    candidate
}

pub(crate) fn out_dir(input: &Path, suffix: &str) -> PathBuf {
    let dir = parent(input);
    let base = format!("{}{}", stem(input).trim_end_matches(".tar"), suffix);
    let mut candidate = dir.join(&base);
    let mut n = 2;
    while candidate.exists() {
        candidate = dir.join(format!("{} ({})", base, n));
        n += 1;
    }
    candidate
}

fn package_for(cmd: &str) -> &'static str {
    match cmd {
        "gs" => "ghostscript",
        "qpdf" => "qpdf",
        "pdftotext" | "pdftoppm" => "poppler-utils",
        "convert" => "imagemagick",
        "ffmpeg" | "ffprobe" => "ffmpeg",
        "soffice" => "libreoffice-writer libreoffice-calc libreoffice-impress",
        "pandoc" => "pandoc",
        "7z" => "7zip",
        "exiftool" => "libimage-exiftool-perl",
        "heif-enc" => "libheif-examples libheif-plugin-x265",
        "wl-copy" => "wl-clipboard",
        "ocrmypdf" => "ocrmypdf tesseract-ocr-ita",
        "tesseract" => "tesseract-ocr tesseract-ocr-ita",
        "espeak-ng" => "espeak-ng",
        _ => "il pacchetto corrispondente",
    }
}

pub(crate) fn run_cmd(cmd: &str, args: &[&str]) -> Result<String, String> {
    if crate::lavoro::cancelled() { return Err("Annullato.".into()); }
    let r = if cmd == "ffmpeg" { ffmpeg_progress(args) } else {
        crate::lavoro::run_tracked(&mut crate::sistema::cmd(cmd).args(args))
            .map(|o| (o.status, String::from_utf8_lossy(&o.stdout).into_owned(), String::from_utf8_lossy(&o.stderr).into_owned()))
    };
    match r {
        Ok((st, out, _)) if st.success() => Ok(out),
        Ok(_) if crate::lavoro::cancelled() => Err("Annullato.".into()),
        Ok((_, _, err)) => {
            let last = err.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("errore sconosciuto");
            Err(format!("{} non è riuscito: {}", cmd, last.trim()))
        }
        Err(e) if e.kind() == ErrorKind::NotFound => Err(missing(cmd)),
        Err(e) => Err(e.to_string()),
    }
}

/// Messaggio per un programma esterno che manca, con il comando per installarlo.
pub(crate) fn missing(cmd: &str) -> String {
    if cfg!(windows) {
        format!("Manca «{}». Installalo con: {}", cmd, winget_for(cmd))
    } else {
        format!("Manca «{}». Installa con: sudo apt install {}", cmd, package_for(cmd))
    }
}

/// Su Windows i programmi si installano con winget (vedi README, sezione Windows).
fn winget_for(cmd: &str) -> &'static str {
    match cmd {
        "convert" | "identify" | "magick" => "winget install ImageMagick.ImageMagick",
        "gs" => "winget install ArtifexSoftware.GhostScript",
        "qpdf" => "winget install QPDF.QPDF",
        "pdftotext" | "pdftoppm" | "pdfinfo" => "winget install oschwartz10612.Poppler",
        "ffmpeg" | "ffprobe" => "winget install Gyan.FFmpeg",
        "soffice" => "winget install TheDocumentFoundation.LibreOffice",
        "pandoc" => "winget install JohnMacFarlane.Pandoc",
        "7z" => "winget install 7zip.7zip",
        "exiftool" => "winget install OliverBetz.ExifTool",
        "tesseract" => "winget install UB-Mannheim.TesseractOCR",
        "python3" => "winget install Python.Python.3.12 (poi: py -m pip install openpyxl msoffcrypto-tool)",
        _ => "il programma corrispondente (vedi README, sezione Windows)",
    }
}

/// ffmpeg con l'avanzamento: `-progress pipe:1` scrive out_time_us, confrontato con la durata
/// del primo file d'ingresso (meno l'eventuale -ss, al massimo l'eventuale -t).
fn ffmpeg_progress(args: &[&str]) -> std::io::Result<(std::process::ExitStatus, String, String)> {
    let arg_after = |k: &str| args.iter().position(|a| *a == k).and_then(|i| args.get(i + 1)).and_then(|v| parse_time(v));
    let input = args.iter().position(|a| *a == "-i").and_then(|i| args.get(i + 1)).map(Path::new);
    let mut total = input.and_then(|i| duration(i).ok()).unwrap_or(0.0) - arg_after("-ss").unwrap_or(0.0);
    if let Some(t) = arg_after("-t") { total = total.min(t); }
    let mut full = vec!["-progress", "pipe:1", "-nostats"];
    full.extend_from_slice(args);
    crate::lavoro::run_lines(crate::sistema::cmd("ffmpeg").args(&full), false, |l| {
        if let (Some(us), true) = (l.strip_prefix("out_time_us=").and_then(|v| v.parse::<f64>().ok()), total > 0.0) {
            crate::lavoro::set(us / 1_000_000.0 / total);
        }
    })
}

/// Come run_cmd, ma scrive `stdin` sull'ingresso del programma (per le password:
/// così non compaiono nella riga di comando, visibile agli altri programmi).
fn run_cmd_in(cmd: &str, args: &[&str], stdin: &str) -> Result<String, String> {
    let child = crate::sistema::cmd(cmd).args(args)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) if e.kind() == ErrorKind::NotFound => return Err(missing(cmd)),
        Err(e) => return Err(e.to_string()),
    };
    if let Some(mut si) = child.stdin.take() {
        let _ = si.write_all(stdin.as_bytes());
    }
    let o = child.wait_with_output().map_err(|e| e.to_string())?;
    if o.status.success() {
        return Ok(String::from_utf8_lossy(&o.stdout).into_owned());
    }
    let all = format!("{}\n{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
    if all.contains("Wrong password") || all.contains("invalid password") {
        return Err("Password errata.".into());
    }
    let last = all.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("errore sconosciuto");
    Err(format!("{} non è riuscito: {}", cmd, last.trim()))
}

/// 7-Zip accetta per gli ZIP solo password con caratteri ASCII (niente accenti).
fn zip_password(pw: &str) -> Result<&str, String> {
    if pw.is_ascii() { Ok(pw) } else {
        Err("Negli ZIP la password non può avere lettere accentate o simboli speciali: usa «Cifra AES» (7z) per averle.".into())
    }
}

/// 7z con la password passata sull'ingresso (va usato con "-p" senza valore,
/// oppure in estrazione, dove 7z la chiede da solo se serve).
pub(crate) fn seven_in(args: &[&str], stdin: &str) -> Result<String, String> {
    // 7z vuole prima il comando ("a", "x", "t"…), poi le opzioni.
    let mut a = vec![args[0], "-bd", "-y"];
    a.extend_from_slice(&args[1..]);
    run_cmd_in("7z", &a, stdin)
}

/// Estrae un archivio in una cartella temporanea e lo ricomprime in `out`
/// (ZIP AES-256, oppure 7z con i nomi dei file cifrati se `out` finisce in .7z).
pub(crate) fn repack(input: &Path, out: &Path, read_pw: Option<&str>, write_pw: Option<&str>, extra: &[&str]) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("halo-repack-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = (|| {
        // Senza password giusta 7z non estrae niente: diamo una riga vuota se non ne abbiamo.
        let rp = format!("{}\n", read_pw.unwrap_or(""));
        seven_in(&["x", &format!("-o{}", s(&tmp)), &s(input)], &rp).map_err(|e| {
            if read_pw.is_none() && e == "Password errata." {
                "L'archivio ha già una password: usa prima «Rimuovi password».".to_string()
            } else { e }
        })?;
        let to_7z = out.extension().and_then(|e| e.to_str()) == Some("7z");
        let mut args = vec!["a".to_string(), if to_7z { "-t7z".into() } else { "-tzip".into() }];
        if write_pw.is_some() {
            args.push("-p".into());
            args.push(if to_7z { "-mhe=on".into() } else { "-mem=AES256".into() });
        }
        args.extend(extra.iter().map(|a| a.to_string()));
        args.push(s(out));
        args.push(format!("{}/*", s(&tmp)));
        let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
        let wp = write_pw.map(|p| format!("{0}\n{0}\n", p)).unwrap_or_default();
        seven_in(&refs, &wp).map(|_| ())
    })();
    let _ = fs::remove_dir_all(&tmp);
    r
}

fn qpdf_encrypted(input: &Path) -> bool {
    // qpdf --is-encrypted esce con 0 se il PDF è cifrato, 2 se non lo è.
    crate::sistema::cmd("qpdf").args(["--is-encrypted", &s(input)]).output().map(|o| o.status.success()).unwrap_or(false)
}

/// qpdf con le opzioni in un file JSON temporaneo leggibile solo dall'utente
/// (--job-json-file): le password non finiscono nella riga di comando.
pub(crate) fn qpdf_job(job: serde_json::Value) -> Result<String, String> {
    let f = std::env::temp_dir().join(format!("halo-qpdf-{}.json", std::process::id()));
    let _ = fs::remove_file(&f);
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    // Su Linux il file è leggibile solo dall'utente; su Windows la cartella temporanea è già personale.
    #[cfg(unix)]
    { use std::os::unix::fs::OpenOptionsExt; opts.mode(0o600); }
    let w = opts.open(&f)
        .and_then(|mut h| h.write_all(job.to_string().as_bytes()));
    // --warning-exit-0: gli avvisi (codice 3) non sono errori, il file viene creato comunque.
    let r = w.map_err(|e| e.to_string())
        .and_then(|_| run_cmd_in("qpdf", &["--warning-exit-0", &format!("--job-json-file={}", s(&f))], ""));
    let _ = fs::remove_file(&f);
    r
}

fn gs_compress(input: &Path, preset: &str) -> Result<String, String> {
    let out = out_path(input, "_compresso", "pdf");
    run_cmd("gs", &["-sDEVICE=pdfwrite", "-dCompatibilityLevel=1.6", &format!("-dPDFSETTINGS={}", preset),
        "-dNOPAUSE", "-dBATCH", "-dQUIET", &format!("-sOutputFile={}", s(&out)), &s(input)])?;
    let before = fs::metadata(input).map(|m| m.len()).unwrap_or(0);
    let after = fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
    Ok(format!("Creato: {} ({} → {})", name(&out), human(before), human(after)))
}

fn im(input: &Path, suffix: &str, ext: &str, ops: &[&str]) -> Result<String, String> {
    im_note(input, suffix, ext, ops, "")
}

/// Come im, con una spiegazione di cosa è stato fatto accanto al nome del file creato.
/// Prima di tutto -auto-orient: le foto del telefono hanno l'orientamento scritto nei metadati,
/// e ruotare, ritagliare o togliere i metadati senza applicarlo darebbe foto storte.
fn im_note(input: &Path, suffix: &str, ext: &str, ops: &[&str], note: &str) -> Result<String, String> {
    let out = out_path(input, suffix, ext);
    let i = s(input);
    let o = s(&out);
    let mut args: Vec<&str> = vec![&i, "-auto-orient"];
    args.extend_from_slice(ops);
    args.push(&o);
    run_cmd("convert", &args)?;
    Ok(if note.is_empty() { format!("Creato: {}", name(&out)) } else { format!("Creato: {} ({})", name(&out), note) })
}

/// Controlli prima di eseguire: opzioni che non avrebbero effetto sul file scelto.
/// Così non si crea un file identico all'originale senza che si capisca perché.
fn precheck(input: &Path, key: &str, ext: &str) -> Result<(), String> {
    // Converti nello stesso formato del file.
    let same = match key {
        "image/converti/png" => &["png"][..], "image/converti/webp" => &["webp"], "image/converti/heic" => &["heic", "heif"],
        "audio/converti/wav" => &["wav"], "audio/converti/aac" => &["aac"], "audio/converti/m4a" => &["m4a"], "audio/converti/flac" => &["flac"],
        "video/converti/mov" => &["mov"], "video/converti/webm" => &["webm"], "archive/converti/7z" => &["7z"], "archive/converti/tar" => &["tar"],
        "text/converti/markdown" => &["md"], "sheet/converti/csv" => &["csv"], "sheet/converti/ods" => &["ods"],
        "slides/converti/powerpoint" => &["pptx"], "slides/converti/odp" => &["odp"],
        "document/converti/pdf" | "sheet/converti/pdf" | "text/converti/pdf" | "image/converti/pdf" | "slides/converti/pdf" => &["pdf"],
        _ => &[][..],
    };
    if same.contains(&ext) { return Err(format!("Il file è già in formato {}.", ext.to_uppercase())); }
    match key {
        "image/sfondo/bianco" => {
            // Solo le immagini con parti trasparenti (PNG, WebP…) hanno uno sfondo da riempire.
            let opaque = run_cmd("identify", &["-format", "%[opaque]", &format!("{}[0]", s(input))]).map(|v| v.trim().eq_ignore_ascii_case("true")).unwrap_or(true);
            if opaque {
                return Err("L'immagine non ha parti trasparenti: non c'è uno sfondo da rendere bianco. Per cambiare lo sfondo di una foto usa Sfondo › Colore.".into());
            }
        }
        "image/metadati/rimuovi-posizione" => {
            let gps = run_cmd("exiftool", &["-s", "-s", "-s", "-gps:all", &s(input)]).unwrap_or_default();
            if gps.trim().is_empty() { return Err("La foto non contiene la posizione (GPS): non c'è niente da togliere.".into()); }
        }
        _ => {}
    }
    Ok(())
}

fn ff(input: &Path, suffix: &str, ext: &str, ops: &[&str]) -> Result<String, String> {
    ff_pre(input, suffix, ext, &[], ops)
}

fn ff_pre(input: &Path, suffix: &str, ext: &str, pre: &[&str], ops: &[&str]) -> Result<String, String> {
    let out = out_path(input, suffix, ext);
    let i = s(input);
    let o = s(&out);
    let mut args: Vec<&str> = vec!["-hide_banner", "-nostdin", "-n"];
    args.extend_from_slice(pre);
    args.push("-i");
    args.push(&i);
    args.extend_from_slice(ops);
    args.push(&o);
    run_cmd("ffmpeg", &args)?;
    done(&out)
}

/// Scrive metadati in un PDF con exiftool e lo riscrive con qpdf, così i valori
/// vecchi non restano nascosti in una revisione precedente del file.
fn pdf_exif(input: &Path, out: &Path, tags: &[&str]) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("halo-meta-{}.pdf", std::process::id()));
    let _ = fs::remove_file(&tmp);
    let mut args: Vec<&str> = tags.to_vec();
    let (t, i) = (s(&tmp), s(input));
    args.extend_from_slice(&["-o", &t, &i]);
    let r = run_cmd("exiftool", &args).and_then(|_| run_cmd("qpdf", &[&t, &s(out)]));
    let _ = fs::remove_file(&tmp);
    r.map(|_| ())
}

/// Data e ora inserite nel pannello.
#[derive(Debug, PartialEq)]
pub struct DataOra { y: u32, mo: u32, d: u32, h: u32, mi: u32 }

impl DataOra {
    fn exif(&self) -> String { format!("{:04}:{:02}:{:02} {:02}:{:02}:00", self.y, self.mo, self.d, self.h, self.mi) }
    fn w3c(&self) -> String { format!("{:04}-{:02}-{:02}T{:02}:{:02}:00Z", self.y, self.mo, self.d, self.h, self.mi) }
}

/// "24/10/2026", "24-10-2026 15:30", "24.10.2026 9:05" → DataOra (ora 00:00 se manca).
pub fn parse_date(v: &str) -> Option<DataOra> {
    let mut it = v.split_whitespace();
    let day = it.next()?;
    let time = it.next();
    if it.next().is_some() { return None; }
    let p: Vec<u32> = day.split(|c| c == '/' || c == '-' || c == '.').map(|x| x.parse().ok()).collect::<Option<_>>()?;
    let [d, mo, y] = p[..] else { return None };
    let (h, mi) = match time {
        None => (0, 0),
        Some(t) => {
            let q: Vec<u32> = t.split(':').map(|x| x.parse().ok()).collect::<Option<_>>()?;
            let [h, mi] = q[..] else { return None };
            (h, mi)
        }
    };
    let leap = (y % 4 == 0 && y % 100 != 0) || y % 400 == 0;
    let days = [31, if leap { 29 } else { 28 }, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    if !(1900..=2100).contains(&y) || !(1..=12).contains(&mo) || d < 1 || d > days[mo as usize - 1] || h > 23 || mi > 59 {
        return None;
    }
    Some(DataOra { y, mo, d, h, mi })
}

fn xml_escape(v: &str) -> String {
    v.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Mette `value` dentro <tag…>…</tag>; se il tag manca lo aggiunge in fondo alle proprietà.
fn set_xml(xml: &str, tag: &str, attrs: &str, value: &str) -> String {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    if let Some(a) = xml.find(&open) {
        let after = &xml[a..];
        if let Some(gt) = after.find('>') {
            if after[..gt].ends_with('/') {
                // <tag/> vuoto
                return format!("{}<{}{}>{}{}{}", &xml[..a], tag, attrs, value, close, &after[gt + 1..]);
            }
            if let Some(c) = after.find(&close) {
                return format!("{}{}{}", &xml[..a + gt + 1], value, &after[c..]);
            }
        }
    }
    match xml.rfind("</cp:coreProperties>") {
        Some(e) => format!("{}<{}{}>{}{}{}", &xml[..e], tag, attrs, value, close, &xml[e..]),
        None => xml.to_string(),
    }
}

/// Copia un file Office (.docx, .xlsx, .pptx) in `out` e ne modifica docProps/core.xml,
/// dove stanno autore e date.
fn ooxml_core(input: &Path, out: &Path, edit: impl Fn(&str) -> String) -> Result<(), String> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if !["docx", "xlsx", "pptx"].contains(&ext.as_str()) {
        return Err("Per ora funziona solo con i file .docx e .xlsx.".into());
    }
    let xml = run_cmd("7z", &["x", "-so", &s(input), "docProps/core.xml"])?;
    if !xml.contains("</cp:coreProperties>") {
        return Err("Il file non contiene le proprietà del documento.".into());
    }
    let tmp = std::env::temp_dir().join(format!("halo-ooxml-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let r = (|| {
        fs::create_dir_all(tmp.join("docProps")).map_err(|e| e.to_string())?;
        fs::write(tmp.join("docProps/core.xml"), edit(&xml)).map_err(|e| e.to_string())?;
        fs::copy(input, out).map_err(|e| e.to_string())?;
        // Percorso relativo dalla cartella temporanea: nell'archivio resta "docProps/core.xml".
        let o = crate::sistema::cmd("7z").args(["u", "-tzip", "-bd", &s(out), "docProps/core.xml"])
            .current_dir(&tmp).output().map_err(|e| e.to_string())?;
        if o.status.success() { Ok(()) } else { Err("7z non è riuscito ad aggiornare il file.".to_string()) }
    })();
    let _ = fs::remove_dir_all(&tmp);
    if r.is_err() { let _ = fs::remove_file(out); }
    r
}

/// Valore attuale da proporre nel pannello (autore, data, tag), se si riesce a leggerlo.
pub fn current_value(input: &Path, key: &str) -> Option<String> {
    let i = s(input);
    let v = if key.ends_with("/cambia-autore") {
        let tag = if key.starts_with("pdf") { "-Author" } else { "-Creator" };
        run_cmd("exiftool", &["-s3", tag, &i]).ok()?
    } else if key.ends_with("/cambia-data") {
        run_cmd("exiftool", &["-s3", "-d", "%d/%m/%Y %H:%M", "-DateTimeOriginal", "-CreateDate", &i]).ok()?
            .lines().next().unwrap_or("").to_string()
    } else if key == "pdf/pagine/riordina" {
        format!("1-{}", crate::rapide::page_count(input).ok()?)
    } else if key == "image/ridimensiona/su-misura" {
        run_cmd("identify", &["-format", "%wx%h", &format!("{}[0]", i)]).ok()?
    } else if let Some(t) = key.strip_prefix("audio/tag/") {
        let field = match t { "titolo" => "title", "artista" => "artist", _ => "album" };
        run_cmd("ffprobe", &["-v", "error", "-show_entries", &format!("format_tags={}", field), "-of", "default=nw=1:nk=1", &i]).ok()?
    } else {
        return None;
    };
    let v = v.trim().to_string();
    if v.is_empty() { None } else { Some(v) }
}

/// Ricodifica video di alta qualità, usata dove serve un taglio preciso al fotogramma.
pub(crate) const VIDEO_HQ: &[&str] = &["-c:v", "libx264", "-crf", "18", "-preset", "medium", "-pix_fmt", "yuv420p", "-c:a", "aac", "-b:a", "192k"];

/// Estensione del video tagliato: H.264 va bene in mp4, mov, mkv; il resto diventa mp4.
pub(crate) fn video_ext(ext: &str) -> &str {
    match ext { "mp4" | "mov" | "mkv" | "m4v" => ext, _ => "mp4" }
}

/// Come ff_pre, ma verso un file d'uscita già scelto.
pub(crate) fn ff_to(input: &Path, out: &Path, pre: &[&str], ops: &[&str]) -> Result<String, String> {
    let i = s(input);
    let o = s(out);
    let mut args: Vec<&str> = vec!["-hide_banner", "-nostdin", "-n"];
    args.extend_from_slice(pre);
    args.extend_from_slice(&["-i", &i]);
    args.extend_from_slice(ops);
    args.push(&o);
    run_cmd("ffmpeg", &args)?;
    done(out)
}

/// "1200" → "1200x" (larghezza, proporzioni mantenute), "1200x800" o "1200 × 800" → dentro
/// quel riquadro, "x800" → altezza, "50%" → percentuale. Restituisce la geometria per ImageMagick.
pub fn parse_size(v: &str) -> Option<String> {
    let v: String = v.chars().filter(|c| !c.is_whitespace()).collect::<String>().to_lowercase().replace('×', "x");
    let px = |x: &str| x.parse::<u32>().ok().filter(|n| (1..=20_000).contains(n));
    if let Some(p) = v.strip_suffix('%') {
        let n: f64 = p.replace(',', ".").parse().ok()?;
        return (n > 0.0 && n <= 1000.0).then(|| format!("{}%", n));
    }
    match v.split_once('x') {
        None => px(&v).map(|w| format!("{}x", w)),
        Some(("", h)) => px(h).map(|h| format!("x{}", h)),
        Some((w, "")) => px(w).map(|w| format!("{}x", w)),
        Some((w, h)) => Some(format!("{}x{}", px(w)?, px(h)?)),
    }
}

/// "2", "0,5", "1.5x", "×2" → fattore di velocità tra 0,25 e 4 (diverso da 1).
pub fn parse_speed(v: &str) -> Option<f64> {
    let v = v.trim().trim_start_matches(['x', '×']).trim_end_matches(['x', '×']).replace(',', ".");
    let f: f64 = v.trim().parse().ok()?;
    (f >= 0.25 && f <= 4.0 && (f - 1.0).abs() > 1e-9).then_some(f)
}

/// Filtro audio per cambiare velocità: atempo accetta 0,5–2, oltre si concatena.
fn atempo(f: f64) -> String {
    if (0.5..=2.0).contains(&f) { format!("atempo={}", f) } else { let r = f.sqrt(); format!("atempo={r},atempo={r}") }
}

pub(crate) fn has_audio(input: &Path) -> bool {
    run_cmd("ffprobe", &["-v", "error", "-select_streams", "a", "-show_entries", "stream=index", "-of", "csv=p=0", &s(input)])
        .map(|o| !o.trim().is_empty()).unwrap_or(false)
}

/// Durata in secondi di un file audio o video.
pub fn duration(input: &Path) -> Result<f64, String> {
    let d = run_cmd("ffprobe", &["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", &s(input)])?;
    d.trim().parse().map_err(|_| "Durata del file non leggibile.".to_string())
}

/// "90", "1:30", "1:02:30", anche con decimali ("1:30,5") → secondi.
pub fn parse_time(v: &str) -> Option<f64> {
    let v = v.trim().replace(',', ".");
    if v.is_empty() { return None; }
    let parts: Vec<&str> = v.split(':').collect();
    if parts.len() > 3 { return None; }
    let mut total = 0.0;
    for (n, p) in parts.iter().enumerate() {
        let x: f64 = p.parse().ok()?;
        let last = n == parts.len() - 1;
        if x < 0.0 || !x.is_finite() || (!last && x.fract() != 0.0) || (n > 0 && x >= 60.0) { return None; }
        total = total * 60.0 + x;
    }
    Some(total)
}

/// Secondi → "3:42" oppure "1:02:30".
pub fn fmt_time(sec: f64) -> String {
    let t = sec.floor() as u64;
    let (h, m, s) = (t / 3600, (t / 60) % 60, t % 60);
    if h > 0 { format!("{}:{:02}:{:02}", h, m, s) } else { format!("{}:{:02}", m, s) }
}

/// Dissolvenza in entrata e in uscita di 2 secondi (serve la durata).
fn fade_filter(input: &Path) -> Result<String, String> {
    let dur = duration(input)?;
    let fade = if dur < 6.0 { dur / 3.0 } else { 2.0 };
    Ok(format!("afade=t=in:st=0:d={:.2},afade=t=out:st={:.2}:d={:.2}", fade, (dur - fade).max(0.0), fade))
}

/// LibreOffice sovrascrive senza chiedere: converte in una cartella temporanea
/// e poi sposta il risultato su un nome libero.
/// Conversione con LibreOffice per gli altri moduli (es. presentazioni).
pub(crate) fn convert_office(input: &Path, target: &str) -> Result<String, String> {
    soffice(input, target, None).map_err(impress_hint)
}

/// Le presentazioni le apre LibreOffice Impress: se manca, LibreOffice dice solo «could not be loaded».
pub(crate) fn impress_hint(e: String) -> String {
    if e.contains("could not be loaded") { "Manca LibreOffice Impress. Installa con: sudo apt install libreoffice-impress".into() } else { e }
}

fn soffice(input: &Path, target: &str, infilter: Option<&str>) -> Result<String, String> {
    let tmp = std::env::temp_dir().join(format!("halo-lo-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let mut args: Vec<String> = vec!["--headless".into()];
    if let Some(f) = infilter { args.push(format!("--infilter={}", f)); }
    args.extend(["--convert-to".into(), target.into(), "--outdir".into(), s(&tmp), s(input)]);
    let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
    run_cmd("soffice", &refs)?;
    let ext = target.split(':').next().unwrap_or(target);
    let produced = tmp.join(format!("{}.{}", stem(input), ext));
    let out = out_path(input, "", ext);
    let moved = fs::rename(&produced, &out).or_else(|_| fs::copy(&produced, &out).map(|_| ()));
    let _ = fs::remove_dir_all(&tmp);
    moved.map_err(|_| "LibreOffice non ha prodotto il file atteso.".to_string())?;
    done(&out)
}

fn pandoc(input: &Path, ext: &str, opts: &[&str]) -> Result<String, String> {
    let out = out_path(input, "", ext);
    let i = s(input);
    let o = s(&out);
    let mut args: Vec<&str> = vec![&i];
    args.extend_from_slice(opts);
    args.push("-o");
    args.push(&o);
    run_cmd("pandoc", &args)?;
    done(&out)
}

fn short(s: &str, n: usize) -> String {
    if s.chars().count() <= n { s.to_string() } else { s.chars().take(n).collect::<String>() + "…" }
}

/// Conta gli a capo: (CRLF, solo LF, solo CR).
fn line_ends(b: &[u8]) -> (usize, usize, usize) {
    let (mut crlf, mut lf, mut cr) = (0, 0, 0);
    let mut k = 0;
    while k < b.len() {
        match b[k] {
            b'\r' if b.get(k + 1) == Some(&b'\n') => { crlf += 1; k += 1; }
            b'\r' => cr += 1,
            b'\n' => lf += 1,
            _ => {}
        }
        k += 1;
    }
    (crlf, lf, cr)
}

#[derive(Debug, PartialEq)]
enum Enc { Utf8, Utf8Bom, Utf16(bool), Win1252 }

/// Riconosce la codifica: BOM, UTF-8 valido, altrimenti Windows-1252 (la più comune nei vecchi file italiani).
fn detect(b: &[u8]) -> Enc {
    if b.starts_with(&[0xEF, 0xBB, 0xBF]) { Enc::Utf8Bom }
    else if b.starts_with(&[0xFF, 0xFE]) { Enc::Utf16(true) }
    else if b.starts_with(&[0xFE, 0xFF]) { Enc::Utf16(false) }
    else if std::str::from_utf8(b).is_ok() { Enc::Utf8 }
    else { Enc::Win1252 }
}

fn encoding_name(b: &[u8]) -> &'static str {
    match detect(b) {
        Enc::Utf8 if b.is_ascii() => "ASCII (compatibile con UTF-8)",
        Enc::Utf8 => "UTF-8",
        Enc::Utf8Bom => "UTF-8 con BOM",
        Enc::Utf16(_) => "UTF-16",
        Enc::Win1252 => "probabilmente Windows-1252 / ISO-8859-1",
    }
}

fn decode_utf16(b: &[u8], le: bool) -> String {
    let units: Vec<u16> = b.chunks_exact(2).map(|c| if le { u16::from_le_bytes([c[0], c[1]]) } else { u16::from_be_bytes([c[0], c[1]]) }).collect();
    String::from_utf16_lossy(&units)
}

/// Un byte Windows-1252 → carattere (0x80–0x9F hanno simboli propri, il resto coincide con Latin-1).
fn cp1252(c: u8) -> char {
    const HI: [u16; 32] = [0x20AC, 0xFFFD, 0x201A, 0x0192, 0x201E, 0x2026, 0x2020, 0x2021, 0x02C6, 0x2030, 0x0160, 0x2039, 0x0152, 0xFFFD, 0x017D, 0xFFFD,
        0xFFFD, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2013, 0x2014, 0x02DC, 0x2122, 0x0161, 0x203A, 0x0153, 0xFFFD, 0x017E, 0x0178];
    if (0x80..0xA0).contains(&c) { char::from_u32(HI[(c - 0x80) as usize] as u32).unwrap_or('\u{FFFD}') } else { c as char }
}

/// Legge un testo da riscrivere: deve essere UTF-8, altrimenti i caratteri accentati
/// del file creato verrebbero rovinati.
fn read_utf8(input: &Path) -> Result<String, String> {
    let b = fs::read(input).map_err(|e| e.to_string())?;
    match detect(&b) {
        Enc::Utf8 => Ok(String::from_utf8(b).unwrap_or_default()),
        Enc::Utf8Bom => Ok(String::from_utf8_lossy(&b[3..]).into_owned()),
        _ => Err("Il file non è in UTF-8: convertilo prima con Codifica › UTF-8.".into()),
    }
}

fn read_text(input: &Path) -> Result<String, String> {
    let bytes = fs::read(input).map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn transform_text(input: &Path, suffix: &str, f: impl Fn(&str) -> String) -> Result<String, String> {
    let t = read_utf8(input)?;
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("txt").to_string();
    let out = out_path(input, suffix, &ext);
    let mut result = f(&t);
    if t.ends_with('\n') && !result.ends_with('\n') { result.push('\n'); }
    fs::write(&out, result).map_err(|e| e.to_string())?;
    done(&out)
}

pub(crate) fn human(b: u64) -> String {
    if b >= 1_000_000 { format!("{:.1} MB", b as f64 / 1_000_000.0).replace('.', ",") }
    else { format!("{} KB", (b / 1000).max(1)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomi_senza_sovrascrivere() {
        let dir = std::env::temp_dir().join("halo-test-nomi");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("note.txt");
        fs::write(&f, "b\na\na\n").unwrap();
        assert_eq!(out_path(&f, "_x", "txt"), dir.join("note_x.txt"));
        fs::write(dir.join("note_x.txt"), "").unwrap();
        assert_eq!(out_path(&f, "_x", "txt"), dir.join("note_x (2).txt"));
        assert_eq!(out_path(&f, "", "txt"), dir.join("note (2).txt"));
    }

    #[test]
    fn azioni_di_testo() {
        let dir = std::env::temp_dir().join("halo-test-testo");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("lista.txt");
        fs::write(&f, "pera\nmela  verde\npera\n").unwrap();
        assert!(run(&s(&f), "text/cerca/righe-doppie").is_ok());
        assert_eq!(fs::read_to_string(dir.join("lista_senza_doppie.txt")).unwrap(), "pera\nmela  verde\n");
        assert!(run(&s(&f), "text/formatta/togli-spazi").is_ok());
        assert_eq!(fs::read_to_string(dir.join("lista_pulito.txt")).unwrap(), "pera\nmela verde\npera\n");
        assert_eq!(run(&s(&f), "text/cerca/conta-parole").unwrap(), "4 parole · 3 righe · 22 caratteri");
        assert_eq!(fs::read_to_string(&f).unwrap(), "pera\nmela  verde\npera\n", "l'originale non va toccato");
        assert!(run(&s(&f), "text/scrittura/riassumi").is_err());
    }

    #[test]
    fn testo_e_codifiche() {
        let dir = std::env::temp_dir().join("halo-test-codifiche");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // "Però costa 5€" in Windows-1252, con a capo CRLF.
        let f = dir.join("vecchio.txt");
        fs::write(&f, b"Per\xf2 costa 5\x80\r\nRiga due\r\n").unwrap();
        assert_eq!(run(&s(&f), "text/codifica/rileva").unwrap(), "Codifica: probabilmente Windows-1252 / ISO-8859-1\nFine riga: CRLF (Windows)");
        assert!(run(&s(&f), "text/codifica/utf-8").is_ok());
        assert_eq!(fs::read_to_string(dir.join("vecchio_utf8.txt")).unwrap(), "Però costa 5€\r\nRiga due\r\n");
        assert!(run(&s(&f), "text/codifica/fine-riga-lf").is_ok());
        assert_eq!(fs::read(dir.join("vecchio_lf.txt")).unwrap(), b"Per\xf2 costa 5\x80\nRiga due\n");
        assert_eq!(run(&s(&f), "text/codifica/fine-riga-crlf"), Err("Il file usa già CRLF.".into()));
        let u = dir.join("vecchio_utf8.txt");
        assert_eq!(run(&s(&u), "text/codifica/utf-8"), Err("Il file è già in UTF-8.".into()));

        let d = |k: &str, v: &str, s2: &str| HashMap::from([("valore".to_string(), k.to_string()), ("sostituto".to_string(), s2.to_string()), ("x".to_string(), v.to_string())]);
        assert_eq!(run_with(&s(&u), "text/cerca/trova", &d("riga", "", "")).unwrap(), "1 risultato:\nriga 2: Riga due");
        assert_eq!(run_with(&s(&u), "text/cerca/sostituisci", &d("Riga", "", "Linea")).unwrap(), "Creato: vecchio_utf8_sostituito.txt (1 sostituzione)");
        assert_eq!(fs::read_to_string(dir.join("vecchio_utf8_sostituito.txt")).unwrap(), "Però costa 5€\r\nLinea due\r\n");
        assert!(run_with(&s(&u), "text/cerca/sostituisci", &d("assente", "", "x")).is_err());
        assert_eq!(run_with(&s(&f), "text/cerca/sostituisci", &d("Riga", "", "Linea")),
            Err("Il file non è in UTF-8: convertilo prima con Codifica › UTF-8.".into()));
        assert!(run(&s(&f), "text/formatta/maiuscole").is_err());
        assert_eq!(fs::read(&f).unwrap(), b"Per\xf2 costa 5\x80\r\nRiga due\r\n", "l'originale non va toccato");
    }

    #[test]
    fn chiavi_coerenti() {
        for k in crate::cartelle::KEYS.iter().chain(crate::oscura::KEYS).chain(crate::annota::KEYS).chain(crate::rapide::KEYS).chain(crate::scegli::KEYS).chain(crate::documenti::KEYS).chain(crate::fogli::KEYS).chain(crate::archivi::KEYS).chain(crate::condividi::KEYS).chain(crate::editor::KEYS).chain(crate::word::KEYS).chain(crate::traduci::KEYS).chain(crate::ingrandisci::KEYS).chain(crate::altri::KEYS).chain(crate::ocr::KEYS).chain(crate::qr::KEYS).chain(crate::sottotitoli::KEYS) { assert!(SUPPORTED.contains(k), "{} manca in SUPPORTED", k); }
        for (k, _) in FORMS { assert!(SUPPORTED.contains(k), "{} manca in SUPPORTED", k); }
    }

    /// Opzioni che sul file scelto non avrebbero effetto: rifiutano con una spiegazione, senza file.
    #[test]
    #[ignore]
    fn controlli_di_senso() {
        let dir = std::env::temp_dir().join("halo-test-senso");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let png = dir.join("logo.png");
        let jpg = dir.join("foto.jpg");
        run_cmd("convert", &["-size", "200x100", "xc:none", "-fill", "red", "-draw", "circle 100,50 100,10", &s(&png)]).unwrap();
        run_cmd("convert", &["-size", "200x100", "plasma:fractal", &s(&jpg)]).unwrap();
        let no = |p: &Path, k: &str| run(&s(p), k).unwrap_err();
        assert_eq!(no(&png, "image/converti/png"), "Il file è già in formato PNG.");
        assert!(no(&jpg, "image/sfondo/bianco").starts_with("L'immagine non ha parti trasparenti"));
        assert!(run(&s(&png), "image/sfondo/bianco").unwrap().contains("parti trasparenti riempite di bianco"));
        assert!(no(&jpg, "image/metadati/rimuovi-posizione").starts_with("La foto non contiene la posizione"));
        // Automatico: la foto senza bordi resta uguale (rifiuto); con una cornice bianca la toglie.
        assert!(no(&jpg, "image/ritaglia/automatico").starts_with("Nessun bordo uniforme"));
        let framed = dir.join("cornice.jpg");
        run_cmd("convert", &[&s(&jpg), "-bordercolor", "white", "-border", "30", &s(&framed)]).unwrap();
        assert!(run(&s(&framed), "image/ritaglia/automatico").unwrap().contains("tolti i bordi"));
        // Foto con orientamento nei metadati (ruotata di 90°): «Per il web» la raddrizza prima di togliere i metadati.
        let tel = dir.join("telefono.jpg");
        run_cmd("convert", &["-size", "400x200", "plasma:fractal", &s(&tel)]).unwrap();
        run_cmd("exiftool", &["-q", "-overwrite_original", "-Orientation#=6", &s(&tel)]).unwrap();
        run(&s(&tel), "image/ridimensiona/per-il-web").unwrap();
        assert_eq!(run_cmd("identify", &["-format", "%wx%h", &s(&dir.join("telefono_web.jpg"))]).unwrap(), "200x400");
        let cartella = dir.join("senza doppioni");
        fs::create_dir_all(&cartella).unwrap();
        fs::write(cartella.join("a.txt"), "a").unwrap();
        assert!(no(&cartella, "folder/pulisci/duplicati").starts_with("Niente da fare"));
        assert!(!dir.join("senza doppioni_pulita").exists());
        assert!(run(&s(&cartella), "folder/analizza/dimensione").unwrap().ends_with("in 1 file"));
    }

    #[test]
    fn salva_sulla_scrivania_se_la_cartella_non_si_scrive() {
        // /usr/share non è scrivibile per l'utente: il risultato va sulla Scrivania.
        let out = out_path(Path::new("/usr/share/common-licenses/GPL-3"), "_copia", "txt");
        assert_eq!(out.parent().unwrap(), desktop_dir());
        let tmp = std::env::temp_dir().join("x.pdf");
        assert_eq!(out_path(&tmp, "_a", "pdf").parent().unwrap(), std::env::temp_dir());
    }

    #[test]
    fn riconosce_i_tipi() {
        assert_eq!(kind_of(Path::new("/x/a.PDF")), "pdf");
        assert_eq!(kind_of(Path::new("/x/a.jpeg")), "image");
        assert_eq!(kind_of(Path::new("/x/a.mp3")), "audio");
        assert_eq!(kind_of(Path::new("/x/a.tar.gz")), "archive");
        assert_eq!(kind_of(Path::new("/x/a.xyz")), "other");
    }

    // ---------------- prova completa con i programmi veri ----------------
    // Esegue ogni chiave di SUPPORTED su una copia di un file d'esempio in /tmp/halo-azioni
    // e stampa la tabella "chiave → esito → file creato". Richiede i programmi del README:
    //   cargo test tutte_le_azioni -- --ignored --nocapture

    const ROOT: &str = "/tmp/halo-azioni";

    fn sh(cmd: &str, args: &[&str]) {
        if let Err(e) = run_cmd(cmd, args) { panic!("campione non creato: {}", e); }
    }

    /// Crea un file d'esempio per ogni tipo in ROOT/campioni.
    fn campioni() -> PathBuf {
        let c = Path::new(ROOT).join("campioni");
        let _ = fs::remove_dir_all(ROOT);
        fs::create_dir_all(&c).unwrap();
        let p = |n: &str| s(&c.join(n));

        fs::write(c.join("note.txt"), "Riga uno\nriga  due   con spazi \nRiga uno\nscrivi a test@example.com\n").unwrap();

        let ps = c.join("doc.ps");
        fs::write(&ps, "%!PS\n[ /Title (Prova Halo) /Author (Mario Rossi) /DOCINFO pdfmark\n\
            /Helvetica findfont 24 scalefont setfont\n72 720 moveto (Pagina uno) show showpage\n\
            72 720 moveto (Pagina due) show showpage\n").unwrap();
        sh("gs", &["-q", "-dBATCH", "-dNOPAUSE", "-sDEVICE=pdfwrite", &format!("-sOutputFile={}", p("documento.pdf")), &s(&ps)]);
        fs::remove_file(&ps).unwrap();

        sh("convert", &["-size", "640x480", "gradient:orange-purple", "-bordercolor", "white", "-border", "40",
            "-fill", "white", "-draw", "circle 360,280 420,280", &p("foto.jpg")]);
        sh("exiftool", &["-q", "-overwrite_original", "-Make=Prova", "-Model=Halo", "-DateTimeOriginal=2026:05:01 10:00:00",
            "-GPSLatitude=41.9", "-GPSLatitudeRef=N", "-GPSLongitude=12.5", "-GPSLongitudeRef=E", &p("foto.jpg")]);

        sh("ffmpeg", &["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i",
            "aevalsrc='if(between(t,2,4),0,0.5*sin(2*PI*440*t))':s=44100:d=8", "-c:a", "libmp3lame", "-b:a", "192k", &p("brano.mp3")]);
        sh("ffmpeg", &["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=640x360:rate=25:d=4",
            "-f", "lavfi", "-i", "sine=f=440:d=4", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", "-shortest", &p("video.mp4")]);

        fs::write(c.join("relazione.md"), "---\ntitle: Relazione\nauthor: Mario Rossi\n---\n\n# Titolo\n\nTesto con **grassetto** e un elenco:\n\n- uno\n- due\n").unwrap();
        sh("pandoc", &[&p("relazione.md"), "-o", &p("relazione.docx")]);
        fs::remove_file(c.join("relazione.md")).unwrap();

        fs::write(c.join("bilancio.csv"), "Voce,Importo\nAffitto,800\nSpesa,350\n").unwrap();
        sh("soffice", &["--headless", "--convert-to", "xlsx", "--outdir", &s(&c), &p("bilancio.csv")]);
        fs::remove_file(c.join("bilancio.csv")).unwrap();

        let dir = c.join("cartella");
        fs::create_dir_all(dir.join("sotto")).unwrap();
        fs::write(dir.join("a.txt"), "uno\n").unwrap();
        fs::write(dir.join("sotto/b.txt"), "due\n").unwrap();
        fs::write(dir.join("vuoto.log"), "").unwrap();
        sh("7z", &["a", "-tzip", "-bd", &p("archivio.zip"), &s(&dir)]);

        // Presentazione (con una foto, per «Comprimi › Immagini interne») e un file di tipo sconosciuto.
        fs::write(c.join("slide.md"), "---\ntitle: Prova\nauthor: Mario Rossi\n---\n\n# Uno\n\nTesto\n\n# Due\n\n![](foto.jpg)\n").unwrap();
        let o = crate::sistema::cmd("pandoc").args(["slide.md", "-o", "presentazione.pptx"]).current_dir(&c).output().unwrap();
        assert!(o.status.success());
        fs::remove_file(c.join("slide.md")).unwrap();
        fs::write(c.join("progetto.blend"), "dati di prova\n").unwrap();

        // Campioni già protetti, per le opzioni "Rimuovi password".
        let (up, op) = (format!("--user-password={}", PW), format!("--owner-password={}", PW));
        sh("qpdf", &["--encrypt", &up, &op, "--bits=256", "--", &p("documento.pdf"), &p("protetto.pdf")]);
        sh("7z", &["a", "-tzip", "-bd", "-mem=AES256", &format!("-p{}", PW), &p("protetto.zip"), &s(&dir)]);
        c
    }

    fn campione_per(key: &str) -> &'static str {
        if key == "pdf/proteggi/rimuovi-password" { return "protetto.pdf"; }
        if key == "archive/proteggi/rimuovi-password" { return "protetto.zip"; }
        match key.split('/').next().unwrap() {
            "pdf" => "documento.pdf", "image" => "foto.jpg", "audio" => "brano.mp3", "video" => "video.mp4",
            "document" => "relazione.docx", "sheet" => "bilancio.xlsx", "folder" => "cartella",
            "archive" => "archivio.zip", "text" => "note.txt", "slides" => "presentazione.pptx", "other" => "progetto.blend",
            _ => unreachable!(),
        }
    }

    /// Contenuto completo di un file o di una cartella (percorso relativo e byte), per confrontarlo.
    fn impronta(p: &Path) -> Vec<(String, Vec<u8>)> {
        let mut v = Vec::new();
        fn walk(base: &Path, p: &Path, v: &mut Vec<(String, Vec<u8>)>) {
            if p.is_dir() {
                let mut e: Vec<_> = fs::read_dir(p).unwrap().map(|e| e.unwrap().path()).collect();
                e.sort();
                v.push((s(p.strip_prefix(base).unwrap()) + "/", Vec::new()));
                for x in e { walk(base, &x, v); }
            } else {
                v.push((s(p.strip_prefix(base).unwrap()), fs::read(p).unwrap()));
            }
        }
        walk(p.parent().unwrap(), p, &mut v);
        v
    }

    fn descrivi(p: &Path) -> String {
        if p.is_dir() {
            let n = fs::read_dir(p).map(|d| d.count()).unwrap_or(0);
            format!("{}/ ({} elementi)", name(p), n)
        } else {
            format!("{} ({})", name(p), human(fs::metadata(p).map(|m| m.len()).unwrap_or(0)))
        }
    }

    #[test]
    #[ignore]
    fn tutte_le_azioni() {
        let c = campioni();
        let mut falliti = Vec::new();
        println!("\n| chiave | esito | file creato |\n|---|---|---|");
        for key in SUPPORTED {
            // Non tocchiamo gli appunti di chi lancia i test.
            // Condividi di un file singolo apre il pannello delle app: si prova in condividi::tests.
            if key.contains("/condividi/") && !key.starts_with("folder/") { println!("| `{}` | non provata in automatico (pannello delle app) | — |", key); continue; }
            if *key == "folder/condividi/copia-percorso" { println!("| `{}` | non provata in automatico (appunti) | — |", key); continue; }
            // Il link carica la cartella online: mai durante i test.
            // Traduci apre il browser: mai durante i test (il prompt è provato in traduci::tests).
            // Ingrandisci usa la scheda video: provato a parte in ingrandisci::tests::file_veri.
            if *key == "image/ridimensiona/upscayl" { println!("| `{}` | non provata in automatico (scheda video) | — |", key); continue; }
            // I sottotitoli servono whisper.cpp e il modello: provati a parte in sottotitoli::tests::file_veri.
            if key.starts_with("video/sottotitoli/") { println!("| `{}` | non provata in automatico (whisper.cpp) | — |", key); continue; }
            if key.ends_with("/traduci") { println!("| `{}` | non provata in automatico (apre il browser) | — |", key); continue; }
            if *key == "folder/condividi/link" { println!("| `{}` | non provata in automatico (caricamento online) | — |", key); continue; }
            let work = Path::new(ROOT).join(key.replace('/', "__"));
            fs::create_dir_all(&work).unwrap();
            let src = c.join(campione_per(key));
            let input = work.join(campione_per(key));
            sh("cp", &["-r", &s(&src), &s(&input)]);
            let prima = impronta(&input);

            let valore = match *key {
                k if k.ends_with("cambia-data") => "01/02/2025 10:30",
                "image/ridimensiona/su-misura" => "320x200",
                "video/fotogrammi/velocita" => "2",
                "archive/ricomprimi/dividi-in-parti" => "1",
                "text/cerca/sostituisci" => "Riga",
                "pdf/annota/evidenzia" => "Pagina",
                "pdf/pagine/riordina" => "2, 1",
                "pdf/pagine/estrai" => "2",
                "video/fotogrammi/estrai-foto" => "1",
                "archive/anteprima/cerca" => "a",
                _ => "Nome Prova",
            };
            let r = run_with(&s(&input), key, &HashMap::from([
                ("password".to_string(), PW.to_string()), ("orario".to_string(), "0:03".to_string()),
                ("valore".to_string(), valore.to_string()), ("sostituto".to_string(), "Linea".to_string())]));

            assert!(impronta(&input) == prima, "{}: l'originale è stato modificato", key);
            let mut nuovi: Vec<String> = fs::read_dir(&work).unwrap().map(|e| e.unwrap().path())
                .filter(|p| p != &input).map(|p| descrivi(&p)).collect();
            nuovi.sort();
            // Il campione è già UTF-8 con a capo LF: queste due devono rifiutare senza creare file
            // (la conversione vera è provata in testo_e_codifiche).
            // Lo stesso per Oscura dati sui campioni senza dati (provata con file veri in oscura::tests::file_veri).
            // Le opzioni con «Apri file» qui non hanno file scelti (provate in scegli::tests::file_veri);
            // HEIC richiede libheif-plugin-x265, che può mancare.
            if matches!(*key, "text/codifica/utf-8" | "text/codifica/fine-riga-lf") || (key.contains("/oscura-dati/")
                && r.as_ref().is_err_and(|e| e.starts_with("Nessun dato trovato")))
                || crate::scegli::KEYS.contains(key) || crate::archivi::KEYS.contains(key) || crate::editor::KEYS.contains(key) || *key == "document/proteggi/firma"
                || (*key == "image/converti/heic" && r.as_ref().is_err_and(|e| e.starts_with("Manca il codificatore HEIC")))
                || key.starts_with("document/comprimi/") || key.ends_with("/confronta")
                || r.as_ref().is_err_and(|e| e.starts_with("Il file è già in formato"))
                // OCR: il PDF d'esempio ha già il testo e la foto non ne ha (provato in ocr::tests::file_veri).
                || (crate::ocr::KEYS.contains(key) && r.as_ref().is_err_and(|e| e.starts_with("Il PDF ha già il testo") || e.starts_with("Nessun testo")))
                // Il campione è una foto senza trasparenza e la cartella non ha doppioni, temporanei o maiuscole:
                // rifiutano senza creare file (provate con file adatti in controlli_di_senso).
                || (*key == "image/sfondo/bianco" && r.as_ref().is_err_and(|e| e.starts_with("L'immagine non ha parti trasparenti")))
                || (key.starts_with("folder/") && r.as_ref().is_err_and(|e| e.starts_with("Niente da fare")))
                || (crate::fogli::KEYS.contains(key) && r.as_ref().is_err_and(|e| e.starts_with("Nessun") || e.starts_with("Non trovo") || e.starts_with("Il foglio non contiene"))) {
                // (il .docx d'esempio non ha foto: la compressione è provata in documenti::tests::file_veri)
                assert!(r.as_ref().is_err_and(|e| e.contains("già") || e.starts_with("Nessun dato") || e == "Scegli prima i file." || e.starts_with("Disegna prima") || e.starts_with("Clicca sullo sfondo") || e.starts_with("Scegli almeno")
                    || e.starts_with("Manca il codificatore") || e.contains("non contiene foto") || e.starts_with("Nessun")
                    || e.starts_with("Non trovo") || e.starts_with("Il foglio non contiene") || e.contains("già in formato") || e.starts_with("Il PDF ha già il testo")
                    || e.starts_with("L'immagine non ha parti trasparenti") || e.starts_with("Niente da fare")) && nuovi.is_empty(), "{}: {:?}", key, r);
                println!("| `{}` | rifiuto atteso: {} | — |", key, r.unwrap_err());
                continue;
            }
            let esito = match &r {
                Ok(m) => format!("OK: {}", m.lines().next().unwrap_or("").replace('|', "/")),
                Err(e) => { falliti.push(key.to_string()); format!("**ERRORE**: {}", e.replace('|', "/")) }
            };
            if r.is_ok() && m_crea_file(r.as_ref().unwrap()) && nuovi.is_empty() {
                falliti.push(format!("{} (nessun file)", key));
            }
            println!("| `{}` | {} | {} |", key, esito, if nuovi.is_empty() { "—".into() } else { nuovi.join(", ") });
        }
        assert!(falliti.is_empty(), "azioni fallite: {:?}", falliti);

        // Controlli sul contenuto dove "è stato creato un file" non basta.
        let r = Path::new(ROOT);
        let pdf = fs::read(r.join("pdf__metadati__rimuovi-tutti/documento_senza_metadati.pdf")).unwrap();
        assert!(!String::from_utf8_lossy(&pdf).contains("Mario Rossi"), "il PDF contiene ancora l'autore");
        for (dir, f) in [("image__metadati__rimuovi-posizione", "foto_senza_gps.jpg"), ("image__metadati__rimuovi-tutti", "foto_senza_metadati.jpg")] {
            let gps = run_cmd("exiftool", &["-s3", "-GPSPosition", &s(&r.join(dir).join(f))]).unwrap();
            assert!(gps.trim().is_empty(), "{}: la posizione GPS è ancora presente", f);
        }
        assert!(!run(&s(&c.join("bilancio.xlsx")), "sheet/metadati/mostra").unwrap().contains("Title:"));

        // Tagli: durate attese (campioni da 8 s di audio e 4 s di video, taglio a 0:03).
        let dur = |f: &str| duration(&r.join(f)).unwrap();
        let vicino = |a: f64, b: f64| (a - b).abs() < 0.15;
        assert!(vicino(dur("audio__taglia__inizio/brano_taglio.mp3"), 5.0));
        assert!(vicino(dur("audio__taglia__fine/brano_taglio.mp3"), 3.0));
        assert!(vicino(dur("audio__taglia__dividi/brano_parte1.mp3"), 3.0));
        assert!(vicino(dur("audio__taglia__dividi/brano_parte2.mp3"), 5.0));
        assert!(vicino(dur("video__taglia__inizio/video_taglio.mp4"), 1.0));
        assert!(vicino(dur("video__taglia__fine/video_taglio.mp4"), 3.0));
        assert!(vicino(dur("video__taglia__dividi/video_parte1.mp4"), 3.0));
        assert!(vicino(dur("video__taglia__dividi/video_parte2.mp4"), 1.0));
        let fuori = run_with(&s(&c.join("brano.mp3")), "audio/taglia/fine", &HashMap::from([("orario".to_string(), "9:00".to_string())]));
        assert_eq!(fuori, Err("L'orario deve essere tra 0:00 e 0:08.".into()));

        // Metadati: il valore nuovo c'è, quello vecchio non resta nascosto nel PDF.
        let leggi = |f: &str, tag: &str| run_cmd("exiftool", &["-s3", tag, &s(&r.join(f))]).unwrap().trim().to_string();
        assert_eq!(leggi("pdf__metadati__cambia-autore/documento_autore.pdf", "-Author"), "Nome Prova");
        let byte = fs::read(r.join("pdf__metadati__cambia-autore/documento_autore.pdf")).unwrap();
        assert!(!String::from_utf8_lossy(&byte).contains("Mario Rossi"), "l'autore vecchio è rimasto nel PDF");
        assert_eq!(leggi("pdf__metadati__cambia-data/documento_data.pdf", "-CreateDate").get(..19), Some("2025:02:01 10:30:00"));
        assert_eq!(leggi("image__metadati__cambia-data/foto_data.jpg", "-DateTimeOriginal"), "2025:02:01 10:30:00");
        assert_eq!(leggi("document__metadati__cambia-autore/relazione_autore.docx", "-Creator"), "Nome Prova");
        assert_eq!(leggi("sheet__metadati__cambia-autore/bilancio_autore.xlsx", "-Creator"), "Nome Prova");
        assert_eq!(leggi("sheet__metadati__cambia-data/bilancio_data.xlsx", "-CreateDate").get(..19), Some("2025:02:01 10:30:00"));
        for (dir, k) in [("audio__tag__titolo", "titolo"), ("audio__tag__artista", "artista"), ("audio__tag__album", "album")] {
            assert_eq!(current_value(&r.join(dir).join("brano_tag.mp3"), &format!("audio/tag/{}", k)), Some("Nome Prova".into()), "{}", k);
        }
        // Dimensioni e velocità.
        let misura = |f: &str| run_cmd("identify", &["-format", "%wx%h", &s(&r.join(f))]).unwrap();
        assert_eq!(misura("image__ritaglia__1-1/foto_1x1.jpg"), "560x560");
        assert_eq!(misura("image__ritaglia__16-9/foto_16x9.jpg"), "720x405");
        assert_eq!(misura("image__ridimensiona__su-misura/foto_ridimensionata.jpg"), "257x200");
        assert!(vicino(dur("video__fotogrammi__velocita/video_velocita.mp4"), 2.0));
        assert!(r.join("archive__ricomprimi__dividi-in-parti/archivio_parti.zip.001").exists());

        // Il documento modificato si apre ancora: LibreOffice lo converte in testo.
        let docx = r.join("document__metadati__cambia-autore/relazione_autore.docx");
        assert!(run(&s(&docx), "document/converti/testo").is_ok());

        // Password: i file protetti si aprono solo con la password giusta, quelli sbloccati senza.
        let pdf = r.join("pdf__proteggi__password/documento_protetto.pdf");
        assert!(qpdf_encrypted(&pdf), "il PDF protetto non è cifrato");
        assert!(!qpdf_encrypted(&r.join("pdf__proteggi__rimuovi-password/protetto_sbloccato.pdf")));
        for f in ["folder__comprimi__zip-protetto/cartella.zip", "archive__proteggi__password/archivio_protetto.zip",
                  "archive__converti__zip-protetto/archivio_protetto.zip", "archive__proteggi__cifra-aes/archivio_cifrato.7z"] {
            let f = s(&r.join(f));
            assert_eq!(seven_in(&["t", &f], "Sbagliata\n"), Err("Password errata.".into()), "{}", f);
            assert!(seven_in(&["t", &f], &format!("{}\n", PW)).is_ok(), "{}", f);
        }
        let aperto = s(&r.join("archive__proteggi__rimuovi-password/protetto_sbloccato.zip"));
        assert!(seven_in(&["t", &aperto], "\n").is_ok(), "lo ZIP sbloccato chiede ancora la password");
        // Nel 7z cifrato anche i nomi dei file sono nascosti.
        let elenco = seven_in(&["l", &s(&r.join("archive__proteggi__cifra-aes/archivio_cifrato.7z"))], "Sbagliata\n");
        assert!(!elenco.unwrap_or_default().contains("a.txt"));
    }

    const PW: &str = "Prova-123 !";

    #[test]
    fn date_e_xml() {
        assert_eq!(parse_date("24/10/2026").map(|d| d.exif()), Some("2026:10:24 00:00:00".into()));
        assert_eq!(parse_date("29-02-2024 9:05").map(|d| d.w3c()), Some("2024-02-29T09:05:00Z".into()));
        for bad in ["", "29/02/2025", "31/04/2026", "1/13/2026", "24/10/2026 25:00", "24/10", "domani"] {
            assert_eq!(parse_date(bad), None, "{}", bad);
        }
        let x = "<cp:coreProperties><dc:creator>Vecchio</dc:creator><cp:lastModifiedBy/></cp:coreProperties>";
        assert_eq!(set_xml(x, "dc:creator", "", "Nuovo"), "<cp:coreProperties><dc:creator>Nuovo</dc:creator><cp:lastModifiedBy/></cp:coreProperties>");
        assert_eq!(set_xml(x, "cp:lastModifiedBy", "", "N"), "<cp:coreProperties><dc:creator>Vecchio</dc:creator><cp:lastModifiedBy>N</cp:lastModifiedBy></cp:coreProperties>");
        assert!(set_xml(x, "dc:title", "", "T").ends_with("<dc:title>T</dc:title></cp:coreProperties>"));
        assert_eq!(xml_escape("A & <B>"), "A &amp; &lt;B&gt;");
    }

    #[test]
    fn misure_e_velocita() {
        assert_eq!(parse_size("1200"), Some("1200x".into()));
        assert_eq!(parse_size("1200 × 800"), Some("1200x800".into()));
        assert_eq!(parse_size("x800"), Some("x800".into()));
        assert_eq!(parse_size("50%"), Some("50%".into()));
        for bad in ["", "0", "abc", "1200x0", "99999", "-5%"] { assert_eq!(parse_size(bad), None, "{}", bad); }
        assert_eq!(parse_speed("2"), Some(2.0));
        assert_eq!(parse_speed("0,5"), Some(0.5));
        assert_eq!(parse_speed("×1.5"), Some(1.5));
        for bad in ["1", "0", "5", "veloce"] { assert_eq!(parse_speed(bad), None, "{}", bad); }
        assert_eq!(atempo(4.0), "atempo=2,atempo=2");
        assert_eq!(atempo(0.5), "atempo=0.5");
    }

    #[test]
    fn orari() {
        assert_eq!(parse_time("90"), Some(90.0));
        assert_eq!(parse_time(" 1:30 "), Some(90.0));
        assert_eq!(parse_time("1:02:30"), Some(3750.0));
        assert_eq!(parse_time("0:03,5"), Some(3.5));
        for bad in ["", "a", "1:75", "1:2:3:4", "-5", "1.5:00"] { assert_eq!(parse_time(bad), None, "{}", bad); }
        assert_eq!(fmt_time(222.7), "3:42");
        assert_eq!(fmt_time(3750.0), "1:02:30");
    }

    fn dati(pw: &str) -> HashMap<String, String> {
        HashMap::from([("password".to_string(), pw.to_string())])
    }

    #[test]
    #[ignore]
    fn password_sbagliate_e_mancanti() {
        let dir = Path::new("/tmp/halo-azioni-pw").to_path_buf();
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("p.pdf");
        let ps = dir.join("p.ps");
        fs::write(&ps, "%!PS\n72 720 moveto showpage\n").unwrap();
        sh("gs", &["-q", "-dBATCH", "-dNOPAUSE", "-sDEVICE=pdfwrite", &format!("-sOutputFile={}", s(&pdf)), &s(&ps)]);
        assert_eq!(run(&s(&pdf), "pdf/proteggi/password"), Err("Serve una password.".into()));
        assert_eq!(run_with(&s(&pdf), "pdf/proteggi/password", &dati("a\nb")), Err("La password non può contenere a capo.".into()));
        assert_eq!(run_with(&s(&pdf), "pdf/proteggi/rimuovi-password", &dati("x")), Err("Questo PDF non ha una password.".into()));
        assert!(run_with(&s(&pdf), "pdf/proteggi/password", &dati("Giusta-1")).is_ok());
        let prot = dir.join("p_protetto.pdf");
        assert_eq!(run_with(&s(&prot), "pdf/proteggi/rimuovi-password", &dati("Sbagliata")), Err("Password errata.".into()));
        assert_eq!(run_with(&s(&prot), "pdf/proteggi/password", &dati("x")), Err("Questo PDF ha già una password.".into()));

        assert_eq!(run_with(&s(&dir), "folder/comprimi/zip-protetto", &dati("Però-1")).unwrap_err().split(':').next(),
            Some("Negli ZIP la password non può avere lettere accentate o simboli speciali"));
        // Nei PDF e nei 7z gli accenti vanno bene.
        let accenti = dir.join("accenti");
        fs::create_dir_all(&accenti).unwrap();
        fs::copy(&pdf, accenti.join("a.pdf")).unwrap();
        assert!(run_with(&s(&accenti.join("a.pdf")), "pdf/proteggi/password", &dati("Però-1")).is_ok());
        assert!(run_with(&s(&accenti.join("a_protetto.pdf")), "pdf/proteggi/rimuovi-password", &dati("Però-1")).is_ok());

        let zip = dir.join("z.zip");
        sh("7z", &["a", "-tzip", "-bd", "-pGiusta-1", &s(&zip), &s(&ps)]);
        assert_eq!(run_with(&s(&zip), "archive/proteggi/rimuovi-password", &dati("Sbagliata")), Err("Password errata.".into()));
        assert_eq!(run_with(&s(&zip), "archive/proteggi/password", &dati("Altra-2")),
            Err("L'archivio ha già una password: usa prima «Rimuovi password».".into()));
        let nuovi: Vec<_> = fs::read_dir(&dir).unwrap().map(|e| name(&e.unwrap().path())).collect();
        assert!(!nuovi.iter().any(|n| n.contains("sbloccato") || n.starts_with("z_")), "file creati dopo un errore: {:?}", nuovi);
    }

    /// Il messaggio dice che è stato creato o estratto qualcosa.
    fn m_crea_file(msg: &str) -> bool { msg.starts_with("Creato") || msg.starts_with("Estratto") }
}
