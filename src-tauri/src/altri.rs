//! Menu per le presentazioni (PowerPoint, Impress) e per tutti gli altri file che Halo non
//! riconosce: comprimi, verifica (impronta SHA-256, confronto), metadati, condividi.
//! Le opzioni condivise con altri tipi riusano quelle esistenti.

use crate::actions::{name, out_dir, run_cmd, s};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const KEYS: &[&str] = &[
    "other/comprimi/zip", "other/comprimi/zip-protetto", "other/comprimi/7z", "slides/comprimi/zip",
    "other/verifica/impronta", "other/verifica/confronta", "other/verifica/metadati", "other/condividi/copia-percorso",
    "slides/converti/pdf", "slides/converti/immagini", "slides/converti/powerpoint", "slides/converti/odp",
    "slides/metadati/mostra", "slides/condividi/copia-percorso",
];

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    match key {
        k if k.starts_with("other/comprimi/") || k == "slides/comprimi/zip" => crate::actions::planned_out(input, &format!("folder/comprimi/{}", k.rsplit('/').next().unwrap())),
        "slides/converti/immagini" => Some(out_dir(input, "_diapositive")),
        _ => None,
    }
}

fn sha256(p: &Path) -> Result<String, String> {
    run_cmd("sha256sum", &[&s(p)])?.split_whitespace().next().map(String::from).ok_or_else(|| "Impronta non calcolata.".into())
}

