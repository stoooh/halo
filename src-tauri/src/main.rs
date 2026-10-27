// Halo: menu radiale per azioni rapide sui file.
// Avvio: `halo /percorso/del/file` (lo fa l'estensione di Nautilus).

mod actions;
mod annota;
mod altri;
mod archivi;
mod documenti;
mod fogli;
mod impostazioni;
mod ingrandisci;
mod lavoro;
mod multi;
mod ocr;
mod qr;
mod cartelle;
mod condividi;
mod configura;
mod editor;
mod esito;
mod word;
mod oscura;
mod rapide;
mod scegli;
mod scorciatoie;
mod sottotitoli;
mod storia;
mod traduci;

use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;

#[derive(Serialize)]
struct Target {
    path: String,
    name: String,
    kind: String,
}

/// Il file su cui aprire il menu: il primo dei file passati.
#[tauri::command]
fn target_file() -> Option<Target> {
    target_files().into_iter().next()
}

/// Tutti i file passati (selezione multipla in Nautilus): gli argomenti che non sono opzioni.
#[tauri::command]
fn target_files() -> Vec<Target> {
    // Dopo "--" sono tutti file (anche se il nome comincia con "-").
    let args: Vec<String> = std::env::args().skip(1).collect();
    let files: Vec<String> = match args.iter().position(|a| a == "--") {
        Some(k) => args[k + 1..].to_vec(),
        None => args.into_iter().filter(|a| !a.starts_with('-')).collect(),
    };
    files.into_iter().filter_map(|a| std::fs::canonicalize(Path::new(&a)).ok())
        .map(|abs| Target {
            name: abs.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
            kind: actions::kind_of(&abs).to_string(),
            path: abs.to_string_lossy().into_owned(),
        }).collect()
}

/// Azioni «Insieme» su più file (ZIP, rinomina in serie, unisci, copia percorsi).
#[tauri::command]
async fn run_multi(paths: Vec<String>, key: String, data: Option<HashMap<String, String>>) -> Result<String, String> {
    let data = data.unwrap_or_default();
    let files: Vec<std::path::PathBuf> = paths.iter().map(std::path::PathBuf::from).collect();
    lavoro::start();
    tauri::async_runtime::spawn_blocking(move || {
        let r = multi::run(&files, &key, &data);
        // In cronologia, con il file creato se il messaggio lo nomina («Creato: …»).
        if let (Ok(msg), Some(first)) = (&r, files.first()) {
            let dir = first.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            let created: Vec<String> = msg.strip_prefix("Creato: ").and_then(|m| m.split(" (").next())
                .map(|n| dir.join(n)).filter(|p| p.exists()).map(|p| p.to_string_lossy().into_owned()).into_iter().collect();
            storia::add(first, files.len(), &key, msg, &created);
        }
        r
    }).await.map_err(|e| e.to_string())?
}

/// Le azioni realmente implementate (le altre il menu le mostra come "presto").
#[tauri::command]
fn supported_actions() -> Vec<&'static str> {
    actions::SUPPORTED.to_vec()
}

/// Le opzioni che prima di eseguire aprono il pannello: [chiave, tipo di modulo].
#[tauri::command]
fn form_actions() -> Vec<(&'static str, &'static str)> {
    actions::FORMS.to_vec()
}

/// Il nome del file che un'opzione con pannello creerà.
#[tauri::command]
fn planned_output(path: String, key: String) -> Option<String> {
    let out = actions::planned_out(Path::new(&path), &key)?;
    Some(out.file_name()?.to_string_lossy().into_owned())
}

/// Valore attuale (autore, data, tag) da proporre nel pannello.
#[tauri::command]
async fn current_value(path: String, key: String) -> Option<String> {
    tauri::async_runtime::spawn_blocking(move || actions::current_value(Path::new(&path), &key)).await.ok().flatten()
}

/// Anteprima di Oscura dati: quanti dati ha trovato e quali (mascherati).
#[tauri::command]
async fn redact_preview(path: String, key: String, data: Option<HashMap<String, String>>) -> Result<(usize, String), String> {
    let data = data.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || oscura::preview(Path::new(&path), &key, &data))
        .await
        .map_err(|e| e.to_string())?
}

/// Durata in secondi di un file audio o video (per il pannello degli orari).
#[tauri::command]
async fn media_duration(path: String) -> Option<f64> {
    tauri::async_runtime::spawn_blocking(move || actions::duration(Path::new(&path)).ok()).await.ok().flatten()
}

