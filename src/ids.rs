//! Identifiants de commandes (menus, accélérateurs, palette).
#![allow(dead_code)]
pub const NEW: usize = 100;
pub const OPEN: usize = 101;
pub const SAVE: usize = 102;
pub const SAVEAS: usize = 103;
pub const SAVEALL: usize = 104;
pub const CLOSE: usize = 105;
pub const EXIT: usize = 106;
pub const REOPEN: usize = 107;
pub const OPENFOLDER: usize = 108;
pub const UNDO: usize = 110;
pub const REDO: usize = 111;
pub const CUT: usize = 112;
pub const COPY: usize = 113;
pub const PASTE: usize = 114;
pub const DEL: usize = 115;
pub const SELALL: usize = 116;
pub const FIND: usize = 117;
pub const FINDNEXT: usize = 118;
pub const FINDPREV: usize = 119;
pub const REPLACE: usize = 120;
pub const DATETIME: usize = 121;
pub const DUPLINE: usize = 122;
pub const DELLINE: usize = 123;
pub const UPPER: usize = 124;
pub const LOWER: usize = 125;
pub const WRAP: usize = 130;
pub const FONT: usize = 131;
pub const EOL: usize = 132; // 132..134 : CRLF, CR, LF
pub const ZOOMIN: usize = 140;
pub const ZOOMOUT: usize = 141;
pub const ZOOMRESET: usize = 142;
pub const STATUSBAR: usize = 143;
pub const LINES: usize = 144;
pub const WS: usize = 145;
pub const TABNEXT: usize = 150;
pub const TABPREV: usize = 151;
pub const ENC: usize = 200; // 200..218
pub const RENC: usize = 230; // 230..248 : recharger avec l'encodage
pub const LANG: usize = 300; // 300..399
pub const THEME: usize = 400; // 400..499
pub const LAYOUT: usize = 500; // 500..599
pub const ABOUT: usize = 600;
pub const RECENT: usize = 700; // 700..719
pub const CLEARRECENT: usize = 720;
pub const MOVEUP: usize = 800;
pub const MOVEDOWN: usize = 801;
pub const SORTASC: usize = 802;
pub const SORTDESC: usize = 803;
pub const DEDUPE: usize = 804;
pub const TRIM: usize = 805;
pub const JOIN: usize = 806;
pub const TOGGLECOMMENT: usize = 807;
pub const SELNEXT: usize = 808;
pub const BOOKTOGGLE: usize = 809;
pub const BOOKNEXT: usize = 810;
pub const BOOKPREV: usize = 811;
pub const BOOKCLEAR: usize = 812;
pub const FOLDALL: usize = 813;
pub const UNFOLDALL: usize = 814;
pub const FOLDTOGGLE: usize = 815;
pub const ZEN: usize = 816;
pub const TOPMOST: usize = 817;
pub const READONLY: usize = 818;
pub const OPENLINK: usize = 819;
pub const TITLECASE: usize = 820;
pub const TABS2SPACES: usize = 821;
pub const SPACES2TABS: usize = 822;
pub const INDENT: usize = 823;
pub const OUTDENT: usize = 824;
pub const RELOAD: usize = 825;
pub const BROWSER: usize = 826;
pub const REVEALFILE: usize = 827;
pub const COPYPATH: usize = 828;
pub const CLOSEOTHERS: usize = 829;
pub const CLOSEALL: usize = 830;
pub const GOTO: usize = 831;
pub const PALETTE: usize = 832;
pub const QUICKOPEN: usize = 833;
pub const FINDINFILES: usize = 834;
pub const SIDEBAR: usize = 835;
pub const OUTLINE: usize = 836;
pub const PREVIEW: usize = 837;
pub const CHROME_WIN: usize = 838;
pub const CHROME_MAC: usize = 839;
pub const FOLDMARGIN: usize = 841;
pub const STATS: usize = 842;
pub const REGISTER: usize = 843;
pub const UNREGISTER: usize = 844;
pub const UPDATE: usize = 845;
pub const AUTOUPDATE: usize = 846;
pub const MENUPOPUP: usize = 850;
pub const SPACING: usize = 851;
pub const PRINT: usize = 852;
pub const NEWWINDOW: usize = 853;
pub const BACKLINKS: usize = 854;
pub const SPLIT: usize = 855;
pub const SPELL: usize = 856;
pub const DWRITE: usize = 857;
pub const FOLLOW: usize = 858;
pub const AUTOCOMP: usize = 859;
pub const INDENT_AUTO: usize = 880; // 880 auto, 881 = 2 espaces, 882 = 4, 883 = 8, 884 = tabulations
pub const KEYS: usize = 885;
pub const LOP: usize = 886; // 886..892 : tri par longueur, tri numerique, inverser, melanger, supprimer lignes vides, rogner debut, doublons consecutifs
pub const GUIDES: usize = 893;
pub const SHOWEOL: usize = 894;
pub const MACRO_REC: usize = 895;
pub const MACRO_PLAY: usize = 896;
pub const MACRO_PLAYN: usize = 897;
pub const COLINS: usize = 898;
pub const DAILY: usize = 899;
pub const CLOSELEFT: usize = 910;
pub const CLOSERIGHT: usize = 911;
pub const CLOSECLEAN: usize = 912;
pub const SMARTHL: usize = 913;
pub const CHGHIST: usize = 914;
pub const COPYNAME: usize = 915;
pub const COPYDIR: usize = 916;
pub const BLOCKCOMM: usize = 917;
pub const PINTAB: usize = 918;
pub const TAGS: usize = 919;
pub const PROPS: usize = 920;
pub const FAVTOGGLE: usize = 921;
pub const FAVS: usize = 922;
pub const SYMBOLS: usize = 923;
pub const SAVECOPY: usize = 924;
pub const PAGESETUP: usize = 925;
pub const RECOVER: usize = 926;
pub const BRACE: usize = 927;
pub const BRACESEL: usize = 928;
pub const TOC: usize = 929;
pub const REFLOW: usize = 931;
pub const CONVERT: usize = 932; // 932..935 : Base64 encoder / décoder, URL encoder / décoder
pub const SPELLLANG: usize = 860; // 860..864
pub const SPELLFIX: usize = 900; // 900..904
pub const SPELLADD: usize = 905;
pub const MD_BOLD: usize = 1000;
pub const MD_ITALIC: usize = 1001;
pub const MD_STRIKE: usize = 1002;
pub const MD_HILITE: usize = 1003;
pub const MD_CODE: usize = 1004;
pub const MD_QUOTE: usize = 1005;
pub const MD_BULLET: usize = 1006;
pub const MD_NUMLIST: usize = 1007;
pub const MD_TASK: usize = 1008;
pub const MD_LINK: usize = 1009;
pub const MD_WIKI: usize = 1010;
pub const MD_TABLE: usize = 1011;
pub const MD_CODEBLOCK: usize = 1012;
pub const MD_HR: usize = 1013;
pub const MD_CALLOUT: usize = 1014;
pub const MD_H1: usize = 1020; // 1020..1025
pub const SPELLIGNORE: usize = 906;

