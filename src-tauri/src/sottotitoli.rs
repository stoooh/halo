//! Video › Sottotitoli: trascrizione automatica dell'audio con Whisper (whisper.cpp), sul
//! computer, senza Internet dopo il primo scaricamento del modello. È intelligenza artificiale,
//! aggiunta su richiesta. Il risultato: un file .srt, sottotitoli attivabili dentro il video,
//! oppure impressi nell'immagine.

use crate::actions::{name, out_path, run_cmd, s, video_ext, VIDEO_HQ};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub const KEYS: &[&str] = &["video/sottotitoli/crea-srt", "video/sottotitoli/nel-video", "video/sottotitoli/impressi"];

/// Modelli di whisper.cpp: id del pannello, file, dimensione indicativa.
const MODELLI: &[(&str, &str, &str)] = &[("base", "ggml-base.bin", "142 MB"), ("small", "ggml-small.bin", "466 MB")];
const URL: &str = "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/";

pub fn planned_out(input: &Path, key: &str) -> Option<PathBuf> {
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
    Some(match key {
        "video/sottotitoli/crea-srt" => out_path(input, "", "srt"),
        "video/sottotitoli/nel-video" => out_path(input, "_sottotitoli", if ext == "mkv" || ext == "webm" { "mkv" } else { video_ext(&ext) }),
        "video/sottotitoli/impressi" => out_path(input, "_sottotitoli", video_ext(&ext)),
        _ => return None,
    })
}

/// Il programma di whisper.cpp (il nome cambia tra le versioni). Su Windows lo mette
/// dipendenze.ps1 in whisper\bin, accanto ai modelli.
fn whisper_bin() -> Result<PathBuf, String> {
    let installed = cache().join("bin").join(crate::sistema::exe("whisper-cli"));
    if installed.is_file() { return Ok(installed); }
    for b in ["whisper-cli", "whisper-cpp", "whisper.cpp", "whisper"] {
        if let Some(p) = crate::sistema::find_program(b) { return Ok(p); }
    }
    Err(if cfg!(windows) { "Manca whisper.cpp: rilancia dipendenze.ps1 dalla cartella di Halo.".into() }
        else { "Manca whisper.cpp. Installa con: sudo apt install whisper.cpp".into() })
}

fn cache() -> PathBuf {
    crate::sistema::cache_dir().join("whisper")
}

/// Il modello scelto; la prima volta lo scarica (solo il file del modello, da Hugging Face).
fn model(id: &str) -> Result<PathBuf, String> {
    let (_, file, size) = *MODELLI.iter().find(|m| m.0 == id).unwrap_or(&MODELLI[0]);
    let path = cache().join(file);
    if path.is_file() { return Ok(path); }
    fs::create_dir_all(cache()).map_err(|e| e.to_string())?;
    let part = path.with_extension("part");
    let st = crate::sistema::cmd("curl").args(["-sSL", "--fail", "--connect-timeout", "20", "-o", &s(&part), &format!("{}{}", URL, file)])
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).status();
    if !st.map(|s| s.success()).unwrap_or(false) || crate::lavoro::cancelled() {
        let _ = fs::remove_file(&part);
        return Err(format!("Non sono riuscito a scaricare il modello ({}): serve Internet solo la prima volta.", size));
    }
    fs::rename(&part, &path).map_err(|e| e.to_string())?;
    Ok(path)
}

