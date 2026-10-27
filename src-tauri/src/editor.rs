//! Opzioni con l'editor sull'anteprima: nel pannello si vede il file, si disegna l'area,
//! la forma o la freccia (o si clicca sullo sfondo) e poi l'azione crea una copia modificata.
//! Le coordinate arrivano come frazioni (0–1) dell'anteprima e qui diventano pixel o punti veri.

use crate::actions::{duration, name, out_path, run_cmd, s, video_ext, VIDEO_HQ};
use crate::annota::{overlay_with, page_sizes, page_start};
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const KEYS: &[&str] = &[
    "image/ritaglia/libero", "image/annota/freccia", "image/annota/forma", "image/annota/evidenzia",
    "image/sfondo/sfocato", "image/sfondo/colore", "image/sfondo/rimuovi",
    "pdf/annota/forma", "video/ruota/ritaglia",
];

/// Colori della tavolozza del pannello (stessi nomi in menu.js).
const COLORI: &[(&str, &str)] = &[
    ("rosso", "#E53935"), ("giallo", "#FDD835"), ("verde", "#43A047"), ("blu", "#1E88E5"), ("nero", "#111111"), ("bianco", "#FFFFFF"),
];

/// Lato massimo dell'anteprima mostrata nel pannello.
const PREVIEW: u32 = 900;

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    Some(match key {
        "image/ritaglia/libero" => out_path(input, "_ritagliata", &ext),
        "image/annota/freccia" | "image/annota/forma" => out_path(input, "_annotata", &ext),
        "image/annota/evidenzia" => out_path(input, "_evidenziata", &ext),
        "image/sfondo/sfocato" => out_path(input, "_sfondo_sfocato", &ext),
        "image/sfondo/colore" => out_path(input, "_sfondo_colore", &ext),
        "image/sfondo/rimuovi" => out_path(input, "_senza_sfondo", "png"),
        "pdf/annota/forma" => out_path(input, "_annotato", "pdf"),
        "video/ruota/ritaglia" => out_path(input, "_ritagliato", video_ext(&ext)),
        _ => return None,
    })
}

#[derive(Serialize)]
pub struct Anteprima {
    /// PNG come data URL.
    pub img: String,
    /// Dimensioni vere (pixel; punti per i PDF).
    pub w: f64,
    pub h: f64,
    pub pagine: usize,
}

fn tmp_dir(tag: &str) -> Result<PathBuf, String> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static N: AtomicUsize = AtomicUsize::new(0);
    let d = std::env::temp_dir().join(format!("halo-editor-{}-{}-{}", tag, std::process::id(), N.fetch_add(1, Ordering::Relaxed)));
    let _ = fs::remove_dir_all(&d);
    fs::create_dir_all(&d).map_err(|e| e.to_string())?;
    Ok(d)
}

fn size_of(img: &str) -> Result<(f64, f64), String> {
    let v: Vec<f64> = run_cmd("convert", &[img, "-auto-orient", "-format", "%w %h", "info:"])?
        .split_whitespace().filter_map(|x| x.parse().ok()).collect();
    match v[..] { [w, h, ..] if w > 0.0 && h > 0.0 => Ok((w, h)), _ => Err("Dimensioni dell'immagine non lette.".into()) }
}

