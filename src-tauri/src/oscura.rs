//! Oscura dati: email, telefoni, IBAN o testo scelto, in testi, .docx e PDF. Senza IA:
//! solo espressioni regolari, con controlli in più (cifre minime, cifra di controllo IBAN).
//!
//! - Testo e .docx: il dato viene sostituito da un'etichetta ([EMAIL], [TELEFONO]…).
//! - PDF: ogni pagina diventa un'immagine con rettangoli neri sopra i dati, così il dato
//!   sparisce davvero (un rettangolo disegnato sopra il testo lo lascerebbe copiabile).

use crate::actions::{name, out_path, run_cmd, s};
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

pub const KEYS: &[&str] = &[
    "pdf/oscura-dati/email", "pdf/oscura-dati/telefoni", "pdf/oscura-dati/iban", "pdf/oscura-dati/manuale",
    "document/oscura-dati/email", "document/oscura-dati/telefoni", "document/oscura-dati/iban", "document/oscura-dati/manuale",
    "text/oscura-dati/email", "text/oscura-dati/telefoni", "text/oscura-dati/iban", "text/oscura-dati/manuale",
];

#[derive(Clone, Copy, PartialEq, Debug)]
enum What { Email, Phone, Iban, Manual }

fn what(key: &str) -> Option<What> {
    Some(match key.rsplit('/').next()? {
        "email" => What::Email, "telefoni" => What::Phone, "iban" => What::Iban, "manuale" => What::Manual,
        _ => return None,
    })
}

fn label(w: What) -> &'static str {
    match w { What::Email => "[EMAIL]", What::Phone => "[TELEFONO]", What::Iban => "[IBAN]", What::Manual => "[OSCURATO]" }
}

/// Le parole scritte nel pannello per "Manuale", separate da virgole.
fn terms(data: &HashMap<String, String>) -> Vec<String> {
    data.get("valore").map(|v| v.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect()).unwrap_or_default()
}

/// Posizioni (in byte) dei dati da oscurare dentro `text`.
fn find(w: What, text: &str, terms: &[String]) -> Vec<(usize, usize)> {
    match w {
        What::Email => {
            let re = Regex::new(r"(?i)\b[a-z0-9._%+-]+@[a-z0-9-]+(?:\.[a-z0-9-]+)*\.[a-z]{2,}\b").unwrap();
            re.find_iter(text).map(|m| (m.start(), m.end())).collect()
        }
        What::Iban => ibans(text),
        What::Phone => {
            // +39 / 0039 facoltativo, poi cellulare (3xx) o fisso (0x…), cifre separate da spazi, punti, trattini o "/".
            let re = Regex::new(r"(?:(?:\+|00)39[ .\-]?)?(?:3\d|0\d)(?:[ .\-/]?\d){6,9}").unwrap();
            let skip = ibans(text);
            re.find_iter(text).filter(|m| {
                let digits = m.as_str().chars().filter(|c| c.is_ascii_digit()).count();
                let prefix = if m.as_str().starts_with('+') || m.as_str().starts_with("0039") { 2 + m.as_str().starts_with("00") as usize * 2 } else { 0 };
                let before = text[..m.start()].chars().last();
                let after = text[m.end()..].chars().next();
                (9..=11).contains(&(digits - prefix))
                    && !before.is_some_and(|c| c.is_alphanumeric() || c == '/' || c == '.' || c == ',')
                    && !after.is_some_and(|c| c.is_alphanumeric())
                    && !skip.iter().any(|&(a, b)| m.start() < b && a < m.end())
            }).map(|m| (m.start(), m.end())).collect()
        }
        What::Manual => {
            if terms.is_empty() { return Vec::new(); }
            let alt: Vec<String> = terms.iter().map(|t| regex::escape(t)).collect();
            let re = Regex::new(&format!("(?i){}", alt.join("|"))).unwrap();
            re.find_iter(text).map(|m| (m.start(), m.end())).collect()
        }
    }
}

/// Lunghezza dell'IBAN nei paesi più comuni (senza spazi).
fn iban_len(country: &str) -> Option<usize> {
    Some(match country {
        "IT" | "SM" | "FR" | "MC" | "GR" => 27, "VA" | "DE" | "GB" | "IE" => 22, "ES" | "SE" | "SK" | "CZ" => 24,
        "CH" | "LI" | "LV" => 21, "AT" | "LU" | "LT" | "EE" | "SI" => 20, "BE" => 16, "NL" => 18, "PT" => 25,
        "PL" | "HU" => 28, "DK" | "FI" => 18, "NO" => 15, "HR" => 21, "RO" => 24, "BG" | "MT" => 31, "CY" => 28,
        _ => return None,
    })
}

