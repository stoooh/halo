//! Azioni su più file insieme (selezione multipla in Nautilus): uno ZIP con tutti, copie
//! rinominate in serie, PDF o video uniti, percorsi copiati. Gli originali non si toccano.
//! Le azioni normali (comprimi, converti…) su più file le esegue il menu, un file alla volta.

use crate::actions::{name, out_dir, out_path, run_cmd, s};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Stdio;

pub const KEYS: &[&str] = &["multi/insieme/zip", "multi/insieme/per-email", "multi/insieme/rinomina-in-serie", "multi/insieme/data-di-scatto",
    "multi/insieme/unisci", "multi/insieme/unisci-in-pdf", "multi/insieme/percorsi"];

/// Limite tipico degli allegati email.
const EMAIL_MAX: u64 = 25_000_000;

fn check(files: &[PathBuf]) -> Result<(), String> {
    if files.len() < 2 { return Err("Scegli almeno due file.".into()); }
    if let Some(f) = files.iter().find(|f| !f.exists()) { return Err(format!("Il file non esiste più: {}", s(f))); }
    Ok(())
}

/// Nome di base dei risultati: «Primo file e altri N».
fn base(files: &[PathBuf]) -> PathBuf {
    let first = &files[0];
    let stem = if first.is_dir() { name(first) } else { first.file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default() };
    let dir = first.parent().unwrap_or(Path::new("/"));
    dir.join(format!("{} e altri {}", stem, files.len() - 1))
}

pub fn planned_out(files: &[PathBuf], key: &str) -> Option<PathBuf> {
    if files.len() < 2 { return None; }
    Some(match key {
        "multi/insieme/zip" => out_path(&base(files), "", "zip"),
        "multi/insieme/unisci-in-pdf" => out_path(&base(files), "", "pdf"),
        "multi/insieme/per-email" => out_path(&base(files), "_email", "zip"),
        _ => return None,
    })
}

