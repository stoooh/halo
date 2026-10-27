//! Opzioni per i documenti Word (.docx) e i fogli Excel (.xlsx) che si fanno modificando
//! direttamente le parti XML del file (che è uno ZIP): comprimere le immagini interne,
//! togliere i metadati, leggere le revisioni, proteggere in sola lettura.

use crate::actions::{human, name, out_path, run_cmd, s};
use regex::Regex;
use std::fs;
use std::path::{Path, PathBuf};

pub const KEYS: &[&str] = &[
    "document/comprimi/immagini-interne", "document/comprimi/bilanciata", "document/comprimi/massima", "document/comprimi/per-email",
    "document/metadati/rimuovi-tutti", "document/metadati/revisioni", "sheet/metadati/rimuovi-tutti",
    "document/proteggi/sola-lettura", "sheet/proteggi/sola-lettura",
    "slides/comprimi/immagini-interne", "slides/metadati/rimuovi-tutti",
];

fn ext(input: &Path) -> String {
    input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase()
}

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let e = ext(input);
    Some(match key {
        k if k.starts_with("document/comprimi/") || k.starts_with("slides/comprimi/") => out_path(input, "_compresso", &e),
        k if k.ends_with("/rimuovi-tutti") => out_path(input, "_senza_metadati", &e),
        k if k.ends_with("/sola-lettura") => out_path(input, "_sola_lettura", &e),
        _ => return None,
    })
}

fn need_ooxml(input: &Path, key: &str) -> Result<(), String> {
    let want = if key.starts_with("sheet/") { "xlsx" } else if key.starts_with("slides/") { "pptx" } else { "docx" };
    if ext(input) != want {
        return Err(format!("Funziona con i file .{}: convertilo prima in .{}.", want, want));
    }
    Ok(())
}

/// Nomi delle parti dentro il file Office.
pub(crate) fn parts(input: &Path) -> Result<Vec<String>, String> {
    Ok(crate::actions::seven_list(input)?.lines().filter_map(|l| l.strip_prefix("Path = ")).skip(1).map(|p| p.to_string()).collect())
}

pub(crate) fn read_part(input: &Path, part: &str) -> Result<String, String> {
    run_cmd("7z", &["x", "-so", &s(input), part])
}

/// Copia `input` in `out` e sostituisce le parti indicate (percorso → contenuto nuovo).
pub(crate) fn write_parts(input: &Path, out: &Path, new: &[(String, Vec<u8>)]) -> Result<(), String> {
    let tmp = std::env::temp_dir().join(format!("halo-office-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    let r = (|| {
        for (p, b) in new {
            let f = tmp.join(p);
            fs::create_dir_all(f.parent().unwrap()).map_err(|e| e.to_string())?;
            fs::write(&f, b).map_err(|e| e.to_string())?;
        }
        fs::copy(input, out).map_err(|e| e.to_string())?;
        let mut args = vec!["u".to_string(), "-tzip".into(), "-bd".into(), s(out)];
        args.extend(new.iter().map(|(p, _)| p.clone()));
        // Percorsi relativi dalla cartella temporanea: nell'archivio restano uguali.
        let o = crate::sistema::cmd("7z").args(&args).current_dir(&tmp).output().map_err(|e| e.to_string())?;
        if o.status.success() { Ok(()) } else { Err("7z non è riuscito ad aggiornare il file.".to_string()) }
    })();
    let _ = fs::remove_dir_all(&tmp);
    if r.is_err() { let _ = fs::remove_file(out); }
    r
}

const EMPTY_CORE: &str = "<?xml version=\"1.0\" encoding=\"UTF-8\" standalone=\"yes\"?>\n<cp:coreProperties \
    xmlns:cp=\"http://schemas.openxmlformats.org/package/2006/metadata/core-properties\" xmlns:dc=\"http://purl.org/dc/elements/1.1/\" \
    xmlns:dcterms=\"http://purl.org/dc/terms/\" xmlns:dcmitype=\"http://purl.org/dc/dcmitype/\" \
    xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\"></cp:coreProperties>";

