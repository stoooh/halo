//! Esito di un'azione per il menu: il messaggio, i file creati (per «Mostra nella cartella») e,
//! se il risultato è un solo file, quanto è cambiato rispetto all'originale (peso e dimensioni).

use crate::actions::{human, kind_of, parent, run_cmd, s};
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Opzioni il cui scopo è un file più piccolo.
const SHRINK: &[&str] = &["pdf/comprimi/", "image/ridimensiona/per-il-web", "image/ridimensiona/per-email", "audio/comprimi/",
    "video/comprimi/", "archive/ricomprimi/massima", "archive/ricomprimi/per-email", "document/comprimi/", "slides/comprimi/"];

#[derive(Serialize)]
pub struct Esito {
    pub msg: String,
    pub creati: Vec<String>,
}

/// Le voci delle cartelle dove può finire il risultato (quella del file e il ripiego sulla Scrivania).
fn snapshot(dirs: &[PathBuf]) -> HashSet<PathBuf> {
    dirs.iter().filter_map(|d| fs::read_dir(d).ok()).flatten().filter_map(|e| e.ok()).map(|e| e.path()).collect()
}

fn dims(p: &Path) -> Option<String> {
    run_cmd("identify", &["-format", "%w×%h", &format!("{}[0]", s(p))]).ok().map(|d| d.trim().to_string()).filter(|d| !d.is_empty())
}

/// «4,2 MB → 1,1 MB (−74%)», con le dimensioni se sono due immagini.
pub fn change(input: &Path, out: &Path) -> Option<String> {
    let (a, b) = (fs::metadata(input).ok()?.len(), fs::metadata(out).ok()?.len());
    if a == 0 { return None; }
    let pct = (b as f64 - a as f64) / a as f64 * 100.0;
    let mut line = format!("{} → {} ({}{:.0}%)", human(a), human(b), if pct < 0.0 { "−" } else { "+" }, pct.abs());
    if kind_of(input) == "image" && kind_of(out) == "image" {
        if let (Some(da), Some(db)) = (dims(input), dims(out)) {
            if da != db { line.push_str(&format!(" · {} → {}", da, db)); }
        }
    }
    Some(line)
}

