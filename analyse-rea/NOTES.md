# Analyse REA (2026-11-02)

## Bloc-notes Windows 11 (Notepad.exe 11.2607.14, 3,3 Mo, x64, Ghidra 12.1.4 via REA)
- 8104 procédures, 5240 chaînes. UI = XAML (NotepadXamlUI.dll, ~90 .xbf) + RichEdit moderne (riched20.dll). Manifeste : PerMonitorV2.
- Fonctions observées (chaînes) : onglets (glisser-déposer entre fenêtres, lot de création), sessions + autosave, fichiers récents, zoom (in/out/restaurer, % en barre d'état), retour à la ligne, Rechercher/Remplacer (boucle `FindWrapAround`), Aller à, barre d'état (fins de ligne CRLF/LF/CR, encodage, zoom), encodages ANSI / UTF-8 / UTF-8 avec BOM / UTF-16, avertissement « enregistrer Unicode en ANSI », mise en forme Markdown (gras/italique/liens/listes/tableaux insérer-supprimer ligne-colonne), dossier d'images Markdown, réglages (thème, police, retour ligne, correcteur, fichiers récents, ouverture).
- Hors périmètre : Cowriter (IA payante, règle « zéro service payant »).

## Notepad++ 8.9.8 (notepad++.exe 8,5 Mo, Scintilla + Lexilla)
- 22 thèmes (themes/), langs.model.xml + stylers.model.xml, 48 functionList (liste de fonctions par langage), 34 fichiers d'autocomplétion, plugins, localization.
- Licence GPL : ne PAS copier son code ; Scintilla/Lexilla (licence permissive type HPND) sont réutilisables.
