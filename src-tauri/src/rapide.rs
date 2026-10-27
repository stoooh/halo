//! Opzioni rapide rimaste: pagine dei PDF, anteprima e ricompressione degli archivi,
//! foto da un video, copia del percorso di una cartella.

use crate::actions::{duration, human, name, out_dir, out_path, run_cmd, s, seven_in};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const KEYS: &[&str] = &[
    "pdf/pagine/dividi", "pdf/pagine/riordina", "pdf/pagine/estrai",
    "archive/anteprima/dimensioni", "archive/anteprima/cerca", "archive/proteggi/verifica-integrita",
    "archive/ricomprimi/massima", "archive/ricomprimi/rapida", "archive/ricomprimi/per-email",
    "video/fotogrammi/estrai-foto", "folder/condividi/copia-percorso",
];

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let arch = if ext == "7z" { "7z" } else { "zip" };
    Some(match key {
        "pdf/pagine/dividi" => out_dir(input, "_pagine"),
        "pdf/pagine/riordina" => out_path(input, "_riordinato", "pdf"),
        "pdf/pagine/estrai" => out_path(input, "_estratto", "pdf"),
        "archive/ricomprimi/massima" => out_path(input, "_massima", arch),
        "archive/ricomprimi/rapida" => out_path(input, "_rapida", arch),
        "archive/ricomprimi/per-email" => out_path(input, "_email", "zip"),
        "video/fotogrammi/estrai-foto" => out_dir(input, "_foto"),
        _ => return None,
    })
}

pub fn page_count(input: &Path) -> Result<usize, String> {
    run_cmd("pdfinfo", &[&s(input)])?.lines().find_map(|l| l.strip_prefix("Pages:"))
        .and_then(|v| v.trim().parse().ok()).ok_or_else(|| "Pagine del PDF non lette.".to_string())
}

