#!/bin/sh
# Dopo l'installazione: lo script di Alt+clic deve essere eseguibile.
set -e
chmod 755 /usr/share/gnome-shell/extensions/halo@halo.dev/halo-pick /usr/lib/halo/realesrgan/realesrgan-ncnn-vulkan
chmod 644 /usr/share/gnome-shell/extensions/halo@halo.dev/extension.js \
  /usr/share/gnome-shell/extensions/halo@halo.dev/metadata.json \
  /usr/share/nautilus-python/extensions/halo_nautilus.py /etc/xdg/autostart/halo-configura.desktop
exit 0
