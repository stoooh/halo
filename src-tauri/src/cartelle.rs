//! Azioni sulle cartelle: Rinomina, Organizza, Pulisci, Analizza.
//!
//! Rinomina, Organizza e Pulisci per natura cambiano i file: per rispettare la regola d'oro
//! lavorano su una COPIA della cartella, creata accanto con un nome libero; l'originale e
//! tutto ciò che contiene restano come sono. Ogni operazione controlla di agire solo dentro
//! la copia. Analizza invece legge soltanto e mostra un riepilogo.

use crate::actions::{human, kind_of, name, parent, run_cmd, s};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

/// Chiavi gestite da questo modulo (vanno anche in actions::SUPPORTED).
pub const KEYS: &[&str] = &[
    "folder/rinomina/in-serie", "folder/rinomina/aggiungi-data", "folder/rinomina/numera", "folder/rinomina/minuscole",
    "folder/organizza/per-tipo", "folder/organizza/per-data", "folder/organizza/per-dimensione", "folder/organizza/appiattisci",
    "folder/pulisci/duplicati", "folder/pulisci/file-vuoti", "folder/pulisci/temporanei", "folder/pulisci/file-grandi",
    "folder/analizza/tipi-di-file", "folder/analizza/modifiche-recenti", "folder/analizza/cerca",
    "folder/rinomina/data-di-scatto",
];

/// File considerati temporanei da "Pulisci › Temporanei".
fn is_temp(n: &str) -> bool {
    let l = n.to_lowercase();
    l.ends_with('~') || l.ends_with(".tmp") || l.ends_with(".temp") || l.ends_with(".bak") || l.ends_with(".swp")
        || l.starts_with("._") || l == ".ds_store" || l == "thumbs.db" || l == "desktop.ini"
}

/// "Progetto cliente" → "Progetto cliente_<suffisso>", con " (2)", " (3)"… se esiste già.
pub fn copy_dir_path(input: &Path, suffix: &str) -> PathBuf {
    let dir = parent(input);
    let base = format!("{}{}", name(input), suffix);
    let mut c = dir.join(&base);
    let mut n = 2;
    while c.exists() {
        c = dir.join(format!("{} ({})", base, n));
        n += 1;
    }
    c
}

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let suffix = match key.split('/').nth(1)? {
        "rinomina" => "_rinominata",
        "organizza" => "_organizzata",
        "pulisci" if key != "folder/pulisci/file-grandi" => "_pulita",
        _ => return None,
    };
    Some(copy_dir_path(input, suffix))
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    if !input.is_dir() {
        return Err("Serve una cartella.".into());
    }
    match key {
        "folder/analizza/tipi-di-file" => tipi(input),
        "folder/analizza/modifiche-recenti" => recenti(input),
        "folder/analizza/cerca" => {
            let q = data.get("valore").map(|v| v.trim().to_lowercase()).filter(|v| !v.is_empty())
                .ok_or("Scrivi cosa cercare.")?;
            cerca(input, &q)
        }
        "folder/pulisci/file-grandi" => grandi(input),
        _ => {
            let out = planned_out(input, key).ok_or("Opzione non disponibile.")?;
            let base_name = data.get("valore").map(|v| v.trim().to_string());
            // Copia completa (cp -a mantiene date e permessi; reflink è istantaneo dove il disco lo permette).
            crate::sistema::copy_all(input, &out)?;
            let r = modify_copy(&out, key, base_name.as_deref());
            if r.is_err() {
                let _ = fs::remove_dir_all(&out);
            }
            let n = r?;
            // Niente da fare (0 nomi cambiati, 0 doppioni…): la copia sarebbe identica, quindi si toglie.
            if n.starts_with("0 ") {
                let _ = fs::remove_dir_all(&out);
                return Err(format!("Niente da fare ({}): nessuna copia creata.", n));
            }
            Ok(format!("Creato: {} ({})", name(&out), n))
        }
    }
}

