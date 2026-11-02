# WinNote

Editeur de texte natif Windows 10/11, 64 bits, leger (exe ~ 2 Mo, aucune dependance a installer).
Il reprend les fonctions du Bloc-notes Windows 11 et de Notepad++ et y ajoute celles des editeurs modernes (Obsidian, VS Code).

- Design : onglets arrondis, barre d'etat cliquable, barre de titre assortie au theme, style Windows ou style macOS (boutons rouge/jaune/vert en haut a gauche, menu dans le bouton hamburger).
- 17 themes (Windows, Apple Notes clair/sombre, Obsidian, Dracula, Nord, Tokyo Night, One Dark, Catppuccin, Monokai, Solarized, Gruvbox, GitHub, sepia, contraste eleve) et 8 dispositions (Classique, Code, Ecriture, Notes Apple clair/sombre, Obsidian, Markdown + apercu, Sans distraction).
- Edition : multi-curseurs, selection rectangulaire (Alt+glisser), repli de code, signets, indentation automatique, deplacer/dupliquer/trier/dedoublonner les lignes, commenter, casse, tabulations/espaces, 30 langages colores (moteur Lexilla, 125 disponibles).
- Recherche : barre integree (Ctrl+F / Ctrl+H) avec regex, mot entier, casse, surlignage de toutes les occurrences ; recherche dans un dossier ; Aller a la ligne (Ctrl+G).
- Navigation : palette de commandes (Ctrl+Maj+P), ouverture rapide (Ctrl+E), explorateur de dossier, plan du document, liens [[note]] et URL (Ctrl+Entree).
- Markdown : apercu natif en direct (Ctrl+Maj+M).
- Fichiers : encodages UTF-8 / UTF-8 BOM / UTF-16 / ANSI detectes automatiquement, fins de ligne, rechargement si le fichier change sur le disque, fichiers recents, rouvrir l'onglet ferme.
- Session : au lancement, les onglets (meme non enregistres) reviennent comme avant.
- Windows : impression (Ctrl+P), menu "Modifier avec WinNote" de l'explorateur (par utilisateur, sans admin), mise a jour automatique avec controle SHA-256.

## Construire
`cargo build --release` (Rust stable MSVC, Visual Studio Build Tools). Scintilla et Lexilla sont compiles depuis `vendor/` par `build.rs`.

## Licences
Voir `THIRD_PARTY_NOTICES.md`.
