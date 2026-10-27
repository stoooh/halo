//! Windows: Alt+clic su un file in Esplora file o sul desktop apre il menu radiale.
//!
//! `halo --residente` resta in sottofondo (avviato all'accesso, vedi windows-hooks.nsh) e
//! ascolta il mouse con un aggancio di sistema (WH_MOUSE_LL). L'Alt+clic sui file non arriva a
//! Esplora file (con Alt premuto non selezionerebbe niente): Halo trova l'elemento sotto il
//! puntatore con UI Automation, lo seleziona e legge il percorso dalla finestra.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use windows::core::{w, Interface};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows::Win32::System::Com::{CoCreateInstance, CoInitializeEx, CoTaskMemFree, IServiceProvider, CLSCTX_ALL, CLSCTX_INPROC_SERVER, COINIT_APARTMENTTHREADED};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Threading::CreateMutexW;
use windows::Win32::System::Variant::{VARIANT, VT_I4};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, IUIAutomationSelectionItemPattern, UIA_ListControlTypeId,
    UIA_ListItemControlTypeId, UIA_SelectionItemPatternId,
};
use windows::Win32::UI::HiDpi::{SetProcessDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_CONTROL, VK_LWIN,
    VK_MENU, VK_RWIN, VK_SHIFT,
};
use windows::Win32::UI::Shell::{
    IFolderView2, IPersistFolder2, IShellBrowser, IShellItem, IShellWindows, SHGetNameFromIDList, ShellWindows,
    CSIDL_DESKTOP, SID_STopLevelBrowser, SIGDN_FILESYSPATH, SWC_DESKTOP, SWFO_NEEDDISPATCH,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetAncestor, GetClassNameW, GetMessageW, SetWindowsHookExW, WindowFromPoint, GA_PARENT,
    GA_ROOT, HC_ACTION, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONDOWN, WM_LBUTTONUP,
};

/// Dopo un'apertura, gli Alt+clic ravvicinati (doppio clic) non aprono un altro menu.
const PAUSA: Duration = Duration::from_millis(800);
/// Tempo lasciato a Esplora file per aggiornare la selezione.
const ATTESA_SELEZIONE: Duration = Duration::from_millis(80);

/// Alt+clic: il punto e la vista dei file (SHELLDLL_DefView) sotto il puntatore.
struct Clic {
    punto: POINT,
    vista: isize,
}

static CANALE: OnceLock<Sender<Clic>> = OnceLock::new();
/// Il tasto è stato premuto con Alt su una vista di file: anche il rilascio non va a Esplora file.
static TRATTENUTO: AtomicBool = AtomicBool::new(false);

/// Resta in ascolto finché la sessione è aperta. Ne gira uno solo per utente.
pub fn run() {
    unsafe {
        let _uno = CreateMutexW(None, true, w!("Local\\HaloAscolta"));
        if GetLastError() == ERROR_ALREADY_EXISTS {
            return;
        }
        // Il punto del clic arriva in pixel reali: senza questo, con lo schermo al 125% o 150%,
        // UI Automation lo riscalerebbe e troverebbe l'elemento più in basso a destra.
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
        let (tx, rx) = channel();
        let _ = CANALE.set(tx);
        std::thread::spawn(move || lavora(rx));
        let modulo = GetModuleHandleW(None).ok().map(|m| m.into());
        let Ok(_aggancio) = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse), modulo, 0) else { return };
        // Gli agganci WH_MOUSE_LL funzionano solo se il thread smista i messaggi.
        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            DispatchMessageW(&msg);
        }
    }
}