/// Elementi di settings.xml che nello schema di Word vengono dopo documentProtection:
/// la protezione va inserita prima del primo di questi, altrimenti Word segnala il file come danneggiato.
const AFTER_PROTECTION: &[&str] = &["w:autoFormatOverride", "w:styleLockTheme", "w:styleLockQFSet", "w:defaultTabStop",
    "w:autoHyphenation", "w:consecutiveHyphenLimit", "w:hyphenationZone", "w:doNotHyphenateCaps", "w:showEnvelope",
    "w:summaryLength", "w:clickAndTypeStyle", "w:defaultTableStyle", "w:evenAndOddHeaders", "w:bookFoldRevPrinting",
    "w:bookFoldPrinting", "w:bookFoldPrintingSheets", "w:drawingGridHorizontalSpacing", "w:drawingGridVerticalSpacing",
    "w:displayHorizontalDrawingGridEvery", "w:displayVerticalDrawingGridEvery", "w:doNotUseMarginsForDrawingGridOrigin",
    "w:drawingGridHorizontalOrigin", "w:drawingGridVerticalOrigin", "w:doNotShadeFormData", "w:noPunctuationKerning",
    "w:characterSpacingControl", "w:printTwoOnOne", "w:strictFirstAndLastChars", "w:noLineBreaksAfter", "w:noLineBreaksBefore",
    "w:savePreviewPicture", "w:doNotValidateAgainstSchema", "w:saveInvalidXml", "w:ignoreMixedContent",
    "w:alwaysShowPlaceholderText", "w:doNotDemarcateInvalidXml", "w:saveXmlDataOnly", "w:useXSLTWhenSaving",
    "w:saveThroughXslt", "w:showXMLTags", "w:alwaysMergeEmptyNamespace", "w:updateFields", "w:hdrShapeDefaults",
    "w:footnotePr", "w:endnotePr", "w:compat", "w:docVars", "w:rsids", "m:mathPr", "w:attachedSchema", "w:themeFontLang",
    "w:clrSchemeMapping", "w:doNotIncludeSubdocsInStats", "w:doNotAutoCompressPictures", "w:forceUpgrade", "w:captions",
    "w:readModeInkLockDown", "w:smartTagType", "sl:schemaLibrary", "w:shapeDefaults", "w:doNotEmbedSmartTags",
    "w:decimalSymbol", "w:listSeparator"];

pub fn protect_read_only(settings: &str) -> String {
    const P: &str = "<w:documentProtection w:edit=\"readOnly\" w:enforcement=\"1\"/>";
    let re = Regex::new(r"<w:documentProtection\b[^>]*/>").unwrap();
    if re.is_match(settings) { return re.replace(settings, P).into_owned(); }
    let pos = AFTER_PROTECTION.iter().filter_map(|t| {
        let open = format!("<{}", t);
        settings.match_indices(&open).find(|(i, _)| settings[i + open.len()..].starts_with([' ', '/', '>'])).map(|(i, _)| i)
    }).min().or_else(|| settings.rfind("</w:settings>"));
    match pos {
        Some(p) => format!("{}{}{}", &settings[..p], P, &settings[p..]),
        None => settings.to_string(),
    }
}