/// "3, 1, 2, 5-7" → "3,1,2,5-7" se tutte le pagine esistono (1..=n).
pub fn parse_order(v: &str, n: usize) -> Option<String> {
    let mut parts = Vec::new();
    for p in v.split(',').map(|p| p.trim()).filter(|p| !p.is_empty()) {
        let ok = |x: &str| x.trim().parse::<usize>().ok().filter(|k| (1..=n).contains(k));
        match p.split_once('-') {
            Some((a, b)) => parts.push(format!("{}-{}", ok(a)?, ok(b)?)),
            None => parts.push(ok(p)?.to_string()),
        }
    }
    (!parts.is_empty()).then(|| parts.join(","))
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let i = s(input);
    let valore = data.get("valore").map(|v| v.trim()).unwrap_or("");
    match key {
        "pdf/pagine/dividi" => {
            let n = page_count(input)?;
            if n < 2 { return Err("Il PDF ha una sola pagina.".into()); }
            let dir = planned_out(input, key).unwrap();
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let stem = input.file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_else(|| "pagina".into());
            run_cmd("qpdf", &["--warning-exit-0", "--split-pages", &i, &s(&dir.join(format!("{}.pdf", stem)))])?;
            Ok(format!("Creato: {} ({} pagine)", name(&dir), n))
        }
        "pdf/pagine/riordina" => {
            let n = page_count(input)?;
            let order = parse_order(valore, n)
                .ok_or_else(|| format!("Scrivi le pagine da 1 a {} separate da virgole, per esempio 3, 1, 2, 4-{}.", n, n.max(4)))?;
            let out = planned_out(input, key).unwrap();
            run_cmd("qpdf", &["--warning-exit-0", "--empty", "--pages", &i, &order, "--", &s(&out)])?;
            Ok(format!("Creato: {}", name(&out)))
        }
        "pdf/pagine/estrai" => {
            // Solo le pagine scritte, nell'ordine scritto: un PDF nuovo, l'originale resta intero.
            let n = page_count(input)?;
            let order = parse_order(valore, n)
                .ok_or_else(|| format!("Scrivi le pagine da tenere (da 1 a {}), per esempio 2, 5-7.", n))?;
            let out = planned_out(input, key).unwrap();
            run_cmd("qpdf", &["--warning-exit-0", "--empty", "--pages", &i, &order, "--", &s(&out)])?;
            let got = page_count(&out).unwrap_or(0);
            Ok(format!("Creato: {} ({} {} su {})", name(&out), got, if got == 1 { "pagina" } else { "pagine" }, n))
        }
        "archive/anteprima/dimensioni" => {
            let list = crate::actions::seven_list(input)?;
            let (mut files, mut size) = (0u64, 0u64);
            for block in list.split("\n\n").filter(|b| b.contains("\nPath = ") || b.starts_with("Path = ")) {
                if block.lines().any(|l| l == "Folder = +") || !block.contains("Size = ") { continue; }
                if let Some(sz) = block.lines().find_map(|l| l.strip_prefix("Size = ")).and_then(|v| v.trim().parse::<u64>().ok()) {
                    files += 1;
                    size += sz;
                }
            }
            let packed = fs::metadata(input).map(|m| m.len()).unwrap_or(0);
            let ratio = if size > 0 { (packed as f64 / size as f64 * 100.0).round() as u64 } else { 100 };
            Ok(format!("{} file, {} in tutto\nArchivio: {} ({}% dell'originale)", files, human(size), human(packed), ratio.min(999)))
        }
        "archive/anteprima/cerca" => {
            if valore.is_empty() { return Err("Scrivi cosa cercare.".into()); }
            let q = valore.to_lowercase();
            let list = crate::actions::seven_list(input)?;
            let found: Vec<&str> = list.lines().filter_map(|l| l.strip_prefix("Path = ")).skip(1)
                .filter(|p| p.rsplit('/').next().unwrap_or(p).to_lowercase().contains(&q)).collect();
            if found.is_empty() { return Ok(format!("Nessun file dell'archivio contiene «{}» nel nome.", valore)); }
            let more = if found.len() > 8 { format!("\n…e altri {}", found.len() - 8) } else { String::new() };
            Ok(format!("{} trovati:\n{}{}", found.len(), found[..found.len().min(8)].join("\n"), more))
        }
        "archive/proteggi/verifica-integrita" => match seven_in(&["t", &i], "\n") {
            Ok(_) => Ok("Archivio integro: nessun errore trovato.".into()),
            Err(e) if e == "Password errata." => Err("L'archivio è protetto da password: per verificarlo serve la password.".into()),
            Err(e) => Err(format!("L'archivio ha degli errori: {}", e)),
        },
        "archive/ricomprimi/massima" | "archive/ricomprimi/rapida" | "archive/ricomprimi/per-email" => {
            let out = planned_out(input, key).unwrap();
            let level = if key.ends_with("rapida") { "-mx=1" } else { "-mx=9" };
            crate::actions::repack(input, &out, None, None, &[level])?;
            let size = fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
            let note = if key.ends_with("per-email") && size > 20_000_000 {
                "; supera i 20 MB accettati da molti servizi email: usa Dividi in parti"
            } else { "" };
            Ok(format!("Creato: {} ({} → {}{})", name(&out), human(fs::metadata(input).map(|m| m.len()).unwrap_or(0)), human(size), note))
        }
        "video/fotogrammi/estrai-foto" => {
            let every: f64 = valore.replace(',', ".").parse().ok().filter(|v: &f64| (0.1..=600.0).contains(v))
                .ok_or("Scrivi ogni quanti secondi estrarre una foto (da 0,1 a 600).")?;
            let count = (duration(input)? / every).ceil() as u64;
            if count > 500 { return Err(format!("Verrebbero {} foto: aumenta l'intervallo (massimo 500 foto).", count)); }
            let dir = planned_out(input, key).unwrap();
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let r = run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-n", "-i", &i, "-vf", &format!("fps=1/{}", every),
                "-q:v", "2", &s(&dir.join("foto_%04d.jpg"))]);
            if let Err(e) = r { let _ = fs::remove_dir_all(&dir); return Err(e); }
            let n = fs::read_dir(&dir).map(|d| d.count()).unwrap_or(0);
            Ok(format!("Creato: {} ({} foto)", name(&dir), n))
        }
        "folder/condividi/copia-percorso" => {
            crate::sistema::clipboard_text(&i)?;
            Ok(format!("Percorso copiato negli appunti:\n{}", i))
        }
        _ => Err("Opzione non disponibile.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordine_pagine() {
        assert_eq!(parse_order("3, 1, 2", 3), Some("3,1,2".into()));
        assert_eq!(parse_order("2-4,1", 5), Some("2-4,1".into()));
        for bad in ["", "0", "6", "1,a", "1-9"] { assert_eq!(parse_order(bad, 5), None, "{}", bad); }
    }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-rapide");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let pdf = dir.join("tre.pdf");
        run_cmd("gs", &["-q", "-dBATCH", "-dNOPAUSE", "-sDEVICE=pdfwrite", &format!("-sOutputFile={}", s(&pdf)), "-c",
            "/Helvetica findfont 20 scalefont setfont 72 700 moveto (UNO) show showpage 72 700 moveto (DUE) show showpage 72 700 moveto (TRE) show showpage"]).unwrap();
        let d = |v: &str| HashMap::from([("valore".to_string(), v.to_string())]);

        assert_eq!(run(&pdf, "pdf/pagine/dividi", &d("")).unwrap(), "Creato: tre_pagine (3 pagine)");
        assert_eq!(fs::read_dir(dir.join("tre_pagine")).unwrap().count(), 3);
        run(&pdf, "pdf/pagine/riordina", &d("3, 1")).unwrap();
        let t = run_cmd("pdftotext", &[&s(&dir.join("tre_riordinato.pdf")), "-"]).unwrap();
        assert_eq!(t.split_whitespace().collect::<Vec<_>>(), ["TRE", "UNO"]);
        assert!(run(&pdf, "pdf/pagine/riordina", &d("4")).is_err());
        assert_eq!(run(&pdf, "pdf/pagine/estrai", &d("2-3")).unwrap(), "Creato: tre_estratto.pdf (2 pagine su 3)");
        let t = run_cmd("pdftotext", &[&s(&dir.join("tre_estratto.pdf")), "-"]).unwrap();
        assert_eq!(t.split_whitespace().collect::<Vec<_>>(), ["DUE", "TRE"]);
        assert!(run(&pdf, "pdf/pagine/estrai", &d("0, 9")).is_err());

        let src = dir.join("cartella");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("relazione finale.txt"), "testo ".repeat(5000)).unwrap();
        fs::write(src.join("foto.txt"), "x").unwrap();
        let zip = dir.join("a.zip");
        run_cmd("7z", &["a", "-tzip", "-mx=0", &s(&zip), &s(&src)]).unwrap();
        assert!(run(&zip, "archive/anteprima/dimensioni", &d("")).unwrap().starts_with("2 file, 30 KB in tutto"));
        assert!(run(&zip, "archive/anteprima/cerca", &d("relazione")).unwrap().starts_with("1 trovati:\ncartella/relazione finale.txt"));
        assert_eq!(run(&zip, "archive/proteggi/verifica-integrita", &d("")).unwrap(), "Archivio integro: nessun errore trovato.");
        run(&zip, "archive/ricomprimi/massima", &d("")).unwrap();
        run(&zip, "archive/ricomprimi/rapida", &d("")).unwrap();
        let (m, z) = (fs::metadata(dir.join("a_massima.zip")).unwrap().len(), fs::metadata(&zip).unwrap().len());
        assert!(m < z / 10, "massima {} non più piccola di {}", m, z);
        assert!(seven_in(&["t", &s(&dir.join("a_rapida.zip"))], "\n").is_ok());

        let video = dir.join("v.mp4");
        run_cmd("ffmpeg", &["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=25:d=3",
            "-pix_fmt", "yuv420p", &s(&video)]).unwrap();
        assert_eq!(run(&video, "video/fotogrammi/estrai-foto", &d("1")).unwrap(), "Creato: v_foto (3 foto)");
        assert!(run(&video, "video/fotogrammi/estrai-foto", &d("0,001")).is_err());
    }
}
