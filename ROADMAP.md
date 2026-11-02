# WinNote - etat v0.6.0 (2026-11-02)

Depot PUBLIC (necessaire a la mise a jour automatique) : https://github.com/Heiphaistos/WinNote/releases
Licence : PolyForm Noncommercial 1.0.0 (code ouvert, pas de revente).

## Verifie en vrai
- Windows : autotest integre `winnote.exe --selftest` (56 controles), mise a jour automatique de bout en bout depuis GitHub, impression vers PDF, menu de l'explorateur, installeur silencieux.
- Linux (GTK3) : autotest `winnote-gtk --selftest` sous Xvfb (44 controles : edition, recherche regex, encodages, plan, liens entrants, apercu Markdown, vue scindee,
  orthographe Enchant, impression PDF, mise a jour simulee avec empreinte), paquets .deb/.rpm/.tar.gz/AppImage installes et lances, captures regardees.
- macOS : compilation, empaquetage .app/.dmg et autotest dans la CI GitHub (macos-14 et macos-13). Pas essaye sur un vrai Mac.

## Limites connues
- Linux/macOS : pas de style de barre de titre macOS ni de menu de l'explorateur (non applicable) ; l'ouverture d'un fichier depuis le Finder passe par la ligne de commande seulement.
- Linux ne teste que Ubuntu 22.04 (glibc 2.35 minimum) ; le PKGBUILD Arch n'a pas ete execute sur Arch.
- macOS : application signee en ad hoc, non notarisee (Gatekeeper : clic droit > Ouvrir au premier lancement).
- Impression testee vers PDF, pas sur une imprimante physique.
- Executables Windows signes avec un certificat auto-signe (SmartScreen avertit hors des machines qui lui font confiance).

## A faire
Glisser un onglet hors de la fenetre, coloration des ~80 autres langages Lexilla, notarisation macOS (compte developpeur Apple), single-instance sous Linux/macOS.
