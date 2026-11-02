#!/bin/bash
# Construit les paquets Linux a partir de target/release/winnote-gtk : tar.gz, .deb, .rpm, AppImage.
# Usage (sous Linux/WSL, depuis la racine du depot) : scripts/package-linux.sh
set -e
ver=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
bin=target/release/winnote-gtk
[ -x "$bin" ] || { echo "compiler d'abord : cargo build --release --features gtk-ui --bin winnote-gtk"; exit 1; }
out=target/dist; rm -rf "$out"; mkdir -p "$out"
abs=$(cd "$out" && pwd)

# ---- tar.gz portable
st="$out/winnote-$ver-linux-x86_64"
mkdir -p "$st"
install -m755 "$bin" "$st/winnote"
cp assets/winnote.desktop assets/winnote.png README.md LICENSE THIRD_PARTY_NOTICES.md "$st/"
cat > "$st/install.sh" <<'EOS'
#!/bin/sh
# Installation pour l'utilisateur courant (sans root) : ~/.local
set -e
d=$(dirname "$0")
mkdir -p ~/.local/bin ~/.local/share/applications ~/.local/share/icons/hicolor/256x256/apps
install -m755 "$d/winnote" ~/.local/bin/winnote
install -m644 "$d/winnote.png" ~/.local/share/icons/hicolor/256x256/apps/winnote.png
sed "s|^Exec=.*|Exec=$HOME/.local/bin/winnote %F|" "$d/winnote.desktop" > ~/.local/share/applications/winnote.desktop
echo "WinNote installe dans ~/.local/bin/winnote"
EOS
chmod +x "$st/install.sh"
tar -C "$out" -czf "$out/winnote-$ver-linux-x86_64.tar.gz" "winnote-$ver-linux-x86_64"

# ---- .deb
d="$out/deb"
mkdir -p "$d/DEBIAN" "$d/usr/bin" "$d/usr/share/applications" "$d/usr/share/icons/hicolor/256x256/apps" "$d/usr/share/doc/winnote"
install -m755 "$bin" "$d/usr/bin/winnote"
install -m644 assets/winnote.desktop "$d/usr/share/applications/winnote.desktop"
install -m644 assets/winnote.png "$d/usr/share/icons/hicolor/256x256/apps/winnote.png"
install -m644 README.md LICENSE THIRD_PARTY_NOTICES.md "$d/usr/share/doc/winnote/"
size=$(du -sk "$d/usr" | cut -f1)
cat > "$d/DEBIAN/control" <<EOC
Package: winnote
Version: $ver
Section: editors
Priority: optional
Architecture: amd64
Depends: libgtk-3-0, libc6 (>= 2.35)
Recommends: libenchant-2-2, hunspell-fr | hunspell-en-us, curl
Installed-Size: $size
Maintainer: Heiphaistos <noreply@users.noreply.github.com>
Homepage: https://github.com/Heiphaistos/WinNote
Description: editeur de texte leger (GTK3, moteur Scintilla)
 Onglets, themes, coloration de 45 langages, recherche regex, palette de commandes, session restauree,
 panneau lateral, apercu Markdown, vue scindee, correcteur orthographique, impression.
EOC
dpkg-deb --build --root-owner-group "$d" "$out/winnote_${ver}_amd64.deb" >/dev/null

# ---- .rpm
top="$abs/rpmbuild"; mkdir -p "$top"/{BUILD,RPMS,SOURCES,SPECS,SRPMS}
cp "$bin" "$top/SOURCES/winnote"; cp assets/winnote.desktop assets/winnote.png README.md LICENSE "$top/SOURCES/"
cat > "$top/SPECS/winnote.spec" <<EOR
Name: winnote
Version: $ver
Release: 1
Summary: Editeur de texte leger (GTK3, moteur Scintilla)
License: PolyForm-Noncommercial-1.0.0 AND HPND
URL: https://github.com/Heiphaistos/WinNote
Requires: gtk3
Recommends: enchant2, hunspell-fr, curl
AutoReqProv: no
%description
Onglets, themes, coloration de 45 langages, recherche regex, palette de commandes, session restauree.
%install
install -Dm755 %{_sourcedir}/winnote %{buildroot}/usr/bin/winnote
install -Dm644 %{_sourcedir}/winnote.desktop %{buildroot}/usr/share/applications/winnote.desktop
install -Dm644 %{_sourcedir}/winnote.png %{buildroot}/usr/share/icons/hicolor/256x256/apps/winnote.png
install -Dm644 %{_sourcedir}/LICENSE %{buildroot}/usr/share/licenses/winnote/LICENSE
%files
/usr/bin/winnote
/usr/share/applications/winnote.desktop
/usr/share/icons/hicolor/256x256/apps/winnote.png
/usr/share/licenses/winnote/LICENSE
EOR
rpmbuild -bb --define "_topdir $top" --define "debug_package %{nil}" --define "__strip /bin/true" "$top/SPECS/winnote.spec" >/dev/null 2>&1
cp "$top"/RPMS/*/winnote-*.rpm "$out/"

# ---- AppImage (utilise le GTK3 du systeme)
ad="$out/WinNote.AppDir"; mkdir -p "$ad/usr/bin"
install -m755 "$bin" "$ad/usr/bin/winnote"
cp assets/winnote.png "$ad/winnote.png"; cp assets/winnote.desktop "$ad/winnote.desktop"
cat > "$ad/AppRun" <<'EOA'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/winnote" "$@"
EOA
chmod +x "$ad/AppRun"
# appimagetool officiel (https://github.com/AppImage/appimagetool, release "continuous") : telecharge puis VERIFIE par son SHA-256 epingle.
# Si l'empreinte change (nouvelle version de l'outil), le script s'arrete : verifier l'empreinte publiee par GitHub puis mettre a jour la ligne ci-dessous.
want=62aebc77c2149e96b593b94da9bbd1881e0984a56399ff6d7a03b7ddd7c9ce68
tool=${APPIMAGETOOL:-target/tools/appimagetool-x86_64.AppImage}
if [ ! -x "$tool" ]; then
  mkdir -p "$(dirname "$tool")"
  curl -fsSL -o "$tool" https://github.com/AppImage/appimagetool/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$tool"
fi
if [ -z "$APPIMAGETOOL" ] && [ "$(sha256sum "$tool" | cut -d' ' -f1)" != "$want" ]; then
  echo "appimagetool : empreinte inattendue, AppImage non fabriquee"; rm -f "$tool"; exit 1
fi
ARCH=x86_64 "$tool" --appimage-extract-and-run "$ad" "$out/WinNote-$ver-x86_64.AppImage" >/dev/null 2>&1

ls -la "$out"/*.tar.gz "$out"/*.deb "$out"/*.rpm "$out"/*.AppImage 2>/dev/null
