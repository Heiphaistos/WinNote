#!/bin/bash
# Lance winnote-gtk sous Xvfb sur un dossier de notes, ouvre panneau + aperçu (raccourcis clavier) et capture l'écran. Usage : scripts/wsl-shot.sh sortie.png [theme]
set -e
out=${1:-/tmp/wn.png}
cd ~/wnbuild
(Xvfb :78 -screen 0 1400x850x24 >/dev/null 2>&1 &) ; sleep 1
export DISPLAY=:78 GDK_BACKEND=x11 HOME=$(mktemp -d)
mkdir -p "$HOME/notes/sous-dossier" "$HOME/.config/winnote"
printf '# Journal\n\nUn texte avec **gras**, *italique* et `code`.\n\n## Liste\n- premier\n- [x] fait\n- [ ] a faire\n\n> citation\n\nVoir [[idee]] et le mondde mal ecrit.\n\n## Table\n| a | b |\n|---|---|\n| 1 | 2 |\n' > "$HOME/notes/journal.md"
printf 'une idee, voir [[journal]]\n' > "$HOME/notes/idee.md"
printf 'theme=%s\nroot=%s\nspell=1\n' "${2:-0}" "$HOME/notes" > "$HOME/.config/winnote/settings.ini"
target/release/winnote-gtk "$HOME/notes/journal.md" >/dev/null 2>&1 &
sleep 3
xdotool mousemove 700 400 click 1; sleep 0.5
xdotool key --clearmodifiers ctrl+shift+m; sleep 1.5
import -window root "${out%.png}-prev.png"
xdotool key --clearmodifiers ctrl+shift+e; sleep 1
import -window root "$out"
pkill -f winnote-gtk || true; pkill -f "Xvfb :78" || true
