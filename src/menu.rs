//! Menus, accélérateurs, menus contextuels.
use crate::doc::Enc;
use crate::ids::*;
use crate::lang::LANGS;
use crate::theme::{LAYOUTS, THEMES};
use crate::{app, w, App};
use std::ptr::null;
use windows_sys::Win32::{UI::Input::KeyboardAndMouse::*, UI::WindowsAndMessaging::*};

pub struct Menus {
    pub bar: HMENU,
    pub popup: HMENU, // même contenu, utilisé quand la fenêtre n'a pas de barre native (style macOS)
    pub recent: HMENU,
    pub enc: HMENU,
    pub lang: HMENU,
    pub eol: HMENU,
}

fn add(m: HMENU, id: usize, t: &str) {
    unsafe { AppendMenuW(m, MF_STRING, id, w(t).as_ptr()) };
}
fn sep(m: HMENU) {
    unsafe { AppendMenuW(m, MF_SEPARATOR, 0, null()) };
}
fn sub(parents: &[HMENU], name: &str) -> HMENU {
    unsafe {
        let m = CreatePopupMenu();
        for &p in parents {
            AppendMenuW(p, MF_POPUP, m as usize, w(name).as_ptr());
        }
        m
    }
}

pub fn build() -> Menus {
    unsafe {
        let (bar, pop) = (CreateMenu(), CreatePopupMenu());
        let both = [bar, pop];
        let f = sub(&both, "Fichier");
        add(f, NEW, "Nouvel onglet\tCtrl+N");
        add(f, OPEN, "Ouvrir...\tCtrl+O");
        add(f, OPENFOLDER, "Ouvrir un dossier...");
        let recent = sub(&[f], "Fichiers récents");
        add(f, REOPEN, "Rouvrir l'onglet fermé\tCtrl+Maj+T");
        sep(f);
        add(f, SAVE, "Enregistrer\tCtrl+S");
        add(f, SAVEAS, "Enregistrer sous...\tCtrl+Maj+S");
        add(f, SAVEALL, "Tout enregistrer\tCtrl+Alt+S");
        add(f, PRINT, "Imprimer...\tCtrl+P");
        add(f, RELOAD, "Recharger depuis le disque");
        add(f, DAILY, "Note du jour");
        add(f, FOLLOW, "Suivre la fin du fichier (tail -f)");
        sep(f);
        add(f, REVEALFILE, "Afficher dans l'explorateur");
        add(f, COPYPATH, "Copier le chemin");
        add(f, COPYNAME, "Copier le nom du fichier");
        add(f, COPYDIR, "Copier le dossier");
        add(f, BROWSER, "Ouvrir dans le navigateur");
        sep(f);
        add(f, CLOSE, "Fermer l'onglet\tCtrl+W");
        add(f, TAGS, "Étiquettes #tag du dossier...");
        add(f, PINTAB, "Épingler / désépingler l'onglet");
        add(f, CLOSEOTHERS, "Fermer les autres onglets");
        add(f, CLOSELEFT, "Fermer les onglets à gauche");
        add(f, CLOSERIGHT, "Fermer les onglets à droite");
        add(f, CLOSECLEAN, "Fermer les onglets non modifiés");
        add(f, CLOSEALL, "Fermer tous les onglets");
        add(f, EXIT, "Quitter");

        let e = sub(&both, "Édition");
        add(e, UNDO, "Annuler\tCtrl+Z");
        add(e, REDO, "Rétablir\tCtrl+Y");
        sep(e);
        add(e, CUT, "Couper\tCtrl+X");
        add(e, COPY, "Copier\tCtrl+C");
        add(e, PASTE, "Coller\tCtrl+V");
        sep(e);
        add(e, COLINS, "Insérer en colonne...\tAlt+C");
        let mc = sub(&[e], "Macro");
        add(mc, MACRO_REC, "Démarrer / arrêter l'enregistrement\tF9");
        add(mc, MACRO_PLAY, "Rejouer\tF10");
        add(mc, MACRO_PLAYN, "Rejouer N fois...");
        add(e, DEL, "Supprimer\tSuppr");
        add(e, SELALL, "Tout sélectionner\tCtrl+A");
        sep(e);
        add(e, FIND, "Rechercher...\tCtrl+F");
        add(e, FINDNEXT, "Suivant\tF3");
        add(e, FINDPREV, "Précédent\tMaj+F3");
        add(e, REPLACE, "Remplacer...\tCtrl+H");
        add(e, FINDINFILES, "Rechercher dans le dossier...\tCtrl+Maj+F");
        add(e, GOTO, "Aller à la ligne...\tCtrl+G");
        sep(e);
        add(e, SELNEXT, "Ajouter l'occurrence suivante\tAlt+D");
        add(e, OPENLINK, "Ouvrir le lien sous le curseur\tCtrl+Entrée");
        add(e, DATETIME, "Heure et date\tF5");
        sep(e);
        add(e, SPELL, "Correcteur orthographique");
        let sp = sub(&[e], "Langue du correcteur");
        for (i, (_, name)) in crate::spell::LANGS.iter().enumerate() {
            add(sp, SPELLLANG + i, name);
        }

        let l = sub(&both, "Lignes");
        add(l, MOVEUP, "Déplacer vers le haut\tAlt+↑");
        add(l, MOVEDOWN, "Déplacer vers le bas\tAlt+↓");
        add(l, DUPLINE, "Dupliquer\tCtrl+D");
        add(l, DELLINE, "Supprimer\tCtrl+Maj+K");
        add(l, JOIN, "Joindre\tCtrl+J");
        sep(l);
        add(l, SORTASC, "Trier A → Z");
        add(l, SORTDESC, "Trier Z → A");
        add(l, DEDUPE, "Supprimer les doublons");
        for (i, t) in ["Trier par longueur", "Trier par valeur numérique", "Inverser l'ordre", "Mélanger", "Supprimer les lignes vides", "Supprimer les espaces en début de ligne", "Supprimer les doublons consécutifs"].iter().enumerate() {
            add(l, LOP + i, t);
        }
        add(l, TRIM, "Supprimer les espaces en fin de ligne");
        sep(l);
        add(l, TOGGLECOMMENT, "Commenter / décommenter\tCtrl+/");
        add(l, BLOCKCOMM, "Commenter en bloc");
        add(l, UPPER, "EN MAJUSCULES\tCtrl+Maj+U");
        add(l, LOWER, "en minuscules\tCtrl+U");
        add(l, TITLECASE, "Première Lettre En Majuscule");
        add(l, TABS2SPACES, "Tabulations → espaces");
        add(l, SPACES2TABS, "Espaces → tabulations");

        let fm = sub(&both, "Format");
        add(fm, WRAP, "Retour automatique à la ligne");
        add(fm, FONT, "Police...");
        add(fm, DWRITE, "Rendu DirectWrite (plus lisse, +35 Mo de mémoire)");
        let eol = sub(&[fm], "Fins de ligne");
        add(eol, EOL, "Windows (CRLF)");
        add(eol, EOL + 1, "Macintosh (CR)");
        add(eol, EOL + 2, "Unix (LF)");

        add(fm, AUTOCOMP, "Autocomplétion des mots (code)");
        let ind = sub(&[fm], "Indentation");
        for (i, t) in ["Selon le langage", "2 espaces", "4 espaces", "8 espaces", "Tabulations"].iter().enumerate() {
            add(ind, INDENT_AUTO + i, t);
        }
        add(fm, KEYS, "Raccourcis clavier personnalisés...");

        let md = sub(&[fm], "Markdown");
        add(md, MD_BOLD, "Gras	Ctrl+B");
        add(md, MD_ITALIC, "Italique	Ctrl+I");
        add(md, MD_STRIKE, "Barré");
        add(md, MD_HILITE, "Surligné");
        add(md, MD_CODE, "Code en ligne");
        sep(md);
        add(md, MD_H1, "Titre 1");
        add(md, MD_H1 + 1, "Titre 2");
        add(md, MD_H1 + 2, "Titre 3");
        add(md, MD_H1 + 3, "Titre 4");
        add(md, MD_H1 + 4, "Titre 5");
        add(md, MD_H1 + 5, "Titre 6");
        sep(md);
        add(md, MD_QUOTE, "Citation");
        add(md, MD_BULLET, "Liste à puces");
        add(md, MD_NUMLIST, "Liste numérotée");
        add(md, MD_TASK, "Liste de tâches");
        sep(md);
        add(md, MD_LINK, "Lien	Ctrl+K");
        add(md, MD_WIKI, "Lien [[note]]");
        add(md, MD_TABLE, "Tableau");
        add(md, MD_CODEBLOCK, "Bloc de code");
        add(md, MD_HR, "Ligne horizontale");
        add(md, MD_CALLOUT, "Encadré (callout)");

        let v = sub(&both, "Affichage");
        add(v, ZOOMIN, "Zoom avant\tCtrl++");
        add(v, ZOOMOUT, "Zoom arrière\tCtrl+-");
        add(v, ZOOMRESET, "Zoom par défaut\tCtrl+0");
        sep(v);
        add(v, SIDEBAR, "Explorateur de fichiers\tCtrl+Maj+E");
        add(v, OUTLINE, "Plan du document\tCtrl+Maj+O");
        add(v, PREVIEW, "Aperçu Markdown\tCtrl+Maj+M");
        add(v, SPLIT, "Vue scindée\tCtrl+\\");
        add(v, BACKLINKS, "Liens entrants (backlinks)");
        add(v, LINES, "Numéros de ligne");
        add(v, FOLDMARGIN, "Marge de repli");
        add(v, WS, "Espaces et tabulations visibles");
        add(v, STATUSBAR, "Barre d'état");
        add(v, ZEN, "Mode sans distraction\tF11");
        add(v, TOPMOST, "Toujours au premier plan");
        add(v, READONLY, "Lecture seule");
        add(v, GUIDES, "Guides d'indentation");
        add(v, SMARTHL, "Surligner les occurrences du mot");
        add(v, CHGHIST, "Historique des modifications (marge)");
        add(v, SHOWEOL, "Fins de ligne visibles");
        sep(v);
        add(v, BOOKTOGGLE, "Signet : basculer\tCtrl+F2");
        add(v, BOOKNEXT, "Signet suivant\tF2");
        add(v, BOOKPREV, "Signet précédent\tMaj+F2");
        add(v, BOOKCLEAR, "Effacer les signets");
        add(v, FOLDALL, "Tout replier");
        add(v, UNFOLDALL, "Tout déplier");
        sep(v);
        add(v, TABNEXT, "Onglet suivant\tCtrl+Tab");
        add(v, TABPREV, "Onglet précédent\tCtrl+Maj+Tab");
        add(v, PALETTE, "Palette de commandes\tCtrl+Maj+P");
        add(v, QUICKOPEN, "Ouverture rapide\tCtrl+E");

        let th = sub(&both, "Thème");
        for (i, t) in THEMES.iter().enumerate() {
            add(th, THEME + i, t.name);
        }
        let ds = sub(&both, "Disposition");
        for (i, l) in LAYOUTS.iter().enumerate() {
            add(ds, LAYOUT + i, l.name);
        }
        sep(ds);
        add(ds, CHROME_WIN, "Fenêtre : style Windows");
        add(ds, CHROME_MAC, "Fenêtre : style macOS (boutons à gauche)");

        let lg = sub(&both, "Langage");
        for (i, la) in LANGS.iter().enumerate() {
            add(lg, LANG + i, la.name);
        }
        let en = sub(&both, "Encodage");
        for (i, e) in Enc::ALL.iter().enumerate().take(5) {
            add(en, ENC + i, e.name());
        }
        let other = sub(&[en], "Autres encodages");
        for (i, e) in Enc::ALL.iter().enumerate().skip(5) {
            add(other, ENC + i, e.name());
        }
        sep(en);
        let re = sub(&[en], "Recharger avec l'encodage");
        for (i, e) in Enc::ALL.iter().enumerate() {
            add(re, RENC + i, e.name());
        }
        let h = sub(&both, "?");
        add(h, STATS, "Statistiques du document");
        add(h, UPDATE, "Rechercher une mise à jour");
        add(h, AUTOUPDATE, "Mises à jour automatiques");
        add(h, REGISTER, "Intégrer à l'explorateur (Modifier avec WinNote)");
        add(h, UNREGISTER, "Retirer l'intégration à l'explorateur");
        add(h, ABOUT, "À propos de WinNote");
        Menus { bar, popup: pop, recent, enc: en, lang: lg, eol }
    }
}