/// Trascrive l'audio del video in un file .srt (in `dir`) e restituisce il percorso.
fn transcribe(input: &Path, dir: &Path, data: &HashMap<String, String>) -> Result<PathBuf, String> {
    let bin = whisper_bin()?;
    let lingua = match data.get("lingua").map(|v| v.as_str()) { Some("it") => "it", Some("en") => "en", _ => "auto" };
    let modello = model(data.get("modello").map(|v| v.as_str()).unwrap_or("base"))?;
    // Whisper vuole audio a 16 kHz mono.
    let wav = dir.join("audio.wav");
    crate::lavoro::range(0.0, 0.1);
    run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-y", "-i", &s(input), "-vn", "-ar", "16000", "-ac", "1", "-c:a", "pcm_s16le", &s(&wav)])
        .map_err(|e| if e.contains("does not contain any stream") || e.contains("matches no streams") { "Il video non ha l'audio: niente da trascrivere.".to_string() } else { e })?;
    crate::lavoro::range(0.1, 0.85);
    let prefix = dir.join("sottotitoli");
    let threads = std::thread::available_parallelism().map(|n| n.get().min(8)).unwrap_or(4).to_string();
    let (st, _, err) = crate::lavoro::run_lines(crate::sistema::no_window(Command::new(&bin)).args(["-m", &s(&modello), "-f", &s(&wav), "-l", lingua,
        "-t", &threads, "-osrt", "-of", &s(&prefix), "-pp"]), true, |l| {
        // «… progress =  42%»
        if let Some(p) = l.split("progress =").nth(1).and_then(|v| v.trim().trim_end_matches('%').trim().parse::<f64>().ok()) {
            crate::lavoro::set(p / 100.0);
        }
    }).map_err(|e| format!("whisper.cpp non parte: {}", e))?;
    if crate::lavoro::cancelled() { return Err("Annullato.".into()); }
    let srt = prefix.with_extension("srt");
    if !st.success() || !srt.is_file() {
        return Err(format!("Trascrizione non riuscita: {}", err.lines().rfind(|l| !l.trim().is_empty()).unwrap_or("errore sconosciuto").trim()));
    }
    if fs::read_to_string(&srt).map(|t| t.trim().is_empty()).unwrap_or(true) {
        return Err("Nessuna parola riconosciuta nell'audio.".into());
    }
    Ok(srt)
}

