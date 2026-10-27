//! Documento e PDF › Traduci: Halo non traduce da sé (niente intelligenza artificiale nell'app).
//! Prepara il prompt, lo copia negli appunti, apre lo strumento di IA scelto nel browser e
//! mostra il file nella cartella, da trascinare nella chat.

use crate::actions::s;
use crate::condividi::{copy_text, file_uri, show_args, spawn};
use std::path::Path;

/// Traduci è un'azione diretta (senza sottomenu): la lingua si sceglie nel pannello.
pub const KEYS: &[&str] = &["document/traduci", "pdf/traduci"];

/// Lingue del pannello: id, nome, nome nel prompt, sigla per il nome del file.
pub const LINGUE: &[(&str, &str, &str, &str)] = &[
    ("inglese", "Inglese", "inglese", "EN"), ("italiano", "Italiano", "italiano", "IT"),
    ("spagnolo", "Spagnolo", "spagnolo", "ES"), ("francese", "Francese", "francese", "FR"),
    ("tedesco", "Tedesco", "tedesco", "DE"), ("portoghese", "Portoghese", "portoghese", "PT"),
    ("russo", "Russo", "russo", "RU"), ("cinese", "Cinese", "cinese semplificato", "ZH"),
    ("giapponese", "Giapponese", "giapponese", "JA"), ("arabo", "Arabo", "arabo", "AR"),
];

/// Strumenti proposti nel pannello: id, nome, indirizzo.
pub const STRUMENTI: &[(&str, &str, &str)] = &[
    ("claude", "Claude", "https://claude.ai/new"),
    ("chatgpt", "ChatGPT", "https://chatgpt.com/"),
    ("gemini", "Gemini", "https://gemini.google.com/app"),
    ("deepseek", "DeepSeek", "https://chat.deepseek.com/"),
];

fn lingua(id: &str) -> Option<(&'static str, &'static str)> {
    LINGUE.iter().find(|l| l.0 == id).map(|l| (l.2, l.3))
}

/// Il prompt per tradurre `input` nella lingua `id`, con il formato del file da ricevere ("docx" o "pdf").
pub fn prompt(input: &Path, id: &str, formato: &str) -> Result<String, String> {
    let (lingua, sigla) = lingua(id).ok_or("Lingua non valida.")?;
    let ext = if formato == "pdf" { "pdf" } else { "docx" };
    let nome = input.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = input.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(format!(
        "Traduci in {lingua} il documento allegato «{nome}».\n\
         - Traduci tutto il testo, compresi titoli, tabelle, didascalie, intestazioni, piè di pagina e note.\n\
         - Mantieni la stessa struttura e formattazione: titoli, elenchi, grassetti, tabelle e impaginazione.\n\
         - Non riassumere e non aggiungere commenti o spiegazioni.\n\
         - Restituiscimi il documento tradotto come file {formato_up} da scaricare, chiamato «{stem}_{sigla}.{ext}».\n\
         Se non puoi creare file, scrivi il testo tradotto completo mantenendo la stessa struttura.",
        formato_up = ext.to_uppercase()))
}

/// Copia il prompt, apre lo strumento e mostra il file nella cartella.
pub fn open(input: &Path, key: &str, lingua: &str, tool: &str, formato: &str) -> Result<String, String> {
    if !KEYS.contains(&key) { return Err("Opzione non disponibile.".into()); }
    if !input.is_file() { return Err(format!("Il file non esiste più: {}", s(input))); }
    let (_, nome, url) = *STRUMENTI.iter().find(|t| t.0 == tool).ok_or("Strumento non valido.")?;
    copy_text(&prompt(input, lingua, formato)?)?;
    spawn("xdg-open", &[url.to_string()]).map_err(|_| format!("Non riesco ad aprire {} nel browser.", nome))?;
    spawn("gdbus", &show_args(&file_uri(input))).ok();
    Ok(format!("{} aperto: incolla il prompt con Ctrl+V e trascina «{}» nella chat.", nome,
        input.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_pronto() {
        let p = prompt(Path::new("/x/Relazione Q3.docx"), "tedesco", "docx").unwrap();
        assert!(p.starts_with("Traduci in tedesco il documento allegato «Relazione Q3.docx»."));
        assert!(p.contains("come file DOCX da scaricare, chiamato «Relazione Q3_DE.docx»"));
        assert!(prompt(Path::new("/x/a.docx"), "inglese", "pdf").unwrap().contains("«a_EN.pdf»"));
        assert!(prompt(Path::new("/x/a.pdf"), "cinese", "pdf").unwrap().starts_with("Traduci in cinese semplificato"));
        assert!(prompt(Path::new("/x/a.docx"), "klingon", "pdf").is_err());
        assert!(open(Path::new("/x/a.docx"), "document/traduci", "inglese", "altro", "docx").is_err());
        assert!(open(Path::new("/x/a.docx"), "document/traduci/inglese", "inglese", "claude", "docx").is_err(), "le vecchie chiavi non valgono più");
    }
}
