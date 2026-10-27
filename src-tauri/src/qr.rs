//! Codici QR (crate qrcode, senza Internet): per il link di Condividi, così lo si apre col
//! telefono inquadrandolo, e per i testi brevi (Testo › Converti › QR code, es. una password del Wi-Fi).

use crate::actions::{name, out_path, run_cmd, s};
use qrcode::render::svg;
use qrcode::{EcLevel, QrCode};
use std::fs;
use std::path::{Path, PathBuf};

pub const KEYS: &[&str] = &["text/converti/qr-code"];
/// Oltre, il codice diventa troppo fitto per essere letto bene da un telefono.
const MAX_TEXT: usize = 1000;

/// Il QR come SVG (nero su bianco, con il margine bianco che serve per leggerlo).
pub fn svg(text: &str) -> Result<String, String> {
    let code = QrCode::with_error_correction_level(text.as_bytes(), EcLevel::M).map_err(|_| "Testo troppo lungo per un codice QR.".to_string())?;
    Ok(code.render::<svg::Color>().min_dimensions(320, 320).dark_color(svg::Color("#000000")).light_color(svg::Color("#ffffff")).quiet_zone(true).build())
}

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    KEYS.contains(&key).then(|| out_path(input, "_qr", "png"))
}

pub fn run(input: &Path, key: &str) -> Result<String, String> {
    let out = planned_out(input, key).ok_or("Opzione non disponibile.")?;
    let text = fs::read_to_string(input).map_err(|_| "Il file non è un testo leggibile (UTF-8).".to_string())?;
    let text = text.trim();
    if text.is_empty() { return Err("Il file è vuoto: niente da mettere nel codice QR.".into()); }
    if text.chars().count() > MAX_TEXT {
        return Err(format!("Testo troppo lungo per un codice QR leggibile ({} caratteri, al massimo {}). Va bene per link, indirizzi, password del Wi-Fi, brevi note.", text.chars().count(), MAX_TEXT));
    }
    let tmp = std::env::temp_dir().join(format!("halo-qr-{}.svg", std::process::id()));
    fs::write(&tmp, svg(text)?).map_err(|e| e.to_string())?;
    let r = run_cmd("convert", &["-density", "300", &s(&tmp), "-resize", "800x800", &s(&out)]);
    let _ = fs::remove_file(&tmp);
    r?;
    Ok(format!("Creato: {} (codice QR con il testo del file: inquadralo con il telefono)", name(&out)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn svg_valido() {
        let s = svg("https://gofile.io/d/Ab12").unwrap();
        assert!(s.starts_with("<?xml") && s.contains("<svg") && s.contains("#000000"));
    }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-qr");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let t = dir.join("wifi.txt");
        fs::write(&t, "WIFI:T:WPA;S:Casa;P:prova123;;\n").unwrap();
        assert!(run(&t, "text/converti/qr-code").unwrap().starts_with("Creato: wifi_qr.png"));
        assert_eq!(run_cmd("identify", &["-format", "%wx%h", &s(&dir.join("wifi_qr.png"))]).unwrap(), "800x800");
        let lungo = dir.join("lungo.txt");
        fs::write(&lungo, "x".repeat(2000)).unwrap();
        assert!(run(&lungo, "text/converti/qr-code").unwrap_err().starts_with("Testo troppo lungo"));
    }
}