pub fn run(input: &Path, key: &str) -> Result<String, String> {
    need_ooxml(input, key)?;
    let out = planned_out(input, key);
    match key {
        k if k.starts_with("document/comprimi/") || k.starts_with("slides/comprimi/") => {
            let (max, q) = match k.rsplit('/').next().unwrap() {
                "immagini-interne" => (2000, 85), "bilanciata" => (1600, 78), "per-email" => (1280, 70), _ => (1024, 60),
            };
            // Le foto stanno in word/media (Word) o ppt/media (PowerPoint).
            let media: Vec<String> = parts(input)?.into_iter().filter(|p| p.starts_with("word/media/") || p.starts_with("ppt/media/"))
                .filter(|p| [".jpg", ".jpeg", ".png"].iter().any(|e| p.to_lowercase().ends_with(e))).collect();
            if media.is_empty() { return Err("Il documento non contiene foto da comprimere (JPEG o PNG).".into()); }
            let tmp = std::env::temp_dir().join(format!("halo-media-{}", std::process::id()));
            let _ = fs::remove_dir_all(&tmp);
            fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
            let mut new = Vec::new();
            for (k2, m) in media.iter().enumerate() {
                let orig = crate::sistema::cmd("7z").args(["x", "-so", &s(input), m]).output().map_err(|e| e.to_string())?.stdout;
                let lower = m.to_lowercase();
                let src = tmp.join(format!("{}{}", k2, if lower.ends_with(".png") { ".png" } else { ".jpg" }));
                let dst = tmp.join(format!("{}_nuovo{}", k2, if lower.ends_with(".png") { ".png" } else { ".jpg" }));
                fs::write(&src, &orig).map_err(|e| e.to_string())?;
                let size = format!("{}x{}>", max, max);
                let qs = q.to_string();
                let mut args = vec![s(&src), "-resize".into(), size, "-strip".into()];
                if lower.ends_with(".png") { args.extend(["-define".into(), "png:compression-level=9".into()]); } else { args.extend(["-quality".into(), qs]); }
                args.push(s(&dst));
                let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
                if run_cmd("convert", &refs).is_ok() {
                    let b = fs::read(&dst).unwrap_or_default();
                    // Si sostituisce solo se l'immagine è davvero più leggera.
                    if !b.is_empty() && b.len() < orig.len() { new.push((m.clone(), b)); }
                }
            }
            let _ = fs::remove_dir_all(&tmp);
            if new.is_empty() { return Err("Le foto del documento sono già leggere: nessun file creato.".into()); }
            let out = out.unwrap();
            write_parts(input, &out, &new)?;
            let (a, b) = (fs::metadata(input).map(|m| m.len()).unwrap_or(0), fs::metadata(&out).map(|m| m.len()).unwrap_or(0));
            Ok(format!("Creato: {} ({} → {}, {} foto ridotte)", name(&out), human(a), human(b), new.len()))
        }
        "document/metadati/rimuovi-tutti" | "sheet/metadati/rimuovi-tutti" | "slides/metadati/rimuovi-tutti" => {
            let all = parts(input)?;
            let mut new: Vec<(String, Vec<u8>)> = Vec::new();
            if all.iter().any(|p| p == "docProps/core.xml") { new.push(("docProps/core.xml".into(), EMPTY_CORE.as_bytes().to_vec())); }
            if all.iter().any(|p| p == "docProps/app.xml") {
                let app = read_part(input, "docProps/app.xml")?;
                let re = Regex::new(r"(?s)<(Company|Manager|Template|HyperlinkBase)>.*?</(Company|Manager|Template|HyperlinkBase)>").unwrap();
                new.push(("docProps/app.xml".into(), re.replace_all(&app, "").into_owned().into_bytes()));
            }
            if new.is_empty() { return Err("Il file non contiene le proprietà del documento.".into()); }
            let out = out.unwrap();
            write_parts(input, &out, &new)?;
            Ok(format!("Creato: {} (autore, titolo, date e azienda tolti; gli autori di commenti e revisioni restano)", name(&out)))
        }
        "document/metadati/revisioni" => {
            let doc = read_part(input, "word/document.xml")?;
            let count = |tag: &str| Regex::new(&format!(r"<w:{}\b[^>]*>", tag)).unwrap().find_iter(&doc).count();
            let (ins, del) = (count("ins"), count("del"));
            let comments = parts(input)?.iter().any(|p| p == "word/comments.xml")
                .then(|| read_part(input, "word/comments.xml").unwrap_or_default()).unwrap_or_default();
            let ncom = Regex::new(r"<w:comment\b").unwrap().find_iter(&comments).count();
            let mut authors: Vec<String> = Vec::new();
            for c in Regex::new(r#"<w:(?:ins|del|comment)\b[^>]*w:author="([^"]*)""#).unwrap().captures_iter(&format!("{}{}", doc, comments)) {
                let a = c[1].replace("&amp;", "&").replace("&quot;", "\"");
                if !authors.contains(&a) { authors.push(a); }
            }
            if ins + del + ncom == 0 { return Ok("Nessuna revisione in sospeso e nessun commento.".into()); }
            Ok(format!("Revisioni non accettate: {} inserimenti, {} cancellazioni\nCommenti: {}\nAutori: {}",
                ins, del, ncom, if authors.is_empty() { "—".into() } else { authors.join(", ") }))
        }
        "document/proteggi/sola-lettura" => {
            let settings = read_part(input, "word/settings.xml")?;
            if !settings.contains("<w:settings") { return Err("Il documento non ha le impostazioni attese.".into()); }
            let out = out.unwrap();
            write_parts(input, &out, &[("word/settings.xml".into(), protect_read_only(&settings).into_bytes())])?;
            Ok(format!("Creato: {} (Word lo apre in sola lettura; è una protezione senza password)", name(&out)))
        }
        "sheet/proteggi/sola-lettura" => {
            let wb = read_part(input, "xl/workbook.xml")?;
            let out = out.unwrap();
            write_parts(input, &out, &[("xl/workbook.xml".into(), read_only_recommended(&wb).into_bytes())])?;
            Ok(format!("Creato: {} (Excel propone di aprirlo in sola lettura; è un consiglio, non una protezione)", name(&out)))
        }
        _ => Err("Opzione non disponibile.".into()),
    }
}

