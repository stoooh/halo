//! Il lavoro in corso: avanzamento (0–1, quando il programma lo dice) e annullamento.
//! ffmpeg e Real-ESRGAN scrivono l'avanzamento mentre lavorano; per gli altri programmi
//! il menu mostra solo il tempo trascorso. «Annulla» ferma il programma in esecuzione.

use std::io::{BufRead, BufReader, Read};
use std::process::{Command, ExitStatus, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

static CANCEL: AtomicBool = AtomicBool::new(false);
/// I programmi lanciati per il lavoro in corso: «Annulla» ferma questi (e solo questi).
static FIGLI: Mutex<Vec<u32>> = Mutex::new(Vec::new());
/// Avanzamento e la parte del lavoro che il passaggio attuale rappresenta (inizio, ampiezza).
static STATO: Mutex<(Option<f64>, f64, f64)> = Mutex::new((None, 0.0, 1.0));

/// Nuovo lavoro: niente avanzamento, niente annullamento.
pub fn start() {
    CANCEL.store(false, Ordering::SeqCst);
    *STATO.lock().unwrap() = (None, 0.0, 1.0);
}

/// Il passaggio attuale copre la parte [inizio, inizio+ampiezza] del lavoro (es. Upscayl 8×: due metà).
pub fn range(start: f64, span: f64) {
    let mut s = STATO.lock().unwrap();
    s.1 = start; s.2 = span;
}

/// Avanzamento del passaggio attuale (0–1).
pub fn set(p: f64) {
    let mut s = STATO.lock().unwrap();
    let v = (s.1 + p.clamp(0.0, 1.0) * s.2).clamp(0.0, 0.99);
    s.0 = Some(s.0.map_or(v, |old| old.max(v)));
}

pub fn progress() -> Option<f64> {
    STATO.lock().unwrap().0
}

pub fn cancelled() -> bool {
    CANCEL.load(Ordering::SeqCst)
}

/// Annulla: segna il lavoro come annullato e ferma i programmi lanciati da Halo.
pub fn cancel() {
    CANCEL.store(true, Ordering::SeqCst);
    let pids: Vec<u32> = FIGLI.lock().unwrap().clone();
    for p in pids { crate::sistema::kill(p); }
}

fn track(pid: u32) { FIGLI.lock().unwrap().push(pid); }
fn untrack(pid: u32) { FIGLI.lock().unwrap().retain(|p| *p != pid); }

/// Come `Command::output()`, ma il programma si può fermare con «Annulla».
pub fn run_tracked(cmd: &mut Command) -> std::io::Result<std::process::Output> {
    let child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let pid = child.id();
    track(pid);
    let o = child.wait_with_output();
    untrack(pid);
    o
}

/// Esegue il comando leggendo riga per riga l'uscita indicata (`stderr` o stdout) mentre lavora.
/// Restituisce esito, stdout e stderr completi.
pub fn run_lines(cmd: &mut Command, stderr: bool, mut on_line: impl FnMut(&str)) -> std::io::Result<(ExitStatus, String, String)> {
    let mut child = cmd.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn()?;
    let pid = child.id();
    track(pid);
    let (mut watch, other): (Box<dyn Read + Send>, Box<dyn Read + Send>) = if stderr {
        (Box::new(child.stderr.take().unwrap()), Box::new(child.stdout.take().unwrap()))
    } else {
        (Box::new(child.stdout.take().unwrap()), Box::new(child.stderr.take().unwrap()))
    };
    // L'altra uscita si legge in un thread, così nessuna delle due si riempie e blocca il programma.
    let t = std::thread::spawn(move || { let mut s = String::new(); let mut o = other; let _ = o.read_to_string(&mut s); s });
    let mut seen = String::new();
    // Real-ESRGAN separa le percentuali con \r o \n: si spezza su entrambi.
    let mut buf = Vec::new();
    let mut r = BufReader::new(&mut watch);
    loop {
        buf.clear();
        let n = r.read_until(b'\n', &mut buf)?;
        if n == 0 { break; }
        let text = String::from_utf8_lossy(&buf).into_owned();
        for part in text.split('\r') {
            let l = part.trim();
            if !l.is_empty() { on_line(l); }
        }
        seen.push_str(&text);
    }
    let status = child.wait();
    untrack(pid);
    let status = status?;
    let other = t.join().unwrap_or_default();
    Ok(if stderr { (status, other, seen) } else { (status, seen, other) })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn avanzamento() {
        start();
        assert_eq!(progress(), None);
        range(0.5, 0.5);
        set(0.5);
        assert_eq!(progress(), Some(0.75));
        set(0.2);
        assert_eq!(progress(), Some(0.75), "non torna indietro");
        let (st, out, err) = run_lines(Command::new("sh").args(["-c", "echo uno; echo 10% >&2; echo due"]), false, |_| {}).unwrap();
        assert!(st.success() && out == "uno\ndue\n" && err == "10%\n");
        start();
    }
}
