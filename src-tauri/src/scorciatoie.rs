//! Scorciatoie dell'utente («Le mie app»): aprire il file con un'app installata, eseguire un
//! comando scelto dall'utente, oppure ingrandire un'immagine con il programma di Upscayl.
//! Si configurano in Impostazioni › Scorciatoie e si salvano in impostazioni.json.

use crate::actions::{kind_of, name, out_path, parent, run_cmd, s};
use crate::condividi::{desktop_entries, parse_desktop};
use serde::Serialize;
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

/// Tipi di file del menu a cui si può legare una scorciatoia.
pub const TIPI: &[&str] = &["pdf", "image", "audio", "video", "document", "sheet", "folder", "archive", "text", "slides", "other"];
const MODI: &[&str] = &["apri", "comando", "upscayl"];
const MAX: usize = 12;

/// Tiene solo le scorciatoie valide (nome unico, tipi noti, modo noto, app o comando presenti).
pub fn clean_list(v: &Value) -> Value {
    let mut out = Vec::new();
    let mut names = HashSet::new();
    for x in v.as_array().map(|a| a.as_slice()).unwrap_or(&[]) {
        let nome = x["nome"].as_str().unwrap_or("").trim().to_string();
        let modo = x["modo"].as_str().unwrap_or("");
        let app = x["app"].as_str().unwrap_or("").trim().to_string();
        let comando = x["comando"].as_str().unwrap_or("").trim().to_string();
        let tipi: Vec<&str> = x["tipi"].as_array().map(|a| a.iter().filter_map(|t| t.as_str()).filter(|t| TIPI.contains(t)).collect()).unwrap_or_default();
        let ok = !nome.is_empty() && nome.chars().count() <= 30 && !nome.contains('\n') && MODI.contains(&modo) && !tipi.is_empty()
            && match modo { "comando" => !comando.is_empty() && comando.len() <= 500, _ => !app.is_empty() }
            && names.insert(nome.to_lowercase());
        if ok && out.len() < MAX {
            let tipi = if modo == "upscayl" { vec!["image"] } else { tipi };
            out.push(json!({ "nome": nome, "tipi": tipi, "modo": modo, "app": app, "comando": comando }));
        }
    }
    Value::Array(out)
}

#[derive(Serialize)]
pub struct AppInstallata {
    pub nome: String,
    pub file: String,
}

/// Le app installate, in ordine alfabetico, per sceglierle nel pannello.
pub fn installed_apps() -> Vec<AppInstallata> {
    let mut v: Vec<AppInstallata> = desktop_entries().into_iter().map(|e| AppInstallata { nome: e.nome, file: s(&e.file) }).collect();
    v.sort_by_key(|a| a.nome.to_lowercase());
    v.dedup_by(|a, b| a.nome == b.nome);
    v
}

/// Il nome da mostrare per un'app (.desktop o programma).
fn app_name(app: &str) -> String {
    let p = Path::new(app);
    if app.ends_with(".desktop") {
        if let Some(e) = fs::read_to_string(p).ok().and_then(|t| parse_desktop(p, &t)) { return e.nome; }
    }
    name(p)
}

/// Un valore pronto per la shell: tra apici singoli, con gli apici interni protetti.
fn quote(v: &str) -> String {
    format!("'{}'", v.replace('\'', "'\\''"))
}

/// Il comando con {file}, {cartella} e {nome} sostituiti (già tra apici).
fn expand(cmd: &str, input: &Path) -> String {
    let stem = input.file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
    let dir = input.parent().map(s).unwrap_or_default();
    cmd.replace("{file}", &quote(&s(input))).replace("{cartella}", &quote(&dir)).replace("{nome}", &quote(&stem))
}