/// Anteprima per il pannello. Per Sfondo › Colore e Rimuovi, se ci sono già dei punti,
/// mostra il risultato applicato all'anteprima.
pub fn preview(input: &Path, key: &str, page: usize, data: &HashMap<String, String>) -> Result<Anteprima, String> {
    if !KEYS.contains(&key) { return Err("Opzione non disponibile.".into()); }
    let tmp = tmp_dir("anteprima")?;
    let r = (|| {
        let png = tmp.join("a.png");
        let (w, h, pagine, src) = if key.starts_with("pdf/") {
            let sizes = page_sizes(input).map_err(|e| if e.contains("assword") { "Il PDF è protetto da password: toglila prima.".to_string() } else { e })?;
            let p = page.clamp(1, sizes.len());
            let (w, h) = sizes[p - 1];
            let dpi = (PREVIEW as f64 / w.max(h) * 72.0).min(150.0);
            let base = tmp.join("p");
            run_cmd("pdftoppm", &["-f", &p.to_string(), "-l", &p.to_string(), "-r", &format!("{:.0}", dpi), "-png", "-singlefile", &s(input), &s(&base)])?;
            (w, h, sizes.len(), tmp.join("p.png"))
        } else if key.starts_with("video/") {
            let frame = tmp.join("f.png");
            let at = format!("{:.2}", duration(input).unwrap_or(0.0) / 2.0);
            run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-y", "-ss", &at, "-i", &s(input), "-frames:v", "1", &s(&frame)])?;
            let (w, h) = size_of(&s(&frame))?;
            (w, h, 1, frame)
        } else {
            let (w, h) = size_of(&format!("{}[0]", s(input)))?;
            (w, h, 1, input.to_path_buf())
        };
        let geo = format!("{}x{}>", PREVIEW, PREVIEW);
        run_cmd("convert", &[&format!("{}[0]", s(&src)), "-auto-orient", "-resize", &geo, &format!("png:{}", s(&png))])?;
        if matches!(key, "image/sfondo/colore" | "image/sfondo/rimuovi") && data.get("punti").is_some_and(|p| !p.trim().is_empty()) {
            let (pw, ph) = size_of(&s(&png))?;
            let done = tmp.join("b.png");
            run_cmd("convert", &image_args(&s(&png), &format!("png:{}", s(&done)), key, data, pw, ph)?.iter().map(|x| x.as_str()).collect::<Vec<_>>())?;
            fs::rename(&done, &png).map_err(|e| e.to_string())?;
        }
        let bytes = fs::read(&png).map_err(|e| e.to_string())?;
        Ok(Anteprima { img: format!("data:image/png;base64,{}", base64(&bytes)), w, h, pagine })
    })();
    let _ = fs::remove_dir_all(&tmp);
    r
}

fn base64(b: &[u8]) -> String {
    crate::sistema::base64(b)
}

/// "x1,y1,x2,y2" in frazioni 0–1.
fn area(data: &HashMap<String, String>) -> Result<[f64; 4], String> {
    let v: Vec<f64> = data.get("area").map(|a| a.split(',').filter_map(|x| x.trim().parse().ok()).collect()).unwrap_or_default();
    match v[..] {
        [a, b, c, d] if v.iter().all(|x| (0.0..=1.0).contains(x)) => Ok([a, b, c, d]),
        _ => Err("Disegna prima sull'anteprima.".into()),
    }
}

/// Rettangolo ordinato (sinistra, alto, destra, basso) in unità vere, non troppo piccolo.
fn rect(data: &HashMap<String, String>, w: f64, h: f64) -> Result<[f64; 4], String> {
    let [a, b, c, d] = area(data)?;
    let r = [a.min(c) * w, b.min(d) * h, a.max(c) * w, b.max(d) * h];
    if (r[2] - r[0]) < w * 0.01 || (r[3] - r[1]) < h * 0.01 { return Err("L'area disegnata è troppo piccola.".into()); }
    Ok(r)
}

fn points(data: &HashMap<String, String>, w: f64, h: f64) -> Result<Vec<(f64, f64)>, String> {
    let mut v = Vec::new();
    for p in data.get("punti").map(|p| p.as_str()).unwrap_or("").split(';').filter(|p| !p.trim().is_empty()) {
        let xy: Vec<f64> = p.split(',').filter_map(|x| x.trim().parse().ok()).collect();
        match xy[..] {
            [x, y] if (0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y) => v.push(((x * w).min(w - 1.0), (y * h).min(h - 1.0))),
            _ => return Err("Punto non valido.".into()),
        }
    }
    if v.is_empty() { return Err("Clicca sullo sfondo nell'anteprima.".into()); }
    Ok(v)
}

fn color(data: &HashMap<String, String>, default: &str) -> Result<&'static str, String> {
    let c = data.get("colore").map(|c| c.as_str()).unwrap_or(default);
    COLORI.iter().find(|(n, _)| *n == c).map(|(_, hex)| *hex).ok_or_else(|| "Colore non valido.".to_string())
}

fn fuzz(data: &HashMap<String, String>) -> &'static str {
    match data.get("tolleranza").map(|t| t.as_str()) { Some("bassa") => "6%", Some("alta") => "25%", _ => "12%" }
}