/// IBAN validi (lunghezza del paese e cifra di controllo mod-97), anche scritti a gruppi di 4.
fn ibans(text: &str) -> Vec<(usize, usize)> {
    let re = Regex::new(r"\b[A-Z]{2}\d{2}(?: ?[A-Z0-9]){11,30}").unwrap();
    let mut out = Vec::new();
    for m in re.find_iter(text) {
        let Some(want) = iban_len(&m.as_str()[..2]) else { continue };
        // Il pezzo trovato può includere una parola maiuscola che segue: si accorcia fino alla lunghezza giusta.
        let mut end = m.end();
        while end > m.start() + 15 {
            let cand = &text[m.start()..end];
            let len = cand.chars().filter(|c| !c.is_whitespace()).count();
            if len == want && !cand.ends_with(' ') && iban_ok(cand) && !text[end..].starts_with(|c: char| c.is_ascii_alphanumeric()) {
                out.push((m.start(), end));
                break;
            }
            end -= text[..end].chars().last().map(|c| c.len_utf8()).unwrap_or(1);
        }
    }
    out
}

fn iban_ok(v: &str) -> bool {
    let c: String = v.chars().filter(|c| !c.is_whitespace()).collect();
    if !(15..=34).contains(&c.len()) { return false; }
    let r = format!("{}{}", &c[4..], &c[..4]);
    let mut rem: u32 = 0;
    for ch in r.chars() {
        let d = match ch.to_digit(36) { Some(d) => d, None => return false };
        for x in d.to_string().chars() {
            rem = (rem * 10 + x.to_digit(10).unwrap()) % 97;
        }
    }
    rem == 1
}

/// Versione mascherata di un dato trovato, da mostrare nell'anteprima.
fn mask(w: What, v: &str) -> String {
    let first: String = v.chars().take(1).collect();
    match w {
        What::Email => match v.find('@') { Some(i) => format!("{}•••{}", first, &v[i..]), None => format!("{}•••", first) },
        What::Phone | What::Iban => {
            let tail: String = v.chars().rev().filter(|c| c.is_ascii_alphanumeric()).take(2).collect::<Vec<_>>().into_iter().rev().collect();
            let head: String = v.chars().filter(|c| !c.is_whitespace()).take(if w == What::Iban { 4 } else { 1 }).collect();
            format!("{}•••{}", head, tail)
        }
        What::Manual => v.to_string(),
    }
}

fn nome_plurale(w: What, n: usize) -> &'static str {
    match (w, n == 1) {
        (What::Email, true) => "indirizzo email", (What::Email, false) => "indirizzi email",
        (What::Phone, true) => "numero di telefono", (What::Phone, false) => "numeri di telefono",
        (What::Iban, true) => "IBAN", (What::Iban, false) => "IBAN",
        (What::Manual, true) => "corrispondenza", (What::Manual, false) => "corrispondenze",
    }
}

/// Tutto il testo del file, per l'anteprima (per il PDF: le parole di tutte le pagine).
fn plain_texts(input: &Path, key: &str) -> Result<Vec<String>, String> {
    if key.starts_with("text/") {
        let b = fs::read(input).map_err(|e| e.to_string())?;
        return String::from_utf8(b).map(|t| vec![t]).map_err(|_| "Il file non è in UTF-8: convertilo prima con Codifica › UTF-8.".into());
    }
    if key.starts_with("pdf/") {
        return Ok(pdf_pages(input)?.into_iter().map(|p| p.text).collect());
    }
    let (_, parts) = docx_parts(input)?;
    Ok(parts.into_iter().map(|(_, xml)| docx_nodes(&xml).1).collect())
}

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    what(key)?;
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    Some(out_path(input, "_oscurato", &ext))
}

