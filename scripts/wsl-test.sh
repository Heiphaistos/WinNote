#!/bin/bash
# Compile winnote-gtk dans ~/wnbuild (WSL) et lance l'autotest sous Xvfb. Usage : scripts/wsl-test.sh
set -e
"$(dirname "$0")/wsl-sync.sh"
cd ~/wnbuild
. ~/.cargo/env
cargo build --release --features gtk-ui --bin winnote-gtk 2>&1 | grep -E "^(error|warning)" -A8 | head -80 || true
(Xvfb :77 -screen 0 1280x800x24 >/dev/null 2>&1 &) ; sleep 1
export DISPLAY=:77 GDK_BACKEND=x11 HOME=$(mktemp -d)
rc=0
timeout 120 target/release/winnote-gtk --selftest "$HOME/selftest.log"  || rc=$?
cat "$HOME/selftest.log"; echo "code de sortie : $rc"
pkill -f "Xvfb :77" || true
