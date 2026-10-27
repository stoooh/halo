//! Fogli Excel (.xlsx) con openpyxl e password di .docx/.xlsx con msoffcrypto:
//! il lavoro lo fa lo script Python fogli.py, incluso nell'eseguibile e lanciato con
//! `/usr/bin/python3 -I -c`. La password passa sull'ingresso standard.

use crate::actions::{name, out_path, s};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const SCRIPT: &str = include_str!("fogli.py");

pub const KEYS: &[&str] = &[
    "sheet/converti/json",
    "sheet/pulisci/duplicati", "sheet/pulisci/righe-vuote", "sheet/pulisci/spazi-in-piu", "sheet/pulisci/formato-date",
    "sheet/grafico/barre", "sheet/grafico/linee", "sheet/grafico/torta", "sheet/grafico/pivot",
    "sheet/ordina/a-z", "sheet/ordina/z-a", "sheet/ordina/per-data", "sheet/ordina/filtra",
    "sheet/proteggi/password", "sheet/proteggi/blocca-celle", "sheet/proteggi/nascondi-formule",
    "document/proteggi/password",
];

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let suffix = match key {
        "sheet/converti/json" => return Some(out_path(input, "", "json")),
        k if k.starts_with("sheet/pulisci/") => "_pulito",
        "sheet/grafico/pivot" => "_riepilogo",
        k if k.starts_with("sheet/grafico/") => "_grafico",
        "sheet/ordina/filtra" => "_filtri",
        k if k.starts_with("sheet/ordina/") => "_ordinato",
        k if k.ends_with("/password") => "_protetto",
        "sheet/proteggi/blocca-celle" => "_bloccato",
        "sheet/proteggi/nascondi-formule" => "_formule_nascoste",
        _ => return None,
    };
    Some(out_path(input, suffix, &ext))
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    let want = if key.starts_with("document/") { "docx" } else { "xlsx" };
    if ext != want {
        return Err(format!("Funziona con i file .{}: convertilo prima in .{}.", want, want));
    }
    let action = if key.ends_with("/password") { "cifra" } else { key.rsplit('/').next().unwrap() };
    let mut stdin = String::new();
    if action == "cifra" {
        let pw = data.get("password").map(|p| p.as_str()).unwrap_or("");
        if pw.is_empty() { return Err("Serve una password.".into()); }
        if pw.contains('\n') || pw.contains('\r') { return Err("La password non può contenere a capo.".into()); }
        stdin = format!("{}\n", pw);
    }
    let out = planned_out(input, key).ok_or("Opzione non disponibile.")?;
    let child = Command::new("/usr/bin/python3").args(["-I", "-c", SCRIPT, action, &s(input), &s(&out)])
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn();
    let mut child = child.map_err(|_| "Manca Python 3. Installa con: sudo apt install python3".to_string())?;
    if let Some(mut si) = child.stdin.take() { let _ = si.write_all(stdin.as_bytes()); }
    let o = child.wait_with_output().map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&o.stdout);
    let stderr = String::from_utf8_lossy(&o.stderr);
    let last = stdout.lines().last().unwrap_or("");
    if o.status.success() {
        if let Some(m) = last.strip_prefix("OK ") {
            return Ok(if action == "cifra" { format!("Creato: {}", name(&out)) } else { format!("Creato: {} ({})", name(&out), m) });
        }
    }
    let _ = fs::remove_file(&out);
    if let Some(m) = last.strip_prefix("NO ") { return Err(m.to_string()); }
    if stderr.contains("No module named 'openpyxl'") { return Err("Manca «python3-openpyxl». Installa con: sudo apt install python3-openpyxl".into()); }
    if stderr.contains("No module named 'msoffcrypto'") { return Err("Manca «python3-msoffcrypto-tool». Installa con: sudo apt install python3-msoffcrypto-tool".into()); }
    Err(format!("Python non è riuscito: {}", stderr.lines().filter(|l| !l.trim().is_empty()).last().unwrap_or("errore sconosciuto")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::run_cmd;

    fn py(code: &str) -> String {
        let o = Command::new("/usr/bin/python3").args(["-I", "-c", code]).output().unwrap();
        assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).into_owned()
    }

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-fogli");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let x = dir.join("spese.xlsx");
        // Intestazione, una riga doppia, una vuota in mezzo, spazi, date come testo, una formula, grassetto.
        py(&format!(r#"
import openpyxl, datetime as dt
wb = openpyxl.Workbook(); ws = wb.active; ws.title = "Spese"
ws.append(["Voce", "Importo", "Data", "Categoria"])
ws.append(["Pane",  3.5, "2026-03-02", "Cibo"])
ws.append(["affitto", 800, dt.datetime(2026, 3, 1), "Casa"])
ws.append([None, None, None, None])
ws.append(["  Luce   di marzo ", 60, "15/03/2026", "Casa"])
ws.append(["Pane",  3.5, "2026-03-02", "Cibo"])
ws.append(["Totale", "=SUM(B2:B6)", None, None])
ws["A1"].font = openpyxl.styles.Font(bold=True)
wb.properties.creator = "Test"
wb.create_sheet("Altro")["A1"] = "resta"
wb.save(r"{}")
"#, s(&x)));
        let prima = fs::read(&x).unwrap();
        let ok = |k: &str| { let r = run(&x, k, &HashMap::new()); assert!(r.is_ok(), "{}: {:?}", k, r); r.unwrap() };
        let leggi = |f: &str, expr: &str| py(&format!("import openpyxl\nwb = openpyxl.load_workbook(r'{}')\nws = wb['Spese']\nprint({})", s(&dir.join(f)), expr)).trim().to_string();

        assert_eq!(ok("sheet/pulisci/duplicati"), "Creato: spese_pulito.xlsx (1 riga doppia tolta)");
        assert_eq!(leggi("spese_pulito.xlsx", "[ws.max_row, ws['A1'].font.bold, wb.sheetnames]"), "[6, True, ['Spese', 'Altro']]");
        assert_eq!(ok("sheet/pulisci/righe-vuote"), "Creato: spese_pulito (2).xlsx (1 riga vuota tolta)");
        assert!(ok("sheet/pulisci/spazi-in-piu").contains("1 cella sistemata"));
        assert_eq!(leggi("spese_pulito (3).xlsx", "ws['A5'].value"), "Luce di marzo");
        assert!(ok("sheet/pulisci/formato-date").contains("4 date"));
        assert_eq!(leggi("spese_pulito (4).xlsx", "[ws['C2'].value.isoformat(), ws['C5'].number_format]"), "['2026-03-02T00:00:00', 'DD/MM/YYYY']");

        assert!(ok("sheet/ordina/a-z").contains("per «Voce»"));
        assert_eq!(leggi("spese_ordinato.xlsx", "[ws.cell(r, 1).value for r in range(2, 8)]"), "['  Luce   di marzo ', 'affitto', 'Pane', 'Pane', 'Totale', None]");
        ok("sheet/ordina/z-a");
        assert_eq!(leggi("spese_ordinato (2).xlsx", "ws['A2'].value"), "Totale");
        assert!(ok("sheet/ordina/per-data").contains("per «Data»"));
        assert_eq!(leggi("spese_ordinato (3).xlsx", "ws['A2'].value"), "affitto");
        ok("sheet/ordina/filtra");
        assert_eq!(leggi("spese_filtri.xlsx", "ws.auto_filter.ref"), "A1:D7");

        for k in ["barre", "linee", "torta"] { ok(&format!("sheet/grafico/{}", k)); }
        assert_eq!(leggi("spese_grafico.xlsx", "len(ws._charts)"), "1");
        // Il grafico si ferma prima della riga "Totale" (riga 7).
        assert!(fs::read(dir.join("spese_grafico.xlsx")).is_ok());
        let xml = run_cmd("7z", &["x", "-so", &s(&dir.join("spese_grafico.xlsx")), "xl/charts/chart1.xml"]).unwrap();
        assert!(xml.contains("$B$2:$B$6") && !xml.contains("$B$7"), "{}", xml);
        assert!(ok("sheet/grafico/pivot").contains("«Riepilogo»"));
        assert_eq!(py(&format!("import openpyxl\nws = openpyxl.load_workbook(r'{}')['Riepilogo']\nprint([list(r) for r in ws.iter_rows(values_only=True)])", s(&dir.join("spese_riepilogo.xlsx")))).trim(),
            "[['Voce', 'Somma di Importo'], ['  Luce   di marzo ', 60], ['affitto', 800], ['Pane', 7]]");

        ok("sheet/proteggi/blocca-celle");
        assert_eq!(leggi("spese_bloccato.xlsx", "ws.protection.sheet"), "True");
        assert!(ok("sheet/proteggi/nascondi-formule").contains("1 formula nascosta"));
        assert_eq!(leggi("spese_formule_nascoste.xlsx", "ws['B7'].protection.hidden"), "True");

        assert!(ok("sheet/converti/json").starts_with("Creato: spese.json (5 righe"));
        let j: serde_json::Value = serde_json::from_str(&fs::read_to_string(dir.join("spese.json")).unwrap()).unwrap();
        assert_eq!(j[1]["Voce"], "affitto");
        assert_eq!(j[1]["Data"], "2026-03-01T00:00:00");

        let pw = HashMap::from([("password".to_string(), "Però-1 !".to_string())]);
        assert_eq!(run(&x, "sheet/proteggi/password", &pw).unwrap(), "Creato: spese_protetto.xlsx");
        let enc = dir.join("spese_protetto.xlsx");
        assert_eq!(py(&format!("import msoffcrypto\nf = msoffcrypto.OfficeFile(open(r'{}', 'rb'))\nprint(f.is_encrypted())", s(&enc))).trim(), "True");
        let dec = dir.join("chiaro.xlsx");
        py(&format!("import msoffcrypto\nf = msoffcrypto.OfficeFile(open(r'{}', 'rb'))\nf.load_key(password='Però-1 !')\nf.decrypt(open(r'{}', 'wb'))", s(&enc), s(&dec)));
        assert_eq!(fs::read(&dec).unwrap().len() > 1000, true);
        assert!(run(&enc, "sheet/pulisci/duplicati", &HashMap::new()).unwrap_err().contains("password"));

        let md = dir.join("doc.md");
        fs::write(&md, "Testo riservato.\n").unwrap();
        let docx = dir.join("doc.docx");
        run_cmd("pandoc", &[&s(&md), "-o", &s(&docx)]).unwrap();
        assert_eq!(run(&docx, "document/proteggi/password", &pw).unwrap(), "Creato: doc_protetto.docx");
        assert_eq!(py(&format!("import msoffcrypto\nprint(msoffcrypto.OfficeFile(open(r'{}', 'rb')).is_encrypted())", s(&dir.join("doc_protetto.docx")))).trim(), "True");
        assert_eq!(fs::read(&x).unwrap(), prima, "l'originale non va toccato");
        assert!(run(&dir.join("spese.json"), "sheet/pulisci/duplicati", &HashMap::new()).is_err());
    }
}