/// Esegue l'operazione dentro la copia `root`. Restituisce un breve riepilogo.
fn modify_copy(root: &Path, key: &str, base_name: Option<&str>) -> Result<String, String> {
    let g = Guard(root.to_path_buf());
    match key {
        "folder/rinomina/in-serie" => {
            let b = base_name.filter(|b| !b.is_empty() && !b.contains('/') && b.chars().count() <= 100)
                .ok_or("Scrivi il nome di base (senza «/»).")?;
            let files = top_files(root)?;
            let w = files.len().to_string().len().max(2);
            for (k, f) in files.iter().enumerate() {
                g.rename(f, &with_ext(&format!("{} {:0w$}", b, k + 1, w = w), f))?;
            }
            Ok(count(files.len(), "file rinominato", "file rinominati"))
        }
        "folder/rinomina/data-di-scatto" => {
            // Foto e video con il nome della data di scatto («2026-08-15 18.30.45.jpg»); gli altri restano come sono.
            let files = top_files(root)?;
            let names = shot_names(&files);
            let mut n = 0;
            for (f, nm) in files.iter().zip(&names) {
                if let Some(d) = nm { g.rename(f, &with_ext(d, f).to_lowercase_ext())?; n += 1; }
            }
            Ok(count(n, "foto rinominata con la data di scatto", "foto rinominate con la data di scatto"))
        }
        "folder/rinomina/aggiungi-data" => {
            let files = top_files(root)?;
            for f in &files {
                let d = crate::sistema::modified_local(f, "%Y-%m-%d")?;
                g.rename(f, &format!("{} {}", d, name(f)))?;
            }
            Ok(count(files.len(), "file rinominato", "file rinominati"))
        }
        "folder/rinomina/numera" => {
            let files = top_files(root)?;
            let w = files.len().to_string().len().max(2);
            for (k, f) in files.iter().enumerate() {
                g.rename(f, &format!("{:0w$} - {}", k + 1, name(f), w = w))?;
            }
            Ok(count(files.len(), "file numerato", "file numerati"))
        }
        "folder/rinomina/minuscole" => {
            // Dal più profondo al più esterno, così i percorsi restano validi.
            let mut all = walk(root)?;
            all.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
            let mut n = 0;
            for p in all {
                let low = name(&p).to_lowercase();
                if low != name(&p) {
                    g.rename(&p, &low)?;
                    n += 1;
                }
            }
            Ok(count(n, "nome cambiato", "nomi cambiati"))
        }
        "folder/organizza/per-tipo" | "folder/organizza/per-data" | "folder/organizza/per-dimensione" => {
            let files = top_files(root)?;
            for f in &files {
                let dir = match key {
                    "folder/organizza/per-tipo" => tipo_it(kind_of(f)).to_string(),
                    "folder/organizza/per-data" => crate::sistema::modified_local(f, "%Y-%m")?,
                    _ => {
                        let b = fs::metadata(f).map(|m| m.len()).unwrap_or(0);
                        if b < 1_000_000 { "Piccoli (meno di 1 MB)" } else if b < 100_000_000 { "Medi (1-100 MB)" } else { "Grandi (oltre 100 MB)" }.to_string()
                    }
                };
                g.move_into(f, &root.join(dir))?;
            }
            Ok(count(files.len(), "file sistemato", "file sistemati"))
        }
        "folder/organizza/appiattisci" => {
            let mut n = 0;
            for p in walk(root)? {
                if p.is_file() && p.parent() != Some(root) {
                    g.move_into(&p, root)?;
                    n += 1;
                }
            }
            remove_empty_dirs(&g, root)?;
            Ok(count(n, "file spostato in cima", "file spostati in cima"))
        }
        "folder/pulisci/duplicati" => {
            // Stesso contenuto byte per byte: si tiene il primo in ordine di percorso.
            let mut files: Vec<PathBuf> = walk(root)?.into_iter().filter(|p| p.is_file()).collect();
            files.sort();
            let mut by_size: BTreeMap<u64, Vec<PathBuf>> = BTreeMap::new();
            for f in files {
                by_size.entry(fs::metadata(&f).map(|m| m.len()).unwrap_or(0)).or_default().push(f);
            }
            let mut n = 0;
            for (_, group) in by_size.into_iter().filter(|(size, g)| *size > 0 && g.len() > 1) {
                let mut kept: Vec<Vec<u8>> = Vec::new();
                for f in group {
                    let bytes = fs::read(&f).map_err(|e| e.to_string())?;
                    if kept.contains(&bytes) {
                        g.remove_file(&f)?;
                        n += 1;
                    } else {
                        kept.push(bytes);
                    }
                }
            }
            Ok(count(n, "doppione tolto", "doppioni tolti"))
        }
        "folder/pulisci/file-vuoti" => {
            let mut n = 0;
            for p in walk(root)? {
                if p.is_file() && fs::metadata(&p).map(|m| m.len() == 0).unwrap_or(false) {
                    g.remove_file(&p)?;
                    n += 1;
                }
            }
            n += remove_empty_dirs(&g, root)?;
            Ok(count(n, "elemento vuoto tolto", "elementi vuoti tolti"))
        }
        "folder/pulisci/temporanei" => {
            let mut n = 0;
            for p in walk(root)? {
                if p.is_file() && is_temp(&name(&p)) {
                    g.remove_file(&p)?;
                    n += 1;
                }
            }
            Ok(count(n, "file temporaneo tolto", "file temporanei tolti"))
        }
        _ => Err("Opzione non disponibile.".into()),
    }
}

