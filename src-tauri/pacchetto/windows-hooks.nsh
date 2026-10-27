; Halo per Windows: la voce «Azioni rapide (Halo)» nel menu del tasto destro di Esplora file,
; su ogni file, sulle cartelle e sullo sfondo di una cartella. Solo per l'utente (HKCU).
; Su Windows 11 compare in «Mostra altre opzioni» (il menu classico).

!macro NSIS_HOOK_POSTINSTALL
  WriteRegStr HKCU "Software\Classes\*\shell\Halo" "MUIVerb" "Azioni rapide (Halo)"
  WriteRegStr HKCU "Software\Classes\*\shell\Halo" "Icon" "$INSTDIR\halo.exe"
  WriteRegStr HKCU "Software\Classes\*\shell\Halo\command" "" '"$INSTDIR\halo.exe" -- "%1"'
  WriteRegStr HKCU "Software\Classes\Directory\shell\Halo" "MUIVerb" "Azioni rapide (Halo)"
  WriteRegStr HKCU "Software\Classes\Directory\shell\Halo" "Icon" "$INSTDIR\halo.exe"
  WriteRegStr HKCU "Software\Classes\Directory\shell\Halo\command" "" '"$INSTDIR\halo.exe" -- "%1"'
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\Halo" "MUIVerb" "Azioni rapide (Halo)"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\Halo" "Icon" "$INSTDIR\halo.exe"
  WriteRegStr HKCU "Software\Classes\Directory\Background\shell\Halo\command" "" '"$INSTDIR\halo.exe" -- "%V"'
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegKey HKCU "Software\Classes\*\shell\Halo"
  DeleteRegKey HKCU "Software\Classes\Directory\shell\Halo"
  DeleteRegKey HKCU "Software\Classes\Directory\Background\shell\Halo"
!macroend