pub fn run(input: &Path, key: &str, data: &HashMap<String, String>) -> Result<String, String> {
    let out = planned_out(input, key).ok_or("Opzione non disponibile.")?;
    let tmp = std::env::temp_dir().join(format!("halo-sub-{}", std::process::id()));
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
    let r = (|| {
        let srt = transcribe(input, &tmp, data)?;
        // Su Windows whisper.cpp va a capo con «\r\n».
        let righe = fs::read_to_string(&srt).unwrap_or_default().replace("\r\n", "\n").split("\n\n").filter(|b| !b.trim().is_empty()).count();
        crate::lavoro::range(0.95, 0.05);
        match key {
            "video/sottotitoli/crea-srt" => { fs::copy(&srt, &out).map_err(|e| e.to_string())?; }
            "video/sottotitoli/nel-video" => {
                // Traccia di sottotitoli attivabile nel lettore; video e audio copiati così come sono.
                let codec = if out.extension().is_some_and(|e| e == "mkv") { "srt" } else { "mov_text" };
                crate::actions::ff_to(input, &out, &[], &["-i", &s(&srt), "-map", "0:v", "-map", "0:a?", "-map", "1:0", "-c:v", "copy", "-c:a", "copy",
                    "-c:s", codec, "-metadata:s:s:0", &format!("language={}", match data.get("lingua").map(|v| v.as_str()) { Some("en") => "eng", Some("it") => "ita", _ => "und" })])?;
            }
            _ => {
                // Impressi nell'immagine: si vedono ovunque, ma il video va ricodificato.
                // Il filtro vuole un percorso senza caratteri speciali: si lavora nella cartella temporanea.
                let vf = "subtitles=sottotitoli.srt:force_style='FontSize=22,Outline=2'";
                let args: Vec<&str> = [&["-vf", vf][..], VIDEO_HQ].concat();
                let mut full = vec!["-hide_banner", "-nostdin", "-n", "-i"];
                let i = s(input); let o = s(&out);
                full.push(&i); full.extend(args); full.push(&o);
                let st = crate::lavoro::run_lines(crate::sistema::cmd("ffmpeg").args(&full).current_dir(&tmp), true, |_| {}).map_err(|e| e.to_string())?;
                if !st.0.success() { return Err("ffmpeg non è riuscito a imprimere i sottotitoli.".into()); }
            }
        }
        Ok(format!("Creato: {} ({} sottotitoli)", name(&out), righe))
    })();
    let _ = fs::remove_dir_all(&tmp);
    if r.is_err() { let _ = fs::remove_file(&out); }
    r
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nomi() {
        assert_eq!(name(&planned_out(Path::new("/x/Demo.mp4"), KEYS[0]).unwrap()), "Demo.srt");
        assert_eq!(name(&planned_out(Path::new("/x/Demo.webm"), KEYS[1]).unwrap()), "Demo_sottotitoli.mkv");
        assert_eq!(name(&planned_out(Path::new("/x/Demo.mov"), KEYS[2]).unwrap()), "Demo_sottotitoli.mov");
    }

    /// Prova vera: serve whisper.cpp (il modello «base» si scarica al primo uso).
    /// La voce di prova la crea espeak-ng su Linux, la sintesi vocale di sistema su Windows
    /// (HALO_VOCE=/percorso/file.wav per usarne un'altra).
    #[test]
    #[ignore]
    fn file_veri() {
        let dir = std::env::temp_dir().join("halo-test-sottotitoli");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        // Windows ha di solito solo la voce inglese: lì si prova in inglese.
        let (frase, lingua, parole) = if cfg!(windows) { ("Good morning everyone, this is a subtitle test.", "en", ["morning", "test"]) }
            else { ("Buongiorno a tutti, questa è una prova dei sottotitoli.", "it", ["buongiorno", "prova"]) };
        let voce = std::env::var("HALO_VOCE").map(PathBuf::from).unwrap_or_else(|_| {
            let w = dir.join("voce.wav");
            if cfg!(windows) {
                let ps = format!("Add-Type -AssemblyName System.Speech; $s = New-Object System.Speech.Synthesis.SpeechSynthesizer; \
                    $s.SetOutputToWaveFile('{}'); $s.Speak('{}'); $s.Dispose()", s(&w), frase);
                let o = Command::new("powershell").args(["-NoProfile", "-Command", &ps]).output().unwrap();
                assert!(o.status.success() && w.is_file(), "sintesi vocale: {}", String::from_utf8_lossy(&o.stderr));
            } else {
                run_cmd("espeak-ng", &["-v", "it", "-w", &s(&w), frase]).expect("serve espeak-ng o HALO_VOCE");
            }
            w
        });
        let v = dir.join("Prova.mp4");
        run_cmd("ffmpeg", &["-hide_banner", "-nostdin", "-y", "-f", "lavfi", "-i", "testsrc2=size=640x360:rate=25", "-i", &s(&voce),
            "-shortest", "-c:v", "libx264", "-pix_fmt", "yuv420p", "-c:a", "aac", &s(&v)]).unwrap();
        let d = HashMap::from([("lingua".to_string(), lingua.to_string()), ("modello".to_string(), "base".to_string())]);
        let r = run(&v, KEYS[0], &d).unwrap();
        assert!(r.starts_with("Creato: Prova.srt"), "{}", r);
        let t = fs::read_to_string(dir.join("Prova.srt")).unwrap().to_lowercase();
        assert!(t.contains("-->") && parole.iter().any(|p| t.contains(p)), "{}", t);
        assert!(run(&v, KEYS[1], &d).unwrap().starts_with("Creato: Prova_sottotitoli.mp4"));
        let streams = run_cmd("ffprobe", &["-v", "error", "-show_entries", "stream=codec_type", "-of", "csv=p=0", &s(&dir.join("Prova_sottotitoli.mp4"))]).unwrap();
        assert!(streams.contains("subtitle"), "{}", streams);
    }
}
