//! Opzioni sugli archivi che scelgono dei file dall'elenco del contenuto:
//! Estrai › Solo alcuni, Contenuto › Rimuovi file, Sostituisci, Rinomina.
//! L'archivio originale non viene mai toccato: si lavora su una copia.

use crate::actions::{name, out_dir, out_path, run_cmd, s, seven_in};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const KEYS: &[&str] = &[
    "archive/estrai/solo-alcuni", "archive/contenuto/rimuovi-file", "archive/contenuto/sostituisci", "archive/contenuto/rinomina",
];

/// Massimo di voci mostrate nell'elenco del pannello.
const MAX_ENTRIES: usize = 2000;

/// I file (non le cartelle) contenuti nell'archivio, con la dimensione.
pub fn entries(input: &Path) -> Result<Vec<(String, u64)>, String> {
    let list = run_cmd("7z", &["l", "-slt", &s(input)]).map_err(|e| {
        if e.contains("Can not open encrypted") || e.contains("Wrong password") { "L'archivio ha i nomi cifrati: toglila prima con Proteggi › Rimuovi password.".to_string() } else { e }
    })?;
    let mut out = Vec::new();
    for block in list.split("\n\n").skip(1) {
        let get = |k: &str| block.lines().find_map(|l| l.strip_prefix(k)).map(|v| v.trim().to_string());
        // Il primo blocco con "Type = " descrive l'archivio stesso, non un file contenuto.
        if get("Folder = ").as_deref() == Some("+") || get("Type = ").is_some() { continue; }
        if let Some(p) = get("Path = ") {
            out.push((p, get("Size = ").and_then(|v| v.parse().ok()).unwrap_or(0)));
        }
        if out.len() > MAX_ENTRIES { return Err(format!("L'archivio contiene più di {} file: troppi da mostrare.", MAX_ENTRIES)); }
    }
    if out.is_empty() { return Err("L'archivio è vuoto.".into()); }
    Ok(out)
}

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    Some(match key {
        "archive/estrai/solo-alcuni" => out_dir(input, ""),
        k if k.starts_with("archive/contenuto/") => out_path(input, "_modificato", if ["zip", "7z"].contains(&ext.as_str()) { &ext } else { "zip" }),
        _ => return None,
    })
}

/// Le voci scelte nel pannello, che devono esistere davvero nell'archivio.
fn chosen(input: &Path, data: &HashMap<String, String>, single: bool) -> Result<Vec<String>, String> {
    let all: Vec<String> = entries(input)?.into_iter().map(|(p, _)| p).collect();
    let v: Vec<String> = data.get("voci").map(|f| f.lines().filter(|l| !l.is_empty()).map(|l| l.to_string()).collect()).unwrap_or_default();
    if v.is_empty() { return Err("Scegli almeno un file dall'elenco.".into()); }
    if single && v.len() != 1 { return Err("Scegli un solo file dall'elenco.".into()); }
    if let Some(x) = v.iter().find(|x| !all.contains(x)) { return Err(format!("«{}» non è nell'archivio.", x)); }
    Ok(v)
}

