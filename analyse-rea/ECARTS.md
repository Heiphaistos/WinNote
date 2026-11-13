# Ecarts WinNote 0.6.1 vs Bloc-notes W11 / Notepad++ 8.9.8 / Obsidian (2026-11-13)

Sources : Notepad++ localization\english.xml (menus, sous-menus), Obsidian obsidian.asar (ids `editor:`, `app:`, `workspace:`, `file-explorer:`, `markdown:`... deja installe), NOTES.md pour le Bloc-notes.
Verification WinNote : ids.rs, menu.rs, palette.rs, findbar.rs, textops.rs, grep sur src/ (hors linux/). Exclus : Cowriter/IA, plugins tiers, services payants, Sync/Publish Obsidian, Plugins Admin NPP.
Priorite : P1 quotidien, P2 utile, P3 rare. Taille : S (<1 j), M, L.

## A. Notepad++ (inventaire des fonctions, aucun code copie)
| Fonction manquante / incomplete | Prio | Taille |
|---|---|---|
| Surlignage des parentheses/crochets/accolades assortis + Aller a l'accolade correspondante + Selectionner entre {} [] () (grep BRACE = 0) | P1 | S |
| Auto-completion de mots du document (SCI_AUTOCSHOW, grep AUTOC = 0) + fermeture auto des paires ( [ { " | P1 | M |
| Rechercher/Remplacer : portee « dans la selection », boucle/retour au debut explicite, mode etendu (\n \t \x), compteur « Compter », « Marquer tout » avec signets, Remplacer dans tous les onglets ouverts, historique des termes | P1 | M |
| Recherche dans les fichiers : filtres (*.txt), recursif on/off, remplacer dans les fichiers, panneau de resultats navigable (actuellement liste simple) | P2 | M |
| Recherche incrementale (barre qui cherche a la frappe) | P2 | S |
| Macros : enregistrer / lecture / lecture N fois / enregistrer sous (SCI_STARTRECORD) | P2 | M |
| Mode colonne : editeur de colonnes (insertion de nombres incrementes / texte sur N lignes) ; la selection rectangulaire native Alt+glisser existe | P2 | M |
| Lignes : fusionner doublons consecutifs, inverser l'ordre, melanger, inserer ligne vide au-dessus/dessous, supprimer lignes vides, scinder les lignes, tri insensible a la casse / par longueur / numerique (WinNote : seulement A-Z / Z-A) | P2 | S |
| Espaces : supprimer espaces de tete, tete+fin, EOL vers espace (WinNote : fin de ligne seulement) ; espace vers tab (tete) | P3 | S |
| Casse : phrase, inversee, aleatoire, proper blend (WinNote : majuscules, minuscules, titre) | P3 | S |
| Commentaire de bloc (/* */) et decommentage de bloc (WinNote : ligne seulement) ; Rogner debut de ligne | P2 | S |
| Guides d'indentation, afficher fins de ligne (EOL), symbole de retour a la ligne, tous les caracteres (SCI_SETINDENTATIONGUIDES / SETVIEWEOL = 0) | P2 | S |
| Surligner une occurrence selectionnee dans tout le document (« Smart highlighting ») et 5 styles de marquage (clic droit « Style un jeton ») | P2 | M |
| Historique des modifications (marge verte/orange des changements enregistres, aller a la modification suivante) | P2 | M |
| Liste des fonctions (panneau avec 48 functionList NPP ; WinNote : plan par mots-cles simples dans notes.rs) | P2 | M |
| Carte du document (minimap) | P3 | L |
| Surveillance de fichier « tail -f » + rechargement auto quand le fichier change sur le disque (mdlive.rs ne couvre pas cela) | P1 | M |
| Presse-papiers : historique, Coller HTML/RTF, copier le nom du fichier / le dossier / tous les chemins, copier le texte colore | P3 | M |
| Fichier : Renommer, Deplacer vers la corbeille, Enregistrer une copie, Charger/Enregistrer une session nommee, Ouvrir un terminal (cmd/PowerShell) dans le dossier, Ouvrir tous les recents (grep recycle = 0) | P2 | S |
| Onglets : epingler (pin), colorer, deplacer onglet, aller a l'onglet N / premier / dernier, liste des documents, trier les onglets, fermer a gauche/droite/inchanges, glisser hors fenetre | P2 | M |
| Deux vues cote a cote avec defilement synchronise, cloner vers l'autre vue (WinNote : vue scindee du meme doc seulement) | P3 | L |
| Encodages : tous les jeux de caracteres (Cyrillique, Europe centrale, Grec, Japonais, Chinois, Coreen, Turc... ; WinNote : 5 encodages), « Convertir en... » distinct de « Ouvrir en... » | P1 | M |
| Somme de controle MD5 / SHA-1 / SHA-256 / SHA-512 du fichier ou de la selection (« Outils ») | P3 | S |
| Editeur hexadecimal / ouverture de fichiers binaires et tres gros fichiers sans blocage | P3 | L |
| Langage utilisateur (UDL) : definir/importer sa coloration ; configurateur de styles (couleurs par langage) | P3 | L |
| Preferences : fenetre unifiee (WinNote : Police + menus seulement), mappeur de raccourcis (rebind), menu contextuel editable, tabulation 2/4/8 + indentation auto par langage, ligne de commande avancee (`-n123`, `-multiInst`) | P1 (raccourcis+tabulation) | L |
| Direction du texte RTL/LTR ; verrouillage lecture seule attribut Windows | P3 | S |
| Post-it, cacher les lignes, tout replier niveau N / niveau courant (WinNote : tout replier/deplier + bascule) | P3 | S |
| Instance unique + reutilisation de la fenetre, ouvrir/deplacer dans une nouvelle instance (NEWWINDOW existe, a verifier pour le deplacement d'onglet) | P3 | S |

## B. Bloc-notes Windows 11
| Fonction manquante / incomplete | Prio | Taille |
|---|---|---|
| Barre d'outils / commandes de mise en forme Markdown (gras, italique, lien, liste, tableau + insertion/suppression ligne-colonne, liste a cocher) : WinNote rend le Markdown mais n'a AUCUNE commande de mise en forme (grep) | P1 | M |
| Avertissement a l'enregistrement « Unicode vers ANSI » (perte de caracteres) | P2 | S |
| Dossier d'images Markdown / collage d'image depuis le presse-papiers | P3 | M |
| Page d'impression (marges, orientation) : WinNote imprime avec en-tete/pied fixes | P2 | S |
| Aller a : verifier saut par colonne/decalage (ligne seule) ; fenetre d'accueil/reglages : police, theme systeme (suivre clair/sombre Windows automatiquement) | P2 | S |
| Fermer la fenetre (Ctrl+Maj+W) distinct de l'onglet ; ouverture fichier par glisser-deposer deja present (WM_DROPFILES OK) | P3 | S |

## C. Obsidian (commandes du coeur, ids verifies dans l'asar)
| Fonction manquante / incomplete | Prio | Taille |
|---|---|---|
| `editor:toggle-bold/italics/strikethrough/highlight/code/inline-math`, `toggle-blockquote`, `toggle-bullet-list`, `toggle-numbered-list`, `toggle-checklist-status` / `cycle-list-checklist`, `set-heading`, `clear-formatting` : aucune commande equivalente | P1 | M |
| `editor:insert-link/wikilink/tag/callout/codeblock/table/horizontal-rule/mathblock/footnote`, `indent-list/unindent-list` ; auto-continuation des listes a l'Entree | P1 | M |
| `editor:follow-link` (existe : OPENLINK) + `open-link-in-new-split/new-window` ; completion `[[` des noms de notes pendant la frappe | P2 | M |
| Panneau Etiquettes (`tag-pane`) et recherche par #tag ; panneau Proprietes / frontmatter YAML (`markdown:add-metadata-property`, `add-alias`) | P2 | M |
| Notes quotidiennes (`daily-notes`, goto-next/prev) et Modeles (`templates`) : grep daily/template = 0 | P2 | M |
| `file-explorer:new-file/new-folder/duplicate-file/move-file`, `app:delete-file`, renommer : l'explorateur lateral de WinNote est en lecture/ouverture | P1 | M |
| `workspace:split-vertical/horizontal`, `toggle-pin`, `copy-path/copy-full-path`, `export-pdf`, `show-trash`, `undo-close-pane` (existe : REOPEN), `goto-last-tab`, onglets empiles | P2 | M |
| `bookmarks` (favoris de notes/sections/recherches, panneau), `switcher:open` (existe : QUICKOPEN), `file-recovery:open` (versions locales automatiques), `random-note` | P2 | M |
| `editor:toggle-readable-line-length`, `toggle-source` (source/mode lecture) ; `editor:rename-heading`, `fold-more/less` (replis par niveau), `note-composer` extraire/fusionner/scinder | P3 | M |
| `graph:open/open-local` (graphe des liens), `canvas`, `bases`, `outline:open-for-current` (plan existe : OUTLINE) | P3 | L |
| `workspace:edit-file-title` (titre du fichier editable en tete), `word-count` panneau (existe : STATS) | P3 | S |

## Deja couvert (verifie, ne pas refaire)
Multi-curseurs (Alt+D), selection rectangulaire native, signets, repli, tri A-Z/Z-A, doublons, joindre, commentaire de ligne, zoom, plein ecran/zen, toujours au premier plan, lecture seule, recharger, rouvrir onglet, rouvrir recents, palette, ouverture rapide, recherche dans le dossier, backlinks, plan, aperçu Markdown, vue scindee, correcteur (5 langues), impression, session + textes non enregistres, 17 themes, glisser-deposer de fichiers, mise a jour auto, nouvelle fenetre.

## Recommandation d'ordre
1) Accolades assorties + auto-completion/paires (A) ; 2) rechargement auto si fichier modifie dehors + tail (A) ; 3) commandes de mise en forme Markdown + listes (B/C, une seule implementation couvre Bloc-notes et Obsidian) ; 4) Rechercher/Remplacer : selection, etendu, compter, marquer ; 5) explorateur : creer/renommer/supprimer/dupliquer ; 6) mappeur de raccourcis + taille de tabulation ; 7) encodages supplementaires.
