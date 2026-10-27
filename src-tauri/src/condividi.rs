//! Cartella › Condividi › Email, Altro e Link: crea lo ZIP accanto alla cartella, lo copia negli
//! appunti (o, per Link, lo carica su Gofile e copia il link) e propone le app installate a cui
//! passarlo. La cartella non viene toccata.

use crate::actions::{name, out_path, run_cmd, s};
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

pub const KEYS: &[&str] = &[
    "folder/condividi/email", "folder/condividi/altro", "folder/condividi/link",
    "other/condividi/email", "other/condividi/altro", "other/condividi/link",
    "slides/condividi/email", "slides/condividi/altro", "slides/condividi/link",
];

/// Servizio per i link: Gofile carica senza account e restituisce una pagina di download.
/// Il primo indirizzo sceglie da solo il server; gli altri sono server di caricamento diretti.
const UPLOAD_URLS: &[&str] = &["https://upload.gofile.io/uploadfile", "https://store1.gofile.io/uploadfile", "https://store2.gofile.io/uploadfile"];

#[derive(Clone, Copy, PartialEq, Debug)]
enum Modo { Email, Altro, Link }

/// Un'app a cui passare il file. `come`: "file" (la apriamo già con il file),
/// "link" (la apriamo con il link già scritto), "incolla" (si apre e si incolla con Ctrl+V),
/// "trascina" (si trascina il file dalla cartella).
#[derive(Serialize, Clone, Debug)]
pub struct App {
    pub id: String,
    pub nome: String,
    pub come: &'static str,
    #[serde(skip)]
    avvio: Avvio,
}

#[derive(Clone, Debug)]
enum Avvio {
    /// Programma con argomenti; "{file}", "{uri}", "{link}" e "{link_enc}" vengono sostituiti.
    Cmd(String, Vec<String>),
    /// File .desktop da aprire con `gio launch`.
    Desktop(PathBuf),
}

#[derive(Serialize)]
pub struct Pronto {
    pub file: String,
    pub link: Option<String>,
    /// Il link come codice QR (SVG), da inquadrare con il telefono.
    pub qr: Option<String>,
    pub app: Vec<App>,
}

/// Lo ZIP preparato e le app proposte: share_open accetta solo queste.
static PREPARATO: Mutex<Option<(PathBuf, Option<String>, Vec<App>)>> = Mutex::new(None);

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    // Una cartella si condivide come ZIP (file nuovo); un file si condivide così com'è.
    (KEYS.contains(&key) && input.is_dir()).then(|| out_path(input, "", "zip"))
}

fn zip(input: &Path) -> Result<PathBuf, String> {
    if !input.is_dir() { return Err("Condividi funziona con le cartelle.".into()); }
    let out = out_path(input, "", "zip");
    run_cmd("7z", &["a", "-tzip", &s(&out), &s(input)])?;
    Ok(out)
}

/// Senza pannello (test): crea solo lo ZIP, senza appunti né app.
pub fn run(input: &Path, key: &str) -> Result<String, String> {
    if !input.is_dir() { return Err("Condividi si usa dal pannello, con la scelta dell'app.".into()); }
    if key.ends_with("/link") { return Err("Il link si crea dal pannello, dopo la conferma del caricamento.".into()); }
    Ok(format!("Creato: {}", name(&zip(input)?)))
}

/// Crea lo ZIP, lo copia negli appunti e trova le app adatte.
pub fn prepare(input: &Path, key: &str) -> Result<Pronto, String> {
    if !KEYS.contains(&key) { return Err("Opzione non disponibile.".into()); }
    let modo = if key.ends_with("/email") { Modo::Email } else if key.ends_with("/link") { Modo::Link } else { Modo::Altro };
    let out = if input.is_dir() { zip(input)? } else { input.to_path_buf() };
    let link = if modo == Modo::Link {
        let (l, _) = upload(&out)?;
        copy_text(&l)?;
        Some(l)
    } else {
        copy_file(&out)?;
        None
    };
    let app = apps(modo, &desktop_entries());
    *PREPARATO.lock().unwrap() = Some((out.clone(), link.clone(), app.clone()));
    let qr = link.as_deref().and_then(|l| crate::qr::svg(l).ok());
    Ok(Pronto { file: name(&out), link, qr, app })
}