/// Copia un testo negli appunti (non durante i test, per non toccare gli appunti di chi li lancia).
fn copy(text: &str) -> bool {
    if cfg!(test) { return false; }
    crate::condividi::copy_text(text).is_ok()
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    match key {
        // Comprimere un file singolo funziona come per le cartelle.
        k if k.starts_with("other/comprimi/") || k == "slides/comprimi/zip" => crate::actions::run_with(&s(input), &format!("folder/comprimi/{}", k.rsplit('/').next().unwrap()), data),
        "other/verifica/impronta" => {
            let h = sha256(input)?;
            Ok(format!("SHA-256 di {}:\n{}{}", name(input), h, if copy(&h) { "\n(copiata negli appunti)" } else { "" }))
        }
        "other/verifica/confronta" => {
            let other = data.get("file").and_then(|f| f.lines().next()).map(PathBuf::from).filter(|f| f.is_file()).ok_or("Scegli prima i file.")?;
            let (a, b) = (fs::metadata(input).map_err(|e| e.to_string())?.len(), fs::metadata(&other).map_err(|e| e.to_string())?.len());
            if a == b && sha256(input)? == sha256(&other)? {
                Ok(format!("I due file sono identici (stessa impronta SHA-256).\n{} = {}", name(input), name(&other)))
            } else {
                Ok(format!("I due file sono diversi.\n{}: {}\n{}: {}", name(input), crate::actions::human(a), name(&other), crate::actions::human(b)))
            }
        }
        "other/verifica/metadati" | "slides/metadati/mostra" => {
            let txt = run_cmd("exiftool", &["-S", "-FileType", "-MIMEType", "-Title", "-Author", "-Creator", "-LastModifiedBy",
                "-Company", "-CreateDate", "-ModifyDate", "-Slides", "-Pages", &s(input)])?;
            let txt: Vec<&str> = txt.lines().filter(|l| !l.trim_end().ends_with(':')).collect();
            Ok(if txt.is_empty() { "Nessun metadato rilevante.".into() } else { txt.join("\n") })
        }
        k if k.ends_with("/condividi/copia-percorso") => {
            let p = s(input);
            if !cfg!(test) {
                let st = Command::new("wl-copy").arg(&p).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status()
                    .map_err(|_| "Manca «wl-copy». Installa con: sudo apt install wl-clipboard".to_string())?;
                if !st.success() { return Err("Non sono riuscito a copiare negli appunti.".into()); }
            }
            Ok(format!("Percorso copiato negli appunti:\n{}", p))
        }
        "slides/converti/pdf" => crate::actions::convert_office(input, "pdf"),
        "slides/converti/powerpoint" => crate::actions::convert_office(input, "pptx"),
        "slides/converti/odp" => crate::actions::convert_office(input, "odp"),
        "slides/converti/immagini" => {
            // Prima un PDF temporaneo con LibreOffice, poi un'immagine per diapositiva.
            let dir = out_dir(input, "_diapositive");
            let tmp = std::env::temp_dir().join(format!("halo-slide-{}", std::process::id()));
            let _ = fs::remove_dir_all(&tmp);
            fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
            let r = (|| {
                run_cmd("soffice", &["--headless", "--convert-to", "pdf", "--outdir", &s(&tmp), &s(input)]).map_err(crate::actions::impress_hint)?;
                let pdf = fs::read_dir(&tmp).map_err(|e| e.to_string())?.filter_map(|e| e.ok()).map(|e| e.path())
                    .find(|p| p.extension().is_some_and(|x| x == "pdf")).ok_or("LibreOffice non ha prodotto il PDF.")?;
                fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                run_cmd("pdftoppm", &["-r", "150", "-png", &s(&pdf), &s(&dir.join("diapositiva"))])?;
                Ok::<usize, String>(fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0))
            })();
            let _ = fs::remove_dir_all(&tmp);
            match r {
                Ok(n) => Ok(format!("Creato: {} ({} diapositive in PNG)", name(&dir), n)),
                Err(e) => { let _ = fs::remove_dir_all(&dir); Err(e) }
            }
        }
        _ => Err("Opzione non disponibile.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::kind_of;

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-altri");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("progetto.blend");
        fs::write(&a, "dati di prova").unwrap();
        assert_eq!(kind_of(&a), "other");
        let none = HashMap::new();
        let h = run(&a, "other/verifica/impronta", &none).unwrap();
        assert!(h.ends_with(&sha256(&a).unwrap()) && h.contains(&run_cmd("sha256sum", &[&s(&a)]).unwrap()[..64]));
        let b = dir.join("copia.blend");
        fs::copy(&a, &b).unwrap();
        let d = |f: &Path| HashMap::from([("file".to_string(), s(f))]);
        assert!(run(&a, "other/verifica/confronta", &d(&b)).unwrap().starts_with("I due file sono identici"));
        fs::write(&b, "dati diversi").unwrap();
        assert!(run(&a, "other/verifica/confronta", &d(&b)).unwrap().starts_with("I due file sono diversi"));
        assert_eq!(run(&a, "other/comprimi/zip", &none).unwrap(), "Creato: progetto.zip");

        // Presentazione con un'immagine: PDF, immagini, ODP, comprimi foto, metadati.
        let md = dir.join("slide.md");
        run_cmd("convert", &["-size", "1600x1200", "plasma:fractal", &s(&dir.join("foto.jpg"))]).unwrap();
        fs::write(&md, "---\ntitle: Prova\nauthor: Mario Rossi\n---\n\n# Uno\n\nTesto\n\n# Due\n\n![](foto.jpg)\n").unwrap();
        let ppt = dir.join("Presentazione.pptx");
        let o = Command::new("pandoc").args([s(&md), "-o".into(), s(&ppt)]).current_dir(&dir).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        assert_eq!(kind_of(&ppt), "slides");
        let prima = fs::read(&ppt).unwrap();
        assert_eq!(run(&ppt, "slides/converti/pdf", &none).unwrap(), "Creato: Presentazione.pdf");
        let r = run(&ppt, "slides/converti/immagini", &none).unwrap();
        assert!(r.starts_with("Creato: Presentazione_diapositive (") && r.contains("diapositive in PNG"), "{}", r);
        assert_eq!(run(&ppt, "slides/converti/odp", &none).unwrap(), "Creato: Presentazione.odp");
        assert!(crate::actions::run_with(&s(&ppt), "slides/converti/powerpoint", &none).unwrap_err().contains("già in formato PPTX"));
        assert!(crate::documenti::run(&ppt, "slides/comprimi/immagini-interne").unwrap().contains("foto ridotte"));
        assert!(run(&ppt, "slides/metadati/mostra", &none).unwrap().contains("Mario Rossi"));
        assert_eq!(fs::read(&ppt).unwrap(), prima, "l'originale non va toccato");
    }
}
