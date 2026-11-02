# WinNote

Éditeur de texte natif, léger et rapide : **Windows 10/11 (64 bits)** et **Linux (GTK3)**.
Il reprend les fonctions du Bloc-notes Windows 11 et de Notepad++ et y ajoute celles des éditeurs modernes (Obsidian, VS Code).

## Télécharger
Dernière version : <https://github.com/Heiphaistos/WinNote/releases/latest>

| Système | Fichier | Détail |
|---|---|---|
| Windows | `WinNote-Setup-x.y.z.exe` | installeur par utilisateur, sans droits administrateur |
| Windows | `winnote.exe` | **version portable**, un seul fichier (c'est aussi le fichier utilisé par la mise à jour automatique) |
| Debian, Ubuntu, Mint… | `winnote_x.y.z_amd64.deb` | `sudo apt install ./winnote_*.deb` |
| Fedora, openSUSE, RHEL… | `winnote-x.y.z-1.x86_64.rpm` | `sudo dnf install ./winnote-*.rpm` |
| Toutes distributions | `winnote-x.y.z-linux-x86_64.tar.gz` | décompresser puis `./install.sh` (installe dans `~/.local`, sans root) |
| Arch, Manjaro, EndeavourOS… | `packaging/arch/PKGBUILD` | `makepkg -si` (compile depuis les sources) |

Linux : la version native utilise GTK3 (présent sur tous les bureaux courants) et le même moteur d'édition que Windows.

## Fonctions
- **Design** : onglets arrondis, thème appliqué à toute la fenêtre ; sous Windows, style Windows ou style macOS (boutons rouge/jaune/vert à gauche).
- **17 thèmes** (Windows, Apple Notes clair/sombre, Obsidian, Dracula, Nord, Tokyo Night, One Dark, Catppuccin, Monokai, Solarized, Gruvbox, GitHub, sépia, contraste élevé) et **8 dispositions**.
- **Édition** : multi-curseurs, sélection rectangulaire, repli de code, signets, indentation automatique, lignes (déplacer, dupliquer, trier, dédoublonner), commenter, casse, 45 langages colorés.
- **Recherche** : barre intégrée avec regex, mot entier, casse, surlignage de toutes les occurrences ; Aller à la ligne ; palette de commandes (Ctrl+Maj+P) ; ouverture rapide (Ctrl+E).
- **Fichiers** : encodages UTF-8 / UTF-8 BOM / UTF-16 / ANSI détectés, fins de ligne, fichiers récents, session restaurée (même les textes non enregistrés).
- **Windows uniquement** : correcteur orthographique (dictionnaires Windows), explorateur de dossier et plan, aperçu Markdown, vue scindée, liens entrants, impression, menu de l'explorateur, mise à jour automatique avec contrôle SHA-256.

## Construire
- Windows : `cargo build --release` (Rust MSVC + Visual Studio Build Tools) ; version complète : `scripts\publier-release.ps1`.
- Linux : `cargo build --release --features gtk-ui --bin winnote-gtk` (nécessite `libgtk-3-dev`, `pkg-config`, g++) puis `scripts/package-linux.sh`.

## Licence
WinNote est sous **PolyForm Noncommercial 1.0.0** (fichier `LICENSE`) : code source ouvert et modifiable, usage libre et gratuit, **mais pas de revente ni d'usage commercial** (seul l'auteur peut vendre le projet). Ce n'est donc pas une licence « open source » au sens strict de l'OSI, qui interdirait de restreindre la revente.
Composants tiers : `THIRD_PARTY_NOTICES.md` (Scintilla et Lexilla).
