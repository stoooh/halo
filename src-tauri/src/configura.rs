//! `halo --configura`: parte all'accesso (autostart installato dal pacchetto) e attiva
//! l'estensione di GNOME per l'Alt+clic. Lavora una sola volta per versione, così se poi
//! l'utente disattiva l'estensione la scelta resta.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};

const UUID: &str = "halo@halo.dev";

/// Esegue la configurazione e dice cosa ha fatto.
pub fn run() -> String {
    if cfg!(windows) { return "Su Windows non serve: il menu del tasto destro lo registra l'installazione.".into(); }
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    let marker = std::env::var("XDG_CONFIG_HOME").ok().filter(|d| !d.is_empty()).map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config")).join("halo").join(format!("configurato-{}", env!("CARGO_PKG_VERSION")));
    if marker.exists() { return "Già configurato per questa versione.".into(); }
    let ok = Command::new("gnome-extensions").args(["enable", UUID]).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .status().map(|s| s.success()).unwrap_or(false);
    if !ok {
        // GNOME non l'ha ancora caricata (serve un nuovo accesso): si riprova la prossima volta.
        return "Estensione non ancora attivabile: si riprova al prossimo accesso.".into();
    }
    let _ = fs::create_dir_all(marker.parent().unwrap());
    let _ = fs::write(&marker, "");
    "Estensione per l'Alt+clic attivata.".into()
}

/// L'app si chiamava Fulcro: impostazioni, cronologia e modelli scaricati passano alle cartelle
/// di Halo (solo se quelle nuove non esistono ancora), così non si perde niente.
pub fn migrate_from_fulcro() {
    let home = PathBuf::from(std::env::var("HOME").unwrap_or_default());
    for base in [".config", ".local/share", ".cache"] {
        let (old, new) = (home.join(base).join("fulcro"), home.join(base).join("halo"));
        if old.is_dir() && !new.exists() { let _ = fs::rename(&old, &new); }
    }
}