pub fn run(path: &str, key: &str, data: &HashMap<String, String>) -> Result<Esito, String> {
    let input = Path::new(path);
    let mut dirs = vec![parent(input)];
    if let Some(p) = input.parent() { if !dirs.contains(&p.to_path_buf()) { dirs.push(p.to_path_buf()); } }
    let before = snapshot(&dirs);
    crate::lavoro::start();
    let r = crate::actions::run_with(path, key, data);
    if r.is_err() && crate::lavoro::cancelled() {
        // Annullato: si tolgono i file lasciati a metà. Solo voci nuove che cominciano con il nome
        // del file di partenza (come tutti i risultati), così non si tocca altro.
        let stem = input.file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
        for p in snapshot(&dirs).difference(&before) {
            if !stem.is_empty() && p.file_name().is_some_and(|n| n.to_string_lossy().starts_with(&stem)) {
                let _ = if p.is_dir() { fs::remove_dir_all(p) } else { fs::remove_file(p) };
            }
        }
        crate::lavoro::start();
        return Err("Annullato: nessun file lasciato a metà.".into());
    }
    let mut msg = r?;
    let mut creati: Vec<PathBuf> = snapshot(&dirs).difference(&before).cloned().collect();
    creati.sort();
    // Le opzioni che servono a rimpicciolire il file: se il risultato non è più piccolo non serve
    // tenerlo (il file era già compresso), e lo si dice invece di lasciare un file inutile.
    if SHRINK.iter().any(|k| key.starts_with(k)) {
        if let [one] = creati.as_slice() {
            let (a, b) = (fs::metadata(input).map(|m| m.len()).unwrap_or(0), fs::metadata(one).map(|m| m.len()).unwrap_or(0));
            if one.is_file() && a > 0 && b >= a {
                let _ = fs::remove_file(one);
                return Err(format!("Il file è già compresso: il risultato non sarebbe più piccolo ({} → {}), quindi non l'ho tenuto.", human(a), human(b)));
            }
        }
    }
    // Il confronto si aggiunge solo se il messaggio non lo dice già.
    if let [one] = creati.as_slice() {
        if one.is_file() && input.is_file() && !msg.contains('→') {
            if let Some(c) = change(input, one) {
                let mut lines: Vec<&str> = msg.splitn(2, '\n').collect();
                let first = lines.remove(0).to_string();
                msg = std::iter::once(first).chain(std::iter::once(c)).chain(lines.into_iter().map(String::from)).collect::<Vec<_>>().join("\n");
            }
        }
    }
    let creati: Vec<String> = creati.into_iter().take(50).map(|p| s(&p)).collect();
    // In cronologia solo le operazioni che hanno creato qualcosa (le altre mostrano solo informazioni).
    if !creati.is_empty() { crate::storia::add(input, 1, key, &msg, &creati); }
    Ok(Esito { msg, creati })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn file_creati_e_confronto() {
        let dir = std::env::temp_dir().join("halo-test-esito");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let img = dir.join("foto.png");
        run_cmd("convert", &["-size", "800x600", "plasma:fractal", &s(&img)]).unwrap();
        let e = run(&s(&img), "image/ridimensiona/meta", &HashMap::new()).unwrap();
        assert_eq!(e.creati, [s(&dir.join("foto_50.png"))]);
        let lines: Vec<&str> = e.msg.lines().collect();
        assert!(lines[0].starts_with("Creato: foto_50.png"), "{}", e.msg);
        assert!(lines[1].contains(" → ") && lines[1].contains('%') && lines[1].ends_with("800×600 → 400×300"), "{}", e.msg);
        // Un'azione che non crea file: nessun file e messaggio invariato.
        let e = run(&s(&img), "image/metadati/mostra", &HashMap::new()).unwrap();
        assert!(e.creati.is_empty());
    }

    /// Un PDF minuscolo non si comprime: il risultato più grande non resta.
    #[test]
    #[ignore]
    fn compressione_inutile() {
        let dir = std::env::temp_dir().join("halo-test-compressione");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("piccolo.pdf");
        run_cmd("convert", &["-size", "60x60", "xc:white", &s(&pdf)]).unwrap();
        let e = run(&s(&pdf), "pdf/comprimi/alta-qualita", &HashMap::new()).err().unwrap();
        assert!(e.starts_with("Il file è già compresso"), "{}", e);
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1, "nessun file lasciato");
    }

    /// Avanzamento di ffmpeg e «Annulla» a metà: nessun file lasciato, il video di partenza intatto.
    #[test]
    #[ignore]
    fn avanzamento_e_annulla() {
        let dir = std::env::temp_dir().join("halo-test-annulla");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let v = dir.join("lungo.mp4");
        run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-y", "-f", "lavfi", "-i", "testsrc2=size=1280x720:rate=30:duration=60",
            "-c:v", "libx264", "-preset", "ultrafast", &s(&v)]).unwrap();
        let prima = fs::read(&v).unwrap();
        let t = std::thread::spawn(move || {
            // Aspetta che ffmpeg dica di essere andato avanti, poi annulla.
            for _ in 0..300 {
                std::thread::sleep(std::time::Duration::from_millis(50));
                if crate::lavoro::progress().is_some_and(|p| p > 0.02) { break; }
            }
            let p = crate::lavoro::progress();
            crate::lavoro::cancel();
            p
        });
        let r = run(&s(&v), "video/converti/webm", &HashMap::new());
        let p = t.join().unwrap();
        assert!(p.is_some_and(|p| p > 0.0 && p < 1.0), "avanzamento letto: {:?}", p);
        assert_eq!(r.err().as_deref(), Some("Annullato: nessun file lasciato a metà."));
        let rest: Vec<_> = fs::read_dir(&dir).unwrap().map(|e| e.unwrap().file_name()).collect();
        assert_eq!(rest, ["lungo.mp4"], "file rimasti: {:?}", rest);
        assert_eq!(fs::read(&v).unwrap(), prima);
    }
}
