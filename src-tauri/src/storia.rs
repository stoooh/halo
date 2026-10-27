//! Cronologia delle operazioni (Impostazioni › Cronologia): le ultime 50, con i file creati.
//! Sta in ~/.local/share/halo/cronologia.json, solo su questo computer. Dalla cronologia si
//! possono mostrare i file creati o spostarli nel cestino: mai i file di partenza.

use crate::actions::s;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Mutex;

const MAX: usize = 50;
static LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Voce {
    /// Secondi dal 1970.
    pub quando: u64,
    /// Il file (o il primo dei file) su cui è stata fatta l'operazione.
    pub file: String,
    /// Quanti file in tutto (più di 1 per le azioni su più file).
    #[serde(default = "uno")]
    pub n: usize,
    pub azione: String,
    pub esito: String,
    pub creati: Vec<String>,
}
fn uno() -> usize { 1 }

/// Per il pannello: la voce e quali file creati esistono ancora.
#[derive(Serialize)]
pub struct VoceVista {
    #[serde(flatten)]
    pub voce: Voce,
    pub esistono: Vec<bool>,
}

fn path() -> PathBuf {
    let base = std::env::var("XDG_DATA_HOME").ok().filter(|d| !d.is_empty()).map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| "/tmp".into())).join(".local/share"));
    base.join("halo").join("cronologia.json")
}

fn load() -> Vec<Voce> {
    fs::read_to_string(path()).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default()
}

fn save(v: &[Voce]) -> Result<(), String> {
    let p = path();
    fs::create_dir_all(p.parent().unwrap()).map_err(|e| e.to_string())?;
    let tmp = p.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string(v).unwrap()).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &p).map_err(|e| e.to_string())
}

/// Aggiunge un'operazione riuscita. Gli errori di scrittura non fermano l'azione.
pub fn add(file: &Path, n: usize, azione: &str, esito: &str, creati: &[String]) {
    // Durante i test non si scrive nella cronologia vera (solo il test della cronologia, che la sposta).
    if cfg!(test) && std::env::var("HALO_STORIA_TEST").is_err() { return; }
    let _g = LOCK.lock().unwrap();
    let mut v = load();
    let quando = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    v.insert(0, Voce { quando, file: s(file), n, azione: azione.into(), esito: esito.lines().next().unwrap_or("").into(), creati: creati.to_vec() });
    v.truncate(MAX);
    let _ = save(&v);
}

pub fn list() -> Vec<VoceVista> {
    load().into_iter().map(|voce| { let esistono = voce.creati.iter().map(|c| Path::new(c).exists()).collect(); VoceVista { voce, esistono } }).collect()
}

pub fn clear() -> Result<(), String> {
    let _g = LOCK.lock().unwrap();
    save(&[])
}

/// Sposta nel cestino i file creati dall'operazione `quando` (solo quelli registrati come creati).
pub fn trash(quando: u64) -> Result<String, String> {
    let _g = LOCK.lock().unwrap();
    let v = load();
    let voce = v.iter().find(|x| x.quando == quando).ok_or("Operazione non trovata nella cronologia.")?;
    let mut n = 0;
    for c in &voce.creati {
        let p = Path::new(c);
        // Mai il file di partenza, anche se per qualche motivo fosse nell'elenco.
        if !p.exists() || c == &voce.file { continue; }
        let ok = Command::new("/usr/bin/gio").args(["trash", c]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .status().map(|st| st.success()).unwrap_or(false);
        if !ok { return Err(format!("Non riesco a spostare nel cestino {} (su questo disco il cestino non c'è).", c)); }
        n += 1;
    }
    Ok(if n == 0 { "I file creati non ci sono più.".into() } else { format!("{} {} nel cestino.", n, if n == 1 { "file spostato" } else { "file spostati" }) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore]
    fn cronologia() {
        // Nella home (non in /tmp, dove il cestino non esiste); cestino e cronologia restano qui dentro.
        let dir = PathBuf::from(std::env::var("HOME").unwrap()).join(".cache").join(format!("halo-test-storia-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Solo per questo test: cronologia e cestino nella cartella temporanea.
        std::env::set_var("XDG_DATA_HOME", &dir);
        std::env::set_var("HALO_STORIA_TEST", "1");
        let orig = dir.join("foto.jpg");
        let out = dir.join("foto_web.jpg");
        fs::write(&orig, "o").unwrap();
        fs::write(&out, "r").unwrap();
        add(&orig, 1, "image/ridimensiona/per-il-web", "Creato: foto_web.jpg\n1 KB → 1 KB", &[s(&out), s(&orig)]);
        let l = list();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].voce.esito, "Creato: foto_web.jpg");
        assert_eq!(l[0].esistono, [true, true]);
        assert_eq!(trash(l[0].voce.quando).unwrap(), "1 file spostato nel cestino.");
        assert!(!out.exists() && orig.exists(), "solo il file creato va nel cestino, mai l'originale");
        for k in 0..60 { add(&orig, 1, "x", &k.to_string(), &[]); }
        assert_eq!(list().len(), MAX);
        clear().unwrap();
        assert!(list().is_empty());
        std::env::remove_var("XDG_DATA_HOME");
        std::env::remove_var("HALO_STORIA_TEST");
        let _ = fs::remove_dir_all(&dir);
    }
}