/// Esegue la scorciatoia numero `index` sul file. `nome` deve coincidere (il menu può essere vecchio).
pub fn run(input: &Path, index: usize, nome: &str) -> Result<String, String> {
    if !input.exists() { return Err(format!("Il file non esiste più: {}", s(input))); }
    let all = crate::impostazioni::load();
    let sc = all["scorciatoie"].get(index).filter(|x| x["nome"] == nome).ok_or("Scorciatoia non trovata: riapri il menu.")?;
    let kind = kind_of(input);
    if !sc["tipi"].as_array().is_some_and(|t| t.iter().any(|x| x == kind)) { return Err("Questa scorciatoia non è per questo tipo di file.".into()); }
    let app = sc["app"].as_str().unwrap_or("");
    match sc["modo"].as_str().unwrap_or("") {
        "apri" => {
            let r = if app.ends_with(".desktop") {
                Command::new("/usr/bin/gio").args(["launch", app, &s(input)]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
            } else {
                Command::new(app).arg(input).current_dir(parent(input)).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
            };
            r.map_err(|_| format!("Non riesco ad aprire {}: controlla la scorciatoia nelle impostazioni.", app_name(app)))?;
            Ok(format!("Aperto con {}.", app_name(app)))
        }
        "comando" => {
            let cmd = expand(sc["comando"].as_str().unwrap_or(""), input);
            let o = Command::new("/bin/sh").args(["-c", &cmd]).current_dir(input.parent().unwrap_or(Path::new("/")))
                .stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
            if o.status.success() {
                let out = String::from_utf8_lossy(&o.stdout);
                let last = out.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("").trim();
                Ok(if last.is_empty() { format!("{}: fatto.", nome) } else { format!("{}: {}", nome, last.chars().take(200).collect::<String>()) })
            } else {
                let err = String::from_utf8_lossy(&o.stderr);
                Err(format!("Il comando non è riuscito: {}", err.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("errore sconosciuto").trim()))
            }
        }
        "upscayl" => upscale(input, app),
        _ => Err("Scorciatoia non valida.".into()),
    }
}

/// Il programma di ingrandimento dentro Upscayl e la cartella dei modelli.
/// Dall'AppImage si estraggono una volta sola in ~/.cache/halo/upscayl (solo il programma e un modello).
fn upscayl_bin(app: &str) -> Result<(PathBuf, PathBuf), String> {
    let mut prog = PathBuf::from(app);
    if app.ends_with(".desktop") {
        let e = fs::read_to_string(app).ok().and_then(|t| parse_desktop(Path::new(app), &t)).ok_or("Upscayl non trovato.")?;
        prog = PathBuf::from(&e.exec[0]);
    }
    if !prog.is_file() { return Err("Upscayl non trovato: forse l'hai spostato. Aggiorna la scorciatoia nelle impostazioni.".into()); }
    // Installazione normale (es. /opt/Upscayl/upscayl): i file stanno in resources/ accanto.
    let res = prog.parent().map(|d| d.join("resources")).filter(|r| r.join("bin/upscayl-bin").is_file());
    if let Some(r) = res { return Ok((r.join("bin/upscayl-bin"), r.join("models"))); }
    if !prog.to_string_lossy().to_lowercase().ends_with(".appimage") {
        return Err("Non trovo il programma di ingrandimento di Upscayl in questa app.".into());
    }
    let m = fs::metadata(&prog).map_err(|e| e.to_string())?;
    let mtime = m.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
    let cache = std::env::var("XDG_CACHE_HOME").ok().filter(|d| !d.is_empty()).map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".cache"))
        .join("halo/upscayl").join(format!("{}-{}", m.len(), mtime));
    let root = cache.join("squashfs-root/resources");
    let bin = root.join("bin/upscayl-bin");
    if !bin.is_file() || !root.join("models/upscayl-standard-4x.param").is_file() {
        fs::create_dir_all(&cache).map_err(|e| e.to_string())?;
        for pat in ["resources/bin/upscayl-bin", "resources/models/upscayl-standard-4x.*"] {
            let o = Command::new(&prog).args(["--appimage-extract", pat]).current_dir(&cache).stdin(Stdio::null()).output()
                .map_err(|e| format!("Non riesco a leggere l'AppImage di Upscayl: {}", e))?;
            if !o.status.success() { return Err("Non riesco a leggere l'AppImage di Upscayl.".into()); }
        }
        if !bin.is_file() { return Err("Questa versione di Upscayl non contiene il programma di ingrandimento.".into()); }
    }
    Ok((bin, root.join("models")))
}

