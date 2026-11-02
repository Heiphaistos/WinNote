#!/bin/bash
# Test de bout en bout de la mise a jour AppImage (API simulee en file://) : dialogue, telechargement, SHA-256, remplacement, relance.
set -e
cd ~/wnbuild
(Xvfb :80 -screen 0 1280x800x24 >/dev/null 2>&1 &) ; sleep 1
export DISPLAY=:80 GDK_BACKEND=x11 HOME=$(mktemp -d)
d=$(mktemp -d); mkdir -p "$HOME/.config/winnote"
printf '#!/bin/sh\necho NOUVELLE-VERSION > %s/relance.txt\n' "$d" > "$d/nouvelle.AppImage"
sha=$(sha256sum "$d/nouvelle.AppImage" | cut -d' ' -f1)
printf '{"tag_name":"v9.9.9","assets":[{"name":"WinNote-9.9.9-x86_64.AppImage","digest":"sha256:%s","browser_download_url":"file://%s/nouvelle.AppImage"}]}' "$sha" "$d" > "$d/api.json"
cp target/release/winnote-gtk "$d/WinNote.AppImage"
export APPIMAGE="$d/WinNote.AppImage" WINNOTE_UPDATE_URL="file://$d/api.json"
"$d/WinNote.AppImage" >/dev/null 2>&1 &
sleep 11
import -window root "${1:-/tmp/upd.png}"
xdotool mousemove 681 398 click 1
sleep 5
echo "AppImage apres MAJ : $(head -c 40 "$d/WinNote.AppImage" | head -1)"
echo "relance : $(cat "$d/relance.txt" 2>/dev/null || echo AUCUNE)"
echo "processus winnote restants : $(pgrep -fc 'winnote-gtk|WinNote.AppImage' || true)"
pkill -f "Xvfb :80" || true