/// Impedisce qualsiasi modifica fuori dalla copia.
struct Guard(PathBuf);

impl Guard {
    fn check(&self, p: &Path) -> Result<(), String> {
        if p.starts_with(&self.0) && p != self.0 { Ok(()) } else { Err(format!("Operazione fuori dalla copia: {}", s(p))) }
    }
    /// Rinomina nello stesso posto; se il nome esiste già aggiunge " (2)"…
    fn rename(&self, p: &Path, new_name: &str) -> Result<(), String> {
        if new_name == name(p) { return Ok(()); }
        let dest = free_name(&parent(p), new_name, Some(p));
        self.check(p)?;
        self.check(&dest)?;
        fs::rename(p, dest).map_err(|e| e.to_string())
    }
    fn move_into(&self, p: &Path, dir: &Path) -> Result<(), String> {
        self.check(p)?;
        fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let dest = free_name(dir, &name(p), None);
        self.check(&dest)?;
        fs::rename(p, dest).map_err(|e| e.to_string())
    }
    fn remove_file(&self, p: &Path) -> Result<(), String> {
        self.check(p)?;
        fs::remove_file(p).map_err(|e| e.to_string())
    }
}

/// Un nome libero in `dir`; un file che cambia solo maiuscole/minuscole non conta come conflitto con sé stesso.
fn free_name(dir: &Path, wanted: &str, itself: Option<&Path>) -> PathBuf {
    let same = |c: &Path| itself.map(|i| name(i).to_lowercase() == name(c).to_lowercase() && parent(i) == parent(c)).unwrap_or(false);
    let mut c = dir.join(wanted);
    let (stem, ext) = match wanted.rfind('.') {
        Some(i) if i > 0 => (&wanted[..i], &wanted[i..]),
        _ => (wanted, ""),
    };
    let mut n = 2;
    while c.exists() && !same(&c) {
        c = dir.join(format!("{} ({}){}", stem, n, ext));
        n += 1;
    }
    c
}

/// Rimuove le cartelle vuote (dalla più profonda), tranne la radice. Restituisce quante.
fn remove_empty_dirs(g: &Guard, root: &Path) -> Result<usize, String> {
    let mut dirs: Vec<PathBuf> = walk(root)?.into_iter().filter(|p| p.is_dir()).collect();
    dirs.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    let mut n = 0;
    for d in dirs {
        if fs::read_dir(&d).map(|mut r| r.next().is_none()).unwrap_or(false) {
            g.check(&d)?;
            fs::remove_dir(&d).map_err(|e| e.to_string())?;
            n += 1;
        }
    }
    Ok(n)
}

