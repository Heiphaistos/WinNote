# WinNote - etat v0.3.1 (2026-11-02)

Depot PUBLIC (necessaire a la mise a jour automatique) : https://github.com/Heiphaistos/WinNote/releases

## Verifie en vrai
- Mise a jour automatique : la 0.3.0 publiee detecte la 0.3.1 sur GitHub, telecharge, verifie le SHA-256, remplace l'exe, relance, supprime l'ancien (aucun processus restant).
- Impression : "Microsoft Print to PDF", 3 pages, en-tete (nom + date), pied "Page n / N", retour a la ligne, accents, tabulations.
- Menu de l'explorateur : cle de registre creee, commande executee (le fichier s'ouvre), cle retiree.
- Autotest integre `winnote.exe --selftest journal.txt` : 49 controles, 6 executions de suite sans echec (trier, doublons, joindre, deplacer/dupliquer/supprimer lignes,
  casse, tabulations/espaces, commenter, multi-curseurs, signets, repli, recherche/remplacement + regex, recherche dans un dossier, ouverture rapide, liens [[note]],
  enregistrer/rouvrir dans les 5 encodages, indentation auto, fins de ligne).
- Tests unitaires : encodages, apercu Markdown, SHA-256, analyse de la release GitHub, prefixes de commentaire.
- Captures regardees : style macOS, themes, recherche, palette, Aller a, explorateur/plan, apercu Markdown.

## Bugs reels trouves par ces tests et corriges
Plantage au dessin de texte vide ; regex aleatoire (motif non termine par un zero) ; deplacer les lignes et MAJUSCULES (mauvais identifiants) ;
multi-curseurs sans effet ; fichiers ANSI mal lus (page de code UTF-8 forcee par le manifeste).

## Limites connues
- Impression testee vers PDF, pas sur une imprimante physique (meme chemin de code).
- Executable non signe : SmartScreen peut avertir au premier lancement (signature de code a acheter/configurer).
- Style macOS teste seulement sous Windows 11.
- ~45 Mo de memoire (DirectWrite) : a optimiser.

## A faire
Correcteur orthographique, backlinks (Obsidian), vue scindee, installeur, signature de code, CI de release, glisser un onglet hors de la fenetre,
coloration des ~95 autres langages Lexilla.
