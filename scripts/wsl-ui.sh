#!/bin/bash
# Captures de l'interface Linux : barre de recherche, menu contextuel de l'explorateur, autocompletion, raccourci personnalise. Usage : scripts/wsl-ui.sh dossier_sortie
set -e
out=${1:-/tmp/ui}; mkdir -p "$out"
"$(dirname "$0")/wsl-sync.sh"
cd ~/wnbuild; . ~/.cargo/env
cargo build --release --features gtk-ui --bin winnote-gtk 2>&1 | grep -E "^error" -A8 | head -40 || true
(Xvfb :80 -screen 0 1300x800x24 >/dev/null 2>&1 &) ; sleep 1
export DISPLAY=:80 GDK_BACKEND=x11 HOME=$(mktemp -d)
mkdir -p "$HOME/notes" "$HOME/.config/winnote"
printf 'theme=0\nroot=%s\n' "$HOME/notes" > "$HOME/.config/winnote/settings.ini"
printf 'Ctrl+Alt+B = Markdown : gras\n' > "$HOME/.config/winnote/raccourcis.txt"
printf '# Titre\n\nmot parlement et parlons\n' > "$HOME/notes/a.md"
printf 'fn calcul_total() {}\nfn calcul_moyenne() {}\n' > "$HOME/notes/b.rs"
target/release/winnote-gtk "$HOME/notes/a.md" >/dev/null 2>&1 &
sleep 3
xdotool mousemove 600 300 click 1
xdotool key --clearmodifiers ctrl+h; sleep 1
xdotool type --delay 50 "parl"; sleep 1
import -window root "$out/find.png"
xdotool key --clearmodifiers Escape; sleep 0.5
xdotool key --clearmodifiers ctrl+shift+e; sleep 1.5
xdotool mousemove 100 160 click 3; sleep 1
import -window root "$out/ctx.png"
xdotool key Escape; sleep 0.5
# raccourci personnalise : gras sur le mot (double-clic)
xdotool mousemove 330 144 click --repeat 2 --delay 120 1; sleep 0.5; xdotool key --clearmodifiers ctrl+alt+b; sleep 1
import -window root "$out/key.png"
pkill -f winnote-gtk || true; sleep 1
# autocompletion dans un fichier code
target/release/winnote-gtk "$HOME/notes/b.rs" >/dev/null 2>&1 &
sleep 3
xdotool mousemove 700 300 click 1; xdotool key --clearmodifiers ctrl+End; xdotool type --delay 90 "calc"; sleep 1.5
import -window root "$out/auto.png"
pkill -f winnote-gtk || true; pkill -f "Xvfb :80" || true
