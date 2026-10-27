//! Annota e Filigrana: testo, evidenziazioni e filigrana su PDF e immagini.
//!
//! Sui PDF si crea con Ghostscript una pagina "trasparente" per ogni pagina del file
//! (stesse dimensioni) e la si sovrappone con `qpdf --overlay`: il testo originale resta
//! selezionabile. Sulle immagini si scrive con ImageMagick.

use crate::actions::{name, out_path, run_cmd, s};
use crate::oscura;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const KEYS: &[&str] = &[
    "pdf/proteggi/filigrana", "pdf/proteggi/limita-stampa", "pdf/annota/testo", "pdf/annota/evidenzia", "image/annota/testo",
];

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    Some(match key {
        "pdf/proteggi/filigrana" => out_path(input, "_filigrana", "pdf"),
        "pdf/proteggi/limita-stampa" => out_path(input, "_senza_stampa", "pdf"),
        "pdf/annota/testo" => out_path(input, "_annotato", "pdf"),
        "pdf/annota/evidenzia" => out_path(input, "_evidenziato", "pdf"),
        "image/annota/testo" => out_path(input, "_annotata", &ext),
        _ => return None,
    })
}

/// Testo del pannello: una riga, non vuota, al massimo 120 caratteri.
fn text(data: &HashMap<String, String>) -> Result<String, String> {
    let v = data.get("valore").map(|v| v.trim()).unwrap_or("");
    if v.is_empty() { return Err("Scrivi il testo.".into()); }
    if v.contains('\n') || v.chars().count() > 120 { return Err("Il testo deve stare su una riga (massimo 120 caratteri).".into()); }
    Ok(v.to_string())
}

/// Posizione 0–8 nella griglia 3×3 (0 = in alto a sinistra, 8 = in basso a destra).
fn position(data: &HashMap<String, String>) -> usize {
    data.get("posizione").and_then(|p| p.parse().ok()).filter(|p| *p < 9).unwrap_or(2)
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let out = planned_out(input, key).ok_or("Opzione non disponibile.")?;
    match key {
        "pdf/proteggi/limita-stampa" => {
            let owner = owner_password();
            let job = serde_json::json!({"inputFile": s(input), "outputFile": s(&out),
                "encrypt": {"userPassword": "", "ownerPassword": owner, "256bit": {"print": "none"}}});
            crate::actions::qpdf_job(job).map_err(|e| if e == "Password errata." { "Il PDF è protetto da password: toglila prima.".into() } else { e })?;
            Ok(format!("Creato: {} (stampa disattivata; alcuni lettori PDF possono ignorare il divieto)", name(&out)))
        }
        "image/annota/testo" => {
            let t = text(data)?;
            const G: [&str; 9] = ["NorthWest", "North", "NorthEast", "West", "Center", "East", "SouthWest", "South", "SouthEast"];
            let h: f64 = run_cmd("identify", &["-format", "%h", &format!("{}[0]", s(input))])?.trim().parse().unwrap_or(600.0);
            let size = (h / 22.0).clamp(14.0, 160.0);
            let margin = (size * 0.8).round();
            run_cmd("convert", &[&s(input), "-gravity", G[position(data)], "-font", "DejaVu-Sans", "-pointsize", &format!("{:.0}", size),
                "-fill", "white", "-undercolor", "#00000099", "-annotate", &format!("+{0}+{0}", margin), &format!(" {} ", t), &s(&out)])?;
            Ok(format!("Creato: {}", name(&out)))
        }
        _ => {
            let pages = page_sizes(input)?;
            let ps = match key {
                "pdf/proteggi/filigrana" => {
                    let t = text(data)?;
                    pages.iter().map(|&(w, h)| watermark_page(w, h, &t)).collect::<Vec<_>>()
                }
                "pdf/annota/testo" => {
                    let t = text(data)?;
                    let first_only = data.get("pagine").map(|p| p == "prima").unwrap_or(false);
                    let n = if first_only { 1 } else { pages.len() };
                    pages.iter().take(n).map(|&(w, h)| note_page(w, h, &t, position(data))).collect()
                }
                _ => {
                    let words = data.get("valore").map(|v| v.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect::<Vec<_>>()).unwrap_or_default();
                    if words.is_empty() { return Err("Scrivi le parole da evidenziare, separate da virgole.".into()); }
                    let boxes = oscura::manual_boxes(input, &words)?;
                    let total: usize = boxes.iter().map(|b| b.len()).sum();
                    if total == 0 { return Err("Nessuna di queste parole compare nel PDF: nessun file creato.".into()); }
                    let pages_ps: Vec<String> = pages.iter().zip(&boxes).map(|(&(w, h), b)| highlight_page(w, h, b)).collect();
                    overlay(input, &out, &pages_ps)?;
                    return Ok(format!("Creato: {} ({} {} evidenziate)", name(&out), total, if total == 1 { "parola" } else { "parole" }));
                }
            };
            overlay(input, &out, &ps)?;
            Ok(format!("Creato: {}", name(&out)))
        }
    }
}

