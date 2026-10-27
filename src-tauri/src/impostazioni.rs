//! Impostazioni di Halo (aspetto e scorciatoie) in ~/.config/halo/impostazioni.json.

use regex::Regex;
use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;

fn path() -> PathBuf {
    crate::sistema::config_dir().join("impostazioni.json")
}

/// Aspetto approvato: arancio, tema del sistema, vetro e grandezza normali.
pub fn defaults() -> Value {
    json!({ "aspetto": { "colore": "#C2410C", "tema": "auto", "vetro": "normale", "grandezza": "normale",
        "ingrandimento": "normale", "animazioni": "normali", "chiusura": "5" }, "scorciatoie": [] })
}

/// Tiene solo i valori validi; quelli mancanti o sbagliati tornano a quelli predefiniti.
pub fn clean(v: &Value) -> Value {
    let mut out = defaults();
    let a = &v["aspetto"];
    let hex = Regex::new(r"^#[0-9A-Fa-f]{6}$").unwrap();
    let pick = |k: &str, ok: &[&str]| a[k].as_str().filter(|x| ok.contains(x)).map(String::from);
    if let Some(c) = a["colore"].as_str().filter(|c| hex.is_match(c)) { out["aspetto"]["colore"] = json!(c.to_uppercase()); }
    if let Some(x) = pick("tema", &["auto", "chiaro", "scuro"]) { out["aspetto"]["tema"] = json!(x); }
    if let Some(x) = pick("vetro", &["leggero", "normale", "pieno"]) { out["aspetto"]["vetro"] = json!(x); }
    if let Some(x) = pick("grandezza", &["piccolo", "normale", "grande"]) { out["aspetto"]["grandezza"] = json!(x); }
    if let Some(x) = pick("ingrandimento", &["nessuno", "lieve", "normale", "forte"]) { out["aspetto"]["ingrandimento"] = json!(x); }
    if let Some(x) = pick("animazioni", &["normali", "ridotte"]) { out["aspetto"]["animazioni"] = json!(x); }
    if let Some(x) = pick("chiusura", &["3", "5", "10", "mai"]) { out["aspetto"]["chiusura"] = json!(x); }
    out["scorciatoie"] = crate::scorciatoie::clean_list(&v["scorciatoie"]);
    out
}

pub fn load() -> Value {
    fs::read_to_string(path()).ok().and_then(|t| serde_json::from_str(&t).ok()).map(|v| clean(&v)).unwrap_or_else(defaults)
}

/// Salva le impostazioni (scrive un file temporaneo e lo rinomina: niente file a metà).
pub fn save(v: &Value) -> Result<Value, String> {
    let p = path();
    let mut all = load();
    if let (Some(o), Some(src)) = (all.as_object_mut(), v.as_object()) {
        for (k, x) in src { o.insert(k.clone(), x.clone()); }
    }
    let v = clean(&all);
    fs::create_dir_all(p.parent().unwrap()).map_err(|e| format!("Non riesco a creare la cartella delle impostazioni: {}", e))?;
    let tmp = p.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string_pretty(&v).unwrap()).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &p).map_err(|e| e.to_string())?;
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valori_validi() {
        let v = clean(&json!({ "aspetto": { "colore": "#1d4ed8", "tema": "scuro", "vetro": "troppo", "grandezza": 3 }, "scorciatoie": [1] }));
        assert_eq!(v["aspetto"], json!({ "colore": "#1D4ED8", "tema": "scuro", "vetro": "normale", "grandezza": "normale",
            "ingrandimento": "normale", "animazioni": "normali", "chiusura": "5" }));
        let w = clean(&json!({ "aspetto": { "ingrandimento": "forte", "animazioni": "ridotte", "chiusura": "mai" } }));
        assert_eq!((w["aspetto"]["ingrandimento"].as_str(), w["aspetto"]["animazioni"].as_str(), w["aspetto"]["chiusura"].as_str()), (Some("forte"), Some("ridotte"), Some("mai")));
        assert_eq!(clean(&json!({ "aspetto": { "chiusura": "7" } }))["aspetto"]["chiusura"], "5");
        assert_eq!(v["scorciatoie"], json!([]), "voci non valide tolte");
        assert_eq!(clean(&json!({ "aspetto": { "colore": "red" } }))["aspetto"]["colore"], "#C2410C");
    }

    #[test]
    fn salva_e_rilegge() {
        let dir = std::env::temp_dir().join(format!("halo-test-impostazioni-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        // Solo per questo test: la cartella di configurazione è temporanea.
        std::env::set_var("XDG_CONFIG_HOME", &dir);
        assert_eq!(load(), defaults());
        save(&json!({ "aspetto": { "colore": "#15803D", "tema": "chiaro", "vetro": "pieno", "grandezza": "grande" } })).unwrap();
        assert_eq!(load()["aspetto"]["vetro"], "pieno");
        assert!(dir.join("halo/impostazioni.json").exists());
        std::env::remove_var("XDG_CONFIG_HOME");
        let _ = fs::remove_dir_all(&dir);
    }
}