/// Tutti i file e le cartelle sotto `root` (senza seguire i collegamenti simbolici).
fn walk(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in fs::read_dir(&d).map_err(|e| e.to_string())? {
            let p = e.map_err(|e| e.to_string())?.path();
            let meta = fs::symlink_metadata(&p).map_err(|e| e.to_string())?;
            if meta.is_dir() {
                stack.push(p.clone());
            }
            out.push(p);
        }
    }
    Ok(out)
}

/// I file (non le cartelle, non i collegamenti) direttamente dentro `root`, in ordine di nome.
fn top_files(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut v: Vec<PathBuf> = fs::read_dir(root).map_err(|e| e.to_string())?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| fs::symlink_metadata(p).map(|m| m.is_file()).unwrap_or(false))
        .collect();
    v.sort_by_key(|p| name(p).to_lowercase());
    Ok(v)
}

fn with_ext(base: &str, f: &Path) -> String {
    match f.extension() {
        Some(e) => format!("{}.{}", base, e.to_string_lossy()),
        None => base.to_string(),
    }
}

/// Data di scatto di foto e video (EXIF / metadati del video) come «2026-08-15 18.30.45»;
/// None per i file che non ce l'hanno. Un solo exiftool per tutti i file.
pub fn shot_names(files: &[PathBuf]) -> Vec<Option<String>> {
    if files.is_empty() { return Vec::new(); }
    let mut args: Vec<String> = ["-j", "-q", "-d", "%Y-%m-%d %H.%M.%S", "-DateTimeOriginal", "-CreateDate", "-MediaCreateDate"].iter().map(|x| x.to_string()).collect();
    args.extend(files.iter().map(|f| s(f)));
    let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
    let json: Vec<serde_json::Value> = run_cmd("exiftool", &refs).ok().and_then(|o| serde_json::from_str(&o).ok()).unwrap_or_default();
    files.iter().map(|f| {
        // Su Windows exiftool scrive i percorsi con «/»: Path li confronta senza badare al separatore.
        let item = json.iter().find(|j| j["SourceFile"].as_str().map(Path::new) == Some(f.as_path()))?;
        ["DateTimeOriginal", "CreateDate", "MediaCreateDate"].iter().filter_map(|k| item[*k].as_str())
            // I video senza data scrivono «0000:00:00 00:00:00».
            .find(|v| v.len() == 19 && !v.starts_with("0000")).map(String::from)
    }).collect()
}

/// L'estensione del nome in minuscolo («.JPG» → «.jpg»), il resto com'è.
trait LowerExt { fn to_lowercase_ext(&self) -> String; }
impl LowerExt for String {
    fn to_lowercase_ext(&self) -> String {
        match self.rsplit_once('.') { Some((b, e)) => format!("{}.{}", b, e.to_lowercase()), None => self.clone() }
    }
}

fn count(n: usize, one: &str, many: &str) -> String {
    format!("{} {}", n, if n == 1 { one } else { many })
}

fn tipo_it(kind: &str) -> &'static str {
    match kind {
        "pdf" => "PDF", "image" => "Immagini", "audio" => "Audio", "video" => "Video",
        "document" => "Documenti", "sheet" => "Fogli", "archive" => "Archivi", "text" => "Testi",
        _ => "Altro",
    }
}

fn tipi(root: &Path) -> Result<String, String> {
    let mut by: BTreeMap<&str, (usize, u64)> = BTreeMap::new();
    for p in walk(root)?.into_iter().filter(|p| p.is_file()) {
        let e = by.entry(tipo_it(kind_of(&p))).or_default();
        e.0 += 1;
        e.1 += fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
    }
    if by.is_empty() { return Ok("La cartella non contiene file.".into()); }
    let mut v: Vec<_> = by.into_iter().collect();
    v.sort_by(|a, b| b.1 .0.cmp(&a.1 .0));
    Ok(v.iter().map(|(t, (n, b))| format!("{}: {} ({})", t, n, human(*b))).collect::<Vec<_>>().join("\n"))
}