/// Carica il file su Gofile. Restituisce la pagina di download e (token ospite, cartella)
/// con cui si potrebbe cancellarlo. Se il server generale non risponde, prova i server di caricamento.
fn upload(file: &Path) -> Result<(String, (String, String)), String> {
    let mut last = String::new();
    for url in UPLOAD_URLS {
        match upload_to(file, url) {
            Ok(r) => return Ok(r),
            Err((retry, e)) => { last = e; if !retry { break; } }
        }
    }
    Err(last)
}

/// Un tentativo; in caso di errore dice anche se ha senso riprovare con un altro server.
fn upload_to(file: &Path, url: &str) -> Result<(String, (String, String)), (bool, String)> {
    // Il file passa da stdin: così il percorso non va interpretato da curl. Il nome mostrato
    // nella pagina di download perde i caratteri che curl userebbe come separatori.
    let shown: String = name(file).chars().map(|c| if "\";,\\".contains(c) || c.is_control() { '_' } else { c }).collect();
    let field = format!("file=@-;filename={}", shown);
    let input = fs::File::open(file).map_err(|e| (false, e.to_string()))?;
    let curl = if Path::new("/usr/bin/curl").exists() { "/usr/bin/curl" } else { "curl" };
    let o = Command::new(curl).args(["-sS", "--fail-with-body", "--connect-timeout", "20", "--max-time", "7200", "-F", &field, url])
        .stdin(input).output().map_err(|_| (false, "Manca «curl». Installa con: sudo apt install curl".to_string()))?;
    let body = String::from_utf8_lossy(&o.stdout);
    let page = serde_json::from_str::<serde_json::Value>(&body).ok()
        .filter(|j| j["status"] == "ok")
        .and_then(|j| {
            let d = &j["data"];
            let page = d["downloadPage"].as_str().filter(|p| p.starts_with("https://gofile.io/"))?;
            Some((page.to_string(), (d["guestToken"].as_str().unwrap_or("").to_string(), d["parentFolder"].as_str().unwrap_or("").to_string())))
        });
    page.ok_or_else(|| {
        let e = String::from_utf8_lossy(&o.stderr);
        if e.contains("resolve host") || e.contains("Failed to connect") { (false, "Nessuna connessione a Internet: il link non è stato creato.".to_string()) }
        else { (true, format!("Il caricamento su Gofile non è riuscito{}.", e.lines().next().map(|l| format!(" ({})", l.trim_start_matches("curl: "))).unwrap_or_default())) }
    })
}

/// Copia un testo negli appunti.
pub(crate) fn copy_text(text: &str) -> Result<(), String> {
    let st = Command::new("wl-copy").arg(text)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status()
        .map_err(|_| "Manca «wl-copy». Installa con: sudo apt install wl-clipboard".to_string())?;
    if st.success() { Ok(()) } else { Err("Non sono riuscito a copiare il link negli appunti.".into()) }
}