/// Password del proprietario casuale: serve solo a impedire di togliere il divieto di stampa.
fn owner_password() -> String {
    let mut buf = [0u8; 16];
    if let Ok(mut f) = fs::File::open("/dev/urandom") {
        use std::io::Read;
        let _ = f.read_exact(&mut buf);
    }
    buf.iter().map(|x| format!("{:02x}", x)).collect()
}

/// Dimensioni (in punti) di ogni pagina, da pdfinfo.
pub(crate) fn page_sizes(input: &Path) -> Result<Vec<(f64, f64)>, String> {
    let info = run_cmd("pdfinfo", &[&s(input)])?;
    let n: usize = info.lines().find_map(|l| l.strip_prefix("Pages:")).and_then(|v| v.trim().parse().ok())
        .ok_or("Pagine del PDF non lette.")?;
    let all = run_cmd("pdfinfo", &["-f", "1", "-l", &n.to_string(), &s(input)])?;
    let sizes: Vec<(f64, f64)> = all.lines().filter(|l| l.starts_with("Page") && l.contains(" size: ")).filter_map(|l| {
        let v: Vec<f64> = l.split(" size: ").nth(1)?.split_whitespace().filter_map(|x| x.parse().ok()).collect();
        Some((*v.first()?, *v.get(1)?))
    }).collect();
    if sizes.len() != n { return Err("Dimensioni delle pagine non lette.".into()); }
    Ok(sizes)
}

/// Stringa PostScript in Latin-1 (i caratteri che non ci sono diventano equivalenti semplici).
fn ps_string(t: &str) -> String {
    let mut out = String::from("(");
    for c in t.chars() {
        let c = match c { '–' | '—' => '-', '‘' | '’' => '\'', '“' | '”' => '"', '€' => 'E', _ => c };
        match c {
            '(' | ')' | '\\' => { out.push('\\'); out.push(c); }
            ' '..='~' => out.push(c),
            _ if (c as u32) < 256 => out.push_str(&format!("\\{:03o}", c as u32)),
            _ => out.push('?'),
        }
    }
    out.push(')');
    out
}

const PS_HEAD: &str = "%!PS\n/Helvetica findfont dup length dict begin {1 index /FID ne {def} {pop pop} ifelse} forall \
    /Encoding ISOLatin1Encoding def currentdict end /HaloFont exch definefont pop\n";

pub(crate) fn page_start(w: f64, h: f64) -> String {
    format!("<< /PageSize [{:.2} {:.2}] >> setpagedevice\n", w, h)
}

fn watermark_page(w: f64, h: f64, t: &str) -> String {
    let len = t.chars().count().max(1) as f64;
    let diag = (w * w + h * h).sqrt();
    let size = (diag * 0.7 / (len * 0.6)).clamp(24.0, 150.0);
    let angle = h.atan2(w).to_degrees();
    format!("{}gsave 0.18 .setfillconstantalpha 0.45 setgray /HaloFont {:.1} selectfont {:.2} {:.2} translate {:.2} rotate \
        {} dup stringwidth pop -2 div {:.1} moveto show grestore\nshowpage\n",
        page_start(w, h), size, w / 2.0, h / 2.0, angle, ps_string(t), -size * 0.35)
}

fn note_page(w: f64, h: f64, t: &str, pos: usize) -> String {
    let (size, m, pad) = (13.0, 28.0, 5.0);
    let (col, row) = (pos % 3, pos / 3);
    // In PostScript la x dipende dalla larghezza del testo, che si calcola sulla pagina (tw).
    let x = match col { 0 => format!("{}", m), 1 => format!("{} tw sub 2 div", w), _ => format!("{} tw sub {} sub", w, m) };
    let y = match row { 0 => h - m - size, 1 => h / 2.0 - size / 2.0, _ => m };
    format!("{}gsave /HaloFont {} selectfont {} dup stringwidth pop /tw exch def /tx {} def\n\
        0.85 .setfillconstantalpha 1 setgray tx {pad} sub {:.2} tw {pad} 2 mul add {:.2} rectfill\n\
        1 .setfillconstantalpha 0.12 setgray tx {:.2} moveto show grestore\nshowpage\n",
        page_start(w, h), size, ps_string(t), x, y - pad - size * 0.22, size + 2.0 * pad, y, pad = pad)
}

