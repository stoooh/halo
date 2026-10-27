//! Immagine › Ridimensiona › Upscayl: Real-ESRGAN (realesrgan-ncnn-vulkan, lo stesso motore di
//! Upscayl), incluso nel pacchetto in /usr/lib/halo/realesrgan (Linux) o nella cartella
//! realesrgan accanto a halo.exe (Windows). Funziona in locale, sulla scheda
//! video (Vulkan). I modelli lavorano a 4×: 2× = 4× e poi metà; 6× e 8× = due passaggi
//! (4×, riduzione a 1,5× o 2× dell'originale, di nuovo 4×), così anche il secondo ingrandimento
//! ricostruisce i dettagli.
//! È una funzione di intelligenza artificiale, aggiunta su richiesta esplicita.

use crate::actions::{kind_of, name, out_path, run_cmd, s};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const KEYS: &[&str] = &["image/ridimensiona/upscayl"];
/// Lato massimo del risultato (oltre, ImageMagick e la memoria non bastano).
const MAX_SIDE: u32 = 16000;
const BIN: &str = "realesrgan-ncnn-vulkan";

/// La cartella con programma e modelli: variabile HALO_REALESRGAN, pacchetto (Linux), accanto a
/// halo.exe (Windows), oppure sorgenti.
pub fn dir() -> Option<PathBuf> {
    let exe_dir = std::env::current_exe().ok().and_then(|e| e.parent().map(|p| p.join("realesrgan")));
    [std::env::var("HALO_REALESRGAN").ok().map(PathBuf::from), Some(PathBuf::from("/usr/lib/halo/realesrgan")), exe_dir,
        Some(Path::new(env!("CARGO_MANIFEST_DIR")).join("risorse/realesrgan"))]
        .into_iter().flatten().find(|d| d.join(crate::sistema::exe(BIN)).is_file())
}

/// Scala (2 o 4) e modello dai dati del pannello.
fn options(data: &HashMap<String, String>) -> (u32, &'static str) {
    let scala = match data.get("scala").map(|v| v.as_str()) { Some("2") => 2, Some("6") => 6, Some("8") => 8, _ => 4 };
    let modello = if data.get("tipo").map(|v| v.as_str()) == Some("disegni") { "realesrgan-x4plus-anime" } else { "realesrgan-x4plus" };
    (scala, modello)
}

pub fn planned_out(input: &Path, key: &str, data: &HashMap<String, String>) -> Option<PathBuf> {
    if !KEYS.contains(&key) { return None; }
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let ext = if ["jpg", "jpeg", "webp"].contains(&ext.as_str()) { ext } else { "png".into() };
    Some(out_path(input, &format!("_{}x", options(data).0), &ext))
}

