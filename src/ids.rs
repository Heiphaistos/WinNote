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
pub const ENC: usize = 200; // 200..204
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
pub const SPELLLANG: usize = 860; // 860..864
pub const SPELLFIX: usize = 900; // 900..904
pub const SPELLADD: usize = 905;
pub const SPELLIGNORE: usize = 906;

/// Commandes proposées dans la palette (Ctrl+Maj+P).
pub static PALETTE_CMDS: &[(usize, &str)] = &[
    (NEW, "Fichier : nouvel onglet"), (OPEN, "Fichier : ouvrir..."), (OPENFOLDER, "Fichier : ouvrir un dossier..."), (SAVE, "Fichier : enregistrer"),
    (SAVEAS, "Fichier : enregistrer sous..."), (SAVEALL, "Fichier : tout enregistrer"), (PRINT, "Fichier : imprimer..."), (NEWWINDOW, "Fichier : ouvrir dans une nouvelle fenêtre"), (BACKLINKS, "Affichage : liens entrants (backlinks)"), (SPLIT, "Affichage : vue scindée"), (SPELL, "Édition : correcteur orthographique"), (DWRITE, "Format : rendu DirectWrite (texte plus lisse, +35 Mo de mémoire)"), (CLOSE, "Fichier : fermer l'onglet"), (CLOSEOTHERS, "Fichier : fermer les autres onglets"),
    (CLOSEALL, "Fichier : fermer tous les onglets"), (REOPEN, "Fichier : rouvrir l'onglet fermé"), (RELOAD, "Fichier : recharger depuis le disque"),
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
    (UPDATE, "Rechercher une mise à jour"), (AUTOUPDATE, "Mises à jour automatiques : activer / couper"), (ABOUT, "À propos de WinNote"),
];