/// Chiamata da Windows a ogni evento del mouse: deve tornare subito, il lavoro lo fa `lavora`.
/// Restituire 1 trattiene l'evento (non arriva alla finestra).
unsafe extern "system" fn mouse(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let punto = (*(lparam.0 as *const MSLLHOOKSTRUCT)).pt;
        match wparam.0 as u32 {
            WM_LBUTTONDOWN if solo_alt() => {
                if let (Some(vista), Some(tx)) = (vista_al_punto(punto), CANALE.get()) {
                    TRATTENUTO.store(true, Ordering::SeqCst);
                    let _ = tx.send(Clic { punto, vista: vista.0 as isize });
                    return LRESULT(1);
                }
            }
            WM_LBUTTONUP if TRATTENUTO.swap(false, Ordering::SeqCst) => return LRESULT(1),
            _ => {}
        }
    }
    CallNextHookEx(None, code, wparam, lparam)
}

fn premuto(k: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(k.0 as i32) as u16 & 0x8000 != 0 }
}

/// Alt senza Ctrl, Maiusc o Windows (come su GNOME).
fn solo_alt() -> bool {
    premuto(VK_MENU) && ![VK_CONTROL, VK_SHIFT, VK_LWIN, VK_RWIN].into_iter().any(premuto)
}

/// La vista dei file di Esplora file o del desktop sotto il punto (non la barra laterale,
/// l'indirizzo o le finestre «Apri file» delle altre app).
unsafe fn vista_al_punto(p: POINT) -> Option<HWND> {
    let mut h = WindowFromPoint(p);
    while !h.is_invalid() {
        if classe(h) == "SHELLDLL_DefView" {
            return matches!(classe(GetAncestor(h, GA_ROOT)).as_str(), "CabinetWClass" | "Progman" | "WorkerW").then_some(h);
        }
        h = GetAncestor(h, GA_PARENT);
    }
    None
}

/// Errori dell'Alt+clic in %TEMP%\halo-ascolta.log (ricomincia oltre 100 KB).
fn registro(riga: &str) {
    use std::io::Write;
    let f = std::env::temp_dir().join("halo-ascolta.log");
    let nuovo = std::fs::metadata(&f).map(|m| m.len() > 100_000).unwrap_or(false);
    let mut o = std::fs::OpenOptions::new();
    o.create(true);
    if nuovo { o.write(true).truncate(true); } else { o.append(true); }
    if let Ok(mut f) = o.open(f) {
        let _ = writeln!(f, "{} {}", chrono::Local::now().format("%H:%M:%S%.3f"), riga);
    }
}

unsafe fn classe(h: HWND) -> String {
    let mut buf = [0u16; 64];
    let n = GetClassNameW(h, &mut buf).max(0) as usize;
    String::from_utf16_lossy(&buf[..n])
}

fn lavora(rx: Receiver<Clic>) {
    let uia: Option<IUIAutomation> = unsafe {
        // A thread singolo: in MTA Esplora file rifiuta IShellView::GetWindow (RPC_E_CANTCALLOUT_ININPUTSYNCCALL).
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).map_err(|e| registro(&format!("UI Automation: {e}"))).ok()
    };
    let mut ultimo: Option<Instant> = None;
    for clic in rx {
        annulla_menu_alt();
        if ultimo.is_some_and(|t| t.elapsed() < PAUSA) {
            continue;
        }
        let Some(vista) = (unsafe { vista_di(clic.vista) }) else {
            registro("Alt+clic: finestra di Esplora file non trovata");
            continue;
        };
        let Some(uia) = uia.as_ref() else { continue };
        let file = unsafe { file_al_punto(uia, &vista, clic.punto) };
        if !file.is_empty() {
            ultimo = Some(Instant::now());
            apri(&file);
        }
    }
}

/// Alt premuto e rilasciato senza altri tasti attiva i menu della finestra in primo piano
/// (in Esplora file le lettere delle scorciatoie): un tasto senza funzione (0xE8) lo evita.
fn annulla_menu_alt() {
    let tasto = |flags| INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(0xE8), dwFlags: flags, ..Default::default() } },
    };
    unsafe {
        SendInput(&[tasto(Default::default()), tasto(KEYEVENTF_KEYUP)], std::mem::size_of::<INPUT>() as i32);
    }
}

