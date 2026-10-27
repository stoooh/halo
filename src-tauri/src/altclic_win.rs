//! Windows: Alt+clic su un file in Esplora file o sul desktop apre Halo, come su Ubuntu.
//! `halo --residente` resta in esecuzione senza finestre (parte all'accesso, lo registra
//! l'installer): ascolta il mouse e, all'Alt+clic, trova il file sotto il puntatore con
//! l'automazione dell'interfaccia di Windows (UI Automation) e la cartella della finestra di
//! Esplora file, poi avvia `halo -- file`. Se il file fa parte di una selezione, passa tutti
//! i file selezionati.

use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetAncestor, GetClassNameW, GetMessageW, SetWindowsHookExW, TranslateMessage, WindowFromPoint,
    GA_ROOT, MSG, MSLLHOOKSTRUCT, WH_MOUSE_LL, WM_LBUTTONUP,
};

/// Ultimo Alt+clic (millisecondi): un secondo clic subito dopo non apre un altro menu.
static ULTIMO: AtomicU64 = AtomicU64::new(0);

/// Trova i file: cartella della finestra (Shell.Application) + nome sotto il puntatore (UI Automation).
/// Stampa un percorso per riga; senza file sotto il puntatore stampa la cartella.
const SCRIPT: &str = r#"
param([int]$x, [int]$y, [string]$hwnd, [string]$kind)
Add-Type -AssemblyName UIAutomationClient, UIAutomationTypes
$folder = $null; $sel = @()
if ($kind -eq 'desktop') { $folder = [Environment]::GetFolderPath('Desktop') }
else {
  $sh = New-Object -ComObject Shell.Application
  foreach ($w in $sh.Windows()) {
    try { if ([int64]$w.HWND -eq [int64]$hwnd) { $folder = $w.Document.Folder.Self.Path; $sel = @($w.Document.SelectedItems() | ForEach-Object { $_.Path }) } } catch {}
  }
}
$name = $null
try {
  $e = [System.Windows.Automation.AutomationElement]::FromPoint((New-Object System.Windows.Point($x, $y)))
  for ($i = 0; $i -lt 6 -and $e; $i++) {
    $ct = $e.Current.ControlType.ProgrammaticName
    if ($ct -match 'ListItem|DataItem|TreeItem') { $name = $e.Current.Name; break }
    $e = [System.Windows.Automation.TreeWalker]::ControlViewWalker.GetParent($e)
  }
} catch {}
$path = $null
if ($name -and $folder) {
  $p = Join-Path $folder $name
  if (Test-Path -LiteralPath $p) { $path = $p }
  else {
    # Esplora file nasconde le estensioni: «Contratto» è «Contratto.pdf».
    $m = Get-ChildItem -LiteralPath $folder -Force -ErrorAction SilentlyContinue | Where-Object { $_.BaseName -eq $name } | Select-Object -First 1
    if ($m) { $path = $m.FullName }
  }
  if (-not $path -and $kind -eq 'desktop') {
    $pub = [Environment]::GetFolderPath('CommonDesktopDirectory')
    $m = Get-ChildItem -LiteralPath $pub -Force -ErrorAction SilentlyContinue | Where-Object { $_.Name -eq $name -or $_.BaseName -eq $name } | Select-Object -First 1
    if ($m) { $path = $m.FullName }
  }
}
if ($path) {
  if ($sel.Count -gt 1 -and ($sel -contains $path)) { $path; $sel | Where-Object { $_ -ne $path } } else { $path }
} elseif ($folder) { $folder }
"#;

fn class_of(h: HWND) -> String {
    let mut buf = [0u16; 128];
    // SAFETY: buffer valido della lunghezza indicata.
    let n = unsafe { GetClassNameW(h, buf.as_mut_ptr(), buf.len() as i32) };
    String::from_utf16_lossy(&buf[..n.max(0) as usize])
}

fn down(vk: u16) -> bool {
    // SAFETY: legge solo lo stato di un tasto.
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

unsafe extern "system" fn hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0 && wparam as u32 == WM_LBUTTONUP && down(VK_MENU)
        && !down(VK_CONTROL) && !down(VK_SHIFT) && !down(VK_LWIN) && !down(VK_RWIN) {
        // SAFETY: con WH_MOUSE_LL lparam punta a un MSLLHOOKSTRUCT valido per questa chiamata.
        let pt = unsafe { (*(lparam as *const MSLLHOOKSTRUCT)).pt };
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0);
        if now.saturating_sub(ULTIMO.swap(now, Ordering::SeqCst)) > 800 {
            std::thread::spawn(move || pick(pt));
        }
    }
    // SAFETY: passa l'evento agli altri, come richiesto dai hook di Windows.
    unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
}

fn pick(pt: POINT) {
    // SAFETY: chiamate di sola lettura sulla finestra sotto il punto.
    let root = unsafe { GetAncestor(WindowFromPoint(pt), GA_ROOT) };
    if root.is_null() { return; }
    let kind = match class_of(root).as_str() {
        "CabinetWClass" | "ExploreWClass" => "explorer",
        "Progman" | "WorkerW" => "desktop",
        _ => return,
    };
    // Esplora file aggiorna la selezione dopo il clic: un attimo di attesa.
    std::thread::sleep(std::time::Duration::from_millis(150));
    // Il comando va a PowerShell codificato (UTF-16 in base64): niente problemi di virgolette.
    let full = format!("& {{ {} }} {} {} {} {}", SCRIPT, pt.x, pt.y, root as usize, kind);
    let utf16: Vec<u8> = full.encode_utf16().flat_map(|u| u.to_le_bytes()).collect();
    let o = crate::sistema::cmd("powershell").args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand",
        &crate::sistema::base64(&utf16)])
        .stdin(Stdio::null()).output();
    let Ok(o) = o else { return };
    let paths: Vec<String> = String::from_utf8_lossy(&o.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    if paths.is_empty() { return; }
    let Ok(exe) = std::env::current_exe() else { return };
    let _ = Command::new(exe).arg("--").args(&paths).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

/// Resta in ascolto per sempre (fino alla chiusura della sessione).
pub fn run() {
    // SAFETY: hook globale del mouse e ciclo dei messaggi standard di Windows.
    unsafe {
        let h = SetWindowsHookExW(WH_MOUSE_LL, Some(hook), std::ptr::null_mut(), 0);
        if h.is_null() { return; }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
