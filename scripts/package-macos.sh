#!/bin/bash
# Construit WinNote.app (GTK3 embarque avec dylibbundler) et un .dmg. A lancer SUR macOS, depuis la racine du depot,
# apres : cargo build --release --features gtk-ui --bin winnote-gtk
# Prerequis (Homebrew) : brew install gtk+3 dylibbundler
set -e
ver=$(grep -m1 '^version' Cargo.toml | cut -d'"' -f2)
arch=$(uname -m)
bin=target/release/winnote-gtk
[ -x "$bin" ] || { echo "compiler d'abord : cargo build --release --features gtk-ui --bin winnote-gtk"; exit 1; }
out=target/dist-mac; rm -rf "$out"; mkdir -p "$out"
app="$out/WinNote.app"
bp=$(brew --prefix)
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources/share" "$app/Contents/Frameworks"
cp "$bin" "$app/Contents/MacOS/winnote-bin"

# icone
is="$out/winnote.iconset"; mkdir -p "$is"
for s in 16 32 128 256; do sips -z $s $s assets/winnote.png --out "$is/icon_${s}x${s}.png" >/dev/null; done
iconutil -c icns "$is" -o "$app/Contents/Resources/winnote.icns"

cat > "$app/Contents/Info.plist" <<EOP
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0"><dict>
<key>CFBundleName</key><string>WinNote</string>
<key>CFBundleDisplayName</key><string>WinNote</string>
<key>CFBundleIdentifier</key><string>org.heiphaistos.winnote</string>
<key>CFBundleVersion</key><string>$ver</string>
<key>CFBundleShortVersionString</key><string>$ver</string>
<key>CFBundleExecutable</key><string>WinNote</string>
<key>CFBundleIconFile</key><string>winnote</string>
<key>CFBundlePackageType</key><string>APPL</string>
<key>LSMinimumSystemVersion</key><string>11.0</string>
<key>NSHighResolutionCapable</key><true/>
<key>CFBundleDocumentTypes</key><array><dict>
<key>CFBundleTypeName</key><string>Texte</string>
<key>CFBundleTypeRole</key><string>Editor</string>
<key>LSItemContentTypes</key><array><string>public.text</string><string>public.plain-text</string><string>public.source-code</string></array>
</dict></array>
</dict></plist>
EOP

# lanceur : donne a GTK ses schemas et ses icones embarques
cat > "$app/Contents/MacOS/WinNote" <<'EOL'
#!/bin/bash
d="$(cd "$(dirname "$0")/.." && pwd)"
export XDG_DATA_DIRS="$d/Resources/share"
export GSETTINGS_SCHEMA_DIR="$d/Resources/share/glib-2.0/schemas"
exec "$d/MacOS/winnote-bin" "$@"
EOL
chmod +x "$app/Contents/MacOS/WinNote"

# donnees GTK : schemas compiles (sinon les boites Ouvrir/Enregistrer plantent) et icones
mkdir -p "$app/Contents/Resources/share/glib-2.0" "$app/Contents/Resources/share/icons"
cp -R "$bp/share/glib-2.0/schemas" "$app/Contents/Resources/share/glib-2.0/"
[ -d "$bp/share/icons/Adwaita" ] && cp -R "$bp/share/icons/Adwaita" "$app/Contents/Resources/share/icons/"
[ -d "$bp/share/icons/hicolor" ] && cp -R "$bp/share/icons/hicolor" "$app/Contents/Resources/share/icons/"
cp LICENSE THIRD_PARTY_NOTICES.md "$app/Contents/Resources/"

# bibliotheques : copiees dans le .app et chemins reecrits
dylibbundler -od -b -x "$app/Contents/MacOS/winnote-bin" -d "$app/Contents/Frameworks" -p @executable_path/../Frameworks >/dev/null

# signature ad hoc (obligatoire sur Apple silicon) ; pas de notarisation : au 1er lancement, clic droit > Ouvrir
codesign --force --deep -s - "$app"

dmg="$out/WinNote-$ver-macos-$arch.dmg"
hdiutil create -volname WinNote -srcfolder "$app" -ov -format UDZO "$dmg" >/dev/null
ls -la "$out"/*.dmg