fn recenti(root: &Path) -> Result<String, String> {
    let mut v: Vec<(SystemTime, PathBuf)> = walk(root)?.into_iter().filter(|p| p.is_file())
        .filter_map(|p| fs::metadata(&p).and_then(|m| m.modified()).ok().map(|t| (t, p))).collect();
    if v.is_empty() { return Ok("La cartella non contiene file.".into()); }
    v.sort_by(|a, b| b.0.cmp(&a.0));
    let lines: Vec<String> = v.iter().take(6).map(|(_, p)| {
        let d = crate::sistema::modified_local(p, "%d/%m %H:%M").unwrap_or_default();
        format!("{} {}", d.trim(), s(p.strip_prefix(root).unwrap_or(p)))
    }).collect();
    Ok(format!("Modificati di recente:\n{}", lines.join("\n")))
}

fn cerca(root: &Path, q: &str) -> Result<String, String> {
    let found: Vec<String> = walk(root)?.iter().filter(|p| name(p).to_lowercase().contains(q))
        .map(|p| s(p.strip_prefix(root).unwrap_or(p))).collect();
    if found.is_empty() { return Ok(format!("Nessun nome contiene «{}».", q)); }
    let mut lines: Vec<&str> = found.iter().map(|x| x.as_str()).take(8).collect();
    lines.sort();
    let more = if found.len() > 8 { format!("\n…e altri {}", found.len() - 8) } else { String::new() };
    Ok(format!("{} trovati:\n{}{}", found.len(), lines.join("\n"), more))
}

