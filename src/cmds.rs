//! Commandes : menus, documents (onglets), fichiers, recherche, apparence.
use crate::doc::{self, Doc, Enc};
use crate::lang::{self, LANGS};
use crate::sci::{self, send};
use crate::theme::{LAYOUTS, THEMES};
use crate::{app, cstr, relayout, w, App};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Dwm::DwmSetWindowAttribute,
    Graphics::Gdi::{InvalidateRect, LOGFONTW},
    System::{LibraryLoader::*, SystemInformation::GetLocalTime},
    UI::{Controls::Dialogs::*, Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

// Identifiants de commandes
const NEW: usize = 100;
const OPEN: usize = 101;
const SAVE: usize = 102;
const SAVEAS: usize = 103;
const SAVEALL: usize = 104;
const CLOSE: usize = 105;
const EXIT: usize = 106;
const UNDO: usize = 110;
const REDO: usize = 111;
const CUT: usize = 112;
const COPY: usize = 113;
const PASTE: usize = 114;
const DEL: usize = 115;
const SELALL: usize = 116;
const FIND: usize = 117;
const FINDNEXT: usize = 118;
const FINDPREV: usize = 119;
const REPLACE: usize = 120;
const DATETIME: usize = 121;
const DUPLINE: usize = 122;
const DELLINE: usize = 123;
const UPPER: usize = 124;
const LOWER: usize = 125;
const WRAP: usize = 130;
const FONT: usize = 131;
const EOL: usize = 132; // 132..134 : CRLF, CR, LF
const ZOOMIN: usize = 140;
const ZOOMOUT: usize = 141;
const ZOOMRESET: usize = 142;
const STATUSBAR: usize = 143;
const LINES: usize = 144;
const WS: usize = 145;
const TABNEXT: usize = 150;
const TABPREV: usize = 151;
const ENC: usize = 200;
const LANG: usize = 300;
const THEME: usize = 400;
const LAYOUT: usize = 500;
const ABOUT: usize = 600;

pub fn msg_box(s: &str) {
    unsafe { MessageBoxW(app().hwnd, w(s).as_ptr(), w("WinNote").as_ptr(), MB_OK | MB_ICONINFORMATION) };
}

fn add(m: HMENU, id: usize, t: &str) {
    unsafe { AppendMenuW(m, MF_STRING, id, w(t).as_ptr()) };
}
fn sep(m: HMENU) {
    unsafe { AppendMenuW(m, MF_SEPARATOR, 0, null()) };
}
fn sub(parent: HMENU, name: &str) -> HMENU {
    unsafe {
        let m = CreatePopupMenu();
        AppendMenuW(parent, MF_POPUP, m as usize, w(name).as_ptr());
        m
    }
}

pub fn build_menu() -> HMENU {
    unsafe {
        let bar = CreateMenu();
        let f = sub(bar, "Fichier");
        add(f, NEW, "Nouvel onglet\tCtrl+N");
        add(f, OPEN, "Ouvrir...\tCtrl+O");
        sep(f);
        add(f, SAVE, "Enregistrer\tCtrl+S");
        add(f, SAVEAS, "Enregistrer sous...\tCtrl+Maj+S");
        add(f, SAVEALL, "Tout enregistrer\tCtrl+Alt+S");
        sep(f);
        add(f, CLOSE, "Fermer l'onglet\tCtrl+W");
        add(f, EXIT, "Quitter");
        let e = sub(bar, "Édition");
        add(e, UNDO, "Annuler\tCtrl+Z");
        add(e, REDO, "Rétablir\tCtrl+Y");
        sep(e);
        add(e, CUT, "Couper\tCtrl+X");
        add(e, COPY, "Copier\tCtrl+C");
        add(e, PASTE, "Coller\tCtrl+V");
        add(e, DEL, "Supprimer\tSuppr");
        sep(e);
        add(e, FIND, "Rechercher...\tCtrl+F");
        add(e, FINDNEXT, "Suivant\tF3");
        add(e, FINDPREV, "Précédent\tMaj+F3");
        add(e, REPLACE, "Remplacer...\tCtrl+H");
        sep(e);
        add(e, DUPLINE, "Dupliquer la ligne\tCtrl+D");
        add(e, DELLINE, "Supprimer la ligne\tCtrl+Maj+K");
        add(e, UPPER, "EN MAJUSCULES\tCtrl+Maj+U");
        add(e, LOWER, "en minuscules\tCtrl+U");
        add(e, DATETIME, "Heure et date\tF5");
        add(e, SELALL, "Tout sélectionner\tCtrl+A");
        let fm = sub(bar, "Format");
        add(fm, WRAP, "Retour automatique à la ligne");
        add(fm, FONT, "Police...");
        let eol = sub(fm, "Fins de ligne");
        add(eol, EOL, "Windows (CRLF)");
        add(eol, EOL + 1, "Macintosh (CR)");
        add(eol, EOL + 2, "Unix (LF)");
        let v = sub(bar, "Affichage");
        add(v, ZOOMIN, "Zoom avant\tCtrl++");
        add(v, ZOOMOUT, "Zoom arrière\tCtrl+-");
        add(v, ZOOMRESET, "Zoom par défaut\tCtrl+0");
        sep(v);
        add(v, LINES, "Numéros de ligne");
        add(v, WS, "Afficher les espaces et tabulations");
        add(v, STATUSBAR, "Barre d'état");
        sep(v);
        add(v, TABNEXT, "Onglet suivant\tCtrl+Tab");
        add(v, TABPREV, "Onglet précédent\tCtrl+Maj+Tab");
        let th = sub(bar, "Thème");
        for (i, t) in THEMES.iter().enumerate() {
            add(th, THEME + i, t.name);
        }
        sep(th);
        for (i, l) in LAYOUTS.iter().enumerate() {
            add(th, LAYOUT + i, &format!("Disposition : {}", l.name));
        }
        let lg = sub(bar, "Langage");
        for (i, l) in LANGS.iter().enumerate() {
            add(lg, LANG + i, l.name);
        }
        let en = sub(bar, "Encodage");
        for (i, e) in Enc::ALL.iter().enumerate() {
            add(en, ENC + i, e.name());
        }
        let h = sub(bar, "?");
        add(h, ABOUT, "À propos de WinNote");
        bar
    }
}

pub fn build_accel() -> HACCEL {
    let c = (FVIRTKEY | FCONTROL) as u8;
    let cs = c | FSHIFT as u8;
    let k = |virt: u8, key: u16, cmd: usize| ACCEL { fVirt: virt, key, cmd: cmd as u16 };
    let t = [
        k(c, b'N' as u16, NEW), k(c, b'T' as u16, NEW), k(c, b'O' as u16, OPEN), k(c, b'S' as u16, SAVE), k(cs, b'S' as u16, SAVEAS),
        k(c | FALT as u8, b'S' as u16, SAVEALL), k(c, b'W' as u16, CLOSE), k(c, b'F' as u16, FIND), k(c, b'H' as u16, REPLACE),
        k(FVIRTKEY as u8, VK_F3 as u16, FINDNEXT), k((FVIRTKEY | FSHIFT) as u8, VK_F3 as u16, FINDPREV), k(FVIRTKEY as u8, VK_F5 as u16, DATETIME),
        k(c, b'D' as u16, DUPLINE), k(cs, b'K' as u16, DELLINE), k(cs, b'U' as u16, UPPER), k(c, b'U' as u16, LOWER),
        k(c, VK_TAB as u16, TABNEXT), k(cs, VK_TAB as u16, TABPREV), k(c, 0xBB, ZOOMIN), k(c, 0xBD, ZOOMOUT), k(c, b'0' as u16, ZOOMRESET),
    ];
    unsafe { CreateAcceleratorTableW(t.as_ptr(), t.len() as i32) }
}

// ---------- Apparence ----------

fn set_lexer(a: &App, lang: usize) {
    let l = &LANGS[lang];
    unsafe {
        if l.lexer.is_empty() {
            send(a.sci, sci::SETILEXER, 0, 0);
        } else {
            let p = crate::sci::CreateLexer(cstr(l.lexer).as_ptr());
            send(a.sci, sci::SETILEXER, 0, p as isize);
            for (i, k) in l.kw.iter().enumerate() {
                send(a.sci, sci::SETKEYWORDS, i, cstr(k).as_ptr() as isize);
            }
        }
    }
    send(a.sci, sci::COLOURISE, 0, -1);
}

fn style_editor(a: &App) {
    let (t, s) = (&THEMES[a.theme], a.sci);
    let sc = |v: i32| (v * a.dpi as i32 / 96) as isize;
    send(s, sci::STYLESETFONT, sci::STYLE_DEFAULT, cstr(&a.font).as_ptr() as isize);
    send(s, sci::STYLESETSIZE, sci::STYLE_DEFAULT, a.size as isize);
    send(s, sci::STYLESETFORE, sci::STYLE_DEFAULT, t.fg as isize);
    send(s, sci::STYLESETBACK, sci::STYLE_DEFAULT, t.bg as isize);
    send(s, sci::STYLECLEARALL, 0, 0);
    send(s, sci::STYLESETFORE, sci::STYLE_LINENUM, t.gutter as isize);
    send(s, sci::STYLESETBACK, sci::STYLE_LINENUM, t.bg as isize);
    send(s, sci::STYLESETFORE, sci::STYLE_INDENT, t.line as isize);
    send(s, sci::SETSELBACK, 1, t.sel as isize);
    send(s, sci::SETCARETFORE, t.fg as usize, 0);
    send(s, sci::SETCARETW, sc(2) as usize, 0);
    send(s, sci::SETCARETLINEVIS, 1, 0);
    send(s, sci::SETCARETLINEBACK, t.line as usize, 0);
    send(s, sci::SETWSFORE, 1, t.gutter as isize);
    if let Some(d) = a.docs.get(a.cur) {
        for &(st, cls) in LANGS[d.lang].map {
            send(s, sci::STYLESETFORE, st as usize, t.color(cls) as isize);
            send(s, sci::STYLESETBOLD, st as usize, matches!(cls, crate::theme::Cls::Head | crate::theme::Cls::Strong) as isize);
            send(s, sci::STYLESETITALIC, st as usize, matches!(cls, crate::theme::Cls::Emph) as isize);
        }
    }
    send(s, sci::SETMARGINLEFT, 0, sc(a.margin));
    send(s, sci::SETMARGINRIGHT, 0, sc(a.margin));
    send(s, sci::SETMARGINTYPE, 0, 1);
    let w0 = if a.lines { send(s, sci::TEXTWIDTH, sci::STYLE_LINENUM, cstr("_99999").as_ptr() as isize) } else { 0 };
    send(s, sci::SETMARGINW, 0, w0);
    for m in 1..3 {
        send(s, sci::SETMARGINW, m, 0);
    }
    send(s, sci::SETWRAP, a.wrap as usize, 0);
    send(s, sci::SETVIEWWS, a.ws as usize, 0);
    send(s, sci::SETINDGUIDES, if a.lines { 3 } else { 0 }, 0);
}

fn dwm_and_menus(a: &App) {
    let t = &THEMES[a.theme];
    unsafe {
        let set = |attr: u32, v: u32| {
            DwmSetWindowAttribute(a.hwnd, attr, &v as *const u32 as *const _, 4);
        };
        set(20, t.dark as u32); // mode sombre de la barre de titre
        set(35, t.ui_bg); // couleur de la barre de titre (Windows 11)
        set(36, t.ui_fg);
        set(34, t.ui_bg);
        // menus contextuels sombres/clairs (ordinaux uxtheme 135 SetPreferredAppMode, 136 FlushMenuThemes)
        let ux = LoadLibraryW(w("uxtheme.dll").as_ptr());
        if !ux.is_null() {
            if let Some(f) = GetProcAddress(ux, 135usize as *const u8) {
                let f: extern "system" fn(i32) -> i32 = std::mem::transmute(f);
                f(if t.dark { 2 } else { 3 });
            }
            if let Some(f) = GetProcAddress(ux, 136usize as *const u8) {
                let f: extern "system" fn() = std::mem::transmute(f);
                f();
            }
        }
    }
}

pub fn apply_all(a: &mut App) {
    if a.docs.is_empty() {
        return;
    }
    send(a.sci, sci::SETTECH, 1, 0);
    send(a.sci, sci::SETBUFDRAW, 0, 0);
    send(a.sci, sci::SETMULTISEL, 1, 0);
    send(a.sci, sci::SETADDSELTYPING, 1, 0);
    send(a.sci, sci::SETMULTIPASTE, 1, 0);
    send(a.sci, sci::SETVIRTUALSPACE, 1, 0);
    send(a.sci, sci::SETTABWIDTH, 4, 0);
    send(a.sci, sci::SETSCROLLW, 1, 0);
    send(a.sci, sci::SETSCROLLWTRACK, 1, 0);
    style_editor(a);
    dwm_and_menus(a);
    refresh();
}

pub fn refresh() {
    let a = app();
    if a.docs.is_empty() {
        return;
    }
    let d = &a.docs[a.cur];
    unsafe {
        let title = format!("{}{} - WinNote", if d.dirty { "*" } else { "" }, d.title());
        SetWindowTextW(a.hwnd, w(&title).as_ptr());
        InvalidateRect(a.tabs, null(), 0);
        InvalidateRect(a.status, null(), 0);
        let chk = |id: usize, on: bool| {
            CheckMenuItem(a.menu, id as u32, MF_BYCOMMAND | if on { MF_CHECKED } else { MF_UNCHECKED });
        };
        chk(WRAP, a.wrap);
        chk(LINES, a.lines);
        chk(WS, a.ws);
        chk(STATUSBAR, a.status_vis);
        CheckMenuRadioItem(a.menu, THEME as u32, (THEME + THEMES.len() - 1) as u32, (THEME + a.theme) as u32, MF_BYCOMMAND);
        CheckMenuRadioItem(a.menu, LANG as u32, (LANG + LANGS.len() - 1) as u32, (LANG + d.lang) as u32, MF_BYCOMMAND);
        let ei = Enc::ALL.iter().position(|&e| e == d.enc).unwrap_or(0);
        CheckMenuRadioItem(a.menu, ENC as u32, (ENC + 4) as u32, (ENC + ei) as u32, MF_BYCOMMAND);
        let em = send(a.sci, sci::GETEOLMODE, 0, 0) as usize;
        CheckMenuRadioItem(a.menu, EOL as u32, (EOL + 2) as u32, (EOL + em) as u32, MF_BYCOMMAND);
    }
}

// ---------- Documents ----------

fn save_view_state(a: &mut App) {
    if let Some(d) = a.docs.get_mut(a.cur) {
        d.caret = send(a.sci, sci::GETCURRENTPOS, 0, 0);
        d.first = send(a.sci, sci::GETFIRSTVISIBLE, 0, 0);
    }
}

pub fn switch_to(a: &mut App, i: usize) {
    if i >= a.docs.len() {
        return;
    }
    save_view_state(a);
    a.cur = i;
    let d = &a.docs[i];
    send(a.sci, sci::SETDOCPTR, 0, d.ptr);
    send(a.sci, sci::SETSEL, d.caret as usize, d.caret);
    send(a.sci, sci::SETFIRSTVISIBLE, d.first as usize, 0);
    style_editor(a);
    refresh();
    unsafe { SetFocus(a.sci) };
}

fn push_doc(a: &mut App, path: Option<String>, text: Option<(Vec<u8>, Enc)>) {
    save_view_state(a);
    let ptr = send(a.sci, sci::CREATEDOC, 0, 0);
    let (lang, untitled) = match &path {
        Some(p) => (lang::by_path(p), 0),
        None => {
            a.untitled += 1;
            (0, a.untitled)
        }
    };
    let enc = text.as_ref().map_or(Enc::Utf8, |t| t.1);
    a.docs.push(Doc { ptr, path, enc, lang, caret: 0, first: 0, untitled, dirty: false });
    a.cur = a.docs.len() - 1;
    send(a.sci, sci::SETDOCPTR, 0, ptr);
    send(a.sci, sci::SETCODEPAGE, 65001, 0);
    if let Some((t, _)) = text {
        send(a.sci, sci::ADDTEXT, t.len(), t.as_ptr() as isize);
        send(a.sci, sci::SETEOLMODE, doc::detect_eol(&t), 0);
        send(a.sci, sci::EMPTYUNDO, 0, 0);
        send(a.sci, sci::SETSAVEPOINT, 0, 0);
        send(a.sci, sci::SETSEL, 0, 0);
    }
    set_lexer(a, lang);
    style_editor(a);
    refresh();
    unsafe { SetFocus(a.sci) };
}

pub fn new_doc(a: &mut App) {
    push_doc(a, None, None);
}
pub fn cmd_new() {
    new_doc(app());
}
pub fn cmd_select(i: usize) {
    switch_to(app(), i);
}

pub fn open_path(p: &str) {
    let a = app();
    if let Some(i) = a.docs.iter().position(|d| d.path.as_deref() == Some(p)) {
        switch_to(a, i);
        return;
    }
    match std::fs::read(p) {
        Ok(b) => {
            let (t, e) = doc::decode(&b);
            let reuse = a.docs.len() == 1 && a.docs[0].path.is_none() && !a.docs[0].dirty && send(a.sci, sci::GETLENGTH, 0, 0) == 0;
            push_doc(a, Some(p.to_string()), Some((t, e)));
            if reuse {
                drop_doc(a, 0);
            }
        }
        Err(e) => msg_box(&format!("Impossible d'ouvrir {p} : {e}")),
    }
}

/// Retire le document i (sans demande de confirmation).
fn drop_doc(a: &mut App, i: usize) {
    let ptr = a.docs[i].ptr;
    if a.docs.len() == 1 {
        new_doc(a);
    }
    if a.cur == i {
        switch_to(a, if i > 0 { i - 1 } else { 1 });
    }
    a.docs.remove(i);
    if a.cur > i {
        a.cur -= 1;
    }
    send(a.sci, sci::RELEASEDOC, 0, ptr);
    refresh();
}

fn ask_save(name: &str) -> i32 {
    unsafe {
        MessageBoxW(app().hwnd, w(&format!("Voulez-vous enregistrer les modifications de « {name} » ?")).as_ptr(), w("WinNote").as_ptr(), MB_YESNOCANCEL | MB_ICONQUESTION)
    }
}

/// Renvoie true si le document peut être fermé.
fn confirm_close(a: &mut App, i: usize) -> bool {
    if !a.docs[i].dirty {
        return true;
    }
    switch_to(a, i);
    match ask_save(&a.docs[i].title()) {
        IDYES => save_doc(a, i, false),
        IDNO => true,
        _ => false,
    }
}

pub fn cmd_close(i: usize) {
    let a = app();
    if i < a.docs.len() && confirm_close(a, i) {
        drop_doc(a, i);
    }
}

pub fn confirm_close_all() -> bool {
    let a = app();
    (0..a.docs.len()).all(|i| confirm_close(a, i))
}

fn get_text(a: &App) -> Vec<u8> {
    let n = send(a.sci, sci::GETLENGTH, 0, 0) as usize;
    let mut b = vec![0u8; n + 1];
    send(a.sci, sci::GETTEXT, n + 1, b.as_mut_ptr() as isize);
    b.truncate(n);
    b
}

fn file_dialog(save: bool, name: &str) -> Vec<String> {
    let a = app();
    let filter = w("Tous les fichiers (*.*)\0*.*\0Texte (*.txt;*.log)\0*.txt;*.log\0Code\0*.c;*.cpp;*.h;*.rs;*.py;*.js;*.ts;*.cs;*.java;*.go\0Web et données\0*.html;*.htm;*.css;*.json;*.xml;*.md;*.yml;*.toml\0\0");
    let mut buf = vec![0u16; 32768];
    for (i, c) in name.encode_utf16().enumerate().take(260) {
        buf[i] = c;
    }
    let mut o: OPENFILENAMEW = unsafe { std::mem::zeroed() };
    o.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
    o.hwndOwner = a.hwnd;
    o.lpstrFilter = filter.as_ptr();
    o.lpstrFile = buf.as_mut_ptr();
    o.nMaxFile = buf.len() as u32;
    o.Flags = OFN_EXPLORER | OFN_PATHMUSTEXIST | OFN_HIDEREADONLY | if save { OFN_OVERWRITEPROMPT } else { OFN_FILEMUSTEXIST | OFN_ALLOWMULTISELECT };
    let ok = unsafe { if save { GetSaveFileNameW(&mut o) } else { GetOpenFileNameW(&mut o) } };
    if ok == 0 {
        return Vec::new();
    }
    let parts: Vec<String> = buf.split(|&c| c == 0).take_while(|s| !s.is_empty()).map(String::from_utf16_lossy).collect();
    match parts.len() {
        0 => Vec::new(),
        1 => parts,
        _ => parts[1..].iter().map(|f| format!("{}\\{}", parts[0], f)).collect(),
    }
}

pub fn save_doc(a: &mut App, i: usize, as_new: bool) -> bool {
    if a.cur != i {
        switch_to(a, i);
    }
    let path = match (&a.docs[i].path, as_new) {
        (Some(p), false) => p.clone(),
        _ => match file_dialog(true, &a.docs[i].title()).into_iter().next() {
            Some(p) => p,
            None => return false,
        },
    };
    let bytes = doc::encode(&get_text(a), a.docs[i].enc);
    if let Err(e) = std::fs::write(&path, bytes) {
        msg_box(&format!("Échec de l'enregistrement : {e}"));
        return false;
    }
    send(a.sci, sci::SETSAVEPOINT, 0, 0);
    let d = &mut a.docs[i];
    if d.path.as_deref() != Some(&path) {
        d.path = Some(path.clone());
        d.lang = lang::by_path(&path);
        set_lexer(a, a.docs[i].lang);
        style_editor(a);
    }
    a.docs[i].dirty = false;
    refresh();
    true
}

// ---------- Recherche ----------

fn utf8_of(buf: &[u16]) -> Vec<u8> {
    let n = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    String::from_utf16_lossy(&buf[..n]).into_bytes()
}

fn search(s: HWND, st: isize, en: isize, what: &[u8]) -> isize {
    send(s, sci::SETTARGETSTART, st as usize, 0);
    send(s, sci::SETTARGETEND, en as usize, 0);
    send(s, sci::SEARCHINTARGET, what.len(), what.as_ptr() as isize)
}

fn flags(a: &App) -> usize {
    let f = a.fr.Flags;
    (if f & FR_MATCHCASE != 0 { 4 } else { 0 }) | (if f & FR_WHOLEWORD != 0 { 2 } else { 0 })
}

fn find_next(a: &mut App, down: bool) -> bool {
    let what = utf8_of(&a.find_buf);
    if what.is_empty() {
        open_find(a, false);
        return false;
    }
    let s = a.sci;
    let len = send(s, sci::GETLENGTH, 0, 0);
    send(s, sci::SETSEARCHFLAGS, flags(a), 0);
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    let mut r = if down { search(s, se, len, &what) } else { search(s, ss, 0, &what) };
    if r < 0 {
        r = if down { search(s, 0, len, &what) } else { search(s, len, 0, &what) };
    }
    if r < 0 {
        msg_box("Texte introuvable.");
        return false;
    }
    let (ts, te) = (send(s, sci::GETTARGETSTART, 0, 0), send(s, sci::GETTARGETEND, 0, 0));
    send(s, sci::SETSEL, ts as usize, te);
    true
}

fn replace_one(a: &mut App) {
    let (what, rep) = (utf8_of(&a.find_buf), utf8_of(&a.repl_buf));
    let s = a.sci;
    send(s, sci::SETSEARCHFLAGS, flags(a), 0);
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    if !what.is_empty() && search(s, ss, se, &what) == ss && send(s, sci::GETTARGETEND, 0, 0) == se {
        send(s, sci::REPLACETARGET, rep.len(), rep.as_ptr() as isize);
        let e = send(s, sci::GETTARGETEND, 0, 0);
        send(s, sci::SETSEL, e as usize, e);
    }
    find_next(a, true);
}

fn replace_all(a: &mut App) {
    let (what, rep) = (utf8_of(&a.find_buf), utf8_of(&a.repl_buf));
    if what.is_empty() {
        return;
    }
    let s = a.sci;
    send(s, sci::SETSEARCHFLAGS, flags(a), 0);
    send(s, sci::BEGINUNDO, 0, 0);
    let (mut pos, mut n) = (0isize, 0);
    loop {
        let len = send(s, sci::GETLENGTH, 0, 0);
        if search(s, pos, len, &what) < 0 {
            break;
        }
        send(s, sci::REPLACETARGET, rep.len(), rep.as_ptr() as isize);
        pos = send(s, sci::GETTARGETEND, 0, 0);
        n += 1;
    }
    send(s, sci::ENDUNDO, 0, 0);
    msg_box(&format!("{n} remplacement(s) effectué(s)."));
}

pub fn open_find(a: &mut App, replace: bool) {
    unsafe {
        if !a.find_dlg.is_null() {
            DestroyWindow(a.find_dlg);
            a.find_dlg = null_mut();
        }
        let (ss, se) = (send(a.sci, sci::GETSELSTART, 0, 0), send(a.sci, sci::GETSELEND, 0, 0));
        if se > ss && se - ss < 200 {
            let mut b = vec![0u8; (se - ss) as usize + 1];
            send(a.sci, 2161, 0, b.as_mut_ptr() as isize); // SCI_GETSELTEXT
            let t = String::from_utf8_lossy(&b[..b.len() - 1]).replace(['\r', '\n'], " ");
            for (i, c) in t.encode_utf16().enumerate().take(500) {
                a.find_buf[i] = c;
                a.find_buf[i + 1] = 0;
            }
        }
        let fr = &mut *a.fr;
        fr.lStructSize = std::mem::size_of::<FINDREPLACEW>() as u32;
        fr.hwndOwner = a.hwnd;
        fr.Flags |= FR_DOWN;
        fr.lpstrFindWhat = a.find_buf.as_mut_ptr();
        fr.wFindWhatLen = a.find_buf.len() as u16;
        fr.lpstrReplaceWith = a.repl_buf.as_mut_ptr();
        fr.wReplaceWithLen = a.repl_buf.len() as u16;
        a.find_dlg = if replace { ReplaceTextW(fr) } else { FindTextW(fr) };
    }
}

pub fn on_find_msg(p: *mut FINDREPLACEW) {
    let a = app();
    let f = unsafe { (*p).Flags };
    if f & FR_DIALOGTERM != 0 {
        a.find_dlg = null_mut();
        a.fr.Flags &= !FR_DIALOGTERM;
    } else if f & FR_REPLACEALL != 0 {
        replace_all(a);
    } else if f & FR_REPLACE != 0 {
        replace_one(a);
    } else if f & FR_FINDNEXT != 0 {
        find_next(a, f & FR_DOWN != 0);
    }
}

// ---------- Commandes ----------

pub fn on_command(id: usize) {
    let a = app();
    let s = a.sci;
    match id {
        NEW => new_doc(a),
        OPEN => {
            for p in file_dialog(false, "") {
                open_path(&p);
            }
        }
        SAVE => {
            save_doc(a, a.cur, false);
        }
        SAVEAS => {
            save_doc(a, a.cur, true);
        }
        SAVEALL => {
            let keep = a.cur;
            for i in 0..a.docs.len() {
                if a.docs[i].dirty && !save_doc(a, i, false) {
                    break;
                }
            }
            switch_to(a, keep);
        }
        CLOSE => cmd_close(a.cur),
        EXIT => unsafe {
            SendMessageW(a.hwnd, WM_CLOSE, 0, 0);
        },
        UNDO => drop(send(s, sci::UNDO, 0, 0)),
        REDO => drop(send(s, sci::REDO, 0, 0)),
        CUT => drop(send(s, sci::CUT, 0, 0)),
        COPY => drop(send(s, sci::COPY, 0, 0)),
        PASTE => drop(send(s, sci::PASTE, 0, 0)),
        DEL => drop(send(s, sci::CLEAR, 0, 0)),
        SELALL => drop(send(s, sci::SELECTALL, 0, 0)),
        FIND => open_find(a, false),
        REPLACE => open_find(a, true),
        FINDNEXT => drop(find_next(a, true)),
        FINDPREV => drop(find_next(a, false)),
        DUPLINE => drop(send(s, sci::LINEDUP, 0, 0)),
        DELLINE => drop(send(s, sci::LINEDELETE, 0, 0)),
        UPPER => drop(send(s, sci::UPPER, 0, 0)),
        LOWER => drop(send(s, sci::LOWER, 0, 0)),
        DATETIME => {
            let t = unsafe { GetLocalTime(&mut std::mem::zeroed()); let mut st = std::mem::zeroed(); GetLocalTime(&mut st); st };
            let txt = format!("{:02}:{:02} {:02}/{:02}/{}", t.wHour, t.wMinute, t.wDay, t.wMonth, t.wYear);
            send(s, sci::REPLACESEL, 0, cstr(&txt).as_ptr() as isize);
        }
        WRAP => {
            a.wrap = !a.wrap;
            style_editor(a);
            refresh();
        }
        LINES => {
            a.lines = !a.lines;
            style_editor(a);
            refresh();
        }
        WS => {
            a.ws = !a.ws;
            style_editor(a);
            refresh();
        }
        STATUSBAR => {
            a.status_vis = !a.status_vis;
            relayout();
            refresh();
        }
        FONT => choose_font(a),
        ZOOMIN => drop(send(s, sci::ZOOMIN, 0, 0)),
        ZOOMOUT => drop(send(s, sci::ZOOMOUT, 0, 0)),
        ZOOMRESET => drop(send(s, sci::SETZOOM, 0, 0)),
        TABNEXT => switch_to(a, (a.cur + 1) % a.docs.len()),
        TABPREV => switch_to(a, (a.cur + a.docs.len() - 1) % a.docs.len()),
        ABOUT => msg_box("WinNote 0.1.0\nÉditeur de texte natif Windows 64 bits.\nMoteur : Scintilla + Lexilla."),
        EOL..=134 => {
            let m = id - EOL;
            send(s, sci::SETEOLMODE, m, 0);
            send(s, sci::CONVERTEOLS, m, 0);
            refresh();
        }
        200..=204 => {
            a.docs[a.cur].enc = Enc::ALL[id - ENC];
            refresh();
        }
        300..=399 if id - LANG < LANGS.len() => {
            a.docs[a.cur].lang = id - LANG;
            set_lexer(a, id - LANG);
            style_editor(a);
            refresh();
        }
        400..=499 if id - THEME < THEMES.len() => {
            a.theme = id - THEME;
            apply_all(a);
        }
        500..=599 if id - LAYOUT < LAYOUTS.len() => {
            let l = &LAYOUTS[id - LAYOUT];
            (a.layout, a.font, a.size, a.wrap, a.lines, a.margin) = (id - LAYOUT, l.font.into(), l.size, l.wrap, l.lines, l.margin);
            send(s, sci::SETZOOM, 0, 0);
            apply_all(a);
        }
        _ => {}
    }
    a.status_vis = a.status_vis; // état conservé
}

fn choose_font(a: &mut App) {
    unsafe {
        let mut lf: LOGFONTW = std::mem::zeroed();
        for (i, c) in a.font.encode_utf16().enumerate().take(31) {
            lf.lfFaceName[i] = c;
        }
        lf.lfHeight = -(a.size * 96 / 72);
        let mut cf: CHOOSEFONTW = std::mem::zeroed();
        cf.lStructSize = std::mem::size_of::<CHOOSEFONTW>() as u32;
        cf.hwndOwner = a.hwnd;
        cf.lpLogFont = &mut lf;
        cf.Flags = CF_SCREENFONTS | CF_INITTOLOGFONTSTRUCT | CF_NOSCRIPTSEL;
        if ChooseFontW(&mut cf) != 0 {
            let n = lf.lfFaceName.iter().position(|&c| c == 0).unwrap_or(32);
            a.font = String::from_utf16_lossy(&lf.lfFaceName[..n]);
            a.size = (cf.iPointSize / 10).clamp(6, 72);
            style_editor(a);
            refresh();
        }
    }
}
