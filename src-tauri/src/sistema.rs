//! Le differenze tra Linux e Windows in un posto solo: dove stanno i programmi esterni
//! (ImageMagick, Ghostscript, 7-Zip, LibreOffice, Python…), appunti, cestino, «Mostra nella
//! cartella», notifiche, cartelle dell'utente. Il resto di Halo usa queste funzioni e funziona
//! uguale sui due sistemi.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Il programma esterno `name` pronto da lanciare. Su Windows alcuni hanno un altro nome
/// (ImageMagick 7 è «magick», Ghostscript «gswin64c») o stanno in Programmi senza essere nel PATH.
pub fn cmd(name: &str) -> Command {
    let (prog, pre) = resolve(name);
    let mut c = Command::new(prog);
    c.args(pre);
    #[cfg(windows)]
    {
        // Niente finestra nera del terminale per ogni programma lanciato.
        use std::os::windows::process::CommandExt;
        c.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    c
}

#[cfg(not(windows))]
fn resolve(name: &str) -> (PathBuf, Vec<&'static str>) {
    match name {
        "python3" => (PathBuf::from("/usr/bin/python3"), vec![]),
        _ => (PathBuf::from(name), vec![]),
    }
}

#[cfg(windows)]
fn resolve(name: &str) -> (PathBuf, Vec<&'static str>) {
    let pf = |sub: &str| -> Option<PathBuf> {
        [std::env::var("ProgramFiles").ok(), std::env::var("ProgramFiles(x86)").ok(), std::env::var("LOCALAPPDATA").ok().map(|l| l + "\\Programs")]
            .into_iter().flatten().map(|b| PathBuf::from(b).join(sub)).find(|p| p.is_file())
    };
    match name {
        // ImageMagick 7: «magick» al posto di «convert» (che su Windows è un altro comando di sistema).
        "convert" => (find_program("magick").unwrap_or_else(|| "magick".into()), vec![]),
        "identify" => (find_program("magick").unwrap_or_else(|| "magick".into()), vec!["identify"]),
        "gs" => (find_program("gswin64c").or_else(gs_dir).unwrap_or_else(|| "gswin64c".into()), vec![]),
        "7z" => (find_program("7z").or_else(|| pf("7-Zip\\7z.exe")).unwrap_or_else(|| "7z".into()), vec![]),
        "soffice" => (find_program("soffice").or_else(|| pf("LibreOffice\\program\\soffice.exe")).unwrap_or_else(|| "soffice".into()), vec![]),
        "pandoc" => (find_program("pandoc").or_else(|| pf("Pandoc\\pandoc.exe")).unwrap_or_else(|| "pandoc".into()), vec![]),
        "tesseract" => (find_program("tesseract").or_else(|| pf("Tesseract-OCR\\tesseract.exe")).unwrap_or_else(|| "tesseract".into()), vec![]),
        // Il lanciatore di Python per Windows («py -3»), altrimenti «python».
        "python3" => match find_program("py") { Some(p) => (p, vec!["-3"]), None => ("python".into(), vec![]) },
        _ => (find_program(name).unwrap_or_else(|| name.into()), vec![]),
    }
}

#[cfg(windows)]
fn gs_dir() -> Option<PathBuf> {
    // C:\Program Files\gs\gs10.xx\bin\gswin64c.exe (la versione cambia).
    let base = PathBuf::from(std::env::var("ProgramFiles").ok()?).join("gs");
    let mut v: Vec<PathBuf> = fs::read_dir(base).ok()?.filter_map(|e| e.ok()).map(|e| e.path().join("bin\\gswin64c.exe")).filter(|p| p.is_file()).collect();
    v.sort();
    v.pop()
}

/// Cerca un programma nel PATH (con «.exe» su Windows).
pub fn find_program(name: &str) -> Option<PathBuf> {
    let exts: &[&str] = if cfg!(windows) { &[".exe", ".cmd", ".bat", ""] } else { &[""] };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).flat_map(|d| exts.iter().map(move |e| d.join(format!("{}{}", name, e)))).find(|p| p.is_file())
}

/// Nome del programma con l'estensione del sistema («realesrgan-ncnn-vulkan.exe» su Windows).
pub fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{}.exe", name) } else { name.to_string() }
}