pub fn rebuild_recent(a: &App) {
    unsafe {
        let m = a.menus.recent;
        while DeleteMenu(m, 0, MF_BYPOSITION) != 0 {}
        for (i, p) in a.recent.iter().enumerate().take(15) {
            add(m, RECENT + i, &format!("{}  {}", i + 1, p.replace('&', "&&")));
        }
        if a.recent.is_empty() {
            AppendMenuW(m, MF_STRING | MF_GRAYED, 0, w("(aucun)").as_ptr());
        } else {
            sep(m);
            add(m, CLEARRECENT, "Effacer la liste");
        }
    }
}

pub fn build_accel() -> HACCEL {
    let c = (FVIRTKEY | FCONTROL) as u8;
    let cs = c | FSHIFT as u8;
    let al = (FVIRTKEY | FALT) as u8;
    let k = |virt: u8, key: u16, cmd: usize| ACCEL { fVirt: virt, key, cmd: cmd as u16 };
    let ch = |x: char| x as u16;
    let mut t: Vec<ACCEL> = crate::keymap::user()
        .iter()
        .filter_map(|(ch, id)| crate::keymap::vk(&ch.key).map(|key| ACCEL { fVirt: FVIRTKEY as u8 | if ch.ctrl { FCONTROL as u8 } else { 0 } | if ch.shift { FSHIFT as u8 } else { 0 } | if ch.alt { FALT as u8 } else { 0 }, key, cmd: *id as u16 }))
        .collect();
    t.extend([
        k(c, ch('N'), NEW), k(c, ch('T'), NEW), k(c, ch('O'), OPEN), k(c, ch('S'), SAVE), k(cs, ch('S'), SAVEAS), k(c | FALT as u8, ch('S'), SAVEALL),
        k(c, ch('W'), CLOSE), k(cs, ch('T'), REOPEN), k(c, ch('F'), FIND), k(c, ch('H'), REPLACE), k(cs, ch('F'), FINDINFILES), k(c, ch('G'), GOTO),
        k(FVIRTKEY as u8, VK_F3, FINDNEXT), k((FVIRTKEY | FSHIFT) as u8, VK_F3, FINDPREV), k(FVIRTKEY as u8, VK_F5, DATETIME), k(FVIRTKEY as u8, 0x78, MACRO_REC), k(FVIRTKEY as u8, 0x79, MACRO_PLAY), k(al, ch('C'), COLINS), k(FVIRTKEY as u8, VK_F11, ZEN),
        k(c, ch('B'), MD_BOLD), k(c, ch('I'), MD_ITALIC), k(c, ch('K'), MD_LINK), k(c, ch('D'), DUPLINE), k(cs, ch('K'), DELLINE), k(cs, ch('U'), UPPER), k(c, ch('U'), LOWER), k(c, ch('J'), JOIN), k(c, 0xBF, TOGGLECOMMENT),
        k(al, ch('D'), SELNEXT), k(al, VK_UP, MOVEUP), k(al, VK_DOWN, MOVEDOWN), k(c, VK_RETURN, OPENLINK),
        k(FVIRTKEY as u8, VK_F2, BOOKNEXT), k((FVIRTKEY | FSHIFT) as u8, VK_F2, BOOKPREV), k(c, VK_F2, BOOKTOGGLE),
        k(c, VK_TAB, TABNEXT), k(cs, VK_TAB, TABPREV), k(c, 0xBB, ZOOMIN), k(c, 0xBD, ZOOMOUT), k(c, ch('0'), ZOOMRESET),
        k(cs, ch('P'), PALETTE), k(c, ch('P'), PRINT), k(c, ch('E'), QUICKOPEN), k(cs, ch('E'), SIDEBAR), k(cs, ch('O'), OUTLINE), k(cs, ch('M'), PREVIEW), k(c, 0xDC, SPLIT),
    ]);
    unsafe { CreateAcceleratorTableW(t.as_ptr(), t.len() as i32) }
}