fn highlight_page(w: f64, h: f64, boxes: &[(f64, f64, f64, f64)]) -> String {
    let mut p = page_start(w, h);
    if !boxes.is_empty() {
        p.push_str("gsave 0.35 .setfillconstantalpha 1 0.85 0 setrgbcolor\n");
        for &(x0, y0, x1, y1) in boxes {
            // pdftotext misura y dall'alto, PostScript dal basso.
            p.push_str(&format!("{:.2} {:.2} {:.2} {:.2} rectfill\n", x0 - 1.0, h - y1 - 1.0, x1 - x0 + 2.0, y1 - y0 + 2.0));
        }
        p.push_str("grestore\n");
    }
    p.push_str("showpage\n");
    p
}

/// Mette un'immagine (es. la firma, meglio PNG trasparente) nella posizione scelta
/// dell'ultima pagina, o di tutte. Larga al massimo il 28% della pagina.
pub fn stamp_image(input: &Path, out: &Path, img: &Path, pos: usize, all_pages: bool) -> Result<(), String> {
    const G: [&str; 9] = ["NorthWest", "North", "NorthEast", "West", "Center", "East", "SouthWest", "South", "SouthEast"];
    let pages = page_sizes(input)?;
    let &(w, h) = pages.last().ok_or("Il PDF non ha pagine.")?;
    // 144 dpi = 2 pixel per punto.
    let (pw, ph) = ((w * 2.0).round(), (h * 2.0).round());
    let (sw, sh) = ((pw * 0.28).round(), (ph * 0.12).round());
    let tmp = std::env::temp_dir().join(format!("halo-firma-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let ov = tmp.join("firma.pdf");
    let r = (|| {
        run_cmd("convert", &["-size", &format!("{}x{}", pw, ph), "xc:none", "(", &format!("{}[0]", s(img)), "-resize", &format!("{}x{}", sw, sh), ")",
            "-gravity", G[pos.min(8)], "-geometry", "+72+72", "-composite", "-units", "PixelsPerInch", "-density", "144", &s(&ov)])?;
        let to = if all_pages { "1-z" } else { "z" };
        run_cmd("qpdf", &["--warning-exit-0", &s(input), "--overlay", &s(&ov), &format!("--to={}", to), "--from=", "--repeat=1", "--", &s(out)]).map(|_| ())
    })();
    let _ = fs::remove_dir_all(&tmp);
    if r.is_err() { let _ = fs::remove_file(out); }
    r
}

/// Crea il PDF di sovrapposizione dalle pagine PostScript e lo applica con qpdf.
pub(crate) fn overlay(input: &Path, out: &Path, pages: &[String]) -> Result<(), String> {
    overlay_with(input, out, pages, &[])
}

/// Come overlay(), con opzioni di qpdf per scegliere le pagine (es. --to=3 --from=1).
/// Ghostscript salta le pagine vuote, quindi per una sola pagina si usa questo.
pub(crate) fn overlay_with(input: &Path, out: &Path, pages: &[String], opts: &[&str]) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("halo-annota-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = (|| {
        let ps = tmp.join("sopra.ps");
        let pdf = tmp.join("sopra.pdf");
        fs::write(&ps, format!("{}{}", PS_HEAD, pages.concat())).map_err(|e| e.to_string())?;
        run_cmd("gs", &["-q", "-dBATCH", "-dNOPAUSE", "-dSAFER", "-dALLOWPSTRANSPARENCY", "-dAutoRotatePages=/None",
            "-sDEVICE=pdfwrite", &format!("-sOutputFile={}", s(&pdf)), &s(&ps)])?;
        let (inp, ov, o) = (s(input), s(&pdf), s(out));
        let mut args = vec!["--warning-exit-0", &inp, "--overlay", &ov];
        args.extend_from_slice(opts);
        args.extend_from_slice(&["--", &o]);
        run_cmd("qpdf", &args).map(|_| ())
    })();
    let _ = fs::remove_dir_all(&tmp);
    if r.is_err() { let _ = fs::remove_file(out); }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stringhe_postscript() {
        assert_eq!(ps_string("Però (bozza) – 5€"), "(Per\\362 \\(bozza\\) - 5E)");
        assert!(note_page(595.0, 842.0, "x", 8).contains("595 tw sub 28"));
        assert!(watermark_page(595.0, 842.0, "BOZZA").contains("54.75 rotate"));
    }

    fn pdf_prova(dir: &Path) -> PathBuf {
        let pdf = dir.join("prova.pdf");
        run_cmd("gs", &["-q", "-dBATCH", "-dNOPAUSE", "-sDEVICE=pdfwrite", &format!("-sOutputFile={}", s(&pdf)), "-c",
            "<< /PageSize [595 842] >> setpagedevice /Helvetica findfont 14 scalefont setfont 72 700 moveto (Contratto tra Rossi e Bianchi) show showpage \
             << /PageSize [842 595] >> setpagedevice 72 500 moveto (Seconda pagina, firma di Rossi) show showpage"]).unwrap();
        pdf
    }

    fn words(pdf: &Path) -> String { run_cmd("pdftotext", &[&s(pdf), "-"]).unwrap() }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-annota");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let pdf = pdf_prova(&dir);
        let prima = fs::read(&pdf).unwrap();
        let d = |v: &str, pos: &str, pag: &str| HashMap::from([
            ("valore".to_string(), v.to_string()), ("posizione".to_string(), pos.to_string()), ("pagine".to_string(), pag.to_string())]);

        // Filigrana: testo originale ancora selezionabile + filigrana su entrambe le pagine.
        run(&pdf, "pdf/proteggi/filigrana", &d("BOZZA", "", "")).unwrap();
        let t = words(&dir.join("prova_filigrana.pdf"));
        assert!(t.contains("Contratto tra Rossi") && t.contains("Seconda pagina"), "{}", t);
        assert!(t.contains("BO"), "filigrana assente: {}", t);

        // Testo in basso a destra, solo sulla prima pagina.
        run(&pdf, "pdf/annota/testo", &d("Approvato – M. Rossi", "8", "prima")).unwrap();
        let out = dir.join("prova_annotato.pdf");
        let bbox = run_cmd("pdftotext", &["-bbox", &s(&out), "-"]).unwrap();
        let pagine: Vec<&str> = bbox.split("<page ").skip(1).collect();
        assert!(pagine[0].contains(">Approvato</word>") && !pagine[1].contains("Approvato"), "{}", bbox);
        let riga = pagine[0].lines().find(|l| l.contains(">Approvato<")).unwrap();
        let xmin: f64 = riga.split("xMin=\"").nth(1).unwrap().split('"').next().unwrap().parse().unwrap();
        let ymin: f64 = riga.split("yMin=\"").nth(1).unwrap().split('"').next().unwrap().parse().unwrap();
        assert!(xmin > 595.0 / 2.0 && ymin > 842.0 * 0.85, "posizione sbagliata: x {} y {}", xmin, ymin);

        // Evidenzia: due "Rossi", nessun errore; parola assente → errore e nessun file.
        assert_eq!(run(&pdf, "pdf/annota/evidenzia", &d("rossi", "", "")).unwrap(), "Creato: prova_evidenziato.pdf (2 parole evidenziate)");
        assert!(words(&dir.join("prova_evidenziato.pdf")).contains("Rossi"));
        assert!(run(&pdf, "pdf/annota/evidenzia", &d("Zanzibar", "", "")).is_err());
        assert!(!dir.join("prova_evidenziato (2).pdf").exists());

        // Limita stampa: si apre senza password, ma la stampa è disattivata.
        run(&pdf, "pdf/proteggi/limita-stampa", &HashMap::new()).unwrap();
        let enc = run_cmd("qpdf", &["--show-encryption", &s(&dir.join("prova_senza_stampa.pdf"))]).unwrap();
        assert!(enc.contains("print low resolution: not allowed") && enc.contains("print high resolution: not allowed"), "{}", enc);
        assert!(words(&dir.join("prova_senza_stampa.pdf")).contains("Contratto"));

        // Immagine.
        let img = dir.join("foto.jpg");
        run_cmd("convert", &["-size", "800x500", "xc:#4a7ab0", &s(&img)]).unwrap();
        run(&img, "image/annota/testo", &d("Città d'arte – 2026", "6", "")).unwrap();
        let a = dir.join("foto_annotata.jpg");
        let bl: f64 = run_cmd("convert", &[&s(&a), "-crop", "300x80+0+420", "-format", "%[fx:standard_deviation]", "info:"]).unwrap().trim().parse().unwrap();
        let tr: f64 = run_cmd("convert", &[&s(&a), "-crop", "300x80+500+0", "-format", "%[fx:standard_deviation]", "info:"]).unwrap().trim().parse().unwrap();
        assert!(bl > 0.05 && tr < 0.01, "testo non in basso a sinistra: {} {}", bl, tr);

        assert_eq!(fs::read(&pdf).unwrap(), prima, "l'originale non va toccato");
    }
}
