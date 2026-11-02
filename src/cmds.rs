//! Documents (onglets), fichiers, apparence et répartition des commandes.
use crate::doc::{self, Doc, Enc};
use crate::ids::*;
use crate::lang::{self, LANGS};
use crate::palette::{self, Mode};
use crate::sci::{self, send};
use crate::theme::{Cls, LAYOUTS, THEMES};
use crate::{app, cstr, relayout, textops, w, App};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Dwm::DwmSetWindowAttribute,
    Graphics::Gdi::{GetMonitorInfoW, InvalidateRect, MonitorFromWindow, LOGFONTW, MONITORINFO, MONITOR_DEFAULTTONEAREST},
    System::{DataExchange::*, LibraryLoader::*, Memory::*, SystemInformation::GetLocalTime},
    UI::{Controls::Dialogs::*, Input::KeyboardAndMouse::SetFocus, Shell::*, WindowsAndMessaging::*},
};

pub fn msg_box(s: &str) {
    unsafe { MessageBoxW(app().hwnd, w(s).as_ptr(), w("WinNote").as_ptr(), MB_OK | MB_ICONINFORMATION) };
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
            for (k, v) in [("fold", "1"), ("fold.compact", "0"), ("fold.html", "1"), ("fold.comment", "1"), ("fold.preprocessor", "1")] {
                send(a.sci, sci::SETPROPERTY, cstr(k).as_ptr() as usize, cstr(v).as_ptr() as isize);
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
            send(s, sci::STYLESETBOLD, st as usize, matches!(cls, Cls::Head | Cls::Strong) as isize);
            send(s, sci::STYLESETITALIC, st as usize, matches!(cls, Cls::Emph) as isize);
        }
    }
    send(s, sci::SETMARGINLEFT, 0, sc(a.margin));
    send(s, sci::SETMARGINRIGHT, 0, sc(a.margin));
    send(s, sci::SETMARGINTYPE, 0, 1);
    let w0 = if a.lines { send(s, sci::TEXTWIDTH, sci::STYLE_LINENUM, cstr("_99999").as_ptr() as isize) } else { 0 };
    send(s, sci::SETMARGINW, 0, w0);
    // marge 1 : signets ; marge 2 : repli
    send(s, sci::SETMARGINTYPE, 1, 0);
    send(s, sci::SETMARGINMASK, 1, 2);
    send(s, sci::SETMARGINW, 1, if a.lines || a.fold { sc(14) } else { 0 });
    send(s, sci::MARKERDEFINE, 1, 0); // cercle
    send(s, sci::MARKERSETFORE, 1, t.accent as isize);
    send(s, sci::MARKERSETBACK, 1, t.accent as isize);
    send(s, sci::SETMARGINTYPE, 2, 0);
    send(s, sci::SETMARGINMASK, 2, 0xFE00_0000u32 as isize);
    send(s, sci::SETMARGINSENSITIVE, 2, 1);
    send(s, sci::SETMARGINW, 2, if a.fold { sc(14) } else { 0 });
    for (m, sym) in [(31usize, 6isize), (30, 2), (29, 5), (28, 5), (25, 5), (26, 5), (27, 5)] {
        send(s, sci::MARKERDEFINE, m, sym);
        send(s, sci::MARKERSETFORE, m, t.gutter as isize);
        send(s, sci::MARKERSETBACK, m, t.gutter as isize);
    }
    send(s, sci::SETAUTOFOLD, 7, 0);
    send(s, sci::SETWRAP, a.wrap as usize, 0);
    send(s, sci::SETVIEWWS, a.ws as usize, 0);
    send(s, sci::SETINDGUIDES, if a.lines { 3 } else { 0 }, 0);
    send(s, sci::SETEXTRAASCENT, (sc(a.spacing) / 2) as usize, 0);
    send(s, sci::SETEXTRADESCENT, (sc(a.spacing) / 2) as usize, 0);
    send(s, sci::USEPOPUP, 0, 0);
}

