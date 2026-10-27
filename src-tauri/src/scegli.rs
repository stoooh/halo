//! Opzioni che usano altri file scelti con la finestra «Apri file»: unire PDF e video,
//! sostituire l'audio di un video, copertina di un brano, firma su un PDF, aggiungere file
//! a un archivio, estrarre un archivio in una cartella scelta.
//! I file scelti arrivano dal pannello nel campo "file", uno per riga.

use crate::actions::{has_audio, name, out_dir, out_path, run_cmd, s, seven_in, VIDEO_HQ};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const KEYS: &[&str] = &[
    "pdf/pagine/unisci", "pdf/annota/firma", "video/taglia/unisci", "video/audio/sostituisci",
    "audio/tag/copertina", "archive/contenuto/aggiungi-file", "archive/estrai/scegli-dove",
];

/// Che cosa far scegliere nella finestra «Apri file» per ogni opzione.
pub fn pick_kind(key: &str) -> Option<&'static str> {
    Some(match key {
        "pdf/pagine/unisci" => "pdf",
        "video/taglia/unisci" => "video",
        "video/audio/sostituisci" => "audio",
        "audio/tag/copertina" | "pdf/annota/firma" | "document/proteggi/firma" => "immagine",
        "archive/contenuto/aggiungi-file" => "file",
        "archive/estrai/scegli-dove" => "cartella",
        "archive/contenuto/sostituisci" | "other/verifica/confronta" => "unfile",
        _ => return None,
    })
}

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    Some(match key {
        "pdf/pagine/unisci" => out_path(input, "_unito", "pdf"),
        "pdf/annota/firma" => out_path(input, "_firmato", "pdf"),
        "video/taglia/unisci" => out_path(input, "_unito", crate::actions::video_ext(&ext)),
        "video/audio/sostituisci" => out_path(input, "_nuovo_audio", crate::actions::video_ext(&ext)),
        "audio/tag/copertina" => out_path(input, "_copertina", &ext),
        "archive/contenuto/aggiungi-file" => out_path(input, "_aggiornato", if ext == "7z" { "7z" } else { "zip" }),
        _ => return None,
    })
}

