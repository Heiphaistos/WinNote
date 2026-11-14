#!/bin/bash
# Test de bout en bout de la mise a jour AppImage (API simulee en file://, build `test-hooks` OBLIGATOIRE) :
# dialogue, telechargement, SHA-256, signature Ed25519 epinglee, remplacement, relance.
# Trois scenarios : signature valide (installee), signature d'une autre cle (refusee), pas de .sig (refuse).
# Prerequis : scripts/wsl-test.sh deja lance (construit target/release/winnote-gtk --features gtk-ui,test-hooks).
cd ~/wnbuild
. ~/.cargo/env
cargo build --release --features gtk-ui,test-hooks --bin winnote-gtk 2>&1 | grep -E "^error" -A8
cargo build --release --example sign 2>&1 | grep -E "^error" -A8
SIGN=target/release/examples/sign
(Xvfb :80 -screen 0 1280x800x24 >/dev/null 2>&1 &) ; sleep 1
export DISPLAY=:80 GDK_BACKEND=x11
keys=$(mktemp -d)
head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > "$keys/good.hex"
head -c 32 /dev/urandom | od -An -tx1 | tr -d ' \n' > "$keys/evil.hex"
export WINNOTE_UPDATE_PUBKEY=$($SIGN pub "$keys/good.hex")
scenario() { # $1 = good|evil|none
  export HOME=$(mktemp -d); mkdir -p "$HOME/.config/winnote"
  d=$(mktemp -d)
  printf '#!/bin/sh\necho NOUVELLE-VERSION > %s/relance.txt\n' "$d" > "$d/nouvelle.AppImage"
  sha=$(sha256sum "$d/nouvelle.AppImage" | cut -d' ' -f1)
  printf '{"tag_name":"v9.9.9","assets":[{"name":"WinNote-9.9.9-x86_64.AppImage","digest":"sha256:%s","browser_download_url":"file://%s/nouvelle.AppImage"}]}' "$sha" "$d" > "$d/api.json"
  [ "$1" != none ] && $SIGN sign "$keys/$1.hex" v9.9.9 "$d/nouvelle.AppImage" "$sha"
  cp target/release/winnote-gtk "$d/WinNote.AppImage"
  export APPIMAGE="$d/WinNote.AppImage" WINNOTE_UPDATE_URL="file://$d/api.json"
  "$d/WinNote.AppImage" >/dev/null 2>&1 &
  sleep 11
  [ "$1" = good ] && import -window root "${2:-/tmp/upd.png}"
  xdotool mousemove 681 398 click 1
  sleep 5
  echo "[$1] AppImage apres MAJ : $(head -c 40 "$d/WinNote.AppImage" | head -1 | tr -d '\0')"
  echo "[$1] relance : $(cat "$d/relance.txt" 2>/dev/null || echo AUCUNE)"
  # un message d'erreur modal peut rester ouvert : on ferme proprement
  xdotool key Return; sleep 1
  pkill -f "WinNote.AppImage" || true; sleep 1
}
scenario good "$1"
scenario evil
scenario none
echo "processus winnote restants : $(pgrep -fc 'winnote-gtk|WinNote.AppImage' || true)"
pkill -f "Xvfb :80" || true