/// Copia un file o una cartella intera (con le date di modifica), come `cp -a`.
pub fn copy_all(src: &Path, dst: &Path) -> Result<(), String> {
    let meta = fs::symlink_metadata(src).map_err(|e| e.to_string())?;
    if meta.is_dir() {
        fs::create_dir_all(dst).map_err(|e| e.to_string())?;
        for e in fs::read_dir(src).map_err(|e| e.to_string())? {
            let e = e.map_err(|e| e.to_string())?;
            copy_all(&e.path(), &dst.join(e.file_name()))?;
        }
    } else if meta.file_type().is_symlink() && !cfg!(windows) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(fs::read_link(src).map_err(|e| e.to_string())?, dst).map_err(|e| e.to_string())?;
    } else {
        fs::copy(src, dst).map_err(|e| format!("Non riesco a copiare {}: {}", src.display(), e))?;
    }
    // La data di modifica resta quella dell'originale (serve a «Organizza per data» e simili).
    if let Ok(m) = meta.modified() {
        if let Ok(f) = fs::OpenOptions::new().write(!meta.is_dir()).read(meta.is_dir()).open(dst) { let _ = f.set_modified(m); }
    }
    Ok(())
}

/// Tutti i file (non le cartelle) sotto `p`, con la dimensione.
fn walk(p: &Path, out: &mut Vec<(PathBuf, u64)>) {
    let Ok(m) = fs::symlink_metadata(p) else { return };
    if m.is_dir() {
        if let Ok(rd) = fs::read_dir(p) { for e in rd.flatten() { walk(&e.path(), out); } }
    } else if m.is_file() {
        out.push((p.to_path_buf(), m.len()));
    }
}

/// Dimensione totale e numero di file di un file o di una cartella.
pub fn size_and_count(p: &Path) -> (u64, usize) {
    let mut v = Vec::new();
    walk(p, &mut v);
    (v.iter().map(|x| x.1).sum(), v.len())
}

/// Data (locale) di modifica di un file nel formato `fmt` di chrono (es. "%Y-%m-%d").
pub fn modified_local(p: &Path, fmt: &str) -> Result<String, String> {
    let m = fs::metadata(p).and_then(|m| m.modified()).map_err(|e| e.to_string())?;
    Ok(chrono::DateTime::<chrono::Local>::from(m).format(fmt).to_string())
}

/// Impronta SHA-256 di un file, in esadecimale.
pub fn sha256(p: &Path) -> Result<String, String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut f = fs::File::open(p).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut buf = vec![0u8; 1 << 16];
    loop {
        let n = f.read(&mut buf).map_err(|e| e.to_string())?;
        if n == 0 { break; }
        h.update(&buf[..n]);
    }
    Ok(h.finalize().iter().map(|b| format!("{:02x}", b)).collect())
}

/// La Scrivania dell'utente (cartella localizzata: ~/Scrivania, OneDrive\Desktop…), o la home.
pub fn desktop_dir() -> PathBuf {
    let home = dirs::home_dir().unwrap_or_else(std::env::temp_dir);
    dirs::desktop_dir().filter(|d| d.is_dir() && *d != home && writable(d)).unwrap_or(home)
}

/// Cartelle di Halo per impostazioni, dati (cronologia) e cache (modelli scaricati).
pub fn config_dir() -> PathBuf { dirs::config_dir().unwrap_or_else(std::env::temp_dir).join("halo") }
pub fn data_dir() -> PathBuf { dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("halo") }
pub fn cache_dir() -> PathBuf { dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("halo") }

/// Si può scrivere nella cartella? (disco in sola lettura, cartella di sistema…)
#[cfg(unix)]
pub fn writable(dir: &Path) -> bool {
    use std::os::unix::ffi::OsStrExt;
    let Ok(c) = std::ffi::CString::new(dir.as_os_str().as_bytes()) else { return false };
    // SAFETY: access() legge solo il percorso, terminato da zero.
    dir.is_dir() && unsafe { libc::access(c.as_ptr(), libc::W_OK) } == 0
}

#[cfg(windows)]
pub fn writable(dir: &Path) -> bool {
    // Su Windows i permessi dipendono dalle ACL: si prova a creare un file e lo si toglie subito.
    if !dir.is_dir() { return false; }
    let probe = dir.join(format!(".halo-prova-{}", std::process::id()));
    let ok = fs::OpenOptions::new().write(true).create_new(true).open(&probe).is_ok();
    let _ = fs::remove_file(&probe);
    ok
}

/// Sposta nel cestino (mai una cancellazione definitiva).
pub fn trash(p: &Path) -> Result<(), String> {
    trash::delete(p).map_err(|e| format!("Non riesco a spostare nel cestino {} ({}).", p.display(), e))
}

/// Apre un indirizzo web o un file con il programma predefinito.
pub fn open(target: &str) -> Result<(), String> {
    open::that_detached(target).map_err(|e| e.to_string())
}