/// Anteprima per il pannello di conferma: cosa è stato trovato, già mascherato.
pub fn preview(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<(usize, String), String> {
    let w = what(key).ok_or("Opzione non disponibile.")?;
    check_format(input, key)?;
    let t = terms(data);
    if w == What::Manual && t.is_empty() { return Err("Scrivi le parole da nascondere, separate da virgole.".into()); }
    let mut found: Vec<String> = Vec::new();
    for text in plain_texts(input, key)? {
        for (a, b) in find(w, &text, &t) {
            found.push(mask(w, &text[a..b]));
        }
    }
    if found.is_empty() {
        let scan = if key.starts_with("pdf/") { " Se il PDF è una scansione, il testo non è leggibile." } else { "" };
        return Ok((0, format!("Nessun dato trovato: non verrà creato nessun file.{}", scan)));
    }
    let n = found.len();
    let mut uniq: Vec<String> = Vec::new();
    for f in found { if !uniq.contains(&f) { uniq.push(f); } }
    let more = if uniq.len() > 3 { ", …" } else { "" };
    Ok((n, format!("Trovati {} {}: {}{}", n, nome_plurale(w, n), uniq[..uniq.len().min(3)].join(", "), more)))
}

fn check_format(input: &Path, key: &str) -> Result<(), String> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if key.starts_with("document/") && ext != "docx" {
        return Err("Per i documenti funziona con i file .docx: convertilo prima in .docx.".into());
    }
    Ok(())
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let (n, _) = preview(input, key, data)?;
    if n == 0 { return Err("Nessun dato trovato: nessun file creato.".into()); }
    let w = what(key).unwrap();
    let t = terms(data);
    let out = planned_out(input, key).unwrap();
    let r = if key.starts_with("text/") {
        let text = String::from_utf8(fs::read(input).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
        fs::write(&out, replace_ranges(&text, &find(w, &text, &t), label(w))).map_err(|e| e.to_string())
    } else if key.starts_with("pdf/") {
        pdf_redact(input, &out, w, &t)
    } else {
        docx_redact(input, &out, w, &t)
    };
    if r.is_err() { let _ = fs::remove_file(&out); }
    r?;
    Ok(format!("Creato: {} ({} {} {})", name(&out), n, nome_plurale(w, n), if n == 1 { "oscurato" } else { "oscurati" }))
}

fn replace_ranges(text: &str, ranges: &[(usize, usize)], lab: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last = 0;
    for &(a, b) in ranges {
        if a < last { continue; }
        out.push_str(&text[last..a]);
        out.push_str(lab);
        last = b;
    }
    out.push_str(&text[last..]);
    out
}

// ---------------- PDF ----------------

struct Word { x0: f64, y0: f64, x1: f64, y1: f64, start: usize, end: usize }
struct Page { w: f64, h: f64, words: Vec<Word>, text: String }

fn unescape(v: &str) -> String {
    v.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&#39;", "'").replace("&amp;", "&")
}

fn attr(tag: &str, a: &str) -> f64 {
    tag.split(&format!("{}=\"", a)).nth(1).and_then(|r| r.split('"').next()).and_then(|v| v.parse().ok()).unwrap_or(0.0)
}

/// Parole con le loro posizioni (in punti) per ogni pagina, da `pdftotext -bbox-layout`.
fn pdf_pages(input: &Path) -> Result<Vec<Page>, String> {
    let html = run_cmd("pdftotext", &["-bbox-layout", &s(input), "-"])?;
    let mut pages = Vec::new();
    for chunk in html.split("<page ").skip(1) {
        let head = chunk.split('>').next().unwrap_or("");
        let mut p = Page { w: attr(head, "width"), h: attr(head, "height"), words: Vec::new(), text: String::new() };
        let mut last_y = f64::NAN;
        for wpart in chunk.split("<word ").skip(1) {
            let (tag, rest) = wpart.split_once('>').unwrap_or(("", ""));
            let txt = unescape(rest.split("</word>").next().unwrap_or(""));
            let y0 = attr(tag, "yMin");
            if !p.text.is_empty() { p.text.push(if (y0 - last_y).abs() > 2.0 { '\n' } else { ' ' }); }
            last_y = y0;
            let start = p.text.len();
            p.text.push_str(&txt);
            p.words.push(Word { x0: attr(tag, "xMin"), y0, x1: attr(tag, "xMax"), y1: attr(tag, "yMax"), start, end: p.text.len() });
        }
        pages.push(p);
    }
    Ok(pages)
}

/// Per ogni pagina, i riquadri (in punti, y dall'alto) delle parole che contengono uno dei termini.
/// Usata da Annota › Evidenzia.
pub fn manual_boxes(input: &Path, terms: &[String]) -> Result<Vec<Vec<(f64, f64, f64, f64)>>, String> {
    Ok(pdf_pages(input)?.iter().map(|p| {
        let hits = find(What::Manual, &p.text, terms);
        p.words.iter().filter(|w| hits.iter().any(|&(a, b)| w.start < b && a < w.end)).map(|w| (w.x0, w.y0, w.x1, w.y1)).collect()
    }).collect())
}

fn pdf_redact(input: &Path, out: &Path, w: What, terms: &[String]) -> Result<(), String> {
    const DPI: f64 = 200.0;
    let pages = pdf_pages(input)?;
    let tmp = std::env::temp_dir().join(format!("halo-oscura-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = (|| {
        run_cmd("pdftoppm", &["-r", "200", "-png", &s(input), &s(&tmp.join("p"))])?;
        let mut imgs: Vec<PathBuf> = fs::read_dir(&tmp).map_err(|e| e.to_string())?.filter_map(|e| e.ok().map(|e| e.path())).collect();
        imgs.sort_by_key(|p| p.file_stem().and_then(|s| s.to_str()).and_then(|s| s.rsplit('-').next()).and_then(|n| n.parse::<u32>().ok()).unwrap_or(0));
        if imgs.len() != pages.len() { return Err("Pagine del PDF non lette correttamente.".into()); }
        for (img, page) in imgs.iter().zip(&pages) {
            let hits = find(w, &page.text, terms);
            let boxes: Vec<String> = page.words.iter()
                .filter(|wd| hits.iter().any(|&(a, b)| wd.start < b && a < wd.end))
                .map(|wd| {
                    let k = DPI / 72.0;
                    format!("rectangle {:.0},{:.0} {:.0},{:.0}", (wd.x0 - 1.5) * k, (wd.y0 - 1.5) * k, (wd.x1 + 1.5) * k, (wd.y1 + 1.5) * k)
                }).collect();
            let _ = (page.w, page.h);
            if boxes.is_empty() { continue; }
            let mut args: Vec<String> = vec![s(img), "-fill".into(), "black".into()];
            for b in boxes { args.push("-draw".into()); args.push(b); }
            args.push(s(img));
            let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
            run_cmd("convert", &refs)?;
        }
        let mut args: Vec<String> = vec!["-units".into(), "PixelsPerInch".into(), "-density".into(), "200".into()];
        args.extend(imgs.iter().map(|p| s(p)));
        args.extend(["-compress".into(), "jpeg".into(), "-quality".into(), "85".into(), s(out)]);
        let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
        run_cmd("convert", &refs).map(|_| ())
    })();
    let _ = fs::remove_dir_all(&tmp);
    r
}

// ---------------- DOCX ----------------

/// Le parti di un .docx che contengono testo: corpo, intestazioni, piè di pagina, note, commenti.
fn docx_parts(input: &Path) -> Result<(Vec<String>, Vec<(String, String)>), String> {
    let list = crate::actions::seven_list(input)?;
    let re = Regex::new(r"^word/(document|header\d*|footer\d*|footnotes|endnotes|comments)\.xml$").unwrap();
    let names: Vec<String> = list.lines().filter_map(|l| l.strip_prefix("Path = ")).filter(|p| re.is_match(p)).map(|p| p.to_string()).collect();
    if names.is_empty() { return Err("Il file non sembra un documento Word valido.".into()); }
    let mut parts = Vec::new();
    for n in &names {
        parts.push((n.clone(), run_cmd("7z", &["x", "-so", &s(input), n])?));
    }
    Ok((names, parts))
}

/// Nodi di testo <w:t> (posizione del contenuto nell'XML) e testo completo, con "\n" tra i paragrafi.
fn docx_nodes(xml: &str) -> (Vec<(usize, usize, usize)>, String) {
    let re = Regex::new(r"<w:t(?:\s[^>]*)?>([^<]*)</w:t>|</w:p>").unwrap();
    let mut nodes = Vec::new();
    let mut text = String::new();
    for c in re.captures_iter(xml) {
        match c.get(1) {
            Some(m) => {
                let start = text.len();
                text.push_str(&unescape(m.as_str()));
                nodes.push((m.start(), m.end(), start));
            }
            None => text.push('\n'),
        }
    }
    (nodes, text)
}

fn xml_esc(v: &str) -> String {
    v.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

/// Sostituisce i dati nei nodi <w:t>, anche quando Word ha spezzato una parola in più nodi:
/// l'etichetta va nel nodo dove inizia il dato, il resto del dato viene tolto dagli altri.
fn docx_edit(xml: &str, w: What, terms: &[String]) -> String {
    let (nodes, text) = docx_nodes(xml);
    let hits = find(w, &text, terms);
    if hits.is_empty() { return xml.to_string(); }
    let mut out = String::with_capacity(xml.len());
    let mut last = 0;
    for (k, &(xs, xe, ts)) in nodes.iter().enumerate() {
        let te = nodes.get(k + 1).map(|n| n.2).unwrap_or(text.len()).min(ts + unescape(&xml[xs..xe]).len());
        if !hits.iter().any(|&(a, b)| a < te && ts < b) { continue; }
        let mut new = String::new();
        for (i, ch) in text[ts..te].char_indices() {
            let pos = ts + i;
            match hits.iter().find(|&&(a, b)| a <= pos && pos < b) {
                Some(&(a, _)) => if a == pos { new.push_str(label(w)); },
                None => new.push(ch),
            }
        }
        // Con xml:space="preserve" Word non toglie gli spazi all'inizio e alla fine del nodo.
        let tag_start = xml[..xs].rfind("<w:t").unwrap_or(xs);
        out.push_str(&xml[last..tag_start]);
        let tag = &xml[tag_start..xs];
        if tag.contains("xml:space") { out.push_str(tag); } else { out.push_str(&tag.replacen("<w:t", "<w:t xml:space=\"preserve\"", 1)); }
        out.push_str(&xml_esc(&new));
        last = xe;
    }
    out.push_str(&xml[last..]);
    out
}

fn docx_redact(input: &Path, out: &Path, w: What, terms: &[String]) -> Result<(), String> {
    let (_, parts) = docx_parts(input)?;
    let tmp = std::env::temp_dir().join(format!("halo-oscura-docx-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let r = (|| {
        fs::copy(input, out).map_err(|e| e.to_string())?;
        let mut changed = Vec::new();
        for (n, xml) in parts {
            let new = docx_edit(&xml, w, terms);
            if new != xml {
                let p = tmp.join(&n);
                fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
                fs::write(&p, new).map_err(|e| e.to_string())?;
                changed.push(n);
            }
        }
        let mut args = vec!["u".to_string(), "-tzip".into(), "-bd".into(), s(out)];
        args.extend(changed);
        let o = crate::sistema::cmd("7z").args(&args).current_dir(&tmp).output().map_err(|e| e.to_string())?;
        if o.status.success() { Ok(()) } else { Err("7z non è riuscito ad aggiornare il documento.".to_string()) }
    })();
    let _ = fs::remove_dir_all(&tmp);
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    const TESTO: &str = "Scrivi a mario.rossi@example.com o a info@studio-bianchi.it.\n\
        Cell. +39 333 123 4567, ufficio 06-1234567, fax 0039 02 9876543.\n\
        IBAN IT60 X054 2811 1010 0000 0123 456 BANCA ROSSI\n\
        Non sono dati: 03/03/2026, 1.234.567 euro, CAP 00184, codice IT60X0542811101000000123457, ordine 33312345678901.\n";

    fn trovati(w: What) -> Vec<&'static str> {
        find(w, TESTO, &["Rossi".to_string()]).into_iter().map(|(a, b)| &TESTO[a..b]).collect()
    }

    #[test]
    fn schemi() {
        assert_eq!(trovati(What::Email), ["mario.rossi@example.com", "info@studio-bianchi.it"]);
        assert_eq!(trovati(What::Phone), ["+39 333 123 4567", "06-1234567", "0039 02 9876543"]);
        assert_eq!(trovati(What::Iban), ["IT60 X054 2811 1010 0000 0123 456"]);
        assert_eq!(trovati(What::Manual), ["rossi", "ROSSI"]);
        assert!(iban_ok("IT60X0542811101000000123456") && !iban_ok("IT60X0542811101000000123457"));
        assert_eq!(mask(What::Email, "mario.rossi@example.com"), "m•••@example.com");
        assert_eq!(mask(What::Iban, "IT60 X054 2811 1010 0000 0123 456"), "IT60•••56");
        assert_eq!(replace_ranges("a b c", &[(2, 3)], "[X]"), "a [X] c");
    }

    fn sh(cmd: &str, args: &[&str]) { run_cmd(cmd, args).unwrap(); }

    fn scuro(pdf: &Path) -> f64 {
        run_cmd("convert", &["-density", "72", &format!("{}[0]", s(pdf)), "-background", "white", "-flatten", "-colorspace", "Gray", "-format", "%[fx:mean]", "info:"])
            .unwrap().trim().parse().unwrap()
    }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-oscura");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let txt = dir.join("dati.txt");
        fs::write(&txt, TESTO).unwrap();
        let md = dir.join("dati.md");
        fs::write(&md, TESTO.replace('\n', "\n\n")).unwrap();
        let docx = dir.join("dati.docx");
        sh("pandoc", &[&s(&md), "-o", &s(&docx)]);
        let ps = dir.join("dati.ps");
        let righe: Vec<String> = TESTO.lines().enumerate()
            .map(|(k, l)| format!("40 {} moveto ({}) show", 760 - k * 20, l.replace('(', "").replace(')', ""))).collect();
        fs::write(&ps, format!("%!PS\n/Helvetica findfont 9 scalefont setfont\n{}\nshowpage\n", righe.join("\n"))).unwrap();
        let pdf = dir.join("dati.pdf");
        sh("gs", &["-q", "-dBATCH", "-dNOPAUSE", "-sDEVICE=pdfwrite", &format!("-sOutputFile={}", s(&pdf)), &s(&ps)]);
        let originali: Vec<Vec<u8>> = [&txt, &docx, &pdf].iter().map(|f| fs::read(f).unwrap()).collect();
        let data = HashMap::from([("valore".to_string(), "Rossi, studio".to_string())]);

        for key in KEYS {
            let f = match key.split('/').next().unwrap() { "pdf" => &pdf, "document" => &docx, _ => &txt };
            let (n, msg) = preview(f, key, &data).unwrap();
            assert!(n > 0, "{}: {}", key, msg);
            let out = planned_out(f, key).unwrap();
            let r = run(f, key, &data);
            assert!(r.is_ok(), "{}: {:?}", key, r);
            let segreto = match what(key).unwrap() { What::Email => "mario.rossi@example.com", What::Phone => "333 123 4567", What::Iban => "IT60 X054", What::Manual => "Rossi" };
            match key.split('/').next().unwrap() {
                "text" => assert!(!fs::read_to_string(&out).unwrap().contains(segreto), "{}", key),
                "document" => {
                    let xml = run_cmd("7z", &["x", "-so", &s(&out), "word/document.xml"]).unwrap();
                    let t = docx_nodes(&xml).1;
                    assert!(!t.contains(segreto) && t.contains(label(what(key).unwrap())), "{}: {}", key, t);
                    assert!(crate::actions::run(&s(&out), "document/converti/testo").is_ok(), "{}: il .docx non si apre", key);
                }
                _ => {
                    assert_eq!(run_cmd("pdfinfo", &[&s(&out)]).unwrap().lines().find(|l| l.starts_with("Pages:")).map(|l| l.split_whitespace().last().unwrap().to_string()), Some("1".into()));
                    assert!(!run_cmd("pdftotext", &[&s(&out), "-"]).unwrap().contains(segreto), "{}", key);
                    assert!(scuro(&out) < scuro(&pdf) - 0.0005, "{}: nessun rettangolo nero", key);
                }
            }
        }
        let dopo: Vec<Vec<u8>> = [&txt, &docx, &pdf].iter().map(|f| fs::read(f).unwrap()).collect();
        assert!(originali == dopo, "un originale è cambiato");
        let vuoto = HashMap::from([("valore".to_string(), "Zanzibar".to_string())]);
        assert_eq!(run(&txt, "text/oscura-dati/manuale", &vuoto), Err("Nessun dato trovato: nessun file creato.".into()));
    }

    #[test]
    fn docx_nodi_spezzati() {
        // Word spezza spesso il testo in più nodi: "mario.rossi@" + "example.com".
        let xml = "<w:p><w:r><w:t>Mail: mario.rossi@</w:t></w:r><w:r><w:t xml:space=\"preserve\">example.com e altro</w:t></w:r></w:p><w:p><w:r><w:t>Fine &amp; saluti</w:t></w:r></w:p>";
        let out = docx_edit(xml, What::Email, &[]);
        assert_eq!(out, "<w:p><w:r><w:t xml:space=\"preserve\">Mail: [EMAIL]</w:t></w:r><w:r><w:t xml:space=\"preserve\"> e altro</w:t></w:r></w:p><w:p><w:r><w:t>Fine &amp; saluti</w:t></w:r></w:p>");
        assert_eq!(docx_nodes(&out).1, "Mail: [EMAIL] e altro\nFine & saluti\n");
    }
}