pub fn popup_at(menu: HMENU, x: i32, y: i32) {
    let a = app();
    unsafe {
        SetForegroundWindow(a.hwnd);
        TrackPopupMenu(menu, TPM_LEFTALIGN | TPM_TOPALIGN | TPM_RIGHTBUTTON, x, y, 0, a.hwnd, null());
    }
}

/// Menu contextuel de l'éditeur (en français).
pub fn editor_context(x: i32, y: i32) {
    unsafe {
        let m = CreatePopupMenu();
        let a = app();
        let mut p = windows_sys::Win32::Foundation::POINT { x, y };
        windows_sys::Win32::Graphics::Gdi::ScreenToClient(a.sci, &mut p);
        let pos = crate::sci::send(a.sci, 2023, p.x as usize, p.y as isize); // SCI_POSITIONFROMPOINTCLOSE
        let sug = if pos >= 0 { crate::spell::prepare_at(pos) } else { Vec::new() };
        #[allow(static_mut_refs)]
        if crate::spell::PENDING.is_some() {
            for (i, s) in sug.iter().enumerate() {
                add(m, SPELLFIX + i, &s.replace('&', "&&"));
            }
            if sug.is_empty() {
                AppendMenuW(m, MF_STRING | MF_GRAYED, 0, w("(aucune suggestion)").as_ptr());
            }
            add(m, SPELLADD, "Ajouter au dictionnaire");
            add(m, SPELLIGNORE, "Ignorer");
            sep(m);
        }
        for (id, t) in [(UNDO, "Annuler"), (REDO, "Rétablir"), (0, ""), (CUT, "Couper"), (COPY, "Copier"), (PASTE, "Coller"), (DEL, "Supprimer"), (0, ""), (SELALL, "Tout sélectionner"),
            (0, ""), (FIND, "Rechercher..."), (TOGGLECOMMENT, "Commenter / décommenter"), (OPENLINK, "Ouvrir le lien"), (PALETTE, "Palette de commandes...")] {
            if id == 0 { sep(m) } else { add(m, id, t) }
        }
        popup_at(m, x, y);
        DestroyMenu(m);
    }
}

/// Menu contextuel d'un onglet.
pub fn tab_context(x: i32, y: i32) {
    unsafe {
        let m = CreatePopupMenu();
        for (id, t) in [(CLOSE, "Fermer"), (PINTAB, "Épingler / désépingler"), (NEWWINDOW, "Ouvrir dans une nouvelle fenêtre"), (CLOSEOTHERS, "Fermer les autres"), (CLOSELEFT, "Fermer à gauche"), (CLOSERIGHT, "Fermer à droite"), (CLOSEALL, "Fermer tous"), (0, ""), (SAVE, "Enregistrer"), (SAVEAS, "Enregistrer sous..."),
            (0, ""), (COPYPATH, "Copier le chemin"), (REVEALFILE, "Afficher dans l'explorateur"), (REOPEN, "Rouvrir l'onglet fermé")] {
            if id == 0 { sep(m) } else { add(m, id, t) }
        }
        popup_at(m, x, y);
        DestroyMenu(m);
    }
}