fn shape(data: &HashMap<String, String>) -> &str {
    data.get("forma").map(|f| f.as_str()).unwrap_or("rettangolo")
}

/// Punte della freccia da (x1,y1) a (x2,y2): fine della linea e triangolo della punta.
fn arrow(x1: f64, y1: f64, x2: f64, y2: f64, lw: f64) -> Result<((f64, f64), [(f64, f64); 3]), String> {
    let (dx, dy) = (x2 - x1, y2 - y1);
    let len = (dx * dx + dy * dy).sqrt();
    if len < lw * 3.0 { return Err("La freccia è troppo corta.".into()); }
    let (ux, uy) = (dx / len, dy / len);
    let hl = (lw * 5.0).min(len * 0.6);
    let hw = hl * 0.6;
    let (bx, by) = (x2 - ux * hl, y2 - uy * hl);
    Ok(((x2 - ux * hl * 0.8, y2 - uy * hl * 0.8), [(x2, y2), (bx - uy * hw, by + ux * hw), (bx + uy * hw, by - ux * hw)]))
}

/// Argomenti di `convert` per le opzioni sulle immagini, con coordinate su un'immagine w×h.
fn image_args(src: &str, out: &str, key: &str, data: &HashMap<String, String>, w: f64, h: f64) -> Result<Vec<String>, String> {
    let mut a: Vec<String> = vec![src.into(), "-auto-orient".into()];
    let push = |a: &mut Vec<String>, xs: &[&str]| a.extend(xs.iter().map(|x| x.to_string()));
    let lw = (w.min(h) / 140.0).max(3.0).round();
    let geo = |r: &[f64; 4]| format!("{:.0}x{:.0}+{:.0}+{:.0}", r[2] - r[0], r[3] - r[1], r[0], r[1]);
    let ellipse = |r: &[f64; 4]| format!("ellipse {:.1},{:.1} {:.1},{:.1} 0,360", (r[0] + r[2]) / 2.0, (r[1] + r[3]) / 2.0, (r[2] - r[0]) / 2.0, (r[3] - r[1]) / 2.0);
    match key {
        "image/ritaglia/libero" => { let r = rect(data, w, h)?; push(&mut a, &["-crop", &geo(&r), "+repage"]); }
        "image/annota/forma" => {
            let r = rect(data, w, h)?;
            let d = if shape(data) == "ovale" { ellipse(&r) } else { format!("rectangle {:.1},{:.1} {:.1},{:.1}", r[0], r[1], r[2], r[3]) };
            push(&mut a, &["-fill", "none", "-stroke", color(data, "rosso")?, "-strokewidth", &lw.to_string(), "-draw", &d]);
        }
        "image/annota/freccia" => {
            let [x1, y1, x2, y2] = area(data)?;
            let (end, tip) = arrow(x1 * w, y1 * h, x2 * w, y2 * h, lw)?;
            let c = color(data, "rosso")?;
            push(&mut a, &["-fill", "none", "-stroke", c, "-strokewidth", &lw.to_string(),
                "-draw", &format!("stroke-linecap round line {:.1},{:.1} {:.1},{:.1}", x1 * w, y1 * h, end.0, end.1),
                "-stroke", "none", "-fill", c, "-draw", &format!("polygon {}", tip.iter().map(|p| format!("{:.1},{:.1}", p.0, p.1)).collect::<Vec<_>>().join(" "))]);
        }
        "image/annota/evidenzia" => {
            // Moltiplica per il giallo: il contenuto sotto resta leggibile.
            let r = rect(data, w, h)?;
            push(&mut a, &["(", "+clone", "-crop", &geo(&r), "+repage", "-fill", "#FFE94D", "-colorize", "100", ")",
                "-geometry", &format!("+{:.0}+{:.0}", r[0], r[1]), "-compose", "multiply", "-composite"]);
        }
        "image/sfondo/sfocato" => {
            // Sfondo sfocato, l'area scelta nitida con il bordo sfumato.
            let r = rect(data, w, h)?;
            let d = if shape(data) == "rettangolo" { format!("rectangle {:.1},{:.1} {:.1},{:.1}", r[0], r[1], r[2], r[3]) } else { ellipse(&r) };
            let blur = format!("0x{:.1}", (w.max(h) / 90.0).max(2.0));
            let soft = format!("0x{:.1}", ((r[2] - r[0]).min(r[3] - r[1]) / 25.0).max(1.0));
            push(&mut a, &["-write", "mpr:o", "+delete", "(", "mpr:o", "-blur", &blur, ")", "mpr:o",
                "(", "mpr:o", "-alpha", "off", "-fill", "black", "-colorize", "100", "-fill", "white", "-draw", &d, "-blur", &soft, ")", "-composite"]);
        }
        "image/sfondo/colore" | "image/sfondo/rimuovi" => {
            let pts = points(data, w, h)?;
            let fill = if key.ends_with("rimuovi") { "none" } else { color(data, "bianco")? };
            push(&mut a, &["-alpha", "set", "-fuzz", fuzz(data), "-fill", fill]);
            for (x, y) in pts { push(&mut a, &["-draw", &format!("color {:.0},{:.0} floodfill", x, y)]); }
            if fill != "none" { push(&mut a, &["-background", fill, "-alpha", "remove", "-alpha", "off"]); }
        }
        _ => return Err("Opzione non disponibile.".into()),
    }
    a.push(out.into());
    Ok(a)
}