/// Copia di lavoro dell'archivio: ZIP e 7z si copiano, gli altri formati diventano ZIP.
fn working_copy(input: &Path, out: &Path) -> Result<(), String> {
    if seven_in(&["t", &s(input)], "\n").is_err_and(|e| e == "Password errata.") {
        return Err("L'archivio ha una password: toglila prima con Proteggi › Rimuovi password.".into());
    }
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    if ext == "zip" || ext == "7z" {
        fs::copy(input, out).map(|_| ()).map_err(|e| e.to_string())
    } else {
        crate::actions::repack(input, out, None, None, &[])
    }
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let i = s(input);
    let out = planned_out(input, key).ok_or("Opzione non disponibile.")?;
    match key {
        "archive/estrai/solo-alcuni" => {
            let v = chosen(input, data, false)?;
            // -spf mantiene i percorsi interni; la cartella di destinazione è sempre nuova.
            let mut args: Vec<String> = vec!["x".into(), format!("-o{}", s(&out)), i.clone()];
            args.extend(v.iter().cloned());
            let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
            seven_in(&refs, "\n").map_err(|e| {
                let _ = fs::remove_dir_all(&out);
                if e == "Password errata." { "L'archivio ha una password: toglila prima con Proteggi › Rimuovi password.".to_string() } else { e }
            })?;
            Ok(format!("Creato: {} ({} file estratti)", name(&out), v.len()))
        }
        "archive/contenuto/rimuovi-file" => {
            let v = chosen(input, data, false)?;
            if v.len() == entries(input)?.len() { return Err("Non puoi togliere tutti i file: l'archivio resterebbe vuoto.".into()); }
            working_copy(input, &out)?;
            let mut args: Vec<String> = vec!["d".into(), s(&out)];
            args.extend(v.iter().cloned());
            let refs: Vec<&str> = args.iter().map(|a| a.as_str()).collect();
            if let Err(e) = seven_in(&refs, "") { let _ = fs::remove_file(&out); return Err(e); }
            Ok(format!("Creato: {} ({} file tolti)", name(&out), v.len()))
        }
        "archive/contenuto/rinomina" => {
            let old = chosen(input, data, true)?.remove(0);
            let new_name = data.get("valore").map(|v| v.trim()).unwrap_or("");
            if new_name.is_empty() || new_name.contains('/') || new_name == "." || new_name == ".." {
                return Err("Scrivi il nuovo nome (senza «/»).".into());
            }
            let new = match old.rsplit_once('/') { Some((dir, _)) => format!("{}/{}", dir, new_name), None => new_name.to_string() };
            if entries(input)?.iter().any(|(p, _)| *p == new) { return Err(format!("Nell'archivio c'è già «{}».", new)); }
            working_copy(input, &out)?;
            if let Err(e) = seven_in(&["rn", &s(&out), &old, &new], "") { let _ = fs::remove_file(&out); return Err(e); }
            Ok(format!("Creato: {} («{}» → «{}»)", name(&out), old, new_name))
        }
        "archive/contenuto/sostituisci" => {
            let old = chosen(input, data, true)?.remove(0);
            let file = data.get("file").map(PathBuf::from).filter(|f| f.is_file()).ok_or("Scegli il file che prende il suo posto.")?;
            working_copy(input, &out)?;
            // Il file nuovo va messo in una cartella temporanea con lo stesso percorso interno.
            let tmp = std::env::temp_dir().join(format!("halo-sostituisci-{}", std::process::id()));
            let _ = fs::remove_dir_all(&tmp);
            let r = (|| {
                let dest = tmp.join(&old);
                fs::create_dir_all(dest.parent().unwrap()).map_err(|e| e.to_string())?;
                fs::copy(&file, &dest).map_err(|e| e.to_string())?;
                let o = Command::new("7z").args(["u", "-bd", "-y", &s(&out), &old]).current_dir(&tmp).output().map_err(|e| e.to_string())?;
                if o.status.success() { Ok(()) } else { Err("7z non è riuscito ad aggiornare l'archivio.".to_string()) }
            })();
            let _ = fs::remove_dir_all(&tmp);
            if let Err(e) = r { let _ = fs::remove_file(&out); return Err(e); }
            Ok(format!("Creato: {} («{}» sostituito con {})", name(&out), old, name(&file)))
        }
        _ => Err("Opzione non disponibile.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-archivi");
        let _ = fs::remove_dir_all(&dir);
        let src = dir.join("progetto");
        fs::create_dir_all(src.join("doc")).unwrap();
        fs::write(src.join("leggimi.txt"), "ciao").unwrap();
        fs::write(src.join("doc/relazione.txt"), "versione 1").unwrap();
        fs::write(src.join("doc/bozza.txt"), "da togliere").unwrap();
        let zip = dir.join("arch.zip");
        run_cmd("7z", &["a", "-tzip", &s(&zip), &s(&src)]).unwrap();
        let prima = fs::read(&zip).unwrap();
        let names = |z: &Path| { let mut v: Vec<String> = entries(z).unwrap().into_iter().map(|e| e.0).collect(); v.sort(); v };
        assert_eq!(names(&zip), ["progetto/doc/bozza.txt", "progetto/doc/relazione.txt", "progetto/leggimi.txt"]);
        let d = |pairs: &[(&str, &str)]| pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect::<HashMap<_, _>>();

        assert_eq!(run(&zip, "archive/estrai/solo-alcuni", &d(&[("voci", "progetto/doc/relazione.txt")])).unwrap(), "Creato: arch (1 file estratti)");
        assert!(dir.join("arch/progetto/doc/relazione.txt").exists() && !dir.join("arch/progetto/leggimi.txt").exists());

        run(&zip, "archive/contenuto/rimuovi-file", &d(&[("voci", "progetto/doc/bozza.txt")])).unwrap();
        assert_eq!(names(&dir.join("arch_modificato.zip")), ["progetto/doc/relazione.txt", "progetto/leggimi.txt"]);

        run(&zip, "archive/contenuto/rinomina", &d(&[("voci", "progetto/leggimi.txt"), ("valore", "LEGGIMI.md")])).unwrap();
        assert!(names(&dir.join("arch_modificato (2).zip")).contains(&"progetto/LEGGIMI.md".to_string()));
        assert!(run(&zip, "archive/contenuto/rinomina", &d(&[("voci", "progetto/leggimi.txt"), ("valore", "a/b")])).is_err());

        let nuovo = dir.join("relazione v2.txt");
        fs::write(&nuovo, "versione 2").unwrap();
        run(&zip, "archive/contenuto/sostituisci", &d(&[("voci", "progetto/doc/relazione.txt"), ("file", &s(&nuovo))])).unwrap();
        let out = dir.join("arch_modificato (3).zip");
        assert_eq!(run_cmd("7z", &["x", "-so", &s(&out), "progetto/doc/relazione.txt"]).unwrap(), "versione 2");
        assert_eq!(names(&out).len(), 3);

        assert!(run(&zip, "archive/contenuto/rimuovi-file", &d(&[("voci", "non/esiste.txt")])).is_err());
        assert!(run(&zip, "archive/contenuto/rimuovi-file", &d(&[("voci", "progetto/doc/bozza.txt\nprogetto/doc/relazione.txt\nprogetto/leggimi.txt")])).is_err());
        assert_eq!(fs::read(&zip).unwrap(), prima, "l'originale non va toccato");
    }
}