fn dwm_and_menus(a: &App) {
    let t = &THEMES[a.theme];
    unsafe {
        let set = |attr: u32, v: u32| {
            DwmSetWindowAttribute(a.hwnd, attr, &v as *const u32 as *const _, 4);
        };
        set(20, t.dark as u32);
        set(35, t.ui_bg);
        set(36, t.ui_fg);
        set(34, t.ui_bg);
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
    for c in [a.sci, a.prev] {
        send(c, sci::SETTECH, 1, 0);
        send(c, sci::SETBUFDRAW, 0, 0);
    }
    send(a.sci, sci::SETMULTISEL, 1, 0);
    send(a.sci, sci::SETADDSELTYPING, 1, 0);
    send(a.sci, sci::SETMULTIPASTE, 1, 0);
    send(a.sci, sci::SETVIRTUALSPACE, 1, 0);
    send(a.sci, sci::SETTABWIDTH, 4, 0);
    send(a.sci, sci::SETSCROLLW, 1, 0);
    send(a.sci, sci::SETSCROLLWTRACK, 1, 0);
    style_editor(a);
    dwm_and_menus(a);
    let sb = w(if THEMES[a.theme].dark { "DarkMode_Explorer" } else { "Explorer" });
    for c in [a.sci, a.prev] {
        unsafe { windows_sys::Win32::UI::Controls::SetWindowTheme(c, sb.as_ptr(), null()) };
    }
    crate::findbar::recolor();
    crate::sidebar::recolor();
    crate::mdpreview::restyle();
    crate::mdpreview::update();
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
        for h in [a.tabs, a.status, a.title] {
            InvalidateRect(h, null(), 0);
        }
        for m in [a.menus.bar, a.menus.popup] {
            let chk = |id: usize, on: bool| {
                CheckMenuItem(m, id as u32, MF_BYCOMMAND | if on { MF_CHECKED } else { MF_UNCHECKED });
            };
            chk(WRAP, a.wrap);
            chk(LINES, a.lines);
            chk(WS, a.ws);
            chk(STATUSBAR, a.status_vis);
            chk(SIDEBAR, a.sidebar_vis && a.side_mode == 0);
            chk(OUTLINE, a.sidebar_vis && a.side_mode == 1);
            chk(PREVIEW, a.preview);
            chk(FOLDMARGIN, a.fold);
            chk(ZEN, a.zen);
            chk(TOPMOST, a.topmost);
            chk(READONLY, send(a.sci, sci::GETREADONLY, 0, 0) != 0);
            chk(AUTOUPDATE, a.auto_update);
            CheckMenuRadioItem(m, THEME as u32, (THEME + THEMES.len() - 1) as u32, (THEME + a.theme) as u32, MF_BYCOMMAND);
            CheckMenuRadioItem(m, LANG as u32, (LANG + LANGS.len() - 1) as u32, (LANG + d.lang) as u32, MF_BYCOMMAND);
            let ei = Enc::ALL.iter().position(|&e| e == d.enc).unwrap_or(0);
            CheckMenuRadioItem(m, ENC as u32, (ENC + 4) as u32, (ENC + ei) as u32, MF_BYCOMMAND);
            let em = send(a.sci, sci::GETEOLMODE, 0, 0) as usize;
            CheckMenuRadioItem(m, EOL as u32, (EOL + 2) as u32, (EOL + em) as u32, MF_BYCOMMAND);
            CheckMenuRadioItem(m, CHROME_WIN as u32, CHROME_MAC as u32, (CHROME_WIN + a.chrome as usize) as u32, MF_BYCOMMAND);
            CheckMenuRadioItem(m, LAYOUT as u32, (LAYOUT + LAYOUTS.len() - 1) as u32, (LAYOUT + a.layout) as u32, MF_BYCOMMAND);
        }
    }
}

