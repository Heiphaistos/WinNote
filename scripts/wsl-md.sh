#!/bin/bash
# Capture le rendu Markdown dans l'editeur Linux, SANS apercu. Usage : scripts/wsl-md.sh sortie.png
set -e
out=${1:-/tmp/md.png}
"$(dirname "$0")/wsl-sync.sh"
cd ~/wnbuild; . ~/.cargo/env
cargo build --release --features gtk-ui --bin winnote-gtk 2>&1 | grep -E "^error" -A8 | head -40 || true
(Xvfb :79 -screen 0 1200x700x24 >/dev/null 2>&1 &) ; sleep 1
export DISPLAY=:79 GDK_BACKEND=x11 HOME=$(mktemp -d)
mkdir -p "$HOME/.config/winnote"
printf 'theme=0\n' > "$HOME/.config/winnote/settings.ini"
printf '# Titre un\n\n## Titre deux\n\nTexte **gras** et *italique* et `code` et [lien](http://x.fr).\n\n- puce\n- [x] fait\n\n> citation\n\n```rust\nfn main() {}\n```\n' > "$HOME/a.md"
target/release/winnote-gtk "$HOME/a.md" >/dev/null 2>&1 &
sleep 3
import -window root "$out"
# document sans nom : taper du Markdown
xdotool mousemove 600 300 click 1; xdotool key --clearmodifiers ctrl+n; sleep 1
xdotool type --delay 40 "# Bonjour"; xdotool key Return; xdotool type --delay 40 "du **gras** et *ital*"; sleep 1.5
import -window root "${out%.png}-new.png"
pkill -f winnote-gtk || true; pkill -f "Xvfb :79" || true
