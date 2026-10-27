//! Documento › Proteggi › Filigrana e Firma per i .docx, modificando l'XML del file:
//! la filigrana è la stessa forma che usa Word (testo diagonale grigio nell'intestazione),
//! la firma è un'immagine aggiunta in fondo al documento.

use crate::actions::{name, out_path, run_cmd, s};
use crate::documenti::{parts, read_part, write_parts};
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

pub const KEYS: &[&str] = &["document/proteggi/filigrana", "document/proteggi/firma"];

const NS_HDR: &str = "xmlns:w=\"http://schemas.openxmlformats.org/wordprocessingml/2006/main\" \
    xmlns:r=\"http://schemas.openxmlformats.org/officeDocument/2006/relationships\"";
/// Namespace usati dalla forma VML della filigrana, da dichiarare se mancano.
const NS_VML: &[(&str, &str)] = &[("v", "urn:schemas-microsoft-com:vml"), ("o", "urn:schemas-microsoft-com:office:office"),
    ("w10", "urn:schemas-microsoft-com:office:word")];
const REL_HEADER: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/header";
const REL_IMAGE: &str = "http://schemas.openxmlformats.org/officeDocument/2006/relationships/image";
const CT_HEADER: &str = "application/vnd.openxmlformats-officedocument.wordprocessingml.header+xml";

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    Some(match key {
        "document/proteggi/filigrana" => out_path(input, "_filigrana", "docx"),
        "document/proteggi/firma" => out_path(input, "_firmato", "docx"),
        _ => return None,
    })
}