/// Style de fenêtre : 0 = Windows (barre native), 1 = macOS (boutons à gauche, menu dans ☰).
pub fn apply_chrome(a: &mut App) {
    unsafe {
        if a.chrome == 1 {
            SetMenu(a.hwnd, null_mut());
            ShowWindow(a.title, SW_SHOW);
            let m = [0i32, 0, 1, 0]; // MARGINS{left,right,top,bottom} : 1 px pour garder l'ombre
            windows_sys::Win32::Graphics::Dwm::DwmExtendFrameIntoClientArea(a.hwnd, m.as_ptr() as *const _);
        } else {
            ShowWindow(a.title, SW_HIDE);
            SetMenu(a.hwnd, a.menus.bar);
        }
        SetWindowPos(a.hwnd, null_mut(), 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
    }
    relayout();
    refresh();
}

pub fn apply_layout(a: &mut App, i: usize) {
    let l = &LAYOUTS[i];
    (a.layout, a.font, a.size, a.wrap, a.lines, a.margin, a.fold, a.spacing) = (i, l.font.into(), l.size, l.wrap, l.lines, l.margin, l.fold, l.spacing);
    if let Some(t) = l.theme {
        a.theme = t;
    }
    a.sidebar_vis = l.sidebar;
    a.preview = l.preview;
    if l.sidebar && a.root.is_none() {
        a.side_mode = 1;
    }
    if a.chrome != l.chrome {
        a.chrome = l.chrome;
        apply_chrome(a);
    }
    send(a.sci, sci::SETZOOM, 0, 0);
    relayout();
    if a.sidebar_vis {
        if a.side_mode == 1 { crate::sidebar::show_outline() } else { crate::sidebar::show_files() }
    }
    apply_all(a);
}

pub fn toggle_zen(a: &mut App) {
    unsafe {
        if !a.zen {
            a.zen_style = GetWindowLongPtrW(a.hwnd, GWL_STYLE);
            GetWindowRect(a.hwnd, &mut a.zen_rect);
            a.zen = true;
            SetWindowLongPtrW(a.hwnd, GWL_STYLE, a.zen_style & !(WS_OVERLAPPEDWINDOW as isize));
            let mut mi: MONITORINFO = std::mem::zeroed();
            mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
            GetMonitorInfoW(MonitorFromWindow(a.hwnd, MONITOR_DEFAULTTONEAREST), &mut mi);
            let r = mi.rcMonitor;
            SetMenu(a.hwnd, null_mut());
            SetWindowPos(a.hwnd, HWND_TOP, r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_FRAMECHANGED | SWP_SHOWWINDOW);
        } else {
            a.zen = false;
            SetWindowLongPtrW(a.hwnd, GWL_STYLE, a.zen_style);
            if a.chrome == 0 {
                SetMenu(a.hwnd, a.menus.bar);
            }
            let r = a.zen_rect;
            SetWindowPos(a.hwnd, null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_FRAMECHANGED | SWP_NOZORDER);
        }
    }
    relayout();
    refresh();
}

// ---------- Documents ----------

pub fn save_view_state_pub(a: &mut App) {
    save_view_state(a);
}

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
    crate::findbar::mark_all();
    crate::sidebar::refresh();
    crate::mdpreview::update();
    refresh();
    unsafe { SetFocus(a.sci) };
}

fn mtime(p: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

pub fn push_doc_full(a: &mut App, path: Option<String>, text: Option<(Vec<u8>, Enc)>, dirty: bool, untitled_in: u32) {
    save_view_state(a);
    let ptr = send(a.sci, sci::CREATEDOC, 0, 0);
    let (lang, untitled) = match &path {
        Some(p) => (lang::by_path(p), 0),
        None => {
            a.untitled = a.untitled.max(untitled_in);
            if untitled_in == 0 {
                a.untitled += 1;
            }
            (0, if untitled_in == 0 { a.untitled } else { untitled_in })
        }
    };
    let enc = text.as_ref().map_or(Enc::Utf8, |t| t.1);
    let mt = path.as_deref().and_then(mtime);
    a.docs.push(Doc { ptr, path, enc, lang, caret: 0, first: 0, untitled, dirty, mtime: mt });
    a.cur = a.docs.len() - 1;
    send(a.sci, sci::SETDOCPTR, 0, ptr);
    send(a.sci, sci::SETCODEPAGE, 65001, 0);
    if let Some((t, _)) = text {
        send(a.sci, sci::ADDTEXT, t.len(), t.as_ptr() as isize);
        send(a.sci, sci::SETEOLMODE, doc::detect_eol(&t), 0);
        send(a.sci, sci::EMPTYUNDO, 0, 0);
        if !dirty {
            send(a.sci, sci::SETSAVEPOINT, 0, 0);
        }
        send(a.sci, sci::SETSEL, 0, 0);
    }
    a.docs[a.cur].dirty = dirty;
    set_lexer(a, lang);
    style_editor(a);
    crate::findbar::mark_all();
    crate::sidebar::refresh();
    crate::mdpreview::update();
    refresh();
    unsafe { SetFocus(a.sci) };
}

pub fn new_doc(a: &mut App) {
    push_doc_full(a, None, None, false, 0);
}
pub fn cmd_new() {
    new_doc(app());
}
pub fn cmd_select(i: usize) {
    switch_to(app(), i);
}

pub fn move_tab(from: usize, to: usize) {
    let a = app();
    if from >= a.docs.len() || to >= a.docs.len() {
        return;
    }
    let d = a.docs.remove(from);
    a.docs.insert(to, d);
    a.cur = to;
    refresh();
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
            push_doc_full(a, Some(p.to_string()), Some((t, e)), false, 0);
            crate::session::add_recent(p);
            if reuse {
                drop_doc(a, 0);
            }
        }
        Err(e) => msg_box(&format!("Impossible d'ouvrir {p} : {e}")),
    }
}

