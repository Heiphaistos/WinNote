# WinNote

Éditeur de texte natif, léger et rapide : **Windows 10/11 (64 bits)**, **Linux** (GTK3) et **macOS** (GTK3).
Il reprend les fonctions du Bloc-notes Windows 11 et de Notepad++ et y ajoute celles des éditeurs modernes (Obsidian, VS Code).

## Télécharger
Dernière version : <https://github.com/Heiphaistos/WinNote/releases/latest>

| Système | Fichier | Détail |
|---|---|---|
| Windows | `WinNote-Setup-x.y.z.exe` | installeur par utilisateur, sans droits administrateur |
| Windows | `winnote.exe` | **version portable**, un seul fichier (c'est aussi le fichier utilisé par la mise à jour automatique) |
| Toutes distributions | `WinNote-x.y.z-x86_64.AppImage` | `chmod +x` puis double-clic ; **se met à jour tout seul** (utilise le GTK3 du système) |
| Debian, Ubuntu, Mint… | `winnote_x.y.z_amd64.deb` | `sudo apt install ./winnote_*.deb` |
| Fedora, openSUSE, RHEL… | `winnote-x.y.z-1.x86_64.rpm` | `sudo dnf install ./winnote-*.rpm` |
| Toutes distributions | `winnote-x.y.z-linux-x86_64.tar.gz` | décompresser puis `./install.sh` (installe dans `~/.local`, sans root) |
| Arch, Manjaro, EndeavourOS… | `packaging/arch/PKGBUILD` | `makepkg -si` (compile depuis les sources) |
| macOS 11+ (Apple silicon et Intel) | `WinNote-x.y.z-macos-arm64.dmg` / `-x86_64.dmg` | glisser WinNote dans Applications ; non notarisé : 1er lancement par clic droit > Ouvrir |

Linux et macOS : mêmes fonctions que Windows (voir ci-dessous), même moteur d'édition (Scintilla). Le correcteur orthographique utilise Enchant s'il est installé (`libenchant-2-2` + `hunspell-fr` sous Debian/Ubuntu, `enchant2` + `hunspell-fr` sous Fedora/Arch, `brew install enchant` sous macOS). La mise à jour se fait seule pour l'AppImage ; pour les paquets du système, WinNote signale la nouvelle version et ouvre la page de téléchargement.

## Fonctions
- **Design** : onglets arrondis, thème appliqué à toute la fenêtre ; sous Windows, style Windows ou style macOS (boutons rouge/jaune/vert à gauche).
- **17 thèmes** (Windows, Apple Notes clair/sombre, Obsidian, Dracula, Nord, Tokyo Night, One Dark, Catppuccin, Monokai, Solarized, Gruvbox, GitHub, sépia, contraste élevé) et **8 dispositions**.
- **Édition** : multi-curseurs, sélection rectangulaire, repli de code, signets, indentation automatique, lignes (déplacer, dupliquer, trier, dédoublonner), commenter, casse, 45 langages colorés.
- **Recherche** : barre intégrée avec regex, mot entier, casse, surlignage de toutes les occurrences ; Aller à la ligne ; palette de commandes (Ctrl+Maj+P) ; ouverture rapide (Ctrl+E).
- **Fichiers** : encodages UTF-8 / UTF-8 BOM / UTF-16 / ANSI détectés, fins de ligne, fichiers récents, session restaurée (même les textes non enregistrés).
- **Partout** : correcteur orthographique (fr, en, es, de, it), explorateur de dossier, plan du document, liens entrants `[[note]]`, aperçu Markdown, vue scindée, impression (en-tête, pied « Page n / N », noir sur blanc), ouvrir dans une nouvelle fenêtre, lecture seule, toujours au premier plan, mise à jour automatique avec contrôle SHA-256.
- **Windows seulement** : menu « Modifier avec WinNote » de l'explorateur, rendu DirectWrite optionnel, barre de titre style macOS.

## Construire
- Windows : `cargo build --release` (Rust MSVC + Visual Studio Build Tools) ; version complète : `scripts\publier-release.ps1`.
- Linux : `cargo build --release --features gtk-ui --bin winnote-gtk` (nécessite `libgtk-3-dev`, `pkg-config`, g++) puis `scripts/package-linux.sh` (tar.gz, .deb, .rpm, AppImage).
- macOS : `brew install gtk+3 pkg-config dylibbundler`, même `cargo build`, puis `scripts/package-macos.sh` (.app + .dmg).

## Licence
WinNote est sous **PolyForm Noncommercial 1.0.0** (fichier `LICENSE`) : code source ouvert et modifiable, usage libre et gratuit, **mais pas de revente ni d'usage commercial** (seul l'auteur peut vendre le projet). Ce n'est donc pas une licence « open source » au sens strict de l'OSI, qui interdirait de restreindre la revente.
Composants tiers : `THIRD_PARTY_NOTICES.md` (Scintilla et Lexilla).