/// Apre il gestore dei file con questi elementi selezionati.
pub fn show_in_folder(paths: &[PathBuf]) -> Result<(), String> {
    let paths: Vec<&PathBuf> = paths.iter().filter(|p| p.exists()).collect();
    if paths.is_empty() { return Err("Il file non c'è più.".into()); }
    #[cfg(windows)]
    {
        // Esplora file seleziona un elemento alla volta: si seleziona il primo.
        let mut c = Command::new("explorer");
        c.arg(format!("/select,{}", paths[0].display()));
        return c.spawn().map(|_| ()).map_err(|e| e.to_string());
    }
    #[cfg(not(windows))]
    {
        let uris: Vec<String> = paths.iter().map(|p| crate::condividi::file_uri(p)).collect();
        let list = format!("[{}]", uris.iter().map(|u| format!("'{}'", u)).collect::<Vec<_>>().join(", "));
        Command::new("gdbus").args(["call", "--session", "--dest", "org.freedesktop.FileManager1", "--object-path",
            "/org/freedesktop/FileManager1", "--method", "org.freedesktop.FileManager1.ShowItems", &list, ""])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn().map(|_| ()).map_err(|e| e.to_string())
    }
}

/// Copia un testo negli appunti.
pub fn clipboard_text(text: &str) -> Result<(), String> {
    #[cfg(windows)]
    {
        return powershell_stdin("[Console]::InputEncoding=[Text.Encoding]::UTF8; Set-Clipboard -Value ([Console]::In.ReadToEnd())", text);
    }
    #[cfg(not(windows))]
    {
        // wl-copy resta attivo in background per servire gli appunti: niente pipe da aspettare.
        let st = Command::new("wl-copy").arg(text).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status()
            .map_err(|_| "Manca «wl-copy». Installa con: sudo apt install wl-clipboard".to_string())?;
        if st.success() { Ok(()) } else { Err("Non sono riuscito a copiare negli appunti.".into()) }
    }
}

/// Copia un file negli appunti come file (si incolla nelle chat, nella posta, nel gestore dei file).
pub fn clipboard_file(file: &Path) -> Result<(), String> {
    #[cfg(windows)]
    {
        return powershell_stdin("[Console]::InputEncoding=[Text.Encoding]::UTF8; Set-Clipboard -Path ([Console]::In.ReadToEnd().Trim())", &file.display().to_string());
    }
    #[cfg(not(windows))]
    {
        let st = Command::new("wl-copy").args(["-t", "text/uri-list", &format!("{}\r\n", crate::condividi::file_uri(file))])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status()
            .map_err(|_| "Manca «wl-copy». Installa con: sudo apt install wl-clipboard".to_string())?;
        if st.success() { Ok(()) } else { Err("Non sono riuscito a copiare il file negli appunti.".into()) }
    }
}

#[cfg(windows)]
fn powershell_stdin(script: &str, input: &str) -> Result<(), String> {
    use std::io::Write;
    use std::os::windows::process::CommandExt;
    let mut child = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|e| e.to_string())?;
    if let Some(mut si) = child.stdin.take() { let _ = si.write_all(input.as_bytes()); }
    let st = child.wait().map_err(|e| e.to_string())?;
    if st.success() { Ok(()) } else { Err("Non sono riuscito a copiare negli appunti.".into()) }
}

/// Un comando scritto dall'utente (scorciatoie): `sh -c` su Linux, `cmd /C` su Windows.
pub fn shell(line: &str) -> Command {
    if cfg!(windows) {
        let mut c = Command::new("cmd");
        c.args(["/C", line]);
        c
    } else {
        let mut c = Command::new("/bin/sh");
        c.args(["-c", line]);
        c
    }
}

/// Un valore pronto per la riga di comando dell'utente (apici su Linux, virgolette su Windows).
pub fn quote(v: &str) -> String {
    if cfg!(windows) { format!("\"{}\"", v.replace('"', "\"\"")) } else { format!("'{}'", v.replace('\'', "'\\''")) }
}

