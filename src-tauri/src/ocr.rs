//! OCR (riconoscimento del testo) senza Internet, con Tesseract: rende ricercabili i PDF
//! scansionati (ocrmypdf aggiunge il testo sotto l'immagine, la pagina resta uguale) e
//! estrae il testo dalle foto di documenti. È intelligenza artificiale, aggiunta su richiesta.

use crate::actions::{name, out_path, run_cmd, s};
use std::fs;
use std::path::{Path, PathBuf};

pub const KEYS: &[&str] = &["pdf/converti/ocr", "image/converti/testo"];

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    Some(match key {
        "pdf/converti/ocr" => out_path(input, "_ocr", "pdf"),
        "image/converti/testo" => out_path(input, "_testo", "txt"),
        _ => return None,
    })
}

/// Lingue: italiano e inglese se ci sono, altrimenti quelle installate tra le due.
fn langs() -> String {
    let have = run_cmd("tesseract", &["--list-langs"]).unwrap_or_default();
    let v: Vec<&str> = ["ita", "eng"].into_iter().filter(|l| have.lines().any(|x| x.trim() == *l)).collect();
    if v.is_empty() { "eng".into() } else { v.join("+") }
}

fn missing_ita() -> &'static str {
    if run_cmd("tesseract", &["--list-langs"]).unwrap_or_default().lines().any(|l| l.trim() == "ita") { "" }
    else if cfg!(windows) { " (per l'italiano rilancia dipendenze.ps1 dalla cartella di Halo)" }
    else { " (per l'italiano installa anche: sudo apt install tesseract-ocr-ita)" }
}

pub fn run(input: &Path, key: &str) -> Result<String, String> {
    let out = planned_out(input, key).ok_or("Opzione non disponibile.")?;
    match key {
        "pdf/converti/ocr" => {
            // Se il PDF ha già il testo selezionabile l'OCR non serve.
            let text = run_cmd("pdftotext", &["-l", "5", &s(input), "-"]).unwrap_or_default();
            if text.split_whitespace().count() > 2 {
                return Err("Il PDF ha già il testo selezionabile: l'OCR non serve. Per estrarlo usa Converti › Testo.".into());
            }
            let r = run_cmd("ocrmypdf", &["-q", "-l", &langs(), "--skip-text", "--output-type", "pdf", &s(input), &s(&out)]);
            if let Err(e) = r {
                let _ = fs::remove_file(&out);
                // Su Windows senza il modulo risponde Python: «No module named ocrmypdf».
                return Err(if e.contains("Manca") || e.contains("No module named ocrmypdf") { crate::actions::missing("ocrmypdf") }
                    else if e.contains("encrypted") || e.contains("password") { "Il PDF è protetto da password: toglila prima.".into() } else { e });
            }
            let words = run_cmd("pdftotext", &[&s(&out), "-"]).unwrap_or_default().split_whitespace().count();
            Ok(format!("Creato: {} (testo riconosciuto: circa {} parole, ora si può cercare e copiare){}", name(&out), words, missing_ita()))
        }
        "image/converti/testo" => {
            // Tesseract legge meglio un'immagine dritta e in scala di grigi.
            let tmp = std::env::temp_dir().join(format!("halo-ocr-{}.png", std::process::id()));
            run_cmd("convert", &[&format!("{}[0]", s(input)), "-auto-orient", "-colorspace", "Gray", &s(&tmp)])?;
            let r = run_cmd("tesseract", &[&s(&tmp), "stdout", "-l", &langs()]);
            let _ = fs::remove_file(&tmp);
            let text = r?;
            let words = text.split_whitespace().count();
            if words == 0 { return Err("Nessun testo riconosciuto nell'immagine: nessun file creato.".into()); }
            fs::write(&out, text.trim_end().to_string() + "\n").map_err(|e| e.to_string())?;
            Ok(format!("Creato: {} (circa {} parole){}", name(&out), words, missing_ita()))
        }
        _ => Err("Opzione non disponibile.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-ocr");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Una «foto» di un documento: testo disegnato come immagine, e un PDF fatto solo di quell'immagine.
        let img = dir.join("scansione.png");
        let font = if cfg!(windows) { "Arial" } else { "DejaVu-Sans" };
        run_cmd("convert", &["-size", "1400x500", "xc:white", "-font", font, "-pointsize", "48", "-fill", "black",
            "-annotate", "+60+150", "Contratto di collaborazione", "-annotate", "+60+260", "Pagamento entro trenta giorni", &s(&img)]).unwrap();
        let pdf = dir.join("scansione.pdf");
        run_cmd("convert", &[&s(&img), "-density", "150", &s(&pdf)]).unwrap();
        let prima = fs::read(&pdf).unwrap();

        let r = run(&img, "image/converti/testo").unwrap();
        assert!(r.starts_with("Creato: scansione_testo.txt"), "{}", r);
        let t = fs::read_to_string(dir.join("scansione_testo.txt")).unwrap();
        assert!(t.contains("Contratto") && t.contains("Pagamento"), "{}", t);

        let r = run(&pdf, "pdf/converti/ocr").unwrap();
        assert!(r.starts_with("Creato: scansione_ocr.pdf"), "{}", r);
        let t = run_cmd("pdftotext", &[&s(&dir.join("scansione_ocr.pdf")), "-"]).unwrap();
        assert!(t.contains("collaborazione"), "{}", t);
        // Il PDF con il testo non ha bisogno dell'OCR.
        assert!(run(&dir.join("scansione_ocr.pdf"), "pdf/converti/ocr").is_err());
        let vuota = dir.join("vuota.png");
        run_cmd("convert", &["-size", "200x200", "xc:white", &s(&vuota)]).unwrap();
        assert!(run(&vuota, "image/converti/testo").unwrap_err().starts_with("Nessun testo"));
        assert_eq!(fs::read(&pdf).unwrap(), prima, "l'originale non va toccato");
    }
}