pub fn run(files: &[PathBuf], key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    if !KEYS.contains(&key) { return Err("Opzione non disponibile.".into()); }
    check(files)?;
    match key {
        "multi/insieme/zip" => {
            let out = planned_out(files, key).unwrap();
            let mut args = vec!["a".to_string(), "-tzip".into(), "-bd".into(), s(&out)];
            args.extend(files.iter().map(|f| s(f)));
            let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
            if let Err(e) = run_cmd("7z", &refs) { let _ = fs::remove_file(&out); return Err(e); }
            Ok(format!("Creato: {} ({} file)", name(&out), files.len()))
        }
        "multi/insieme/rinomina-in-serie" => {
            // Copie con il nuovo nome in una cartella nuova: «Nome 01.jpg», «Nome 02.jpg»…
            let nome = data.get("valore").map(|v| v.trim()).unwrap_or("");
            if nome.is_empty() || nome.contains('/') { return Err("Scrivi il nome di base (senza «/»).".into()); }
            let dir = out_dir(&files[0].parent().unwrap_or(Path::new("/")).join(nome), "");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let mut sorted = files.to_vec();
            sorted.sort_by_key(|f| name(f).to_lowercase());
            let w = sorted.len().to_string().len().max(2);
            for (k, f) in sorted.iter().enumerate() {
                let ext = f.extension().and_then(|e| e.to_str()).filter(|_| f.is_file()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
                let dest = dir.join(format!("{} {:0w$}{}", nome, k + 1, ext, w = w));
                let ok = crate::sistema::copy_all(f, &dest).is_ok();
                if !ok { let _ = fs::remove_dir_all(&dir); return Err(format!("Non riesco a copiare {}.", name(f))); }
            }
            Ok(format!("Creato: {} ({} copie rinominate)", name(&dir), sorted.len()))
        }
        "multi/insieme/per-email" => per_email(files),
        "multi/insieme/unisci-in-pdf" => {
            // Una pagina per immagine, nell'ordine della selezione, raddrizzate (foto del telefono).
            if let Some(f) = files.iter().find(|f| crate::actions::kind_of(f) != "image") {
                return Err(format!("«{}» non è un'immagine: Unisci in PDF funziona solo con le immagini.", name(f)));
            }
            let out = planned_out(files, key).unwrap();
            let mut args: Vec<String> = files.iter().map(|f| format!("{}[0]", s(f))).collect();
            args.extend(["-auto-orient".into(), "-units".into(), "PixelsPerInch".into(), "-density".into(), "150".into(), s(&out)]);
            let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
            if let Err(e) = run_cmd("convert", &refs) { let _ = fs::remove_file(&out); return Err(e); }
            Ok(format!("Creato: {} ({} pagine)", name(&out), files.len()))
        }
        "multi/insieme/data-di-scatto" => {
            // Copie con il nome della data di scatto, in una cartella nuova; i file senza data restano col loro nome.
            let dir = out_dir(&files[0].parent().unwrap_or(Path::new("/")).join("Per data di scatto"), "");
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let names = crate::cartelle::shot_names(files);
            let mut n = 0;
            for (f, nm) in files.iter().zip(&names) {
                let ext = f.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
                let wanted = match nm { Some(d) => { n += 1; format!("{}{}", d, ext) } None => name(f) };
                // Due foto nello stesso secondo: «… (2).jpg».
                let stem = wanted.strip_suffix(&ext).unwrap_or(&wanted).to_string();
                let mut dest = dir.join(&wanted);
                let mut k = 2;
                while dest.exists() { dest = dir.join(format!("{} ({}){}", stem, k, ext)); k += 1; }
                let ok = crate::sistema::copy_all(f, &dest).is_ok();
                if !ok { let _ = fs::remove_dir_all(&dir); return Err(format!("Non riesco a copiare {}.", name(f))); }
            }
            if n == 0 { let _ = fs::remove_dir_all(&dir); return Err("Nessuno dei file ha una data di scatto nei metadati: nessuna copia creata.".into()); }
            Ok(format!("Creato: {} ({} di {} file con la data di scatto)", name(&dir), n, files.len()))
        }
        "multi/insieme/unisci" => {
            // PDF o video uniti nell'ordine della selezione: usa le opzioni Unisci già esistenti.
            let kinds: Vec<&str> = files.iter().map(|f| crate::actions::kind_of(f)).collect();
            let key = match kinds[0] {
                "pdf" if kinds.iter().all(|k| *k == "pdf") => "pdf/pagine/unisci",
                "video" if kinds.iter().all(|k| *k == "video") => "video/taglia/unisci",
                _ => return Err("Unisci funziona con soli PDF o soli video.".into()),
            };
            let rest = files[1..].iter().map(|f| s(f)).collect::<Vec<_>>().join("\n");
            crate::actions::run_with(&s(&files[0]), key, &HashMap::from([("file".to_string(), rest)]))
        }
        "multi/insieme/percorsi" => {
            let text = files.iter().map(|f| s(f)).collect::<Vec<_>>().join("\n");
            crate::condividi::copy_text(&text)?;
            Ok(format!("{} percorsi copiati negli appunti.", files.len()))
        }
        _ => Err("Opzione non disponibile.".into()),
    }
}

/// Pacchetto per email: ogni file alleggerito (PDF, foto, video, audio), poi un unico ZIP.
/// I file che non si possono alleggerire (o che non diventano più piccoli) entrano così come sono.
fn per_email(files: &[PathBuf]) -> Result<String, String> {
    let out = out_path(&base(files), "_email", "zip");
    let tmp = std::env::temp_dir().join(format!("halo-email-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = (|| {
        let mut lighter = 0;
        let mut before = 0u64;
        for (k, f) in files.iter().enumerate() {
            crate::lavoro::range(k as f64 / files.len() as f64, 1.0 / files.len() as f64);
            if crate::lavoro::cancelled() { return Err("Annullato.".to_string()); }
            before += du(f);
            // Nome libero nella cartella temporanea (due file con lo stesso nome non si sovrascrivono).
            let dest_name = |ext: &str| {
                let stem = f.file_stem().map(|x| x.to_string_lossy().into_owned()).unwrap_or_default();
                let mut d = tmp.join(format!("{}.{}", stem, ext));
                let mut k = 2;
                while d.exists() { d = tmp.join(format!("{} ({}).{}", stem, k, ext)); k += 1; }
                d
            };
            let small = match crate::actions::kind_of(f) {
                "pdf" => { let d = dest_name("pdf"); run_cmd("gs", &["-sDEVICE=pdfwrite", "-dCompatibilityLevel=1.6", "-dPDFSETTINGS=/ebook", "-dNOPAUSE", "-dBATCH", "-dQUIET", &format!("-sOutputFile={}", s(&d)), &s(f)]).ok().map(|_| d) }
                "image" => { let d = dest_name("jpg"); run_cmd("convert", &[&format!("{}[0]", s(f)), "-auto-orient", "-resize", "1600x1600>", "-strip", "-quality", "80", &s(&d)]).ok().map(|_| d) }
                "video" => { let d = dest_name("mp4"); run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-y", "-i", &s(f), "-vf", "scale=-2:'min(720,ih)'", "-c:v", "libx264", "-crf", "30", "-preset", "medium", "-c:a", "aac", "-b:a", "96k", &s(&d)]).ok().map(|_| d) }
                "audio" => { let d = dest_name("mp3"); run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-y", "-i", &s(f), "-vn", "-b:a", "128k", &s(&d)]).ok().map(|_| d) }
                _ => None,
            };
            // Si tiene la versione alleggerita solo se è davvero più piccola; altrimenti l'originale.
            match small {
                Some(d) if f.is_file() && fs::metadata(&d).map(|m| m.len()).unwrap_or(u64::MAX) < du(f) => lighter += 1,
                Some(d) => { let _ = fs::remove_file(&d); copy_in(f, &tmp)?; }
                None => copy_in(f, &tmp)?,
            }
        }
        let list: Vec<String> = fs::read_dir(&tmp).map_err(|e| e.to_string())?.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        let mut args = vec!["a".to_string(), "-tzip".into(), "-mx=9".into(), "-bd".into(), s(&out)];
        args.extend(list);
        let o = crate::sistema::cmd("7z").args(&args).current_dir(&tmp).stdin(Stdio::null()).output().map_err(|e| e.to_string())?;
        if !o.status.success() { return Err("7z non è riuscito a creare lo ZIP.".into()); }
        let size = fs::metadata(&out).map(|m| m.len()).unwrap_or(0);
        let mut msg = format!("Creato: {} ({} file, {} alleggeriti: {} → {})", name(&out), files.len(), lighter,
            crate::actions::human(before), crate::actions::human(size));
        if size > EMAIL_MAX {
            msg.push_str("\nSupera 25 MB: molti servizi email non lo accettano. Prova Condividi › Link.");
        }
        Ok(msg)
    })();
    let _ = fs::remove_dir_all(&tmp);
    if r.is_err() { let _ = fs::remove_file(&out); }
    r
}

/// Dimensione di un file o di una cartella (somma dei file).
fn du(p: &Path) -> u64 {
    crate::sistema::size_and_count(p).0
}

/// Copia (file o cartella) dentro la cartella temporanea, con un nome libero.
fn copy_in(f: &Path, dir: &Path) -> Result<(), String> {
    let mut dest = dir.join(name(f));
    let mut k = 2;
    while dest.exists() { dest = dir.join(format!("{} ({})", name(f), k)); k += 1; }
    let ok = crate::sistema::copy_all(f, &dest).is_ok();
    if ok { Ok(()) } else { Err(format!("Non riesco a copiare {}.", name(f))) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-multi");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("cartella")).unwrap();
        let a = dir.join("Bozza.txt");
        let b = dir.join("appunti.TXT");
        fs::write(&a, "uno").unwrap();
        fs::write(&b, "due").unwrap();
        fs::write(dir.join("cartella/c.txt"), "tre").unwrap();
        let files = vec![a.clone(), b.clone(), dir.join("cartella")];
        let d = |k: &str, v: &str| HashMap::from([(k.to_string(), v.to_string())]);

        assert_eq!(run(&files, "multi/insieme/zip", &HashMap::new()).unwrap(), "Creato: Bozza e altri 2.zip (3 file)");
        let list = run_cmd("7z", &["l", "-slt", &s(&dir.join("Bozza e altri 2.zip"))]).unwrap();
        assert!(list.contains("Path = Bozza.txt") && list.contains("Path = cartella/c.txt") || list.contains("Path = cartella"), "{}", list);

        assert_eq!(run(&files, "multi/insieme/rinomina-in-serie", &d("valore", "Note")).unwrap(), "Creato: Note (3 copie rinominate)");
        // Ordine alfabetico: appunti, Bozza, cartella.
        assert_eq!(fs::read_to_string(dir.join("Note/Note 01.txt")).unwrap(), "due");
        assert_eq!(fs::read_to_string(dir.join("Note/Note 02.txt")).unwrap(), "uno");
        assert_eq!(fs::read_to_string(dir.join("Note/Note 03/c.txt")).unwrap(), "tre");
        assert!(a.exists() && b.exists(), "gli originali restano");

        assert!(run(&files, "multi/insieme/unisci", &HashMap::new()).unwrap_err().contains("soli PDF"));
        let p1 = dir.join("uno.pdf");
        let p2 = dir.join("due.pdf");
        run_cmd("convert", &["xc:white", &s(&p1)]).unwrap();
        run_cmd("convert", &["xc:white", "xc:white", &s(&p2)]).unwrap();
        assert_eq!(run(&[p1.clone(), p2], "multi/insieme/unisci", &HashMap::new()).unwrap(), "Creato: uno_unito.pdf (2 PDF uniti)");
        assert!(run(&[p1.clone()], "multi/insieme/zip", &HashMap::new()).is_err());

        // Unisci in PDF: due immagini, due pagine; con un file non immagine rifiuta.
        let i1 = dir.join("uno.png");
        let i2 = dir.join("due.jpg");
        run_cmd("convert", &["-size", "300x200", "xc:red", &s(&i1)]).unwrap();
        run_cmd("convert", &["-size", "200x300", "xc:blue", &s(&i2)]).unwrap();
        assert_eq!(run(&[i1.clone(), i2.clone()], "multi/insieme/unisci-in-pdf", &HashMap::new()).unwrap(), "Creato: uno e altri 1.pdf (2 pagine)");
        assert!(run_cmd("pdfinfo", &[&s(&dir.join("uno e altri 1.pdf"))]).unwrap().contains("Pages:           2"));
        assert!(run(&[i1, a.clone()], "multi/insieme/unisci-in-pdf", &HashMap::new()).unwrap_err().contains("non è un'immagine"));

        // Per email: una foto grande viene alleggerita, il testo entra così com'è.
        let foto = dir.join("grande.png");
        run_cmd("convert", &["-size", "3000x2000", "plasma:fractal", &s(&foto)]).unwrap();
        let r = run(&[foto.clone(), a.clone()], "multi/insieme/per-email", &HashMap::new()).unwrap();
        assert!(r.starts_with("Creato: grande e altri 1_email.zip (2 file, 1 alleggeriti:"), "{}", r);
        let list = run_cmd("7z", &["l", "-slt", &s(&dir.join("grande e altri 1_email.zip"))]).unwrap();
        assert!(list.contains("Path = grande.jpg") && list.contains("Path = Bozza.txt"), "{}", list);
        assert!(foto.exists());

        // Data di scatto: la foto con la data viene copiata con il nuovo nome, l'altra col suo.
        let scatto = dir.join("IMG_9.JPG");
        run_cmd("convert", &["-size", "20x20", "xc:blue", &s(&scatto)]).unwrap();
        run_cmd("exiftool", &["-q", "-overwrite_original", "-DateTimeOriginal=2026:08:15 18:30:45", &s(&scatto)]).unwrap();
        assert_eq!(run(&[scatto.clone(), a.clone()], "multi/insieme/data-di-scatto", &HashMap::new()).unwrap(),
            "Creato: Per data di scatto (1 di 2 file con la data di scatto)");
        assert!(dir.join("Per data di scatto/2026-08-15 18.30.45.jpg").exists() && dir.join("Per data di scatto/Bozza.txt").exists());
        assert!(scatto.exists());
        assert!(run(&[a.clone(), b.clone()], "multi/insieme/data-di-scatto", &HashMap::new()).unwrap_err().starts_with("Nessuno"));
    }
}