fn grandi(root: &Path) -> Result<String, String> {
    let mut v: Vec<(u64, PathBuf)> = walk(root)?.into_iter().filter(|p| p.is_file())
        .map(|p| (fs::metadata(&p).map(|m| m.len()).unwrap_or(0), p)).collect();
    if v.is_empty() { return Ok("La cartella non contiene file.".into()); }
    v.sort_by(|a, b| b.0.cmp(&a.0));
    let lines: Vec<String> = v.iter().take(6).map(|(b, p)| format!("{} {}", human(*b), s(p.strip_prefix(root).unwrap_or(p)))).collect();
    Ok(format!("I file più grandi (nessuno è stato toccato):\n{}", lines.join("\n")))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cartella d'esempio con doppioni, vuoti, temporanei, sottocartelle e maiuscole.
    fn esempio(root: &Path) -> PathBuf {
        let _ = fs::remove_dir_all(root);
        let d = root.join("Progetto Cliente");
        fs::create_dir_all(d.join("Sotto/Profonda")).unwrap();
        fs::create_dir_all(d.join("Vuota")).unwrap();
        fs::write(d.join("Foto.JPG"), "immagine").unwrap();
        fs::write(d.join("copia di foto.jpg"), "immagine").unwrap();
        fs::write(d.join("Relazione.pdf"), "pdf").unwrap();
        fs::write(d.join("note.txt"), "").unwrap();
        fs::write(d.join("bozza.txt~"), "x").unwrap();
        fs::write(d.join("Sotto/Profonda/Dati.CSV"), "a,b").unwrap();
        fs::write(d.join("Sotto/note.txt"), "altro").unwrap();
        d
    }

    fn albero(p: &Path) -> Vec<String> {
        let mut v: Vec<String> = walk(p).unwrap().iter().map(|x| s(x.strip_prefix(p).unwrap())).collect();
        v.sort();
        v
    }

    #[test]
    fn operazioni_su_copia() {
        let root = std::env::temp_dir().join("halo-test-cartelle");
        let d = esempio(&root);
        let prima: Vec<(String, Vec<u8>)> = albero(&d).into_iter().map(|r| {
            let b = fs::read(d.join(&r)).unwrap_or_default();
            (r, b)
        }).collect();
        let data = HashMap::from([("valore".to_string(), "Vacanze".to_string())]);

        for key in KEYS {
            // Provata a parte (data_di_scatto): qui le foto d'esempio non hanno la data nei metadati.
            if *key == "folder/rinomina/data-di-scatto" { continue; }
            let r = run(&d, key, &data);
            assert!(r.is_ok(), "{}: {:?}", key, r);
        }
        let dopo: Vec<(String, Vec<u8>)> = albero(&d).into_iter().map(|r| {
            let b = fs::read(d.join(&r)).unwrap_or_default();
            (r, b)
        }).collect();
        assert_eq!(prima, dopo, "l'originale è cambiato");

        let c = |sfx: &str| root.join(format!("Progetto Cliente_{}", sfx));
        // Le copie si chiamano _rinominata, _rinominata (2)… nell'ordine di KEYS.
        assert!(albero(&c("rinominata")).contains(&"Vacanze 03.JPG".to_string()));
        assert!(albero(&c("rinominata (2)")).iter().any(|n| n.ends_with(" Foto.JPG") && n.starts_with("20")));
        assert!(albero(&c("rinominata (3)")).contains(&"01 - bozza.txt~".to_string()));
        let low = albero(&c("rinominata (4)"));
        assert!(low.contains(&"sotto/profonda/dati.csv".to_string()) && low.contains(&"foto.jpg".to_string()), "{:?}", low);
        assert!(albero(&c("organizzata")).contains(&"PDF/Relazione.pdf".to_string()));
        assert!(albero(&c("organizzata (4)")).contains(&"Dati.CSV".to_string()));
        assert!(albero(&c("organizzata (4)")).contains(&"note (2).txt".to_string()), "{:?}", albero(&c("organizzata (4)")));
        let dup = albero(&c("pulita"));
        assert_eq!(dup.iter().filter(|n| n.to_lowercase().ends_with(".jpg")).count(), 1, "{:?}", dup);
        let vuoti = albero(&c("pulita (2)"));
        assert!(!vuoti.contains(&"note.txt".to_string()) && !vuoti.contains(&"Vuota".to_string()));
        assert!(!albero(&c("pulita (3)")).contains(&"bozza.txt~".to_string()));

        assert!(run(&d, "folder/analizza/cerca", &data).unwrap().starts_with("Nessun nome"));
        let q = HashMap::from([("valore".to_string(), "note".to_string())]);
        assert!(run(&d, "folder/analizza/cerca", &q).unwrap().starts_with("2 trovati"));
        assert!(run(&d, "folder/rinomina/in-serie", &HashMap::new()).is_err());
        assert!(!root.join("Progetto Cliente_rinominata (5)").exists(), "copia lasciata dopo un errore");
    }

    #[test]
    #[ignore]
    fn data_di_scatto() {
        let root = std::env::temp_dir().join("halo-test-scatto");
        let _ = fs::remove_dir_all(&root);
        let d = root.join("Viaggio");
        fs::create_dir_all(&d).unwrap();
        for (n, when) in [("IMG_1.JPG", Some("2026:08:15 18:30:45")), ("IMG_2.jpg", Some("2026:08:16 09:05:00")), ("nota.txt", None)] {
            let p = d.join(n);
            if n.ends_with("txt") { fs::write(&p, "x").unwrap(); continue; }
            run_cmd("convert", &["-size", "20x20", "xc:red", &s(&p)]).unwrap();
            run_cmd("exiftool", &["-q", "-overwrite_original", &format!("-DateTimeOriginal={}", when.unwrap()), &s(&p)]).unwrap();
        }
        assert_eq!(run(&d, "folder/rinomina/data-di-scatto", &HashMap::new()).unwrap(), "Creato: Viaggio_rinominata (2 foto rinominate con la data di scatto)");
        let mut v = albero(&root.join("Viaggio_rinominata"));
        v.sort();
        assert_eq!(v, ["2026-08-15 18.30.45.jpg", "2026-08-16 09.05.00.jpg", "nota.txt"]);
        assert!(d.join("IMG_1.JPG").exists(), "l'originale resta");
    }

    #[test]
    fn guardia() {
        let g = Guard(PathBuf::from("/tmp/x/copia"));
        assert!(g.check(Path::new("/tmp/x/copia/a")).is_ok());
        assert!(g.check(Path::new("/tmp/x/copia")).is_err());
        assert!(g.check(Path::new("/tmp/x/originale/a")).is_err());
        assert!(g.check(Path::new("/tmp/x/copia2/a")).is_err());
        assert!(is_temp("bozza.txt~") && is_temp(".DS_Store") && !is_temp("relazione.pdf"));
    }
}