/// I file scelti: esistenti, diversi dall'originale; cartelle solo per "Scegli dove".
fn chosen(input: &Path, data: &HashMap<String, String>, folder: bool) -> Result<Vec<PathBuf>, String> {
    let v: Vec<PathBuf> = data.get("file").map(|f| f.lines().filter(|l| !l.trim().is_empty()).map(PathBuf::from).collect()).unwrap_or_default();
    if v.is_empty() { return Err("Scegli prima i file.".into()); }
    for p in &v {
        let ok = if folder { p.is_dir() } else { p.is_file() };
        if !ok { return Err(format!("Non trovo «{}».", s(p))); }
        if !folder && p.canonicalize().ok() == input.canonicalize().ok() {
            return Err("Hai scelto lo stesso file di partenza.".into());
        }
    }
    Ok(v)
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let i = s(input);
    let out = planned_out(input, key);
    match key {
        "pdf/pagine/unisci" => {
            let files = chosen(input, data, false)?;
            let out = out.unwrap();
            let mut args: Vec<String> = vec!["--warning-exit-0".into(), "--empty".into(), "--pages".into(), i.clone()];
            args.extend(files.iter().map(|f| s(f)));
            args.extend(["--".into(), s(&out)]);
            let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
            run_cmd("qpdf", &refs).map_err(|e| if e.contains("password") { "Uno dei PDF è protetto da password: toglila prima.".into() } else { e })?;
            Ok(format!("Creato: {} ({} PDF uniti)", name(&out), files.len() + 1))
        }
        "video/taglia/unisci" => {
            let files = chosen(input, data, false)?;
            let out = out.unwrap();
            let all: Vec<PathBuf> = std::iter::once(input.to_path_buf()).chain(files).collect();
            let audio: Vec<bool> = all.iter().map(|f| has_audio(f)).collect();
            let with_audio = audio.iter().all(|a| *a);
            if !with_audio && audio.iter().any(|a| *a) {
                return Err("Alcuni video hanno l'audio e altri no: non si possono unire così.".into());
            }
            // Tutti alla risoluzione del primo, con bande nere se le proporzioni sono diverse.
            let wh = run_cmd("ffprobe", &["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x", &i])?;
            let (w, h) = wh.trim().split_once('x').ok_or("Video non leggibile.")?;
            let mut filter = String::new();
            for k in 0..all.len() {
                filter.push_str(&format!("[{k}:v]scale={w}:{h}:force_original_aspect_ratio=decrease,pad={w}:{h}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30,format=yuv420p[v{k}];"));
                if with_audio { filter.push_str(&format!("[{k}:a]aresample=48000,aformat=channel_layouts=stereo[a{k}];")); }
            }
            for k in 0..all.len() { filter.push_str(&format!("[v{k}]")); if with_audio { filter.push_str(&format!("[a{k}]")); } }
            filter.push_str(&format!("concat=n={}:v=1:a={}[v]{}", all.len(), with_audio as u8, if with_audio { "[a]" } else { "" }));
            let mut args: Vec<String> = vec!["-hide_banner".into(), "-nostdin".into(), "-n".into()];
            for f in &all { args.push("-i".into()); args.push(s(f)); }
            args.extend(["-filter_complex".into(), filter, "-map".into(), "[v]".into()]);
            if with_audio { args.extend(["-map".into(), "[a]".into()]); }
            args.extend(VIDEO_HQ.iter().map(|a| a.to_string()));
            args.push(s(&out));
            let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
            run_cmd("ffmpeg", &refs)?;
            Ok(format!("Creato: {} ({} video uniti)", name(&out), all.len()))
        }
        "video/audio/sostituisci" => {
            let a = chosen(input, data, false)?.remove(0);
            let out = out.unwrap();
            run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-n", "-i", &i, "-i", &s(&a), "-map", "0:v:0", "-map", "1:a:0",
                "-c:v", "copy", "-c:a", "aac", "-b:a", "192k", "-shortest", &s(&out)])?;
            Ok(format!("Creato: {}", name(&out)))
        }
        "audio/tag/copertina" => {
            let img = chosen(input, data, false)?.remove(0);
            let out = out.unwrap();
            // La copertina dev'essere JPEG o PNG: le altre immagini si convertono prima in JPEG.
            let iext = img.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            let tmp = std::env::temp_dir().join(format!("halo-copertina-{}.jpg", std::process::id()));
            let cover = if ["jpg", "jpeg", "png"].contains(&iext.as_str()) { img.clone() } else {
                run_cmd("convert", &[&format!("{}[0]", s(&img)), "-resize", "1400x1400>", &s(&tmp)])?;
                tmp.clone()
            };
            let mut args = vec!["-hide_banner", "-nostdin", "-n"];
            let c = s(&cover);
            let o = s(&out);
            args.extend_from_slice(&["-i", &i, "-i", &c, "-map", "0:a", "-map", "1:v", "-c", "copy", "-disposition:v:0", "attached_pic"]);
            if out.extension().and_then(|e| e.to_str()) == Some("mp3") { args.extend_from_slice(&["-id3v2_version", "3"]); }
            args.push(&o);
            let r = run_cmd("ffmpeg", &args);
            let _ = fs::remove_file(&tmp);
            r.map_err(|e| if e.contains("codec") { "Questo formato audio non supporta la copertina: prova con un MP3 o un M4A.".into() } else { e })?;
            Ok(format!("Creato: {}", name(&out)))
        }
        "pdf/annota/firma" => {
            let img = chosen(input, data, false)?.remove(0);
            let out = out.unwrap();
            let pos = data.get("posizione").and_then(|p| p.parse::<usize>().ok()).filter(|p| *p < 9).unwrap_or(8);
            let all_pages = data.get("pagine").map(|p| p == "tutte").unwrap_or(false);
            crate::annota::stamp_image(input, &out, &img, pos, all_pages)?;
            Ok(format!("Creato: {}", name(&out)))
        }
        "archive/contenuto/aggiungi-file" => {
            let files = chosen(input, data, false)?;
            let out = out.unwrap();
            // Si lavora su una copia; un archivio protetto non si può aggiornare senza password.
            if seven_in(&["t", &i], "\n").is_err_and(|e| e == "Password errata.") {
                return Err("L'archivio ha una password: toglila prima con Proteggi › Rimuovi password.".into());
            }
            let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
            if ext == "zip" || ext == "7z" {
                fs::copy(input, &out).map_err(|e| e.to_string())?;
            } else {
                crate::actions::repack(input, &out, None, None, &[])?;
            }
            let mut args: Vec<String> = vec!["a".into(), s(&out)];
            args.extend(files.iter().map(|f| s(f)));
            let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
            if let Err(e) = seven_in(&refs, "") { let _ = fs::remove_file(&out); return Err(e); }
            Ok(format!("Creato: {} ({} file aggiunti)", name(&out), files.len()))
        }
        "archive/estrai/scegli-dove" => {
            let dest = chosen(input, data, true)?.remove(0);
            // Dentro la cartella scelta, in una sottocartella con il nome dell'archivio (mai sopra file esistenti).
            let target = out_dir(&dest.join(name(input)), "");
            seven_in(&["x", &format!("-o{}", s(&target)), &i], "\n").map_err(|e| {
                let _ = fs::remove_dir_all(&target);
                if e == "Password errata." { "L'archivio ha una password: toglila prima con Proteggi › Rimuovi password.".to_string() } else { e }
            })?;
            Ok(format!("Estratto in {}", s(&target)))
        }
        _ => Err("Opzione non disponibile.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(files: &[&Path]) -> HashMap<String, String> {
        HashMap::from([("file".to_string(), files.iter().map(|f| s(f)).collect::<Vec<_>>().join("\n"))])
    }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-scegli");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let gs = |n: &str, t: &str| {
            let p = dir.join(n);
            run_cmd("gs", &["-q", "-dBATCH", "-dNOPAUSE", "-sDEVICE=pdfwrite", &format!("-sOutputFile={}", s(&p)), "-c",
                &format!("/Helvetica findfont 20 scalefont setfont 72 700 moveto ({}) show showpage", t)]).unwrap();
            p
        };
        let (a, b) = (gs("a.pdf", "PRIMO"), gs("b.pdf", "SECONDO"));
        assert_eq!(run(&a, "pdf/pagine/unisci", &d(&[&b])).unwrap(), "Creato: a_unito.pdf (2 PDF uniti)");
        let t = run_cmd("pdftotext", &[&s(&dir.join("a_unito.pdf")), "-"]).unwrap();
        assert_eq!(t.split_whitespace().collect::<Vec<_>>(), ["PRIMO", "SECONDO"]);
        assert_eq!(run(&a, "pdf/pagine/unisci", &d(&[&a])), Err("Hai scelto lo stesso file di partenza.".into()));
        assert_eq!(run(&a, "pdf/pagine/unisci", &HashMap::new()), Err("Scegli prima i file.".into()));

        let ff = |n: &str, src: &str, dur: &str| {
            let p = dir.join(n);
            let mut args = vec!["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", src];
            if n.ends_with(".mp4") { args.extend_from_slice(&["-f", "lavfi", "-i", "sine=f=440", "-pix_fmt", "yuv420p", "-shortest"]); }
            args.extend_from_slice(&["-t", dur]);
            let ps = s(&p);
            args.push(&ps);
            run_cmd("ffmpeg", &args).unwrap();
            p
        };
        let v1 = ff("v1.mp4", "testsrc=size=640x360:rate=25", "2");
        let v2 = ff("v2.mp4", "testsrc=size=320x240:rate=30", "1");
        run(&v1, "video/taglia/unisci", &d(&[&v2])).unwrap();
        let dur = crate::actions::duration(&dir.join("v1_unito.mp4")).unwrap();
        assert!((dur - 3.0).abs() < 0.2, "durata {}", dur);

        let mus = ff("musica.mp3", "sine=f=660", "5");
        run(&v1, "video/audio/sostituisci", &d(&[&mus])).unwrap();
        assert!(has_audio(&dir.join("v1_nuovo_audio.mp4")));

        let img = dir.join("cover.png");
        run_cmd("convert", &["-size", "300x300", "xc:orange", &s(&img)]).unwrap();
        run(&mus, "audio/tag/copertina", &d(&[&img])).unwrap();
        let st = run_cmd("ffprobe", &["-v", "error", "-show_entries", "stream=codec_type:stream_disposition=attached_pic", "-of", "csv=p=0", &s(&dir.join("musica_copertina.mp3"))]).unwrap();
        assert!(st.contains("video,1"), "{}", st);

        let firma = dir.join("firma.png");
        run_cmd("convert", &["-size", "400x150", "xc:none", "-fill", "navy", "-draw", "line 10,120 390,30", "-draw", "rectangle 10,100 390,110", &s(&firma)]).unwrap();
        let fd = HashMap::from([("file".to_string(), s(&firma)), ("posizione".to_string(), "8".to_string())]);
        run(&a, "pdf/annota/firma", &fd).unwrap();
        let signed = dir.join("a_firmato.pdf");
        assert!(run_cmd("pdftotext", &[&s(&signed), "-"]).unwrap().contains("PRIMO"), "testo originale perso");
        let br: f64 = run_cmd("convert", &["-density", "50", &format!("{}[0]", s(&signed)), "-background", "white", "-flatten", "-gravity", "SouthEast",
            "-crop", "45%x25%+0+0", "-format", "%[fx:mean]", "info:"]).unwrap().trim().parse().unwrap();
        assert!(br < 0.995, "firma non in basso a destra (luminosità {})", br);

        let src = dir.join("dentro");
        fs::create_dir_all(&src).unwrap();
        fs::write(src.join("uno.txt"), "1").unwrap();
        let zip = dir.join("arch.zip");
        run_cmd("7z", &["a", "-tzip", &s(&zip), &s(&src)]).unwrap();
        let prima = fs::read(&zip).unwrap();
        let extra = dir.join("nuovo.txt");
        fs::write(&extra, "2").unwrap();
        run(&zip, "archive/contenuto/aggiungi-file", &d(&[&extra])).unwrap();
        let l = run_cmd("7z", &["l", "-ba", &s(&dir.join("arch_aggiornato.zip"))]).unwrap();
        assert!(l.contains("nuovo.txt") && l.contains("uno.txt"), "{}", l);
        assert_eq!(fs::read(&zip).unwrap(), prima, "l'archivio originale non va toccato");

        let dest = dir.join("destinazione");
        fs::create_dir_all(&dest).unwrap();
        assert!(run(&zip, "archive/estrai/scegli-dove", &d(&[&dest])).unwrap().starts_with("Estratto in"));
        assert!(dest.join("arch").join("dentro/uno.txt").exists());
        run(&zip, "archive/estrai/scegli-dove", &d(&[&dest])).unwrap();
        assert!(dest.join("arch (2)").join("dentro/uno.txt").exists(), "la seconda estrazione deve andare in una cartella nuova");
    }
}