/// In workbook.xml, fileSharing va subito dopo fileVersion (se c'è) o in cima.
pub fn read_only_recommended(wb: &str) -> String {
    const F: &str = "<fileSharing readOnlyRecommended=\"1\"/>";
    let re = Regex::new(r"<fileSharing\b[^>]*/>").unwrap();
    if re.is_match(wb) { return re.replace(wb, F).into_owned(); }
    let after = Regex::new(r"<fileVersion\b[^>]*/>").unwrap().find(wb).map(|m| m.end())
        .or_else(|| Regex::new(r"<workbook\b[^>]*>").unwrap().find(wb).map(|m| m.end()));
    match after { Some(p) => format!("{}{}{}", &wb[..p], F, &wb[p..]), None => wb.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protezione_nel_posto_giusto() {
        let s = "<w:settings xmlns:w=\"x\"><w:zoom w:percent=\"100\"/><w:defaultTabStop w:val=\"720\"/><w:compat/></w:settings>";
        assert_eq!(protect_read_only(s), "<w:settings xmlns:w=\"x\"><w:zoom w:percent=\"100\"/><w:documentProtection w:edit=\"readOnly\" w:enforcement=\"1\"/><w:defaultTabStop w:val=\"720\"/><w:compat/></w:settings>");
        let gia = "<w:settings><w:documentProtection w:edit=\"comments\"/></w:settings>";
        assert_eq!(protect_read_only(gia), "<w:settings><w:documentProtection w:edit=\"readOnly\" w:enforcement=\"1\"/></w:settings>");
        assert!(protect_read_only("<w:settings><w:compatSetting/></w:settings>").starts_with("<w:settings><w:compatSetting/><w:documentProtection"));
    }

    #[test]
    fn sola_lettura_excel() {
        assert_eq!(read_only_recommended("<workbook xmlns=\"x\"><fileVersion appName=\"xl\"/><workbookPr/></workbook>"),
            "<workbook xmlns=\"x\"><fileVersion appName=\"xl\"/><fileSharing readOnlyRecommended=\"1\"/><workbookPr/></workbook>");
        assert_eq!(read_only_recommended("<workbook><workbookPr/></workbook>"), "<workbook><fileSharing readOnlyRecommended=\"1\"/><workbookPr/></workbook>");
    }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-documenti");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let img = dir.join("foto.jpg");
        run_cmd("convert", &["-size", "3000x2000", "plasma:", "-quality", "95", &s(&img)]).unwrap();
        let md = dir.join("doc.md");
        fs::write(&md, "---\ntitle: Segreto\nauthor: Mario Rossi\n---\n\nTesto.\n\n![foto](foto.jpg)\n").unwrap();
        let docx = dir.join("doc.docx");
        let o = crate::sistema::cmd("pandoc").args([s(&md), "-o".into(), s(&docx)]).current_dir(&dir).output().unwrap();
        assert!(o.status.success());
        let prima = fs::read(&docx).unwrap();

        let r = run(&docx, "document/comprimi/massima").unwrap();
        assert!(r.starts_with("Creato: doc_compresso.docx") && r.contains("1 foto ridotte"), "{}", r);
        let small = dir.join("doc_compresso.docx");
        assert!(fs::metadata(&small).unwrap().len() < prima.len() as u64 / 3);
        assert!(crate::actions::run(&s(&small), "document/converti/testo").is_ok(), "il documento compresso non si apre");

        run(&docx, "document/metadati/rimuovi-tutti").unwrap();
        let clean = dir.join("doc_senza_metadati.docx");
        let meta = run_cmd("exiftool", &["-s3", "-Creator", "-Title", &s(&clean)]).unwrap();
        assert!(!meta.contains("Mario") && !meta.contains("Segreto"), "{}", meta);
        assert!(crate::actions::run(&s(&clean), "document/converti/testo").is_ok());

        run(&docx, "document/proteggi/sola-lettura").unwrap();
        let ro = dir.join("doc_sola_lettura.docx");
        assert!(read_part(&ro, "word/settings.xml").unwrap().contains("w:edit=\"readOnly\""));
        assert!(crate::actions::run(&s(&ro), "document/converti/testo").is_ok());

        assert_eq!(run(&docx, "document/metadati/revisioni").unwrap(), "Nessuna revisione in sospeso e nessun commento.");
        assert_eq!(fs::read(&docx).unwrap(), prima, "l'originale non va toccato");
        assert!(run(&dir.join("doc.md"), "document/comprimi/massima").is_err());
    }
}