/// Ouvre un fichier, ou crée un onglet nommé s'il n'existe pas encore (liens [[note]]).
pub fn open_or_create(p: &str) {
    if std::path::Path::new(p).exists() {
        open_path(p);
    } else {
        push_doc_full(app(), Some(p.to_string()), Some((Vec::new(), Enc::Utf8)), true, 0);
    }
}

fn drop_doc(a: &mut App, i: usize) {
    let ptr = a.docs[i].ptr;
    if let Some(p) = a.docs[i].path.clone() {
        a.closed.push(p);
    }
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
    unsafe { MessageBoxW(app().hwnd, w(&format!("Voulez-vous enregistrer les modifications de « {name} » ?")).as_ptr(), w("WinNote").as_ptr(), MB_YESNOCANCEL | MB_ICONQUESTION) }
}

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

fn close_many(keep: Option<usize>) {
    let a = app();
    let keep_ptr = keep.map(|k| a.docs[k].ptr);
    loop {
        let Some(i) = a.docs.iter().position(|d| Some(d.ptr) != keep_ptr) else { break };
        if keep.is_none() && a.docs.len() == 1 && a.docs[0].path.is_none() && !a.docs[0].dirty {
            break;
        }
        if !confirm_close(a, i) {
            break;
        }
        drop_doc(a, i);
    }
}

fn get_text(a: &App) -> Vec<u8> {
    let n = send(a.sci, sci::GETLENGTH, 0, 0) as usize;
    let mut b = vec![0u8; n + 1];
    send(a.sci, sci::GETTEXT, n + 1, b.as_mut_ptr() as isize);
    b.truncate(n);
    b
}

