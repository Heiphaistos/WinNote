# WinNote - etat v0.2.0 (2026-11-02)

## Fait et verifie (exe lance + capture regardee, ou test automatise)
Onglets (glisser, menu contextuel), style macOS, 17 themes / 8 dispositions, barre de recherche + regex + surlignage, palette, Aller a,
explorateur + plan, apercu Markdown, session et sortie "a chaud", repli, 30 langages, chaine de remplacement de la mise a jour (test local),
restauration de session (test local), fermeture sans processus restant.
Tests unitaires : encodages, apercu Markdown, SHA-256 (BCrypt), versions/JSON du module de mise a jour, prefixes de commentaire.

## Ecrit mais NON teste en vrai
- Impression (Ctrl+P) : pas d'imprimante de test.
- Telechargement de mise a jour depuis GitHub : depot prive, aucune release (regle : rien publier sans Momo).
- Menu "Modifier avec WinNote" (registre) : code ecrit, jamais execute.
- Style macOS sur Windows 10 (teste seulement sur Windows 11).
- Multi-curseurs, signets, deplacer/trier les lignes, ouverture rapide, recherche dans le dossier : code ecrit, pas exerce par un test.

## A faire
Correcteur orthographique, backlinks (Obsidian), vue scindee, installeur, signature de code, CI de release, glisser un onglet hors de la fenetre,
coloration des ~95 autres langages Lexilla, tests e2e automatises de l'interface, optimisation memoire (~45 Mo prives, DirectWrite).