/// Apre un'app (programma, collegamento .lnk o .desktop) con un file.
pub fn launch_with(app: &str, file: &Path) -> Result<(), String> {
    let r = if app.ends_with(".desktop") {
        Command::new("/usr/bin/gio").args(["launch", app, &file.display().to_string()]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
    } else if cfg!(windows) && app.to_lowercase().ends_with(".lnk") {
        // Un collegamento del menu Start: lo apre Windows, passando il file.
        Command::new("cmd").args(["/C", "start", "", app, &file.display().to_string()]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
    } else {
        Command::new(app).arg(file).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
    };
    r.map(|_| ()).map_err(|e| e.to_string())
}

/// Notifica di sistema a fine lavoro; con dei file, su Linux c'è il pulsante «Mostra nella cartella».
/// Parte in un processo a sé, così resta anche dopo la chiusura di Halo.
pub fn notify(title: &str, body: &str, files: &[PathBuf]) {
    #[cfg(windows)]
    {
        let _ = files;
        let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('\'', "''");
        let script = format!(concat!(
            "[Windows.UI.Notifications.ToastNotificationManager, Windows.UI.Notifications, ContentType = WindowsRuntime] > $null;",
            "$x = [Windows.Data.Xml.Dom.XmlDocument, Windows.Data.Xml.Dom.XmlDocument, ContentType = WindowsRuntime]::New();",
            "$x.LoadXml('<toast><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>');",
            "[Windows.UI.Notifications.ToastNotificationManager]::CreateToastNotifier('Halo').Show([Windows.UI.Notifications.ToastNotification]::New($x))"),
            esc(title), esc(body));
        use std::os::windows::process::CommandExt;
        let _ = Command::new("powershell").args(["-NoProfile", "-NonInteractive", "-Command", &script]).creation_flags(0x0800_0000)
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    }
    #[cfg(not(windows))]
    {
        let uris: Vec<String> = files.iter().filter(|p| p.exists()).map(|p| crate::condividi::file_uri(p)).collect();
        let list = format!("[{}]", uris.iter().map(|u| format!("'{}'", u)).collect::<Vec<_>>().join(", "));
        let script = r#"if [ "$3" = "[]" ]; then notify-send -a Halo -i halo "$1" "$2"; else
a=$(notify-send -a Halo -i halo -A show="Mostra nella cartella" "$1" "$2"); [ "$a" = show ] && gdbus call --session \
--dest org.freedesktop.FileManager1 --object-path /org/freedesktop/FileManager1 \
--method org.freedesktop.FileManager1.ShowItems "$3" "" >/dev/null; fi"#;
        let _ = Command::new("sh").args(["-c", script, "halo", title, body, &list])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
    }
}

/// Ferma un processo lanciato da Halo (Annulla), con i suoi figli su Windows.
pub fn kill(pid: u32) {
    #[cfg(unix)]
    // SAFETY: kill() manda solo un segnale al processo indicato.
    unsafe { libc::kill(pid as i32, libc::SIGTERM); }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        let _ = Command::new("taskkill").args(["/PID", &pid.to_string(), "/T", "/F"]).creation_flags(0x0800_0000)
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    }
}

/// Le app installate (nome, file da lanciare): `.desktop` su Linux, collegamenti del menu Start su Windows.
pub fn installed_apps() -> Vec<(String, String)> {
    #[cfg(windows)]
    {
        let mut v = Vec::new();
        let roots = [std::env::var("APPDATA").ok().map(|a| PathBuf::from(a).join("Microsoft\\Windows\\Start Menu\\Programs")),
            std::env::var("ProgramData").ok().map(|a| PathBuf::from(a).join("Microsoft\\Windows\\Start Menu\\Programs"))];
        fn lnk(d: &Path, v: &mut Vec<(String, String)>, depth: usize) {
            if depth > 3 { return; }
            let Ok(rd) = fs::read_dir(d) else { return };
            for e in rd.flatten() {
                let p = e.path();
                if p.is_dir() { lnk(&p, v, depth + 1); }
                else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("lnk")) {
                    let n = p.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
                    // Le voci di disinstallazione e simili non servono.
                    if !n.to_lowercase().contains("uninstall") && !n.to_lowercase().contains("disinstalla") { v.push((n, p.display().to_string())); }
                }
            }
        }
        for r in roots.into_iter().flatten() { lnk(&r, &mut v, 0); }
        v
    }
    #[cfg(not(windows))]
    {
        crate::condividi::desktop_entries().into_iter().map(|e| (e.nome, e.file.display().to_string())).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn utilita() {
        let dir = std::env::temp_dir().join(format!("halo-test-sistema-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("a/b")).unwrap();
        fs::write(dir.join("a/b/x.txt"), "ciao").unwrap();
        fs::write(dir.join("a/y.txt"), "12345").unwrap();
        assert_eq!(size_and_count(&dir.join("a")), (9, 2));
        copy_all(&dir.join("a"), &dir.join("copia")).unwrap();
        assert_eq!(fs::read_to_string(dir.join("copia/b/x.txt")).unwrap(), "ciao");
        assert_eq!(fs::metadata(dir.join("copia/y.txt")).unwrap().modified().unwrap(), fs::metadata(dir.join("a/y.txt")).unwrap().modified().unwrap());
        assert_eq!(sha256(&dir.join("a/b/x.txt")).unwrap(), "b133a0c0e9bee3be20163d2ad31d6248db292aa6dcb1ee087a2aa50e0fc75ae2");
        assert_eq!(modified_local(&dir.join("a/y.txt"), "%Y").unwrap().len(), 4);
        assert_eq!(quote("l'anno"), "'l'\\''anno'");
        assert!(find_program("sh").is_some());
        let _ = fs::remove_dir_all(&dir);
    }
}