fn attr(v: &str) -> String {
    v.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// Aggiunge al tag radice (es. "w:hdr") le dichiarazioni di namespace che mancano.
fn ensure_ns(xml: &str, root: &str, ns: &[(&str, &str)]) -> String {
    let Some(start) = xml.find(&format!("<{}", root)) else { return xml.to_string() };
    let end = start + xml[start..].find('>').unwrap_or(0);
    let tag = &xml[start..end];
    let add: String = ns.iter().filter(|(p, _)| !tag.contains(&format!("xmlns:{}=", p)))
        .map(|(p, u)| format!(" xmlns:{}=\"{}\"", p, u)).collect();
    let cut = if tag.ends_with('/') { end - 1 } else { end };
    format!("{}{}{}", &xml[..cut], add, &xml[cut..])
}

/// Un Id di relazione libero ("rIdHalo1", "rIdHalo2"…).
fn free_id(rels: &str) -> String {
    (1..).map(|n| format!("rIdHalo{}", n)).find(|id| !rels.contains(&format!("Id=\"{}\"", id))).unwrap()
}

fn add_rel(rels: &str, id: &str, kind: &str, target: &str) -> String {
    rels.replacen("</Relationships>", &format!("<Relationship Id=\"{}\" Type=\"{}\" Target=\"{}\"/></Relationships>", id, kind, target), 1)
}

/// I riferimenti alle intestazioni di un sectPr: tipo → Id della relazione.
fn header_refs(sect: &str) -> HashMap<String, String> {
    let tag = Regex::new(r"<w:headerReference\b[^>]*>").unwrap();
    let typ = Regex::new(r#"w:type="(\w+)""#).unwrap();
    let id = Regex::new(r#"r:id="([^"]+)""#).unwrap();
    tag.find_iter(sect).filter_map(|m| {
        let t = m.as_str();
        Some((typ.captures(t)?[1].to_string(), id.captures(t)?[1].to_string()))
    }).collect()
}

/// Paragrafo con la filigrana come la crea Word (forma di testo VML ruotata, dietro al testo).
fn watermark_par(text: &str, n: usize) -> String {
    format!(concat!(
        "<w:p><w:pPr><w:pStyle w:val=\"Header\"/></w:pPr><w:r><w:rPr><w:noProof/></w:rPr><w:pict>",
        "<v:shapetype id=\"_x0000_t136\" coordsize=\"21600,21600\" o:spt=\"136\" adj=\"10800\" path=\"m@7,l@8,m@5,21600l@6,21600e\">",
        "<v:formulas><v:f eqn=\"sum #0 0 10800\"/><v:f eqn=\"prod #0 2 1\"/><v:f eqn=\"sum 21600 0 @1\"/><v:f eqn=\"sum 0 0 @2\"/>",
        "<v:f eqn=\"sum 21600 0 @3\"/><v:f eqn=\"if @0 @3 0\"/><v:f eqn=\"if @0 21600 @1\"/><v:f eqn=\"if @0 0 @2\"/>",
        "<v:f eqn=\"if @0 @4 21600\"/><v:f eqn=\"mid @5 @6\"/><v:f eqn=\"mid @8 @5\"/><v:f eqn=\"mid @7 @8\"/>",
        "<v:f eqn=\"mid @6 @7\"/><v:f eqn=\"sum @6 0 @5\"/></v:formulas>",
        "<v:path textpathok=\"t\" o:connecttype=\"custom\" o:connectlocs=\"@9,0;@10,10800;@11,21600;@12,10800\" o:connectangles=\"270,180,90,0\"/>",
        "<v:textpath on=\"t\" fitshape=\"t\"/><v:handles><v:h position=\"#0,bottomRight\" xrange=\"6629,14971\"/></v:handles>",
        "<o:lock v:ext=\"edit\" text=\"t\" shapetype=\"t\"/></v:shapetype>",
        "<v:shape id=\"PowerPlusWaterMarkObject{n}\" o:spid=\"_x0000_s{spid}\" type=\"#_x0000_t136\" ",
        "style=\"position:absolute;margin-left:0;margin-top:0;width:440pt;height:110pt;rotation:315;z-index:-251657216;",
        "mso-position-horizontal:center;mso-position-horizontal-relative:margin;mso-position-vertical:center;mso-position-vertical-relative:margin\" ",
        "o:allowincell=\"f\" fillcolor=\"silver\" stroked=\"f\"><v:fill opacity=\".5\"/>",
        "<v:textpath style=\"font-family:&quot;Calibri&quot;;font-size:1pt\" string=\"{text}\"/>",
        "<w10:wrap anchorx=\"margin\" anchory=\"margin\"/></v:shape></w:pict></w:r></w:p>"),
        n = n, spid = 2049 + n, text = attr(text))
}

/// Mette la filigrana in tutte le intestazioni usate dal documento. Restituisce le parti nuove/modificate.
fn add_watermark(doc: &str, rels: &str, ctypes: &str, headers: &HashMap<String, String>, text: &str)
    -> Result<Vec<(String, String)>, String> {
    let open = Regex::new(r"<w:sectPr\b[^>]*>").unwrap();
    let target = Regex::new(r#"<Relationship\b[^>]*>"#).unwrap();
    let rel_target: HashMap<String, String> = target.find_iter(rels).filter_map(|m| {
        let t = m.as_str();
        let get = |k: &str| Regex::new(&format!(r#"\b{}="([^"]+)""#, k)).unwrap().captures(t).map(|c| c[1].to_string());
        Some((get("Id")?, get("Target")?))
    }).collect();
    let new_id = free_id(rels);
    let mut seen: HashSet<String> = HashSet::new();
    let mut to_mark: Vec<String> = Vec::new();
    let mut need_new = false;
    let mut out = String::new();
    let mut last = 0;
    for m in open.find_iter(doc) {
        let self_closing = m.as_str().ends_with("/>");
        let body_end = if self_closing { m.end() } else { m.end() + doc[m.end()..].find("</w:sectPr>").unwrap_or(0) };
        let refs = header_refs(&doc[m.end()..body_end]);
        let mut add = String::new();
        for t in ["default", "first", "even"] {
            match refs.get(t) {
                Some(id) => { if !to_mark.contains(id) { to_mark.push(id.clone()); } }
                // Una sezione senza intestazione eredita quella della precedente: si aggiunge solo se nessuna prima la ha.
                None if !seen.contains(t) => { add.push_str(&format!("<w:headerReference w:type=\"{}\" r:id=\"{}\"/>", t, new_id)); need_new = true; }
                None => {}
            }
            seen.insert(t.to_string());
        }
        out.push_str(&doc[last..m.start()]);
        if self_closing && !add.is_empty() {
            out.push_str(&format!("{}>{}</w:sectPr>", m.as_str().trim_end_matches("/>"), add));
        } else {
            out.push_str(m.as_str());
            out.push_str(&add);
        }
        last = m.end();
    }
    out.push_str(&doc[last..]);
    if last == 0 { return Err("Il documento non ha sezioni: non so dove mettere la filigrana.".into()); }

    let mut changed = vec![("word/document.xml".to_string(), out)];
    let mut n = 1;
    for id in &to_mark {
        let t = rel_target.get(id).ok_or("Intestazione del documento non trovata.")?;
        let part = format!("word/{}", t.trim_start_matches('/').trim_start_matches("word/"));
        let h = headers.get(&part).ok_or("Intestazione del documento non trovata.")?;
        let h = ensure_ns(h, "w:hdr", NS_VML);
        let at = h.find("<w:hdr").and_then(|i| h[i..].find('>').map(|j| i + j + 1)).ok_or("Intestazione non valida.")?;
        changed.push((part, format!("{}{}{}", &h[..at], watermark_par(text, n), &h[at..])));
        n += 1;
    }
    if need_new {
        let part = (1..).map(|k| format!("header_halo{}.xml", k)).find(|p| !rel_target.values().any(|t| t.ends_with(p.as_str()))).unwrap();
        let ns: String = NS_VML.iter().map(|(p, u)| format!(" xmlns:{}=\"{}\"", p, u)).collect();
        changed.push((format!("word/{}", part), format!("<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<w:hdr {}{}>{}</w:hdr>", NS_HDR, ns, watermark_par(text, n))));
        changed.push(("word/_rels/document.xml.rels".into(), add_rel(rels, &new_id, REL_HEADER, &part)));
        changed.push(("[Content_Types].xml".into(), ctypes.replacen("</Types>", &format!("<Override PartName=\"/word/{}\" ContentType=\"{}\"/></Types>", part, CT_HEADER), 1)));
    }
    Ok(changed)
}

/// Paragrafo con l'immagine della firma (in linea, larga al massimo 5 cm).
fn signature_par(id: &str, doc_pr: u64, w: f64, h: f64, align: &str) -> String {
    let mut cx = 1_800_000.0;
    let mut cy = cx * h / w;
    if cy > 1_000_000.0 { cx *= 1_000_000.0 / cy; cy = 1_000_000.0; }
    let (cx, cy) = (cx.round() as u64, cy.round() as u64);
    format!(concat!(
        "<w:p><w:pPr><w:jc w:val=\"{al}\"/></w:pPr><w:r><w:drawing><wp:inline distT=\"0\" distB=\"0\" distL=\"0\" distR=\"0\">",
        "<wp:extent cx=\"{cx}\" cy=\"{cy}\"/><wp:docPr id=\"{dp}\" name=\"Firma\"/>",
        "<a:graphic xmlns:a=\"http://schemas.openxmlformats.org/drawingml/2006/main\">",
        "<a:graphicData uri=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">",
        "<pic:pic xmlns:pic=\"http://schemas.openxmlformats.org/drawingml/2006/picture\">",
        "<pic:nvPicPr><pic:cNvPr id=\"{dp}\" name=\"firma.png\"/><pic:cNvPicPr/></pic:nvPicPr>",
        "<pic:blipFill><a:blip r:embed=\"{id}\"/><a:stretch><a:fillRect/></a:stretch></pic:blipFill>",
        "<pic:spPr><a:xfrm><a:off x=\"0\" y=\"0\"/><a:ext cx=\"{cx}\" cy=\"{cy}\"/></a:xfrm><a:prstGeom prst=\"rect\"><a:avLst/></a:prstGeom></pic:spPr>",
        "</pic:pic></a:graphicData></a:graphic></wp:inline></w:drawing></w:r></w:p>"),
        al = align, cx = cx, cy = cy, dp = doc_pr, id = id)
}

/// Inserisce la firma in fondo al corpo (prima delle impostazioni dell'ultima sezione).
fn add_signature(doc: &str, rels: &str, ctypes: &str, media: &str, w: f64, h: f64, align: &str) -> Result<Vec<(String, String)>, String> {
    let id = free_id(rels);
    let doc_pr = Regex::new(r#"<wp:docPr\b[^>]*\bid="(\d+)""#).unwrap().captures_iter(doc)
        .filter_map(|c| c[1].parse::<u64>().ok()).max().unwrap_or(0) + 1;
    let body_end = doc.rfind("</w:body>").ok_or("Documento non valido.")?;
    let last_block = doc[..body_end].rfind("</w:p>").max(doc[..body_end].rfind("</w:tbl>"));
    let at = match doc[..body_end].rfind("<w:sectPr") { Some(sp) if last_block.is_none_or(|b| sp > b) => sp, _ => body_end };
    let wp = [("wp", "http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing"),
        ("r", "http://schemas.openxmlformats.org/officeDocument/2006/relationships")];
    let doc2 = format!("{}{}{}", &doc[..at], signature_par(&id, doc_pr, w, h, align), &doc[at..]);
    let ct = if Regex::new(r#"<Default\b[^>]*Extension="png""#).unwrap().is_match(ctypes) { ctypes.to_string() }
        else { ctypes.replacen("</Types>", "<Default Extension=\"png\" ContentType=\"image/png\"/></Types>", 1) };
    Ok(vec![
        ("word/document.xml".into(), ensure_ns(&doc2, "w:document", &wp)),
        ("word/_rels/document.xml.rels".into(), add_rel(rels, &id, REL_IMAGE, &format!("media/{}", media))),
        ("[Content_Types].xml".into(), ct),
    ])
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    if input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase() != "docx" {
        return Err("Funziona con i file .docx: convertilo prima in .docx.".into());
    }
    let out = planned_out(input, key).ok_or("Opzione non disponibile.")?;
    let all = parts(input).map_err(|_| "Il file non si apre: è protetto da password o danneggiato.".to_string())?;
    if !all.iter().any(|p| p == "word/document.xml") { return Err("Il file non si apre: è protetto da password o danneggiato.".into()); }
    let doc = read_part(input, "word/document.xml")?;
    let rels = read_part(input, "word/_rels/document.xml.rels")?;
    let ctypes = read_part(input, "[Content_Types].xml")?;
    let changed: Vec<(String, Vec<u8>)> = match key {
        "document/proteggi/filigrana" => {
            let text = data.get("valore").map(|v| v.trim()).unwrap_or("");
            if text.is_empty() { return Err("Scrivi il testo della filigrana.".into()); }
            if text.contains('\n') || text.chars().count() > 60 { return Err("La filigrana deve stare su una riga (massimo 60 caratteri).".into()); }
            let mut headers = HashMap::new();
            for p in all.iter().filter(|p| p.starts_with("word/header") && p.ends_with(".xml")) {
                let h = read_part(input, p)?;
                if h.contains("PowerPlusWaterMarkObject") { return Err("Il documento ha già una filigrana.".into()); }
                headers.insert(p.clone(), h);
            }
            add_watermark(&doc, &rels, &ctypes, &headers, text)?.into_iter().map(|(p, x)| (p, x.into_bytes())).collect()
        }
        _ => {
            let img = data.get("file").map(PathBuf::from).filter(|f| f.is_file()).ok_or("Scegli prima i file.")?;
            let align = match data.get("allineamento").map(|a| a.as_str()) { Some("sinistra") => "left", Some("centro") => "center", _ => "right" };
            let tmp = std::env::temp_dir().join(format!("halo-firma-word-{}.png", std::process::id()));
            run_cmd("convert", &[&format!("{}[0]", s(&img)), "-auto-orient", "-resize", "1200x1200>", &format!("png:{}", s(&tmp))])
                .map_err(|_| "L'immagine della firma non si apre.".to_string())?;
            let dims: Vec<f64> = run_cmd("identify", &["-format", "%w %h", &s(&tmp)])?.split_whitespace().filter_map(|x| x.parse().ok()).collect();
            let png = std::fs::read(&tmp).map_err(|e| e.to_string());
            let _ = std::fs::remove_file(&tmp);
            let png = png?;
            let (w, h) = match dims[..] { [w, h] if w > 0.0 && h > 0.0 => (w, h), _ => return Err("L'immagine della firma non si apre.".into()) };
            let media = (1..).map(|k| format!("halo_firma{}.png", k)).find(|m| !all.contains(&format!("word/media/{}", m))).unwrap();
            let mut v: Vec<(String, Vec<u8>)> = add_signature(&doc, &rels, &ctypes, &media, w, h, align)?.into_iter().map(|(p, x)| (p, x.into_bytes())).collect();
            v.push((format!("word/media/{}", media), png));
            v
        }
    };
    write_parts(input, &out, &changed)?;
    Ok(format!("Creato: {}", name(&out)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    const RELS: &str = "<Relationships xmlns=\"x\"><Relationship Id=\"rId7\" Type=\"h\" Target=\"header1.xml\"/></Relationships>";

    #[test]
    fn intestazioni() {
        let hdr = "<w:hdr xmlns:w=\"w\"><w:p/></w:hdr>";
        let headers = HashMap::from([("word/header1.xml".to_string(), hdr.to_string())]);
        // Due sezioni: la prima ha solo l'intestazione normale, la seconda eredita.
        let doc = "<w:body><w:p><w:pPr><w:sectPr><w:headerReference w:type=\"default\" r:id=\"rId7\"/><w:titlePg/></w:sectPr></w:pPr></w:p><w:sectPr/></w:body>";
        let ch = add_watermark(doc, RELS, "<Types></Types>", &headers, "BOZZA \"1\"").unwrap();
        let d = &ch[0].1;
        assert!(d.contains("<w:sectPr><w:headerReference w:type=\"first\" r:id=\"rIdHalo1\"/><w:headerReference w:type=\"even\" r:id=\"rIdHalo1\"/><w:headerReference w:type=\"default\" r:id=\"rId7\"/>"), "{}", d);
        assert!(d.ends_with("<w:sectPr/></w:body>"), "la seconda sezione eredita: {}", d);
        let h1 = &ch.iter().find(|(p, _)| p == "word/header1.xml").unwrap().1;
        assert!(h1.contains("xmlns:v=\"urn:schemas-microsoft-com:vml\"") && h1.contains("string=\"BOZZA &quot;1&quot;\""));
        assert!(ch.iter().any(|(p, x)| p == "[Content_Types].xml" && x.contains("/word/header_halo1.xml")));
        assert!(ch.iter().any(|(p, x)| p == "word/_rels/document.xml.rels" && x.contains("Id=\"rIdHalo1\"") && x.contains("Target=\"header_halo1.xml\"")));
        // Senza intestazioni: <w:sectPr/> diventa un sectPr con i tre riferimenti.
        let ch = add_watermark("<w:body><w:p/><w:sectPr/></w:body>", RELS, "<Types></Types>", &HashMap::new(), "X").unwrap();
        assert!(ch[0].1.contains("<w:sectPr><w:headerReference w:type=\"default\" r:id=\"rIdHalo1\"/>"));

        let s = add_signature("<w:document xmlns:w=\"w\"><w:body><w:p/><w:tbl></w:tbl><w:sectPr><w:pgSz/></w:sectPr></w:body></w:document>",
            RELS, "<Types><Default Extension=\"png\" ContentType=\"image/png\"/></Types>", "f.png", 400.0, 100.0, "right").unwrap();
        assert!(s[0].1.contains("</w:tbl><w:p><w:pPr><w:jc w:val=\"right\"/>") && s[0].1.contains("xmlns:wp="));
        assert!(s[0].1.contains("cx=\"1800000\" cy=\"450000\""));
        assert_eq!(s[2].1.matches("Extension=\"png\"").count(), 1);
    }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-word");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let md = dir.join("doc.md");
        fs::write(&md, "# Relazione\n\nTesto della relazione.\n").unwrap();
        let docx = dir.join("Relazione.docx");
        run_cmd("pandoc", &[&s(&md), "-o", &s(&docx)]).unwrap();
        let prima = fs::read(&docx).unwrap();
        let d = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>();

        assert_eq!(run(&docx, "document/proteggi/filigrana", &d(&[("valore", "BOZZA")])).unwrap(), "Creato: Relazione_filigrana.docx");
        let firma = dir.join("firma.png");
        run_cmd("convert", &["-size", "600x200", "xc:none", "-stroke", "#1E3A8A", "-strokewidth", "8", "-fill", "none", "-draw", "bezier 20,150 150,20 300,180 580,40", &s(&firma)]).unwrap();
        assert_eq!(run(&docx, "document/proteggi/firma", &d(&[("file", &s(&firma)), ("allineamento", "destra")])).unwrap(), "Creato: Relazione_firmato.docx");
        // Seconda filigrana sullo stesso file: rifiutata.
        assert!(run(&dir.join("Relazione_filigrana.docx"), "document/proteggi/filigrana", &d(&[("valore", "X")])).unwrap_err().contains("già"));

        // LibreOffice apre entrambi i file: la filigrana scurisce il centro della pagina, la firma è un'immagine.
        for f in ["Relazione.docx", "Relazione_filigrana.docx", "Relazione_firmato.docx"] {
            run_cmd("soffice", &["--headless", "--convert-to", "pdf", "--outdir", &s(&dir), &s(&dir.join(f))]).unwrap();
        }
        let centro = |f: &str| {
            run_cmd("pdftoppm", &["-r", "40", "-png", "-singlefile", &s(&dir.join(f)), &s(&dir.join("p"))]).unwrap();
            run_cmd("convert", &[&s(&dir.join("p.png")), "-gravity", "center", "-crop", "60%x30%+0+0", "-format", "%[fx:mean]", "info:"]).unwrap().parse::<f64>().unwrap()
        };
        let (a, b) = (centro("Relazione.pdf"), centro("Relazione_filigrana.pdf"));
        assert!(b < a - 0.02, "filigrana non visibile: {} {}", a, b);
        assert!(run_cmd("pdfimages", &["-list", &s(&dir.join("Relazione_firmato.pdf"))]).unwrap().lines().count() > 2, "firma non trovata");
        assert_eq!(fs::read(&docx).unwrap(), prima, "l'originale non va toccato");
    }
}
