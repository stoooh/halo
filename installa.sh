#!/bin/bash
# Installa Halo in un colpo solo: scarica l'ultima versione da GitHub e la installa con tutti i
# programmi che servono (installa-halo.sh della release). Uso, nel terminale:
#   curl -fsSL https://raw.githubusercontent.com/stoooh/halo/main/installa.sh | bash
# Su Windows (Git Bash) passa da solo a installa.ps1.
set -euo pipefail
REPO=stoooh/halo

main() {
  case "$(uname -s)" in
    Linux) ;;
    MINGW*|MSYS*|CYGWIN*)
      echo "Sistema: Windows. Uso installa.ps1."
      powershell.exe -NoProfile -ExecutionPolicy Bypass -Command "irm https://raw.githubusercontent.com/$REPO/main/installa.ps1 | iex"
      return ;;
    *) echo "Halo funziona solo su Ubuntu (GNOME) e su Windows 10/11."; exit 1 ;;
  esac
  if ! command -v apt-get >/dev/null; then
    echo "Halo per Linux è un pacchetto .deb: serve Ubuntu (o un'altra distribuzione con apt)."; exit 1
  fi
  if [ "$(uname -m)" != x86_64 ]; then
    echo "Halo per Linux c'è solo per processori a 64 bit (x86_64)."; exit 1
  fi
  echo "Sistema: Linux ($(. /etc/os-release 2>/dev/null; echo "${PRETTY_NAME:-sconosciuto}"))."

  local json deb script tag dir
  json=$(curl -fsSL -H "User-Agent: halo-installa" "https://api.github.com/repos/$REPO/releases/latest")
  tag=$(grep -o '"tag_name": *"[^"]*"' <<<"$json" | head -1 | cut -d'"' -f4)
  deb=$(grep -o '"browser_download_url": *"[^"]*_amd64\.deb"' <<<"$json" | head -1 | cut -d'"' -f4)
  script=$(grep -o '"browser_download_url": *"[^"]*/installa-halo\.sh"' <<<"$json" | head -1 | cut -d'"' -f4)
  if [ -z "$deb" ] || [ -z "$script" ]; then
    echo "Nella release $tag non trovo il pacchetto per Linux."; exit 1
  fi

  dir=$(mktemp -d)
  trap 'rm -rf "$dir"' EXIT
  echo "Scarico Halo $tag..."
  curl -fL --progress-bar -o "$dir/${deb##*/}" "$deb"
  curl -fsSL -o "$dir/installa-halo.sh" "$script"
  # Con «curl | bash» l'ingresso è lo script stesso: le domande (password di sudo) vanno al terminale.
  if [ -r /dev/tty ]; then bash "$dir/installa-halo.sh" </dev/tty; else bash "$dir/installa-halo.sh"; fi
}

main "$@"