/// Commandes proposées dans la palette (Ctrl+Maj+P).
pub static PALETTE_CMDS: &[(usize, &str)] = &[
    (NEW, "Fichier : nouvel onglet"), (OPEN, "Fichier : ouvrir..."), (OPENFOLDER, "Fichier : ouvrir un dossier..."), (SAVE, "Fichier : enregistrer"),
    (SAVEAS, "Fichier : enregistrer sous..."), (SAVEALL, "Fichier : tout enregistrer"), (PRINT, "Fichier : imprimer..."), (NEWWINDOW, "Fichier : ouvrir dans une nouvelle fenêtre"), (BACKLINKS, "Affichage : liens entrants (backlinks)"), (SPLIT, "Affichage : vue scindée"), (SPELL, "Édition : correcteur orthographique"), (DWRITE, "Format : rendu DirectWrite (texte plus lisse, +35 Mo de mémoire)"), (CLOSE, "Fichier : fermer l'onglet"), (CLOSEOTHERS, "Fichier : fermer les autres onglets"),
    (CLOSEALL, "Fichier : fermer tous les onglets"), (REOPEN, "Fichier : rouvrir l'onglet fermé"), (RELOAD, "Fichier : recharger depuis le disque"), (FOLLOW, "Fichier : suivre la fin du fichier (tail -f)"),
    (REVEALFILE, "Fichier : afficher dans l'explorateur"), (COPYPATH, "Fichier : copier le chemin"), (BROWSER, "Fichier : ouvrir dans le navigateur"),
    (FIND, "Rechercher"), (REPLACE, "Remplacer"), (FINDINFILES, "Rechercher dans le dossier ouvert"), (GOTO, "Aller à la ligne..."), (QUICKOPEN, "Ouverture rapide (onglets, récents, dossier)"),
    (MOVEUP, "Lignes : déplacer vers le haut"), (MOVEDOWN, "Lignes : déplacer vers le bas"), (DUPLINE, "Lignes : dupliquer"), (DELLINE, "Lignes : supprimer"),
    (SORTASC, "Lignes : trier A → Z"), (SORTDESC, "Lignes : trier Z → A"), (DEDUPE, "Lignes : supprimer les doublons"), (JOIN, "Lignes : joindre"),
    (TRIM, "Texte : supprimer les espaces en fin de ligne"), (TOGGLECOMMENT, "Texte : commenter / décommenter"), (UPPER, "Texte : MAJUSCULES"), (LOWER, "Texte : minuscules"),
    (TITLECASE, "Texte : Première Lettre En Majuscule"), (TABS2SPACES, "Texte : tabulations → espaces"), (SPACES2TABS, "Texte : espaces → tabulations"), (DATETIME, "Insérer l'heure et la date"),
    (SELNEXT, "Sélection : ajouter l'occurrence suivante (multi-curseurs)"), (BOOKTOGGLE, "Signets : basculer"), (BOOKNEXT, "Signets : suivant"), (BOOKPREV, "Signets : précédent"), (BOOKCLEAR, "Signets : tout effacer"),
    (FOLDALL, "Repli : tout replier"), (UNFOLDALL, "Repli : tout déplier"), (FOLDMARGIN, "Affichage : marge de repli"),
    (WRAP, "Affichage : retour automatique à la ligne"), (LINES, "Affichage : numéros de ligne"), (WS, "Affichage : espaces visibles"), (STATUSBAR, "Affichage : barre d'état"),
    (SIDEBAR, "Affichage : explorateur de fichiers"), (OUTLINE, "Affichage : plan du document"), (PREVIEW, "Affichage : aperçu Markdown"), (ZEN, "Affichage : mode sans distraction (F11)"),
    (TOPMOST, "Affichage : toujours au premier plan"), (READONLY, "Affichage : lecture seule"), (ZOOMIN, "Zoom avant"), (ZOOMOUT, "Zoom arrière"), (ZOOMRESET, "Zoom par défaut"),
    (CHROME_WIN, "Style de fenêtre : Windows"), (CHROME_MAC, "Style de fenêtre : macOS (boutons à gauche)"), (FONT, "Format : police..."),
    (OPENLINK, "Ouvrir le lien / [[note]] sous le curseur"), (STATS, "Statistiques du document"), (REGISTER, "Intégrer à l'explorateur (Modifier avec WinNote)"), (UNREGISTER, "Retirer l'intégration à l'explorateur"),
    (MD_BOLD, "Markdown : gras"), (MD_ITALIC, "Markdown : italique"), (MD_STRIKE, "Markdown : barré"), (MD_HILITE, "Markdown : surligné"), (MD_CODE, "Markdown : code en ligne"),
    (MD_QUOTE, "Markdown : citation"), (MD_BULLET, "Markdown : liste à puces"), (MD_NUMLIST, "Markdown : liste numérotée"), (MD_TASK, "Markdown : liste de tâches"),
    (MD_LINK, "Markdown : lien"), (MD_WIKI, "Markdown : lien [[note]]"), (MD_TABLE, "Markdown : tableau"), (MD_CODEBLOCK, "Markdown : bloc de code"), (MD_HR, "Markdown : ligne horizontale"), (MD_CALLOUT, "Markdown : encadré (callout)"),
    (MD_H1, "Markdown : titre 1"), (MD_H1 + 1, "Markdown : titre 2"), (MD_H1 + 2, "Markdown : titre 3"), (MD_H1 + 3, "Markdown : titre 4"), (MD_H1 + 4, "Markdown : titre 5"), (MD_H1 + 5, "Markdown : titre 6"),
    (AUTOCOMP, "Format : autocomplétion des mots"), (INDENT_AUTO, "Format : indentation selon le langage"), (INDENT_AUTO + 1, "Format : indenter avec 2 espaces"), (INDENT_AUTO + 2, "Format : indenter avec 4 espaces"), (INDENT_AUTO + 3, "Format : indenter avec 8 espaces"), (INDENT_AUTO + 4, "Format : indenter avec des tabulations"), (KEYS, "Raccourcis clavier personnalisés..."), (LOP, "Lignes : trier par longueur"), (LOP + 1, "Lignes : trier par valeur numérique"), (LOP + 2, "Lignes : inverser l'ordre"), (LOP + 3, "Lignes : mélanger"), (LOP + 4, "Lignes : supprimer les lignes vides"), (LOP + 5, "Lignes : supprimer les espaces en début de ligne"), (LOP + 6, "Lignes : supprimer les doublons consécutifs"), (GUIDES, "Affichage : guides d'indentation"), (SHOWEOL, "Affichage : fins de ligne visibles"), (MACRO_REC, "Macro : démarrer / arrêter l'enregistrement (F9)"), (MACRO_PLAY, "Macro : rejouer (F10)"), (MACRO_PLAYN, "Macro : rejouer N fois (0 = jusqu'à la fin)"), (COLINS, "Édition : insérer en colonne (texte ou numéros)"), (DAILY, "Fichier : note du jour (dossier ouvert)"), (CLOSELEFT, "Fichier : fermer les onglets à gauche"), (CLOSERIGHT, "Fichier : fermer les onglets à droite"), (CLOSECLEAN, "Fichier : fermer les onglets non modifiés"), (SMARTHL, "Affichage : surligner les occurrences du mot sélectionné"), (CHGHIST, "Affichage : historique des modifications (marge)"), (COPYNAME, "Fichier : copier le nom du fichier"), (COPYDIR, "Fichier : copier le dossier"), (BLOCKCOMM, "Texte : commenter / décommenter en bloc"), (PINTAB, "Onglet : épingler / désépingler"), (TAGS, "Fichier : étiquettes #tag"), (PROPS, "Document : propriétés YAML (front matter)"), (FAVTOGGLE, "Favoris : ajouter / retirer le fichier"), (FAVS, "Favoris : ouvrir un favori"), (SYMBOLS, "Aller à une fonction / un titre du fichier"), (SAVECOPY, "Fichier : enregistrer une copie..."), (PAGESETUP, "Fichier : mise en page de l'impression..."), (RECOVER, "Fichier : restaurer les documents récupérés"), (TAGS, "Notes : étiquettes #tag du dossier"),
    (BRACE, "Aller au crochet correspondant"), (BRACESEL, "Sélectionner jusqu'au crochet correspondant"), (TOC, "Markdown : insérer le sommaire"), (REFLOW, "Texte : renvoyer les paragraphes à 80 colonnes"), (CONVERT, "Texte : encoder la sélection en Base64"), (CONVERT + 1, "Texte : décoder la sélection Base64"), (CONVERT + 2, "Texte : encoder la sélection pour une URL (%XX)"), (CONVERT + 3, "Texte : décoder la sélection URL (%XX)"), (UPDATE, "Rechercher une mise à jour"), (AUTOUPDATE, "Mises à jour automatiques : activer / couper"), (ABOUT, "À propos de WinNote"),
];