/// La finestra di Esplora file (o il desktop) che mostra la vista `h`.
/// Con le schede di Windows 11 più viste stanno nella stessa finestra: vale quella cliccata.
unsafe fn vista_di(h: isize) -> Option<IFolderView2> {
    let finestre: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL).map_err(|e| registro(&format!("ShellWindows: {e}"))).ok()?;
    let mut candidati: Vec<_> = (0..finestre.Count().ok()?).filter_map(|i| finestre.Item(&intero(i)).ok()).collect();
    // Il desktop non è nell'elenco delle finestre: va chiesto a parte.
    let mut hwnd = 0;
    if let Ok(d) = finestre.FindWindowSW(&intero(CSIDL_DESKTOP as i32), &VARIANT::default(), SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH) {
        candidati.push(d);
    }
    candidati.into_iter().find_map(|d| {
        let browser: IShellBrowser = d.cast::<IServiceProvider>().ok()?.QueryService(&SID_STopLevelBrowser).ok()?;
        let vista = browser.QueryActiveShellView().ok()?;
        (vista.GetWindow().ok()?.0 as isize == h).then(|| vista.cast().ok()).flatten()
    })
}

/// Un VARIANT con un numero intero (VT_I4).
fn intero(i: i32) -> VARIANT {
    let mut v = VARIANT::default();
    unsafe {
        (*v.Anonymous.Anonymous).vt = VT_I4;
        (*v.Anonymous.Anonymous).Anonymous.lVal = i;
    }
    v
}

/// I file su cui aprire il menu: l'elemento sotto il puntatore (selezionato come con un clic);
/// se era già tra quelli selezionati, tutta la selezione; su uno spazio vuoto, la cartella.
unsafe fn file_al_punto(uia: &IUIAutomation, vista: &IFolderView2, p: POINT) -> Vec<PathBuf> {
    let prima = selezione(vista);
    let Some(el) = elemento(uia, p) else { return cartella(vista).into_iter().collect() };
    let Ok(sel) = el.GetCurrentPatternAs::<IUIAutomationSelectionItemPattern>(UIA_SelectionItemPatternId) else { return vec![] };
    if sel.CurrentIsSelected().is_ok_and(|b| b.as_bool()) && prima.len() > 1 {
        return prima;
    }
    if sel.Select().is_err() {
        return vec![];
    }
    std::thread::sleep(ATTESA_SELEZIONE);
    selezione(vista)
}

/// L'elemento della lista (file o cartella) sotto il punto; None sullo spazio vuoto.
unsafe fn elemento(uia: &IUIAutomation, p: POINT) -> Option<IUIAutomationElement> {
    let su = uia.ControlViewWalker().ok()?;
    let mut el = uia.ElementFromPoint(p).ok()?;
    // Dal testo del nome (o dalla colonna, nella vista Dettagli) si sale fino alla voce.
    for _ in 0..6 {
        let tipo = el.CurrentControlType().ok()?;
        if tipo == UIA_ListItemControlTypeId {
            return Some(el);
        }
        if tipo == UIA_ListControlTypeId {
            return None;
        }
        el = su.GetParentElement(&el).ok()?;
    }
    None
}

/// I file selezionati nella vista (solo quelli veri sul disco, non «Questo PC» o simili).
fn selezione(v: &IFolderView2) -> Vec<PathBuf> {
    unsafe {
        let Ok(elenco) = v.GetSelection(false) else { return vec![] };
        let n = elenco.GetCount().unwrap_or(0);
        (0..n).filter_map(|i| elenco.GetItemAt(i).ok()).filter_map(|it| percorso(&it)).collect()
    }
}

unsafe fn percorso(it: &IShellItem) -> Option<PathBuf> {
    let p = it.GetDisplayName(SIGDN_FILESYSPATH).ok()?;
    let s = p.to_string().ok();
    CoTaskMemFree(Some(p.0 as _));
    s.map(PathBuf::from)
}

