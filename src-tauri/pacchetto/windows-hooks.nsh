; Halo per Windows: la voce "Azioni rapide (Halo)" nel menu del tasto destro di Esplora file,
; su ogni file, sulle cartelle e sullo sfondo di una cartella. Solo per l'utente (HKCU).
; Su Windows 11 compare in "Mostra altre opzioni" (il menu classico).

; Prima di installare o aggiornare: chiude Halo in ascolto dell'Alt+clic (altrimenti halo.exe e' in uso).
!macro NSIS_HOOK_PREINSTALL
  nsExec::Exec 'taskkill /IM halo.exe /F'
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; Alt+clic su un file in Esplora file o sul desktop: Halo resta in ascolto, avviato a ogni accesso.
  WriteRegStr HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Halo" '"$INSTDIR\halo.exe" --residente'
  Exec '"$INSTDIR\halo.exe" --residente'

  WriteRegStr HKCU "Software\Classes\*\shell\Halo" "MUIVerb" "Azioni rapide (Halo)"
  WriteRegStr HKCU "Software\Classes\*\shell\Halo" "Icon" "$INSTDIR\halo.exe"
  WriteRegStr HKCU "Software\Classes\*\shell\Halo\command" "" '"$INSTDIR\halo.exe" -- "%1"'
  WriteRegStr HKCU "Software\Classes\Directory\shell\Halo" "MUIVerb" "Azioni rapide (Halo)"
  WriteRegStr HKCU "Software\Classes\Directory\shell\Halo" "Icon" "$INSTDIR\halo.exe"
  WriteRegStr HKCU "Software\Classes\Directory\shell\Halo\command" "" '"$INSTDIR\halo.exe" -- "%1"'
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\Halo" "MUIVerb" "Azioni rapide (Halo)"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\Halo" "Icon" "$INSTDIR\halo.exe"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\Halo\command" "" '"$INSTDIR\halo.exe" -- "%V"'
  ; Programmi necessari alle azioni (ImageMagick, FFmpeg, LibreOffice...): installati con winget, se l'utente vuole.
  ; Nell'installazione silenziosa (/S) non si chiede niente.
  IfSilent halo_fine
  MessageBox MB_YESNO|MB_ICONQUESTION "Halo usa alcuni programmi gratuiti per lavorare sui file (ImageMagick, FFmpeg, Ghostscript, 7-Zip, LibreOffice, Python...).$\r$\n$\r$\nVuoi installare adesso quelli che mancano? Servono Internet e qualche minuto (circa 1 GB in tutto).$\r$\n$\r$\nPuoi farlo anche dopo: $INSTDIR\dipendenze.ps1" IDNO halo_fine
  ExecWait 'powershell.exe -NoProfile -ExecutionPolicy Bypass -File "$INSTDIR\dipendenze.ps1"'
  halo_fine:
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  nsExec::Exec 'taskkill /IM halo.exe /F'
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "Halo"
  DeleteRegKey HKCU "Software\Classes\*\shell\Halo"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\Halo"
  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\Halo"
!macroend