fn upscale(input: &Path, app: &str) -> Result<String, String> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let (bin, models) = upscayl_bin(app)?;
    let model = if models.join("upscayl-standard-4x.param").is_file() { "upscayl-standard-4x".to_string() } else {
        fs::read_dir(&models).ok().and_then(|d| d.filter_map(|e| e.ok()).map(|e| e.path())
            .find(|p| p.extension().is_some_and(|x| x == "param")))
            .and_then(|p| p.file_stem().map(|x| x.to_string_lossy().into_owned())).ok_or("Nessun modello di Upscayl trovato.")?
    };
    // Upscayl legge JPEG, PNG e WebP; gli altri formati passano da un PNG temporaneo.
    let fmt = match ext.as_str() { "jpg" | "jpeg" => "jpg", "webp" => "webp", _ => "png" };
    let tmp = std::env::temp_dir().join(format!("halo-upscayl-{}.png", std::process::id()));
    let src = if ["jpg", "jpeg", "png", "webp"].contains(&ext.as_str()) { input.to_path_buf() } else {
        run_cmd("convert", &[&format!("{}[0]", s(input)), "-auto-orient", &s(&tmp)])?;
        tmp.clone()
    };
    let out = out_path(input, "_4x", if fmt == "jpg" { &ext } else { fmt });
    let o = Command::new(&bin).args(["-i", &s(&src), "-o", &s(&out), "-s", "4", "-m", &s(&models), "-n", &model, "-f", fmt])
        .stdin(Stdio::null()).output();
    let _ = fs::remove_file(&tmp);
    let o = o.map_err(|e| format!("Upscayl non parte: {}", e))?;
    if !o.status.success() || !out.is_file() {
        let _ = fs::remove_file(&out);
        let e = String::from_utf8_lossy(&o.stderr);
        return Err(format!("Upscayl non è riuscito: {}", e.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("errore sconosciuto").trim()));
    }
    let dims = run_cmd("identify", &["-format", "%w×%h", &s(&out)]).unwrap_or_default();
    Ok(format!("Creato: {} ({})", name(&out), dims.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elenco_valido() {
        let v = clean_list(&json!([
            { "nome": "Upscayl", "tipi": ["image", "pdf"], "modo": "upscayl", "app": "/x/upscayl.AppImage" },
            { "nome": "upscayl", "tipi": ["image"], "modo": "apri", "app": "/x/a.desktop" },
            { "nome": "Copia", "tipi": ["text", "boh"], "modo": "comando", "comando": "cp {file} /tmp" },
            { "nome": "", "tipi": ["text"], "modo": "apri", "app": "x" },
            { "nome": "Senza tipi", "tipi": [], "modo": "apri", "app": "x" },
            { "nome": "Senza comando", "tipi": ["text"], "modo": "comando" },
        ]));
        let names: Vec<&str> = v.as_array().unwrap().iter().map(|x| x["nome"].as_str().unwrap()).collect();
        assert_eq!(names, ["Upscayl", "Copia"], "nomi doppi e scorciatoie incomplete vengono tolti");
        assert_eq!(v[0]["tipi"], json!(["image"]), "Upscayl solo per le immagini");
        assert_eq!(v[1]["tipi"], json!(["text"]));
        assert_eq!(quote("l'anno"), "'l'\\''anno'");
        assert_eq!(expand("cp {file} {cartella}/{nome}.bak", Path::new("/a b/c'd.txt")), "cp '/a b/c'\\''d.txt' '/a b'/'c'\\''d'.bak");
    }

    #[test]
    #[ignore]
    fn comando_vero() {
        let dir = std::env::temp_dir().join("halo-test-scorciatoie");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let cfg = dir.join("config");
        // Solo per questo test: impostazioni in una cartella temporanea.
        std::env::set_var("XDG_CONFIG_HOME", &cfg);
        let f = dir.join("nota di prova.txt");
        fs::write(&f, "ciao").unwrap();
        crate::impostazioni::save(&json!({ "scorciatoie": [
            { "nome": "Duplica", "tipi": ["text"], "modo": "comando", "comando": "cp {file} {cartella}/{nome}_copia.txt && echo copiato" },
            { "nome": "Sbaglia", "tipi": ["text"], "modo": "comando", "comando": "ls /non/esiste" },
        ] })).unwrap();
        assert_eq!(run(&f, 0, "Duplica").unwrap(), "Duplica: copiato");
        assert_eq!(fs::read_to_string(dir.join("nota di prova_copia.txt")).unwrap(), "ciao");
        assert!(run(&f, 1, "Sbaglia").unwrap_err().starts_with("Il comando non è riuscito"));
        assert!(run(&f, 0, "Altro nome").is_err(), "menu vecchio: la scorciatoia non corrisponde");
        assert!(run(&dir, 0, "Duplica").unwrap_err().contains("tipo di file"));
        // Upscayl vero, solo se c'è l'AppImage (HALO_UPSCAYL=/percorso/upscayl.AppImage).
        if let Ok(app) = std::env::var("HALO_UPSCAYL") {
            std::env::set_var("XDG_CACHE_HOME", dir.join("cache"));
            let img = dir.join("foto.png");
            run_cmd("convert", &["-size", "60x40", "plasma:fractal", &s(&img)]).unwrap();
            assert_eq!(upscale(&img, &app).unwrap(), "Creato: foto_4x.png (240×160)");
            std::env::remove_var("XDG_CACHE_HOME");
        }
        std::env::remove_var("XDG_CONFIG_HOME");
    }
}
