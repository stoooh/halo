#!/bin/bash
# Installa Halo su Ubuntu (GNOME) in un colpo solo: il pacchetto con tutti i programmi che servono
# (apt li scarica da solo), la voce nel tasto destro di Nautilus e l'Alt+clic.
# Uso: metti questo script nella stessa cartella di Halo_*_amd64.deb e lancia:  bash installa-halo.sh
set -e
cd "$(dirname "$0")"
DEB=$(ls -1 Halo_*_amd64.deb 2>/dev/null | sort -V | tail -1)
if [ -z "$DEB" ]; then
  echo "Non trovo Halo_*_amd64.deb accanto a questo script: scaricalo dalla pagina delle release di Halo."
  exit 1
fi
echo "Installo $DEB con i programmi necessari (serve la password di amministratore)..."
sudo apt-get update -qq || true
sudo apt-get install -y "./$DEB"

# Vecchie copie dell'estensione (quando l'app si chiamava Fulcro), che avrebbero la precedenza.
rm -rf "$HOME/.local/share/gnome-shell/extensions/fulcro@fulcro.dev"
rm -f "$HOME/.local/share/nautilus-python/extensions/fulcro_nautilus.py"

# Nautilus ricarica le estensioni al prossimo avvio; l'Alt+clic si attiva al prossimo accesso
# (lo fa da solo «halo --configura», installato dal pacchetto).
nautilus -q 2>/dev/null || true
gnome-extensions enable halo@halo.dev 2>/dev/null || true

echo
echo "Fatto. Il tasto destro di Nautilus ha già «Azioni rapide…»."
echo "Per l'Alt+clic esci e rientra nella sessione (una volta sola)."