/// Apre l'app scelta nel pannello.
pub fn open(id: &str) -> Result<String, String> {
    let (file, link, app) = PREPARATO.lock().unwrap().clone().ok_or("Prepara prima il file da condividere.")?;
    let a = app.into_iter().find(|a| a.id == id).ok_or("App non trovata.")?;
    let uri = file_uri(&file);
    let l = link.clone().unwrap_or_default();
    match &a.avvio {
        Avvio::Cmd(prog, args) => {
            let args: Vec<String> = args.iter().map(|x| x.replace("{file}", &s(&file)).replace("{uri}", &uri)
                .replace("{link_enc}", &encode(&l, b"-._~")).replace("{link}", &l)).collect();
            spawn(prog, &args).map_err(|_| format!("Non riesco ad aprire {}.", a.nome))?;
            if a.id == "gmail" && link.is_none() { spawn("gdbus", &show_args(&uri)).ok(); }
        }
        Avvio::Desktop(d) => spawn("/usr/bin/gio", &["launch".to_string(), s(d)]).map_err(|_| format!("Non riesco ad aprire {}.", a.nome))?,
    }
    if link.is_some() {
        return Ok(match a.come {
            "incolla" => format!("{} aperto: incolla il link con Ctrl+V.", a.nome),
            _ => format!("{} aperto con il link.", a.nome),
        });
    }
    Ok(match a.come {
        "file" => format!("{} aperto con {}.", a.nome, name(&file)),
        "incolla" => format!("{} aperto: incolla {} con Ctrl+V.", a.nome, name(&file)),
        _ => format!("{} aperto: trascina {} dalla cartella.", a.nome, name(&file)),
    })
}

/// Avvia un programma staccato da Halo (resta aperto quando il menu si chiude).
pub(crate) fn spawn(prog: &str, args: &[String]) -> std::io::Result<()> {
    Command::new(prog).args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map(|_| ())
}

/// Copia il file negli appunti come file (text/uri-list): Telegram, i browser e Nautilus lo incollano.
fn copy_file(file: &Path) -> Result<(), String> {
    // wl-copy resta attivo in background per servire gli appunti: niente pipe da aspettare.
    let st = Command::new("wl-copy").args(["-t", "text/uri-list", &format!("{}\r\n", file_uri(file))])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status()
        .map_err(|_| "Manca «wl-copy». Installa con: sudo apt install wl-clipboard".to_string())?;
    if st.success() { Ok(()) } else { Err("Non sono riuscito a copiare il file negli appunti.".into()) }
}

pub fn file_uri(p: &Path) -> String {
    format!("file://{}", encode(&s(p), b"/-._~"))
}

/// Codifica percentuale: restano solo lettere, cifre e i caratteri di `keep`.
fn encode(text: &str, keep: &[u8]) -> String {
    let mut u = String::new();
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || keep.contains(&b) { u.push(b as char); } else { u.push_str(&format!("%{:02X}", b)); }
    }
    u
}

/// Argomenti di gdbus per mostrare il file selezionato nel gestore dei file.
pub(crate) fn show_args(uri: &str) -> Vec<String> {
    ["call", "--session", "--dest", "org.freedesktop.FileManager1", "--object-path", "/org/freedesktop/FileManager1",
        "--method", "org.freedesktop.FileManager1.ShowItems"].iter().map(|x| x.to_string())
        .chain([format!("['{}']", uri), "".to_string()]).collect()
}

/// Le voci utili di un file .desktop (solo il gruppo principale).
#[derive(Debug, Default, Clone)]
pub struct Entry {
    pub(crate) file: PathBuf,
    pub(crate) nome: String,
    pub(crate) exec: Vec<String>,
    categorie: String,
}

pub(crate) fn parse_desktop(file: &Path, text: &str) -> Option<Entry> {
    let mut e = Entry { file: file.to_path_buf(), ..Default::default() };
    let mut main = false;
    for l in text.lines() {
        let l = l.trim();
        if l.starts_with('[') { main = l == "[Desktop Entry]"; continue; }
        if !main { continue; }
        let Some((k, v)) = l.split_once('=') else { continue };
        match k.trim() {
            "Name" => e.nome = v.trim().to_string(),
            "Exec" => e.exec = v.split_whitespace().map(|t| t.trim_matches('"').to_string()).collect(),
            "Categories" => e.categorie = v.to_string(),
            "Type" if v.trim() != "Application" => return None,
            "NoDisplay" | "Hidden" if v.trim() == "true" => return None,
            _ => {}
        }
    }
    (!e.nome.is_empty() && !e.exec.is_empty()).then_some(e)
}