/// Finestra «Apri file» per le opzioni che usano altri file. Restituisce i percorsi scelti.
/// Durante la scelta la finestra di Halo smette di stare sopra le altre, così il dialogo resta visibile.
#[tauri::command]
async fn pick_files(window: tauri::WebviewWindow, path: String, key: String) -> Option<Vec<String>> {
    use tauri_plugin_dialog::DialogExt;
    let kind = scegli::pick_kind(&key)?;
    let start = Path::new(&path).parent().map(|p| p.to_path_buf());
    let _ = window.set_always_on_top(false);
    let w = window.clone();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut d = w.dialog().file().set_parent(&w);
        if let Some(dir) = start { d = d.set_directory(dir); }
        let (title, filter): (&str, &[&str]) = match kind {
            "pdf" => ("Scegli i PDF da aggiungere", &["pdf"]),
            "video" => ("Scegli i video da aggiungere", &["mp4", "mov", "mkv", "webm", "avi", "m4v"]),
            "audio" => ("Scegli il nuovo audio", &["mp3", "wav", "m4a", "aac", "flac", "ogg", "opus"]),
            "immagine" => ("Scegli l'immagine", &["png", "jpg", "jpeg", "webp", "gif", "bmp", "tif", "tiff"]),
            "cartella" => ("Scegli dove estrarre", &[]),
            "unfile" => ("Scegli il file che prende il suo posto", &[]),
            _ => ("Scegli i file da aggiungere", &[]),
        };
        d = d.set_title(title);
        if !filter.is_empty() { d = d.add_filter(title, filter); }
        match kind {
            "cartella" => d.blocking_pick_folder().map(|p| vec![p]),
            "pdf" | "video" | "file" => d.blocking_pick_files(),
            _ => d.blocking_pick_file().map(|p| vec![p]),
        }
    }).await.ok().flatten();
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
    picked.map(|v| v.into_iter().filter_map(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned()).collect())
}

/// I file contenuti in un archivio, per l'elenco a scelta del pannello.
#[tauri::command]
async fn archive_entries(path: String) -> Result<Vec<(String, u64)>, String> {
    tauri::async_runtime::spawn_blocking(move || archivi::entries(Path::new(&path))).await.map_err(|e| e.to_string())?
}

/// Anteprima per le opzioni con l'editor (immagine, pagina del PDF o fotogramma del video).
#[tauri::command]
async fn editor_preview(path: String, key: String, page: usize, data: Option<HashMap<String, String>>) -> Result<editor::Anteprima, String> {
    let data = data.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || editor::preview(Path::new(&path), &key, page, &data)).await.map_err(|e| e.to_string())?
}

/// Impostazioni salvate (o quelle predefinite).
#[tauri::command]
fn get_settings() -> serde_json::Value {
    impostazioni::load()
}

/// Salva le impostazioni; restituisce quelle effettivamente salvate.
#[tauri::command]
fn save_settings(settings: serde_json::Value) -> Result<serde_json::Value, String> {
    impostazioni::save(&settings)
}

/// Le app installate, per scegliere quella di una scorciatoia.
#[tauri::command]
async fn list_apps() -> Vec<scorciatoie::AppInstallata> {
    tauri::async_runtime::spawn_blocking(scorciatoie::installed_apps).await.unwrap_or_default()
}

/// «Scegli…» un programma o un AppImage per una scorciatoia.
#[tauri::command]
async fn pick_program(window: tauri::WebviewWindow) -> Option<String> {
    use tauri_plugin_dialog::DialogExt;
    let _ = window.set_always_on_top(false);
    let w = window.clone();
    let home = std::env::var("HOME").ok();
    let picked = tauri::async_runtime::spawn_blocking(move || {
        let mut d = w.dialog().file().set_parent(&w).set_title("Scegli il programma o l'AppImage");
        if let Some(h) = home { d = d.set_directory(h); }
        d.blocking_pick_file()
    }).await.ok().flatten();
    let _ = window.set_always_on_top(true);
    let _ = window.set_focus();
    picked.and_then(|p| p.into_path().ok()).map(|p| p.to_string_lossy().into_owned())
}

/// Esegue una scorciatoia di «Le mie app» sul file.
#[tauri::command]
async fn run_shortcut(path: String, index: usize, nome: String) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || scorciatoie::run(Path::new(&path), index, &nome)).await.map_err(|e| e.to_string())?
}

/// Traduci: gli strumenti di IA e le lingue proposti nel pannello, come [id, nome].
#[tauri::command]
fn translate_tools() -> (Vec<(&'static str, &'static str)>, Vec<(&'static str, &'static str)>) {
    (traduci::STRUMENTI.iter().map(|t| (t.0, t.1)).collect(), traduci::LINGUE.iter().map(|l| (l.0, l.1)).collect())
}

/// Traduci: copia il prompt, apre lo strumento scelto e mostra il file nella cartella.
#[tauri::command]
fn translate_open(path: String, key: String, lingua: String, tool: String, formato: String) -> Result<String, String> {
    traduci::open(Path::new(&path), &key, &lingua, &tool, &formato)
}

/// Prima posizione del menu passata da chi avvia Halo (`--pos=X,Y`, coordinate nella finestra).
#[tauri::command]
fn start_position() -> Option<(f64, f64)> {
    let v = std::env::args().find_map(|a| a.strip_prefix("--pos=").map(String::from))?;
    let (x, y) = v.split_once(',')?;
    Some((x.trim().parse().ok()?, y.trim().parse().ok()?)).filter(|(x, y): &(f64, f64)| x.is_finite() && y.is_finite() && *x >= 0.0 && *y >= 0.0)
}