/// Un passaggio del modello (sempre 4×).
fn pass(d: &Path, src: &Path, dst: &Path, modello: &str, fmt: Option<&str>, tmp: &Path) -> Result<(), String> {
    let mut args = vec!["-i".to_string(), s(src), "-o".into(), s(dst), "-n".into(), modello.into(), "-s".into(), "4".into(), "-m".into(), s(&d.join("models"))];
    if let Some(f) = fmt { args.extend(["-f".to_string(), f.to_string()]); }
    // Real-ESRGAN scrive l'avanzamento come «12,50%» sull'errore standard.
    let (status, _, e) = crate::lavoro::run_lines(crate::sistema::no_window(Command::new(d.join(crate::sistema::exe(BIN)))).args(&args).current_dir(tmp), true, |l| {
        if let Some(p) = l.strip_suffix('%').and_then(|v| v.trim().replace(',', ".").parse::<f64>().ok()) { crate::lavoro::set(p / 100.0); }
    }).map_err(|e| format!("Il programma di ingrandimento non parte: {}", e))?;
    if crate::lavoro::cancelled() { return Err("Annullato.".into()); }
    if status.success() && dst.is_file() { return Ok(()); }
    if e.contains("vkCreateInstance") || e.contains("invalid gpu") {
        return Err(if cfg!(windows) { "Serve una scheda video compatibile con Vulkan: aggiorna i driver della scheda video." }
            else { "Serve una scheda video compatibile con Vulkan (o il pacchetto mesa-vulkan-drivers)." }.to_string());
    }
    Err(format!("Ingrandimento non riuscito: {}", e.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("errore sconosciuto").trim()))
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    if kind_of(input) != "image" { return Err("Funziona con le immagini.".into()); }
    let d = dir().ok_or("Manca il programma di ingrandimento: reinstalla Halo.")?;
    let (scala, modello) = options(data);
    let out = planned_out(input, key, data).ok_or("Opzione non disponibile.")?;
    let fmt = out.extension().and_then(|e| e.to_str()).map(|e| if e == "jpeg" { "jpg" } else { e }).unwrap_or("png").to_string();
    let tmp = std::env::temp_dir().join(format!("halo-ingrandisci-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = (|| {
        // Il programma legge JPEG, PNG e WebP: il resto (e le foto ruotate) passano da un PNG.
        let src = tmp.join("in.png");
        run_cmd("convert", &[&format!("{}[0]", s(input)), "-auto-orient", &s(&src)])?;
        let wh: Vec<u32> = run_cmd("identify", &["-format", "%w %h", &s(&src)])?.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        let side = wh.iter().copied().max().unwrap_or(0);
        // Il passaggio più grande è a 4× (e per 6×/8× anche il risultato finale).
        if side * scala.max(4) > MAX_SIDE {
            return Err(format!("Troppo grande: a {}× verrebbe {} pixel di lato (massimo {}). Scegli una scala più piccola.", scala, side * scala.max(4), MAX_SIDE));
        }
        let x4 = tmp.join(format!("4x.{}", if scala == 4 { fmt.as_str() } else { "png" }));
        // Con due passaggi (6×, 8×) ognuno vale metà dell'avanzamento.
        crate::lavoro::range(0.0, if scala > 4 { 0.5 } else { 1.0 });
        pass(&d, &src, &x4, modello, (scala == 4).then_some(fmt.as_str()), &tmp)?;
        match scala {
            4 => fs::copy(&x4, &out).map(|_| ()).map_err(|e| e.to_string()),
            2 => run_cmd("convert", &[&s(&x4), "-filter", "Lanczos", "-resize", "50%", &s(&out)]).map(|_| ()),
            _ => {
                // 6×: 4× ridotto a 1,5× dell'originale (37,5%); 8×: ridotto a 2× (50%). Poi di nuovo 4×.
                let mid = tmp.join("mid.png");
                run_cmd("convert", &[&s(&x4), "-filter", "Lanczos", "-resize", if scala == 6 { "37.5%" } else { "50%" }, &s(&mid)])?;
                crate::lavoro::range(0.5, 0.5);
                pass(&d, &mid, &tmp.join(format!("fin.{}", fmt)), modello, Some(&fmt), &tmp)?;
                fs::copy(tmp.join(format!("fin.{}", fmt)), &out).map(|_| ()).map_err(|e| e.to_string())
            }
        }
    })();
    let _ = fs::remove_dir_all(&tmp);
    if let Err(e) = r { let _ = fs::remove_file(&out); return Err(e); }
    Ok(format!("Creato: {}", name(&out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomi_e_opzioni() {
        let d = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>();
        assert_eq!(options(&d(&[])), (4, "realesrgan-x4plus"));
        assert_eq!(options(&d(&[("scala", "2"), ("tipo", "disegni")])), (2, "realesrgan-x4plus-anime"));
        assert_eq!(options(&d(&[("scala", "8")])).0, 8);
        assert_eq!(options(&d(&[("scala", "7")])).0, 4, "scala non prevista: 4×");
        let p = planned_out(Path::new("/tmp/foto.heic"), KEYS[0], &d(&[("scala", "2")])).unwrap();
        assert_eq!(name(&p), "foto_2x.png");
        assert_eq!(name(&planned_out(Path::new("/tmp/foto.JPG"), KEYS[0], &d(&[])).unwrap()), "foto_4x.jpg");
        assert!(dir().is_some(), "programma non trovato in risorse/realesrgan");
    }

    /// Esegue davvero Real-ESRGAN: serve la scheda video (Vulkan).
    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-ingrandisci");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let img = dir.join("piccola.jpg");
        run_cmd("convert", &["-seed", "3", "-size", "120x80", "plasma:fractal", &s(&img)]).unwrap();
        let prima = fs::read(&img).unwrap();
        let d = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>();
        assert_eq!(run(&img, KEYS[0], &d(&[])).unwrap(), "Creato: piccola_4x.jpg");
        assert_eq!(run_cmd("identify", &["-format", "%wx%h", &s(&dir.join("piccola_4x.jpg"))]).unwrap(), "480x320");
        assert_eq!(run(&img, KEYS[0], &d(&[("scala", "2"), ("tipo", "disegni")])).unwrap(), "Creato: piccola_2x.jpg");
        assert_eq!(run_cmd("identify", &["-format", "%wx%h", &s(&dir.join("piccola_2x.jpg"))]).unwrap(), "240x160");
        assert_eq!(run(&img, KEYS[0], &d(&[("scala", "6")])).unwrap(), "Creato: piccola_6x.jpg");
        assert_eq!(run_cmd("identify", &["-format", "%wx%h", &s(&dir.join("piccola_6x.jpg"))]).unwrap(), "720x480");
        assert_eq!(run(&img, KEYS[0], &d(&[("scala", "8")])).unwrap(), "Creato: piccola_8x.jpg");
        assert_eq!(run_cmd("identify", &["-format", "%wx%h", &s(&dir.join("piccola_8x.jpg"))]).unwrap(), "960x640");
        // Troppo grande: 3000 px a 8× supererebbe il limite, e non resta nessun file.
        let big = dir.join("grande.png");
        run_cmd("convert", &["-size", "3000x2000", "xc:gray", &s(&big)]).unwrap();
        assert!(run(&big, KEYS[0], &d(&[("scala", "8")])).unwrap_err().starts_with("Troppo grande"));
        assert!(!dir.join("grande_8x.png").exists());
        assert_eq!(fs::read(&img).unwrap(), prima, "l'originale non va toccato");
    }
}