/// Le app installate (utente, sistema, snap, flatpak); a parità di nome file vince la prima.
pub(crate) fn desktop_entries() -> Vec<Entry> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut dirs = vec![format!("{}/.local/share/applications", home), format!("{}/.local/share/flatpak/exports/share/applications", home)];
    for d in std::env::var("XDG_DATA_DIRS").unwrap_or_default().split(':').filter(|d| !d.is_empty()) {
        dirs.push(format!("{}/applications", d.trim_end_matches('/')));
    }
    dirs.extend(["/usr/local/share/applications", "/usr/share/applications", "/var/lib/snapd/desktop/applications",
        "/var/lib/flatpak/exports/share/applications"].map(String::from));
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for d in dirs {
        let Ok(rd) = fs::read_dir(&d) else { continue };
        let mut files: Vec<PathBuf> = rd.filter_map(|x| x.ok()).map(|x| x.path()).filter(|p| p.extension().is_some_and(|e| e == "desktop")).collect();
        files.sort();
        for f in files {
            if !seen.insert(f.file_name().unwrap().to_owned()) { continue; }
            if let Some(e) = fs::read_to_string(&f).ok().and_then(|t| parse_desktop(&f, &t)) { out.push(e); }
        }
    }
    out
}

/// Le app da proporre: per Email i programmi di posta e Gmail, per Altro chat, Bluetooth e posta,
/// per Link chat, posta e Gmail con il link già scritto. In fondo «Mostra nella cartella»
/// (per Link «Apri nel browser», per controllare la pagina di download).
fn apps(modo: Modo, entries: &[Entry]) -> Vec<App> {
    let (email, link) = (modo == Modo::Email, modo == Modo::Link);
    let mut v: Vec<App> = Vec::new();
    fn add(v: &mut Vec<App>, id: &str, nome: &str, come: &'static str, avvio: Avvio) {
        if !v.iter().any(|a| a.id == id) { v.push(App { id: id.to_string(), nome: nome.to_string(), come, avvio }); }
    }
    let cmd = |e: &Entry, args: &[&str]| Avvio::Cmd(e.exec[0].clone(), args.iter().map(|x| x.to_string()).collect());
    // flatpak/env vogliono i loro argomenti: per queste app si apre e si incolla.
    let plain = |e: &Entry| !["flatpak", "env"].contains(&Path::new(&e.exec[0]).file_name().and_then(|n| n.to_str()).unwrap_or(""));
    for e in entries {
        let n = e.nome.to_lowercase();
        let prog = Path::new(&e.exec[0]).file_name().map(|p| p.to_string_lossy().to_lowercase()).unwrap_or_default();
        let id = e.file.file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
        let posta = e.categorie.split(';').any(|c| c == "Email") && !e.categorie.split(';').any(|c| c == "WebBrowser");
        let chat = e.categorie.split(';').any(|c| c == "InstantMessaging" || c == "Chat")
            || ["whatsapp", "localsend", "signal", "element"].iter().any(|k| n.contains(k));
        if posta && link {
            match () {
                _ if prog.contains("thunderbird") && plain(e) => add(&mut v, &id, &e.nome, "link", cmd(e, &["-compose", "body='{link}'"])),
                _ if (prog.contains("evolution") || prog.contains("geary")) && plain(e) => add(&mut v, &id, &e.nome, "link", cmd(e, &["mailto:?body={link_enc}"])),
                _ => add(&mut v, &id, &e.nome, "incolla", Avvio::Desktop(e.file.clone())),
            }
        } else if posta {
            match () {
                _ if prog.contains("thunderbird") && plain(e) => add(&mut v, &id, &e.nome, "file", cmd(e, &["-compose", "attachment='{uri}'"])),
                _ if (prog.contains("evolution") || prog.contains("geary")) && plain(e) => add(&mut v, &id, &e.nome, "file", cmd(e, &["mailto:?attach={uri}"])),
                _ => add(&mut v, &id, &e.nome, "incolla", Avvio::Desktop(e.file.clone())),
            }
        } else if chat && !email {
            if n.starts_with("telegram") && link {
                // Telegram è registrato per tg://: si apre la scelta della chat con il link.
                add(&mut v, &id, &e.nome, "link", Avvio::Cmd("xdg-open".into(), vec!["tg://msg_url?url={link_enc}".into()]));
            } else if n.starts_with("telegram") && plain(e) {
                add(&mut v, &id, &e.nome, "file", cmd(e, &["-sendpath", "{file}"]));
            } else {
                add(&mut v, &id, &e.nome, "incolla", Avvio::Desktop(e.file.clone()));
            }
        }
    }
    if link {
        add(&mut v, "gmail", "Gmail (nel browser)", "link", Avvio::Cmd("xdg-open".into(), vec!["https://mail.google.com/mail/?view=cm&fs=1&body={link_enc}".into()]));
    } else if email {
        add(&mut v, "gmail", "Gmail (nel browser)", "trascina", Avvio::Cmd("xdg-open".into(), vec!["https://mail.google.com/mail/?view=cm&fs=1".into()]));
    } else if Path::new("/usr/bin/bluetooth-sendto").exists() {
        add(&mut v, "bluetooth", "Bluetooth", "file", Avvio::Cmd("/usr/bin/bluetooth-sendto".into(), vec!["{file}".into()]));
    }
    v.sort_by_key(|a| (a.come != "file" && a.come != "link", a.nome.to_lowercase()));
    if link {
        add(&mut v, "browser", "Apri nel browser", "link", Avvio::Cmd("xdg-open".into(), vec!["{link}".into()]));
        return v;
    }
    add(&mut v, "cartella", "Mostra nella cartella", "trascina", Avvio::Cmd("gdbus".into(), show_args("{uri}")));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(file: &str, text: &str) -> Option<Entry> { parse_desktop(Path::new(file), text) }

    #[test]
    fn app_proposte() {
        let tg = entry("/a/org.telegram.desktop.desktop", "[Desktop Entry]\nName=Telegram\nExec=/opt/Telegram/Telegram -- %U\nType=Application\nCategories=Chat;Network;\n\n[Desktop Action quit]\nName=Quit Telegram\nExec=/opt/Telegram/Telegram -quit\n").unwrap();
        assert_eq!(tg.exec, ["/opt/Telegram/Telegram", "--", "%U"]);
        let wa = entry("/a/chrome-x.desktop", "[Desktop Entry]\nName=WhatsApp Web\nExec=/opt/google/chrome/google-chrome --app-id=x\n").unwrap();
        let tb = entry("/a/thunderbird.desktop", "[Desktop Entry]\nName=Thunderbird\nExec=thunderbird %u\nCategories=Network;Email;\n").unwrap();
        let ff = entry("/a/firefox.desktop", "[Desktop Entry]\nName=Firefox\nExec=firefox %u\nCategories=Network;WebBrowser;Email;\n").unwrap();
        let fp = entry("/a/org.signal.Signal.desktop", "[Desktop Entry]\nName=Signal\nExec=/usr/bin/flatpak run org.signal.Signal @@u %U @@\nCategories=Network;InstantMessaging;\n").unwrap();
        assert!(entry("/a/x.desktop", "[Desktop Entry]\nName=X\nExec=x\nNoDisplay=true\n").is_none());
        let all = [tg, wa, tb, ff, fp];

        let altro = apps(Modo::Altro, &all);
        let ids: Vec<(&str, &str)> = altro.iter().map(|a| (a.id.as_str(), a.come)).collect();
        assert!(ids.contains(&("org.telegram.desktop", "file")) && ids.contains(&("chrome-x", "incolla")) && ids.contains(&("thunderbird", "file")));
        assert!(ids.contains(&("org.signal.Signal", "incolla")), "flatpak: si incolla");
        assert!(!ids.iter().any(|(i, _)| *i == "firefox" || *i == "gmail"));
        assert_eq!(altro.last().unwrap().id, "cartella");
        let Avvio::Cmd(p, a) = &altro.iter().find(|a| a.nome == "Telegram").unwrap().avvio else { panic!() };
        assert_eq!((p.as_str(), a.as_slice()), ("/opt/Telegram/Telegram", ["-sendpath".to_string(), "{file}".to_string()].as_slice()));

        let email: Vec<String> = apps(Modo::Email, &all).into_iter().map(|a| a.id).collect();
        assert_eq!(email, ["thunderbird", "gmail", "cartella"]);
        let link: Vec<(String, &str)> = apps(Modo::Link, &all).into_iter().map(|a| (a.id, a.come)).collect();
        assert_eq!(link.iter().map(|(i, c)| (i.as_str(), *c)).collect::<Vec<_>>(),
            [("gmail", "link"), ("org.telegram.desktop", "link"), ("thunderbird", "link"), ("org.signal.Signal", "incolla"), ("chrome-x", "incolla"), ("browser", "link")]);
        assert_eq!(encode("https://gofile.io/d/Ab1", b"-._~"), "https%3A%2F%2Fgofile.io%2Fd%2FAb1");
        assert_eq!(file_uri(Path::new("/tmp/Foto è 1.zip")), "file:///tmp/Foto%20%C3%A8%201.zip");
    }

    /// Carica davvero un piccolo ZIP di prova su Gofile e poi lo cancella.
    /// Parte solo con HALO_PROVA_ONLINE=1, mai durante i test normali.
    #[test]
    #[ignore]
    fn link_online() {
        if std::env::var("HALO_PROVA_ONLINE").as_deref() != Ok("1") { return; }
        let dir = std::env::temp_dir().join("halo-test-link");
        let _ = fs::remove_dir_all(&dir);
        let src = dir.join("Prova; link \"Halo\"");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("leggimi.txt"), "prova di Halo").unwrap();
        let out = zip(&src).unwrap();
        let (page, (token, folder)) = upload(&out).unwrap();
        println!("link: {}", page);
        assert!(page.starts_with("https://gofile.io/d/"));
        let del = run_cmd("/usr/bin/curl", &["-sS", "-X", "DELETE", "-H", &format!("Authorization: Bearer {}", token), "-H", "Content-Type: application/json",
            "-d", &format!("{{\"contentsId\":\"{}\"}}", folder), "https://api.gofile.io/contents"]).unwrap();
        assert!(del.contains("\"status\":\"ok\""), "{}", del);
    }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-condividi");
        let _ = fs::remove_dir_all(&dir);
        let src = dir.join("Progetto cliente");
        fs::create_dir_all(src.join("doc")).unwrap();
        fs::write(src.join("doc/a.txt"), "ciao").unwrap();
        // run() crea solo lo ZIP: niente appunti e nessuna app aperta durante i test.
        assert_eq!(run(&src, "folder/condividi/email").unwrap(), "Creato: Progetto cliente.zip");
        assert_eq!(run(&src, "folder/condividi/altro").unwrap(), "Creato: Progetto cliente (2).zip");
        assert_eq!(run_cmd("7z", &["x", "-so", &s(&dir.join("Progetto cliente.zip")), "Progetto cliente/doc/a.txt"]).unwrap(), "ciao");
        assert_eq!(fs::read_to_string(src.join("doc/a.txt")).unwrap(), "ciao");
        assert!(run(&src.join("doc/a.txt"), "folder/condividi/email").is_err());
        assert!(run(&src, "folder/condividi/link").is_err(), "senza pannello non si carica nulla online");
        // Le app installate su questo computer: elenco solo stampato.
        for a in apps(Modo::Altro, &desktop_entries()) { println!("{} [{}] {}", a.nome, a.come, a.id); }
    }
}