/// Texte UTF-8 d'un document quelconque (utilisé par la session).
pub fn text_of_doc(a: &mut App, i: usize) -> Vec<u8> {
    if i == a.cur {
        return get_text(a);
    }
    let cur_ptr = a.docs[a.cur].ptr;
    send(a.sci, sci::SETDOCPTR, 0, a.docs[i].ptr);
    let t = get_text(a);
    send(a.sci, sci::SETDOCPTR, 0, cur_ptr);
    t
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

fn pick_folder() -> Option<String> {
    unsafe {
        let title = w("Choisir un dossier");
        let mut name = vec![0u16; 260];
        let bi = BROWSEINFOW { hwndOwner: app().hwnd, pidlRoot: null_mut(), pszDisplayName: name.as_mut_ptr(), lpszTitle: title.as_ptr(), ulFlags: 0x41, lpfn: None, lParam: 0, iImage: 0 };
        let pidl = SHBrowseForFolderW(&bi);
        if pidl.is_null() {
            return None;
        }
        let mut path = vec![0u16; 1024];
        let ok = SHGetPathFromIDListW(pidl, path.as_mut_ptr()) != 0;
        windows_sys::Win32::System::Com::CoTaskMemFree(pidl as *const _);
        ok.then(|| String::from_utf16_lossy(&path[..path.iter().position(|&c| c == 0).unwrap_or(0)]))
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
    d.mtime = mtime(&path);
    if d.path.as_deref() != Some(&path) {
        d.path = Some(path.clone());
        d.lang = lang::by_path(&path);
        set_lexer(a, a.docs[i].lang);
        style_editor(a);
        crate::session::add_recent(&path);
    }
    a.docs[i].dirty = false;
    refresh();
    true
}

fn reload(a: &mut App) {
    let Some(p) = a.docs[a.cur].path.clone() else { return };
    match std::fs::read(&p) {
        Ok(b) => {
            let (t, e) = doc::decode(&b);
            let (caret, first) = (send(a.sci, sci::GETCURRENTPOS, 0, 0), send(a.sci, sci::GETFIRSTVISIBLE, 0, 0));
            let mut z = t.clone();
            z.push(0);
            send(a.sci, sci::SETTEXT, 0, z.as_ptr() as isize);
            send(a.sci, sci::EMPTYUNDO, 0, 0);
            send(a.sci, sci::SETSAVEPOINT, 0, 0);
            send(a.sci, sci::SETSEL, caret as usize, caret);
            send(a.sci, sci::SETFIRSTVISIBLE, first as usize, 0);
            let d = &mut a.docs[a.cur];
            (d.enc, d.dirty, d.mtime) = (e, false, mtime(&p));
            refresh();
        }
        Err(e) => msg_box(&format!("Rechargement impossible : {e}")),
    }
}

/// Appelé à l'activation de la fenêtre : détecte une modification externe du fichier courant.
pub fn check_external() {
    let a = app();
    let Some(p) = a.docs.get(a.cur).and_then(|d| d.path.clone()) else { return };
    let (old, new) = (a.docs[a.cur].mtime, mtime(&p));
    if old.is_none() || new.is_none() || old == new {
        return;
    }
    a.docs[a.cur].mtime = new;
    if !a.docs[a.cur].dirty {
        reload(a);
    } else if unsafe { MessageBoxW(a.hwnd, w("Le fichier a été modifié en dehors de WinNote. Recharger (vos changements seront perdus) ?").as_ptr(), w("WinNote").as_ptr(), MB_YESNO | MB_ICONQUESTION) } == IDYES {
        reload(a);
    }
}

fn copy_text(s: &str) {
    unsafe {
        if OpenClipboard(app().hwnd) == 0 {
            return;
        }
        EmptyClipboard();
        let v = w(s);
        let h = GlobalAlloc(GMEM_MOVEABLE, v.len() * 2);
        if !h.is_null() {
            std::ptr::copy_nonoverlapping(v.as_ptr(), GlobalLock(h) as *mut u16, v.len());
            GlobalUnlock(h);
            SetClipboardData(13, h as HANDLE); // CF_UNICODETEXT
        }
        CloseClipboard();
    }
}

fn shell_open(target: &str) {
    unsafe { ShellExecuteW(null_mut(), w("open").as_ptr(), w(target).as_ptr(), null(), null(), SW_SHOWNORMAL) };
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
        OPENFOLDER => {
            if let Some(p) = pick_folder() {
                crate::sidebar::set_root(&p);
            }
        }
        SAVE => drop(save_doc(a, a.cur, false)),
        SAVEAS => drop(save_doc(a, a.cur, true)),
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
        CLOSEOTHERS => close_many(Some(a.cur)),
        CLOSEALL => close_many(None),
        REOPEN => {
            if let Some(p) = a.closed.pop() {
                open_path(&p);
            }
        }
        EXIT => unsafe {
            SendMessageW(a.hwnd, WM_CLOSE, 0, 0);
        },
        PRINT => crate::print::print_doc(),
        RELOAD => reload(a),
        REVEALFILE => {
            if let Some(p) = &a.docs[a.cur].path {
                let _ = std::process::Command::new("explorer.exe").arg(format!("/select,{p}")).spawn();
            }
        }
        COPYPATH => {
            if let Some(p) = a.docs[a.cur].path.clone() {
                copy_text(&p);
            }
        }
        BROWSER => {
            if let Some(p) = a.docs[a.cur].path.clone() {
                shell_open(&p);
            }
        }
        UNDO => drop(send(s, sci::UNDO, 0, 0)),
        REDO => drop(send(s, sci::REDO, 0, 0)),
        CUT => drop(send(s, sci::CUT, 0, 0)),
        COPY => drop(send(s, sci::COPY, 0, 0)),
        PASTE => drop(send(s, sci::PASTE, 0, 0)),
        DEL => drop(send(s, sci::CLEAR, 0, 0)),
        SELALL => drop(send(s, sci::SELECTALL, 0, 0)),
        FIND => crate::findbar::open(false),
        REPLACE => crate::findbar::open(true),
        FINDNEXT => crate::findbar::find_next(true),
        FINDPREV => crate::findbar::find_next(false),
        FINDINFILES => palette::open(Mode::InFiles),
        GOTO => palette::open(Mode::Goto),
        PALETTE => palette::open(Mode::Commands),
        QUICKOPEN => palette::open(Mode::Quick),
        DUPLINE => drop(send(s, sci::LINEDUP, 0, 0)),
        DELLINE => drop(send(s, sci::LINEDELETE, 0, 0)),
        UPPER => drop(send(s, sci::UPPER, 0, 0)),
        LOWER => drop(send(s, sci::LOWER, 0, 0)),
        MOVEUP => drop(send(s, sci::MOVELINESUP, 0, 0)),
        MOVEDOWN => drop(send(s, sci::MOVELINESDOWN, 0, 0)),
        SORTASC => textops::sort_lines(false),
        SORTDESC => textops::sort_lines(true),
        DEDUPE => textops::dedupe(),
        TRIM => textops::trim(),
        JOIN => textops::join(),
        TOGGLECOMMENT => textops::toggle_comment(),
        TITLECASE => textops::title_case(),
        TABS2SPACES => textops::tabs_to_spaces(),
        SPACES2TABS => textops::spaces_to_tabs(),
        SELNEXT => drop(send(s, sci::SELADDNEXT, 0, 0)),
        OPENLINK => textops::open_link(),
        STATS => textops::stats(),
        BOOKTOGGLE => {
            send(s, sci::SETMARGINW, 1, (14 * a.dpi as i32 / 96) as isize);
            textops::bookmark_toggle();
        }
        BOOKNEXT => textops::bookmark_step(true),
        BOOKPREV => textops::bookmark_step(false),
        BOOKCLEAR => drop(send(s, sci::MARKERDELETEALL, 1, 0)),
        FOLDALL => drop(send(s, sci::FOLDALL, 0, 0)),
        UNFOLDALL => drop(send(s, sci::FOLDALL, 1, 0)),
        FOLDMARGIN => {
            a.fold = !a.fold;
            style_editor(a);
            refresh();
        }
        DATETIME => {
            let t = unsafe {
                let mut st = std::mem::zeroed();
                GetLocalTime(&mut st);
                st
            };
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
        SIDEBAR | OUTLINE => {
            let mode = (id == OUTLINE) as u8;
            a.sidebar_vis = !(a.sidebar_vis && a.side_mode == mode);
            relayout();
            if a.sidebar_vis {
                if mode == 1 { crate::sidebar::show_outline() } else { crate::sidebar::show_files() }
            }
            refresh();
        }
        PREVIEW => {
            a.preview = !a.preview;
            relayout();
            crate::mdpreview::update();
            refresh();
        }
        ZEN => toggle_zen(a),
        TOPMOST => unsafe {
            a.topmost = !a.topmost;
            SetWindowPos(a.hwnd, if a.topmost { HWND_TOPMOST } else { HWND_NOTOPMOST }, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
            refresh();
        },
        READONLY => {
            let r = send(s, sci::GETREADONLY, 0, 0) == 0;
            send(s, sci::SETREADONLY, r as usize, 0);
            refresh();
        }
        FONT => choose_font(a),
        ZOOMIN => drop(send(s, sci::ZOOMIN, 0, 0)),
        ZOOMOUT => drop(send(s, sci::ZOOMOUT, 0, 0)),
        ZOOMRESET => drop(send(s, sci::SETZOOM, 0, 0)),
        TABNEXT => switch_to(a, (a.cur + 1) % a.docs.len()),
        TABPREV => switch_to(a, (a.cur + a.docs.len() - 1) % a.docs.len()),
        CHROME_WIN | CHROME_MAC => {
            a.chrome = (id - CHROME_WIN) as u8;
            apply_chrome(a);
            apply_all(a);
        }
        UPDATE => crate::updater::check(true),
        AUTOUPDATE => {
            a.auto_update = !a.auto_update;
            refresh();
        }
        REGISTER => msg_box(if crate::shell::register() { "WinNote est ajouté au menu « Modifier avec WinNote » de l'explorateur." } else { "Intégration impossible." }),
        UNREGISTER => msg_box(if crate::shell::unregister() { "Intégration retirée." } else { "Rien à retirer." }),
        ABOUT => msg_box(&format!("WinNote {}\nÉditeur de texte natif Windows 64 bits.\nMoteur : Scintilla + Lexilla.", env!("CARGO_PKG_VERSION"))),
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
            crate::sidebar::refresh();
            crate::mdpreview::update();
            refresh();
        }
        400..=499 if id - THEME < THEMES.len() => {
            a.theme = id - THEME;
            apply_all(a);
        }
        500..=599 if id - LAYOUT < LAYOUTS.len() => apply_layout(a, id - LAYOUT),
        700..=719 => {
            if let Some(p) = a.recent.get(id - RECENT).cloned() {
                open_path(&p);
            }
        }
        CLEARRECENT => {
            a.recent.clear();
            crate::session::save_recent();
            crate::menu::rebuild_recent(a);
        }
        _ => {}
    }
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
            crate::mdpreview::restyle();
            refresh();
        }
    }
}