/// Pagina PostScript con la forma, per la sovrapposizione sul PDF (y dal basso).
fn pdf_shape(w: f64, h: f64, data: &HashMap<String, String>) -> Result<String, String> {
    let hex = color(data, "rosso")?;
    let c: Vec<f64> = (0..3).map(|k| u8::from_str_radix(&hex[1 + 2 * k..3 + 2 * k], 16).unwrap() as f64 / 255.0).collect();
    let lw = 2.5;
    let mut p = format!("{}gsave {:.3} {:.3} {:.3} setrgbcolor {} setlinewidth 1 setlinejoin 1 setlinecap\n", page_start(w, h), c[0], c[1], c[2], lw);
    match shape(data) {
        "freccia" => {
            let [x1, y1, x2, y2] = area(data)?;
            let (x1, y1, x2, y2) = (x1 * w, h - y1 * h, x2 * w, h - y2 * h);
            let (end, t) = arrow(x1, y1, x2, y2, lw)?;
            p.push_str(&format!("newpath {:.2} {:.2} moveto {:.2} {:.2} lineto stroke\nnewpath {:.2} {:.2} moveto {:.2} {:.2} lineto {:.2} {:.2} lineto closepath fill\n",
                x1, y1, end.0, end.1, t[0].0, t[0].1, t[1].0, t[1].1, t[2].0, t[2].1));
        }
        f => {
            let r = rect(data, w, h)?;
            let (x0, y0, x1, y1) = (r[0], h - r[3], r[2], h - r[1]);
            if f == "ovale" {
                p.push_str(&format!("newpath matrix currentmatrix {:.2} {:.2} translate {:.3} {:.3} scale 0 0 1 0 360 arc closepath setmatrix stroke\n",
                    (x0 + x1) / 2.0, (y0 + y1) / 2.0, (x1 - x0) / 2.0, (y1 - y0) / 2.0));
            } else {
                p.push_str(&format!("{:.2} {:.2} {:.2} {:.2} rectstroke\n", x0, y0, x1 - x0, y1 - y0));
            }
        }
    }
    p.push_str("grestore\nshowpage\n");
    Ok(p)
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let out = planned_out(input, key).ok_or("Opzione non disponibile.")?;
    match key {
        "pdf/annota/forma" => {
            area(data)?;
            let sizes = page_sizes(input).map_err(|e| if e.contains("assword") { "Il PDF è protetto da password: toglila prima.".to_string() } else { e })?;
            let p: usize = data.get("pagina").and_then(|p| p.parse().ok()).filter(|p| (1..=sizes.len()).contains(p)).ok_or("Pagina non valida.")?;
            let info = run_cmd("pdfinfo", &["-f", &p.to_string(), "-l", &p.to_string(), &s(input)])?;
            let rot = info.lines().find(|l| l.contains(" rot:")).and_then(|l| l.split_whitespace().last()).unwrap_or("0");
            if rot != "0" { return Err("Questa pagina è ruotata nel PDF: non posso ancora disegnarci sopra.".into()); }
            let (w, h) = sizes[p - 1];
            overlay_with(input, &out, &[pdf_shape(w, h, data)?], &[&format!("--to={}", p), "--from=1"])?;
            Ok(format!("Creato: {} (pagina {})", name(&out), p))
        }
        "video/ruota/ritaglia" => {
            let tmp = tmp_dir("video")?;
            let frame = tmp.join("f.png");
            let r = run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-y", "-i", &s(input), "-frames:v", "1", &s(&frame)]).and_then(|_| size_of(&s(&frame)));
            let _ = fs::remove_dir_all(&tmp);
            let (w, h) = r?;
            let r = rect(data, w, h)?;
            // libx264 vuole lati pari.
            let even = |v: f64| ((v / 2.0).floor() * 2.0) as i64;
            let (cw, ch) = (even(r[2] - r[0]).max(2), even(r[3] - r[1]).max(2));
            let crop = format!("crop={}:{}:{}:{}", cw, ch, even(r[0]), even(r[1]));
            crate::actions::ff_to(input, &out, &[], &[&["-vf", crop.as_str()][..], VIDEO_HQ].concat())?;
            Ok(format!("Creato: {} ({}×{})", name(&out), cw, ch))
        }
        _ => {
            let (w, h) = size_of(&format!("{}[0]", s(input)))?;
            let args = image_args(&s(input), &s(&out), key, data, w, h)?;
            if let Err(e) = run_cmd("convert", &args.iter().map(|x| x.as_str()).collect::<Vec<_>>()) { let _ = fs::remove_file(&out); return Err(e); }
            if key == "image/ritaglia/libero" {
                let (cw, ch) = size_of(&s(&out))?;
                return Ok(format!("Creato: {} ({:.0}×{:.0})", name(&out), cw, ch));
            }
            Ok(format!("Creato: {}", name(&out)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(pairs: &[(&str, &str)]) -> HashMap<String, String> { pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect() }

    #[test]
    fn geometria() {
        assert_eq!(base64(b"Halo!"), "SGFsbyE=");
        assert_eq!(rect(&d(&[("area", "0.75,0.5,0.25,0.1")]), 200.0, 100.0).unwrap(), [50.0, 10.0, 150.0, 50.0]);
        assert!(rect(&d(&[("area", "0.5,0.5,0.501,0.9")]), 200.0, 100.0).is_err());
        assert!(area(&d(&[("area", "0,0,1.5,1")])).is_err());
        let (end, tip) = arrow(0.0, 0.0, 100.0, 0.0, 4.0).unwrap();
        assert_eq!(tip[0], (100.0, 0.0));
        assert!(end.0 < 100.0 && tip[1].0 == 80.0 && tip[1].1 == 12.0);
        assert!(color(&d(&[("colore", "#123456")]), "rosso").is_err());
        assert_eq!(points(&d(&[("punti", "0,0;1,1")]), 10.0, 10.0).unwrap(), [(0.0, 0.0), (9.0, 9.0)]);
        assert!(pdf_shape(600.0, 800.0, &d(&[("area", "0.1,0.1,0.5,0.2"), ("forma", "rettangolo")])).unwrap().contains("60.00 640.00 240.00 80.00 rectstroke"));
    }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-editor");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Foto 400×300: sfondo azzurro uniforme, un quadrato rosso al centro.
        let img = dir.join("foto.png");
        run_cmd("convert", &["-size", "400x300", "xc:#88CCEE", "-fill", "#CC2222", "-draw", "rectangle 150,100 250,200", &s(&img)]).unwrap();
        let prima = fs::read(&img).unwrap();
        let px = |f: &str, x: u32, y: u32| run_cmd("convert", &[&s(&dir.join(f)), "-format", &format!("%[hex:u.p{{{},{}}}]", x, y), "info:"]).unwrap();
        let alpha = |f: &str, x: u32, y: u32| run_cmd("convert", &[&s(&dir.join(f)), "-format", &format!("%[fx:p{{{},{}}}.a]", x, y), "info:"]).unwrap();

        assert_eq!(run(&img, "image/ritaglia/libero", &d(&[("area", "0.25,0.25,0.75,0.75")])).unwrap(), "Creato: foto_ritagliata.png (200×150)");
        run(&img, "image/annota/forma", &d(&[("area", "0.1,0.1,0.3,0.3"), ("colore", "nero")])).unwrap();
        assert!(px("foto_annotata.png", 40, 60).starts_with("111111"));
        assert!(px("foto_annotata.png", 80, 60).starts_with("88CCEE"), "dentro la forma resta com'era");
        run(&img, "image/annota/freccia", &d(&[("area", "0.1,0.9,0.9,0.9"), ("colore", "blu")])).unwrap();
        assert!(px("foto_annotata (2).png", 200, 270).starts_with("1E88E5"));
        run(&img, "image/annota/evidenzia", &d(&[("area", "0,0,0.2,0.2")])).unwrap();
        assert_ne!(px("foto_evidenziata.png", 10, 10), px("foto_evidenziata.png", 300, 280));
        run(&img, "image/sfondo/sfocato", &d(&[("area", "0.3,0.3,0.7,0.7"), ("forma", "rettangolo")])).unwrap();
        assert!(px("foto_sfondo_sfocato.png", 200, 150).starts_with("CC2222"), "il centro resta nitido");
        run(&img, "image/sfondo/colore", &d(&[("punti", "0.02,0.02"), ("colore", "verde")])).unwrap();
        assert!(px("foto_sfondo_colore.png", 5, 5).starts_with("43A047") && px("foto_sfondo_colore.png", 200, 150).starts_with("CC2222"));
        assert_eq!(run(&img, "image/sfondo/rimuovi", &d(&[("punti", "0.02,0.02")])).unwrap(), "Creato: foto_senza_sfondo.png");
        assert_eq!((alpha("foto_senza_sfondo.png", 5, 5).as_str(), alpha("foto_senza_sfondo.png", 200, 150).as_str()), ("0", "1"));
        assert!(run(&img, "image/sfondo/rimuovi", &HashMap::new()).is_err());
        let a = preview(&img, "image/sfondo/rimuovi", 1, &d(&[("punti", "0.5,0.02")])).unwrap();
        assert!(a.img.starts_with("data:image/png;base64,iVBOR") && a.w == 400.0 && a.h == 300.0);

        // PDF di 2 pagine: la forma solo sulla seconda.
        let pdf = dir.join("doc.pdf");
        run_cmd("convert", &["-size", "595x842", "xc:white", "xc:white", &s(&pdf)]).unwrap();
        assert_eq!(preview(&pdf, "pdf/annota/forma", 2, &HashMap::new()).unwrap().pagine, 2);
        assert_eq!(run(&pdf, "pdf/annota/forma", &d(&[("area", "0.2,0.2,0.8,0.4"), ("forma", "ovale"), ("pagina", "2")])).unwrap(), "Creato: doc_annotato.pdf (pagina 2)");
        let mean = |pg: &str| {
            run_cmd("pdftoppm", &["-f", pg, "-l", pg, "-r", "30", "-png", "-singlefile", &s(&dir.join("doc_annotato.pdf")), &s(&dir.join("pg"))]).unwrap();
            run_cmd("convert", &[&s(&dir.join("pg.png")), "-format", "%[fx:mean]", "info:"]).unwrap()
        };
        let (m1, m2): (f64, f64) = (mean("1").parse().unwrap(), mean("2").parse().unwrap());
        assert!(m2 < m1 - 0.001, "la forma è solo sulla pagina 2: {} {}", m1, m2);

        // Video 320×240: ritaglio della metà sinistra.
        let mp4 = dir.join("clip.mp4");
        run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-y", "-f", "lavfi", "-i", "testsrc=size=320x240:duration=1", "-pix_fmt", "yuv420p", &s(&mp4)]).unwrap();
        assert_eq!(preview(&mp4, "video/ruota/ritaglia", 1, &HashMap::new()).unwrap().w, 320.0);
        assert_eq!(run(&mp4, "video/ruota/ritaglia", &d(&[("area", "0,0,0.5,1")])).unwrap(), "Creato: clip_ritagliato.mp4 (160×240)");
        assert_eq!(fs::read(&img).unwrap(), prima, "l'originale non va toccato");
    }
}