/// Avanzamento del lavoro in corso (0–1), se il programma lo dice.
#[tauri::command]
fn job_status() -> Option<f64> {
    lavoro::progress()
}

/// Annulla il lavoro in corso.
#[tauri::command]
fn job_cancel() {
    lavoro::cancel();
}

/// «Continua in sottofondo»: nasconde la finestra, il lavoro va avanti.
#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

/// Notifica di fine lavoro (quando la finestra è nascosta), con «Mostra nella cartella».
/// Parte in un processo a sé, così resta attiva anche dopo la chiusura di Halo.
#[tauri::command]
fn notify_done(msg: String, files: Vec<String>, ok: bool) {
    let uris: Vec<String> = files.iter().filter(|p| Path::new(p).exists()).map(|p| condividi::file_uri(Path::new(p))).collect();
    let list = format!("[{}]", uris.iter().map(|u| format!("'{}'", u)).collect::<Vec<_>>().join(", "));
    let (title, body) = msg.split_once('\n').map(|(a, b)| (a.to_string(), b.to_string())).unwrap_or((msg.clone(), String::new()));
    let script = r#"if [ "$3" = "[]" ]; then notify-send -a Halo -i halo "$1" "$2"; else
a=$(notify-send -a Halo -i halo -A show="Mostra nella cartella" "$1" "$2"); [ "$a" = show ] && gdbus call --session --dest org.freedesktop.FileManager1 --object-path /org/freedesktop/FileManager1 --method org.freedesktop.FileManager1.ShowItems "$3" "" >/dev/null; fi"#;
    let t = if ok { title } else { format!("Halo: {}", title) };
    let _ = std::process::Command::new("sh").args(["-c", script, "halo", &t, &body, &list])
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).spawn();
}

/// Cronologia delle operazioni (la più recente per prima).
#[tauri::command]
fn history_list() -> Vec<storia::VoceVista> {
    storia::list()
}

/// Svuota la cronologia (i file restano dove sono).
#[tauri::command]
fn history_clear() -> Result<(), String> {
    storia::clear()
}

/// Sposta nel cestino i file creati da un'operazione della cronologia.
#[tauri::command]
fn history_trash(quando: u64) -> Result<String, String> {
    storia::trash(quando)
}

/// «Mostra nella cartella»: apre il gestore dei file con i risultati selezionati.
#[tauri::command]
fn show_in_folder(paths: Vec<String>) -> Result<(), String> {
    let uris: Vec<String> = paths.iter().filter(|p| Path::new(p).exists()).map(|p| condividi::file_uri(Path::new(p))).collect();
    if uris.is_empty() { return Err("Il file non c'è più.".into()); }
    let list = format!("[{}]", uris.iter().map(|u| format!("'{}'", u)).collect::<Vec<_>>().join(", "));
    std::process::Command::new("gdbus").args(["call", "--session", "--dest", "org.freedesktop.FileManager1", "--object-path",
        "/org/freedesktop/FileManager1", "--method", "org.freedesktop.FileManager1.ShowItems", &list, ""])
        .stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null())
        .spawn().map(|_| ()).map_err(|e| e.to_string())
}

/// Condividi: crea lo ZIP, lo copia negli appunti e restituisce le app a cui passarlo.
#[tauri::command]
async fn share_prepare(path: String, key: String) -> Result<condividi::Pronto, String> {
    tauri::async_runtime::spawn_blocking(move || condividi::prepare(Path::new(&path), &key)).await.map_err(|e| e.to_string())?
}

/// Condividi: apre l'app scelta nel pannello.
#[tauri::command]
fn share_open(id: String) -> Result<String, String> {
    condividi::open(&id)
}

/// Esegue un'azione fuori dal thread dell'interfaccia; `data` sono i campi del pannello.
#[tauri::command]
async fn run_action(path: String, key: String, data: Option<HashMap<String, String>>) -> Result<esito::Esito, String> {
    let data = data.unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || esito::run(&path, &key, &data))
        .await
        .map_err(|e| e.to_string())?
}

fn main() {
    configura::migrate_from_fulcro();
    // Configurazione all'accesso (autostart del pacchetto): niente finestra.
    if std::env::args().any(|a| a == "--configura") {
        println!("{}", configura::run());
        return;
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![target_file, supported_actions, form_actions, planned_output, media_duration, current_value, redact_preview, pick_files, archive_entries, target_files, run_multi, show_in_folder, history_list, history_clear, history_trash, job_status, job_cancel, hide_window, notify_done, get_settings, save_settings, translate_tools, translate_open, start_position, list_apps, pick_program, run_shortcut, editor_preview, share_prepare, share_open, run_action])
        .run(tauri::generate_context!())
        .expect("errore all'avvio di Halo");
}