/// La cartella mostrata nella vista (Alt+clic su uno spazio vuoto).
fn cartella(v: &IFolderView2) -> Option<PathBuf> {
    unsafe {
        let pidl = v.GetFolder::<IPersistFolder2>().ok()?.GetCurFolder().ok()?;
        let nome = SHGetNameFromIDList(pidl, SIGDN_FILESYSPATH);
        CoTaskMemFree(Some(pidl as _));
        let nome = nome.ok()?;
        let s = nome.to_string().ok();
        CoTaskMemFree(Some(nome.0 as _));
        s.map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Legge vista, selezione e cartella di ogni finestra di Esplora file aperta (e del desktop),
    /// come fa l'Alt+clic: `cargo test altclic -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn finestre_aperte() {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let finestre: IShellWindows = CoCreateInstance(&ShellWindows, None, CLSCTX_ALL).unwrap();
            let mut viste = vec![];
            for i in 0..finestre.Count().unwrap() {
                let d = finestre.Item(&intero(i)).unwrap();
                let b: IShellBrowser = d.cast::<IServiceProvider>().unwrap().QueryService(&SID_STopLevelBrowser).unwrap();
                viste.push(b.QueryActiveShellView().unwrap().GetWindow().unwrap().0 as isize);
            }
            let mut hwnd = 0;
            let d = finestre.FindWindowSW(&intero(CSIDL_DESKTOP as i32), &VARIANT::default(), SWC_DESKTOP, &mut hwnd, SWFO_NEEDDISPATCH).unwrap();
            let b: IShellBrowser = d.cast::<IServiceProvider>().unwrap().QueryService(&SID_STopLevelBrowser).unwrap();
            viste.push(b.QueryActiveShellView().unwrap().GetWindow().unwrap().0 as isize);
            for h in viste {
                let v = vista_di(h).expect("vista non ritrovata");
                println!("vista {h:#x}: cartella {:?}, selezione {:?}", cartella(&v), selezione(&v));
            }
        }
    }

    /// L'elemento sotto il centro di ogni icona del desktop è proprio quell'icona (senza cliccare).
    #[test]
    #[ignore]
    fn elementi_del_desktop() {
        use windows::Win32::UI::Accessibility::{TreeScope_Descendants, UIA_ControlTypePropertyId};
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
            let uia: IUIAutomation = CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).unwrap();
            let progman = windows::Win32::UI::WindowsAndMessaging::FindWindowW(w!("Progman"), None).unwrap();
            let radice = uia.ElementFromHandle(progman).unwrap();
            let tipo: VARIANT = intero(UIA_ListItemControlTypeId.0);
            let cond = uia.CreatePropertyCondition(UIA_ControlTypePropertyId, &tipo).unwrap();
            let voci = radice.FindAll(TreeScope_Descendants, &cond).unwrap();
            let n = voci.Length().unwrap();
            assert!(n > 0, "nessuna icona trovata");
            for i in 0..n {
                let voce = voci.GetElement(i).unwrap();
                let r = voce.CurrentBoundingRectangle().unwrap();
                let centro = POINT { x: (r.left + r.right) / 2, y: (r.top + r.bottom) / 2 };
                // Le icone coperte da altre finestre non si possono provare.
                if vista_al_punto(centro).is_none() {
                    continue;
                }
                let trovato = elemento(&uia, centro).map(|e| e.CurrentName().unwrap().to_string());
                println!("{} in {},{} -> {:?}", voce.CurrentName().unwrap(), centro.x, centro.y, trovato);
                assert_eq!(trovato, Some(voce.CurrentName().unwrap().to_string()));
            }
        }
    }
}

fn apri(file: &[PathBuf]) {
    if let Ok(exe) = std::env::current_exe() {
        let _ = std::process::Command::new(exe).arg("--").args(file).spawn();
    }
}
