//! Onglets, styles, fichiers, session et réglages (Linux / GTK).
use crate::doc::{self, Enc};
use crate::lang::{self, LANGS};
use crate::sci::{self, send, HWND};
use crate::theme::{Cls, LAYOUTS, THEMES};
use crate::{app, cstr, Tab};
use gtk::glib::translate::FromGlibPtrNone;
use gtk::prelude::*;
use std::ffi::{c_int, c_void};
use std::path::PathBuf;

extern "C" {
    fn scintilla_object_new() -> *mut c_void;
}

pub fn msg_box(s: &str) {
    let a = app();
    if a.quiet {
        return;
    }
    let d = gtk::MessageDialog::new(Some(&a.win), gtk::DialogFlags::MODAL, gtk::MessageType::Info, gtk::ButtonsType::Ok, s);
    d.run();
    d.close();
}

/// Polices Windows des thèmes -> polices disponibles sous Linux / macOS.
pub fn linux_font(f: &str) -> &str {
    let mac = cfg!(target_os = "macos");
    match f {
        "Consolas" => if mac { "Menlo" } else { "Monospace" },
        "Segoe UI" => if mac { "Helvetica Neue" } else { "Sans" },
        "Georgia" => if mac { "Georgia" } else { "Serif" },
        o => o,
    }
}

/// Crée un contrôle Scintilla GTK (UTF-8) et renvoie son pointeur et son widget.
pub fn new_sci() -> (HWND, gtk::Widget) {
    let ptr = unsafe { scintilla_object_new() };
    let widget: gtk::Widget = unsafe { gtk::Widget::from_glib_none(ptr as *mut gtk::ffi::GtkWidget) };
    widget.set_hexpand(true);
    widget.set_vexpand(true);
    send(ptr, sci::SETCODEPAGE, 65001, 0);
    (ptr, widget)
}

fn mtext(s: HWND) -> Vec<u8> {
    let n = send(s, sci::GETLENGTH, 0, 0) as usize;
    let mut b = vec![0u8; n + 1];
    send(s, sci::GETTEXT, n + 1, b.as_mut_ptr() as isize);
    b.truncate(n);
    b
}

pub fn text_of(i: usize) -> Vec<u8> {
    mtext(app().docs[i].sci)
}

// ---------- Style ----------

pub fn set_lexer(s: HWND, lang: usize) {
    let l = &LANGS[lang];
    unsafe {
        if l.lexer.is_empty() || l.lexer == "markdown" {
            // Markdown : styles posés par mdstyle (titres agrandis, gras, italique...), pas de Lexilla
            send(s, sci::SETILEXER, 0, 0);
            if !l.lexer.is_empty() {
                crate::mdstyle::apply(s);
            }
        } else {
            let p = sci::CreateLexer(cstr(l.lexer).as_ptr());
            send(s, sci::SETILEXER, 0, p as isize);
            for (i, k) in l.kw.iter().enumerate() {
                send(s, sci::SETKEYWORDS, i, cstr(k).as_ptr() as isize);
            }
            for (k, v) in [("fold", "1"), ("fold.compact", "0"), ("fold.html", "1"), ("fold.comment", "1"), ("fold.preprocessor", "1")] {
                send(s, sci::SETPROPERTY, cstr(k).as_ptr() as usize, cstr(v).as_ptr() as isize);
            }
        }
    }
    send(s, sci::COLOURISE, 0, -1);
}

pub fn style_view(s: HWND, lang: usize) {
    let a = app();
    let t = &THEMES[a.theme];
    send(s, sci::STYLESETFONT, sci::STYLE_DEFAULT, cstr(linux_font(&a.font)).as_ptr() as isize);
    send(s, sci::STYLESETSIZE, sci::STYLE_DEFAULT, a.size as isize);
    send(s, sci::STYLESETFORE, sci::STYLE_DEFAULT, t.fg as isize);
    send(s, sci::STYLESETBACK, sci::STYLE_DEFAULT, t.bg as isize);
    send(s, sci::STYLECLEARALL, 0, 0);
    send(s, sci::STYLESETFORE, sci::STYLE_LINENUM, t.gutter as isize);
    send(s, sci::STYLESETBACK, sci::STYLE_LINENUM, t.bg as isize);
    send(s, sci::STYLESETFORE, sci::STYLE_INDENT, t.line as isize);
    crate::assist::style(s, t.accent, t.sel, t.fg);
    send(s, sci::SETSELBACK, 1, t.sel as isize);
    send(s, sci::SETCARETFORE, t.fg as usize, 0);
    send(s, sci::SETCARETW, 2, 0);
    send(s, sci::SETCARETLINEVIS, 1, 0);
    send(s, sci::SETCARETLINEBACK, t.line as usize, 0);
    send(s, sci::SETWSFORE, 1, t.gutter as isize);
    if LANGS[lang].lexer == "markdown" {
        crate::mdstyle::attrs(s, t, a.size, [180, 150, 130, 115, 105, 100], linux_font("Consolas"));
    }
    for &(st, cls) in LANGS[lang].map.iter().filter(|_| LANGS[lang].lexer != "markdown") {
        send(s, sci::STYLESETFORE, st as usize, t.color(cls) as isize);
        send(s, sci::STYLESETBOLD, st as usize, matches!(cls, Cls::Head | Cls::Strong) as isize);
        send(s, sci::STYLESETITALIC, st as usize, matches!(cls, Cls::Emph) as isize);
    }
    send(s, sci::SETMARGINLEFT, 0, a.margin as isize);
    send(s, sci::SETMARGINRIGHT, 0, a.margin as isize);
    send(s, sci::SETMARGINTYPE, 0, 1);
    let w0 = if a.lines { send(s, sci::TEXTWIDTH, sci::STYLE_LINENUM, cstr("_99999").as_ptr() as isize) } else { 0 };
    send(s, sci::SETMARGINW, 0, w0);
    send(s, sci::SETMARGINTYPE, 1, 0);
    send(s, sci::SETMARGINMASK, 1, 2 | 0x01E0_0000);
    send(s, sci::SETMARGINW, 1, if a.lines || a.fold || crate::prefs::history() { 14 } else { 0 });
    send(s, sci::MARKERDEFINE, 1, 0);
    send(s, sci::MARKERSETFORE, 1, t.accent as isize);
    send(s, sci::MARKERSETBACK, 1, t.accent as isize);
    send(s, sci::SETMARGINTYPE, 2, 0);
    send(s, sci::SETMARGINMASK, 2, 0xFE00_0000u32 as isize);
    send(s, sci::SETMARGINSENSITIVE, 2, 1);
    send(s, sci::SETMARGINW, 2, if a.fold { 14 } else { 0 });
    for (m, sym) in [(31usize, 6isize), (30, 2), (29, 5), (28, 5), (25, 5), (26, 5), (27, 5)] {
        send(s, sci::MARKERDEFINE, m, sym);
        send(s, sci::MARKERSETFORE, m, t.gutter as isize);
        send(s, sci::MARKERSETBACK, m, t.gutter as isize);
    }
    send(s, 2290, 1, t.bg as isize); // SCI_SETFOLDMARGINCOLOUR
    send(s, 2291, 1, t.bg as isize); // SCI_SETFOLDMARGINHICOLOUR
    send(s, sci::SETAUTOFOLD, 7, 0);
    send(s, sci::SETWRAP, a.wrap as usize, 0);
    send(s, sci::SETVIEWWS, a.ws as usize, 0);
    send(s, sci::SETINDGUIDES, if a.lines || crate::prefs::view().0 { 3 } else { 0 }, 0);
    send(s, sci::SETEXTRAASCENT, (a.spacing / 2) as usize, 0);
    send(s, sci::SETEXTRADESCENT, (a.spacing / 2) as usize, 0);
    send(s, sci::SETMULTISEL, 1, 0);
    send(s, sci::SETADDSELTYPING, 1, 0);
    send(s, sci::SETMULTIPASTE, 1, 0);
    send(s, sci::SETVIRTUALSPACE, 1, 0);
    crate::prefs::apply_indent(s, lang);
    send(s, sci::SETSCROLLW, 1, 0);
    send(s, sci::SETSCROLLWTRACK, 1, 0);
    send(s, sci::USEPOPUP, 1, 0);
}

pub fn restyle_all() {
    let a = app();
    for t in &a.docs {
        style_view(t.sci, t.lang);
    }
    crate::find::style_indicator();
    crate::spell::style_indicator();
    crate::view::restyle();
}

// ---------- Onglets ----------

unsafe extern "C" fn on_notify(_w: *mut c_void, _id: c_int, n: *mut c_void, data: *mut c_void) {
    // Sci_NotifyHeader { hwndFrom, idFrom, code } puis position, ch (offset 32), modifiers, modificationType (offset 40)
    let base = n as *const u8;
    let code = *(base.add(16) as *const u32);
    let a = app();
    let Some(i) = a.docs.iter().position(|t| t.sci == data) else { return };
    match code {
        sci::SCN_SAVEPOINTLEFT | sci::SCN_SAVEPOINTREACHED => {
            a.docs[i].dirty = code == sci::SCN_SAVEPOINTLEFT;
            refresh();
        }
        sci::SCN_UPDATEUI => {
            crate::ui::update_status();
            crate::view::schedule();
            crate::assist::brace_highlight(a.docs[i].sci);
            crate::assist::smart_highlight(a.docs[i].sci);
        }
        2009 => crate::macros::on_record(*(base.add(72) as *const i32) as u32, *(base.add(80) as *const usize), *(base.add(88) as *const isize)),
        2001 => {
            let lx = LANGS[a.docs[i].lang].lexer;
            crate::assist::on_char(a.docs[i].sci, *(base.add(32) as *const i32) as u8, !lx.is_empty() && lx != "markdown", lx.is_empty());
            if *(base.add(32) as *const i32) == 10 {
                let s = a.docs[i].sci;
                let ln = send(s, sci::LINEFROMPOS, send(s, sci::GETCURRENTPOS, 0, 0) as usize, 0);
                if ln > 0 {
                    let ind = send(s, 2127, (ln - 1) as usize, 0);
                    send(s, 2126, ln as usize, ind);
                    let p = send(s, 2128, ln as usize, 0);
                    send(s, sci::GOTOPOS, p as usize, 0);
                    if LANGS[a.docs[i].lang].lexer == "markdown" {
                        crate::mdedit::on_enter(s);
                    }
                }
            }
        }
        sci::SCN_MODIFIED => {
            let mt = *(base.add(40) as *const i32) as u32;
            if mt & (sci::SC_MOD_INSERTTEXT | sci::SC_MOD_DELETETEXT) != 0 {
                crate::view::md_live(i);
            }
        }
        _ => {}
    }
}

pub fn new_tab(path: Option<String>, text: Option<(Vec<u8>, Enc)>, dirty: bool, untitled_in: u32) {
    let a = app();
    let (ptr, widget) = new_sci();
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
    let label = gtk::Label::new(None);
    let close = gtk::Button::from_icon_name(Some("window-close-symbolic"), gtk::IconSize::Menu);
    close.set_relief(gtk::ReliefStyle::None);
    close.set_focus_on_click(false);
    let tb = gtk::Box::new(gtk::Orientation::Horizontal, 4);
    tb.pack_start(&label, true, true, 0);
    tb.pack_start(&close, false, false, 0);
    tb.show_all();
    close.connect_clicked(move |_| {
        if let Some(i) = app().docs.iter().position(|t| t.sci == ptr) {
            close_tab(i);
        }
    });
    let mtime = path.as_deref().and_then(file_mtime);
    a.docs.push(Tab { sci: ptr, path, enc, lang, dirty, follow: false, pinned: false, mtime, untitled, page: widget.clone(), label });
    let idx = a.nb.append_page(&widget, Some(&tb));
    a.nb.set_tab_reorderable(&widget, true);
    widget.show();
    unsafe {
        gtk::glib::gobject_ffi::g_signal_connect_data(
            ptr as *mut gtk::glib::gobject_ffi::GObject,
            b"sci-notify\0".as_ptr() as *const _,
            Some(std::mem::transmute::<unsafe extern "C" fn(*mut c_void, c_int, *mut c_void, *mut c_void), unsafe extern "C" fn()>(on_notify)),
            ptr,
            None,
            0,
        );
    }
    a.nb.set_current_page(Some(idx));
    a.cur = idx as usize;
    a.sci = ptr;
    if let Some((t, _)) = text {
        send(ptr, sci::ADDTEXT, t.len(), t.as_ptr() as isize);
        send(ptr, sci::SETEOLMODE, doc::detect_eol(&t), 0);
        send(ptr, sci::EMPTYUNDO, 0, 0);
        if !dirty {
            send(ptr, sci::SETSAVEPOINT, 0, 0);
        }
        send(ptr, sci::SETSEL, 0, 0);
    }
    a.docs[idx as usize].dirty = dirty;
    set_lexer(ptr, lang);
    style_view(ptr, lang);
    crate::view::hook_tab(&widget, ptr);
    crate::spell::style_indicator();
    refresh();
    crate::view::on_tab_changed();
    unsafe { gtk::ffi::gtk_widget_grab_focus(ptr as *mut _) };
}

pub fn title_of(t: &Tab) -> String {
    match &t.path {
        Some(p) => p.rsplit('/').next().unwrap_or(p).to_string(),
        None if t.untitled <= 1 => "Sans titre".into(),
        None => format!("Sans titre {}", t.untitled),
    }
}

/// Met à jour libellés d'onglets, titre de la fenêtre et barre d'état.
pub fn refresh() {
    let a = app();
    if a.docs.is_empty() {
        return;
    }
    for t in &a.docs {
        t.label.set_text(&format!("{}{}{}", if t.pinned { "◆ " } else { "" }, if t.dirty { "● " } else { "" }, title_of(t)));
    }
    let d = &a.docs[a.cur.min(a.docs.len() - 1)];
    let title = format!("{}{} - WinNote", if d.dirty { "*" } else { "" }, title_of(d));
    a.win.set_title(&title);
    a.bar.title.set_text(&title);
    crate::ui::update_status();
}

pub fn switch_to(i: usize) {
    let a = app();
    if i < a.docs.len() {
        a.nb.set_current_page(Some(i as u32));
    }
}

fn ask_save(name: &str) -> gtk::ResponseType {
    let a = app();
    let d = gtk::MessageDialog::new(Some(&a.win), gtk::DialogFlags::MODAL, gtk::MessageType::Question, gtk::ButtonsType::None, &format!("Enregistrer les modifications de « {name} » ?"));
    d.add_button("Enregistrer", gtk::ResponseType::Yes);
    d.add_button("Ne pas enregistrer", gtk::ResponseType::No);
    d.add_button("Annuler", gtk::ResponseType::Cancel);
    let r = d.run();
    d.close();
    r
}

fn confirm_close(i: usize) -> bool {
    let a = app();
    if !a.docs[i].dirty {
        return true;
    }
    switch_to(i);
    match ask_save(&title_of(&a.docs[i])) {
        gtk::ResponseType::Yes => save_doc(i, false),
        gtk::ResponseType::No => true,
        _ => false,
    }
}

pub fn close_tab(i: usize) {
    if i < app().docs.len() && !app().docs[i].pinned && confirm_close(i) {
        drop_tab(i);
    }
}

pub fn drop_tab(i: usize) {
    let a = app();
    if let Some(p) = a.docs[i].path.clone() {
        a.closed.push(p);
    }
    let page = a.docs[i].page.clone();
    a.docs.remove(i);
    if a.docs.is_empty() {
        a.nb.remove_page(Some(i as u32));
        new_tab(None, None, false, 0);
    } else {
        a.nb.remove_page(Some(i as u32));
        let c = a.nb.current_page().unwrap_or(0) as usize;
        a.cur = c.min(a.docs.len() - 1);
        a.sci = a.docs[a.cur].sci;
    }
    drop(page);
    refresh();
    crate::view::on_tab_changed();
}

pub fn close_side(keep: usize, dir: i32) {
    let a = app();
    let keep_ptr = a.docs[keep].sci;
    loop {
        let k = a.docs.iter().position(|t| t.sci == keep_ptr).unwrap_or(0);
        let Some(i) = a.docs.iter().enumerate().position(|(j, t)| match dir {
            _ if t.pinned => false,
            x if x < 0 => j < k,
            0 => !t.dirty && t.sci != keep_ptr && t.path.is_some(),
            _ => j > k,
        }) else { break };
        if !confirm_close(i) {
            break;
        }
        drop_tab(i);
    }
}

pub fn close_others(keep: usize) {
    let a = app();
    let keep_ptr = a.docs[keep].sci;
    while let Some(i) = a.docs.iter().position(|t| t.sci != keep_ptr && !t.pinned) {
        if !confirm_close(i) {
            break;
        }
        drop_tab(i);
    }
}

pub fn reopen_closed() {
    if let Some(p) = app().closed.pop() {
        open_path(&p);
    }
}

// ---------- Fichiers ----------

pub fn open_path(p: &str) {
    let a = app();
    if let Some(i) = a.docs.iter().position(|d| d.path.as_deref() == Some(p)) {
        switch_to(i);
        return;
    }
    match std::fs::read(p) {
        Ok(b) => {
            let (t, e) = doc::decode(&b);
            let reuse = a.docs.len() == 1 && a.docs[0].path.is_none() && !a.docs[0].dirty && send(a.docs[0].sci, sci::GETLENGTH, 0, 0) == 0;
            new_tab(Some(p.to_string()), Some((t, e)), false, 0);
            add_recent(p);
            if reuse {
                drop_tab(0);
            }
        }
        Err(e) => msg_box(&format!("Impossible d'ouvrir {p} : {e}")),
    }
}

pub fn open_or_create(p: &str) {
    if std::path::Path::new(p).exists() {
        open_path(p);
    } else {
        new_tab(Some(p.to_string()), Some((Vec::new(), Enc::Utf8)), true, 0);
    }
}

pub fn save_doc(i: usize, as_new: bool) -> bool {
    let a = app();
    switch_to(i);
    let path = match (&a.docs[i].path, as_new) {
        (Some(p), false) => p.clone(),
        _ => {
            let d = gtk::FileChooserNative::new(Some("Enregistrer sous"), Some(&a.win), gtk::FileChooserAction::Save, Some("Enregistrer"), Some("Annuler"));
            d.set_do_overwrite_confirmation(true);
            d.set_current_name(&title_of(&a.docs[i]));
            if d.run() != gtk::ResponseType::Accept {
                return false;
            }
            match d.filename() {
                Some(f) => f.to_string_lossy().to_string(),
                None => return false,
            }
        }
    };
    let text = mtext(a.docs[i].sci);
    if !doc::lossless(&text, a.docs[i].enc) && !confirm_lossy(a.docs[i].enc.name()) {
        return false;
    }
    let bytes = doc::encode(&text, a.docs[i].enc);
    if let Err(e) = std::fs::write(&path, bytes) {
        msg_box(&format!("Échec de l'enregistrement : {e}"));
        return false;
    }
    let s = a.docs[i].sci;
    send(s, sci::SETSAVEPOINT, 0, 0);
    a.docs[i].mtime = file_mtime(&path);
    let changed = a.docs[i].path.as_deref() != Some(&path);
    if changed {
        a.docs[i].path = Some(path.clone());
        a.docs[i].lang = lang::by_path(&path);
        set_lexer(s, a.docs[i].lang);
        style_view(s, a.docs[i].lang);
        add_recent(&path);
    }
    a.docs[i].dirty = false;
    refresh();
    true
}

pub fn pick_files() -> Vec<String> {
    let a = app();
    let d = gtk::FileChooserNative::new(Some("Ouvrir"), Some(&a.win), gtk::FileChooserAction::Open, Some("Ouvrir"), Some("Annuler"));
    d.set_select_multiple(true);
    if d.run() != gtk::ResponseType::Accept {
        return Vec::new();
    }
    d.filenames().iter().map(|f| f.to_string_lossy().to_string()).collect()
}

pub fn pick_folder() -> Option<String> {
    let a = app();
    let d = gtk::FileChooserNative::new(Some("Choisir un dossier"), Some(&a.win), gtk::FileChooserAction::SelectFolder, Some("Choisir"), Some("Annuler"));
    (d.run() == gtk::ResponseType::Accept).then(|| d.filename().map(|f| f.to_string_lossy().to_string())).flatten()
}

fn file_mtime(p: &str) -> Option<std::time::SystemTime> {
    std::fs::metadata(p).and_then(|m| m.modified()).ok()
}

pub fn reload() {
    reload_at(app().cur, None);
}

pub fn reload_with(enc: Option<Enc>) {
    reload_at(app().cur, enc);
}

fn reload_at(i: usize, enc: Option<Enc>) {
    let a = app();
    let Some(p) = a.docs.get(i).and_then(|d| d.path.clone()) else { return };
    match std::fs::read(&p) {
        Ok(b) => {
            let (t, e) = match enc {
                Some(x) => (doc::decode_as(&b, x), x),
                None => doc::decode(&b),
            };
            let s = a.docs[i].sci;
            let (caret, first) = (send(s, sci::GETCURRENTPOS, 0, 0), send(s, sci::GETFIRSTVISIBLE, 0, 0));
            let mut z = t;
            z.push(0);
            send(s, sci::SETTEXT, 0, z.as_ptr() as isize);
            send(s, sci::EMPTYUNDO, 0, 0);
            send(s, sci::SETSAVEPOINT, 0, 0);
            if a.docs[i].follow {
                let n = send(s, sci::GETLENGTH, 0, 0);
                send(s, sci::SETSEL, n as usize, n);
                send(s, 2169, 0, 0); // SCROLLCARET
            } else {
                send(s, sci::SETSEL, caret as usize, caret);
                send(s, sci::SETFIRSTVISIBLE, first as usize, 0);
            }
            a.docs[i].enc = e;
            a.docs[i].dirty = false;
            a.docs[i].mtime = file_mtime(&p);
            refresh();
        }
        Err(e) => msg_box(&format!("Rechargement impossible : {e}")),
    }
}

/// Appelé par un minuteur : un fichier ouvert a changé sur le disque -> rechargement (confirmation si l'onglet est modifié).
pub fn check_external() {
    let a = app();
    for i in 0..a.docs.len() {
        let Some(p) = a.docs[i].path.clone() else { continue };
        let (old, new) = (a.docs[i].mtime, file_mtime(&p));
        if old.is_none() || new.is_none() || old == new {
            continue;
        }
        a.docs[i].mtime = new;
        if !a.docs[i].dirty {
            reload_at(i, None);
        } else if !a.docs[i].follow {
            let d = gtk::MessageDialog::new(Some(&a.win), gtk::DialogFlags::MODAL, gtk::MessageType::Question, gtk::ButtonsType::YesNo, &format!("« {} » a été modifié en dehors de WinNote. Recharger (vos changements seront perdus) ?", a.docs[i].label.text()));
            let r = d.run();
            d.close();
            if r == gtk::ResponseType::Yes {
                reload_at(i, None);
            }
        }
    }
}

pub fn toggle_follow() {
    let a = app();
    let f = !a.docs[a.cur].follow;
    a.docs[a.cur].follow = f;
    if f {
        reload();
    }
}

// ---------- Réglages, récents, session ----------

fn cfg_dir() -> PathBuf {
    gtk::glib::user_config_dir().join("winnote")
}

pub fn add_recent(p: &str) {
    let a = app();
    a.recent.retain(|r| r != p);
    a.recent.insert(0, p.to_string());
    a.recent.truncate(15);
    let _ = std::fs::create_dir_all(cfg_dir());
    let _ = std::fs::write(cfg_dir().join("recent.txt"), a.recent.join("\n"));
}

pub fn load_settings() {
    let a = app();
    crate::prefs::init(cfg_dir());
    crate::keymap::init(cfg_dir());
    if let Ok(s) = std::fs::read_to_string(cfg_dir().join("recent.txt")) {
        a.recent = s.lines().filter(|l| !l.is_empty()).map(String::from).collect();
    }
    let Ok(s) = std::fs::read_to_string(cfg_dir().join("settings.ini")) else { return };
    for l in s.lines() {
        let Some((k, v)) = l.split_once('=') else { continue };
        let b = v == "1";
        match k {
            "theme" => a.theme = v.parse().ok().filter(|&x| x < THEMES.len()).unwrap_or(a.theme),
            "layout" => a.layout = v.parse().ok().filter(|&x| x < LAYOUTS.len()).unwrap_or(a.layout),
            "wrap" => a.wrap = b,
            "lines" => a.lines = b,
            "ws" => a.ws = b,
            "fold" => a.fold = b,
            "spell" => a.spell = b,
            "chrome" => a.chrome = v.parse().ok().filter(|&x| x < 2).unwrap_or(0),
            "spell_lang" if crate::spell::LANGS.iter().any(|l| l.0 == v) => a.spell_lang = v.to_string(),
            "autoupdate" => a.auto_update = b,
            "font" => a.font = v.to_string(),
            "size" => a.size = v.parse().unwrap_or(a.size).clamp(6, 72),
            "margin" => a.margin = v.parse().unwrap_or(a.margin).clamp(0, 400),
            "spacing" => a.spacing = v.parse().unwrap_or(0).clamp(0, 30),
            "root" if std::path::Path::new(v).is_dir() => a.root = Some(v.to_string()),
            _ => {}
        }
    }
}

pub fn save_settings() {
    let a = app();
    let _ = std::fs::create_dir_all(cfg_dir());
    let s = format!(
        "theme={}\nlayout={}\nwrap={}\nlines={}\nws={}\nfold={}\nchrome={}\nspell={}\nspell_lang={}\nautoupdate={}\nfont={}\nsize={}\nmargin={}\nspacing={}\nroot={}\n",
        a.theme, a.layout, a.wrap as u8, a.lines as u8, a.ws as u8, a.fold as u8, a.chrome, a.spell as u8, a.spell_lang, a.auto_update as u8, a.font, a.size, a.margin, a.spacing, a.root.clone().unwrap_or_default()
    );
    let _ = std::fs::write(cfg_dir().join("settings.ini"), s);
}

/// Sortie « à chaud » : les onglets non enregistrés sont conservés et restaurés au prochain lancement.
pub fn save_session() {
    let a = app();
    if a.nosession {
        return;
    }
    let d = cfg_dir().join("session");
    let _ = std::fs::remove_dir_all(&d);
    if std::fs::create_dir_all(&d).is_err() {
        return;
    }
    let mut lines = vec![format!("cur\t{}", a.cur)];
    for (i, t) in a.docs.iter().enumerate() {
        if t.path.is_none() && !t.dirty {
            continue;
        }
        let enc = Enc::ALL.iter().position(|&e| e == t.enc).unwrap_or(0);
        let backup = if t.dirty || t.path.is_none() {
            let n = format!("{i}.txt");
            if std::fs::write(d.join(&n), mtext(t.sci)).is_err() {
                continue;
            }
            n
        } else {
            String::new()
        };
        lines.push(format!("doc\t{}\t{}\t{}\t{}\t{}", t.path.clone().unwrap_or_default(), enc, t.dirty as u8, t.untitled, backup));
    }
    let _ = std::fs::write(d.join("session.txt"), lines.join("\n"));
}

pub fn restore_session() -> bool {
    let d = cfg_dir().join("session");
    let Ok(s) = std::fs::read_to_string(d.join("session.txt")) else { return false };
    let a = app();
    let (mut cur, mut n) = (0usize, 0);
    for l in s.lines() {
        let f: Vec<&str> = l.split('\t').collect();
        match f.as_slice() {
            ["cur", c] => cur = c.parse().unwrap_or(0),
            ["doc", path, enc, dirty, untitled, backup] => {
                let enc = Enc::ALL.get(enc.parse().unwrap_or(0)).copied().unwrap_or(Enc::Utf8);
                let path = (!path.is_empty()).then(|| path.to_string());
                let text = if backup.is_empty() {
                    match path.as_ref().and_then(|p| std::fs::read(p).ok()) {
                        Some(b) => doc::decode(&b).0,
                        None => continue,
                    }
                } else {
                    std::fs::read(d.join(backup)).unwrap_or_default()
                };
                new_tab(path, Some((text, enc)), *dirty == "1", untitled.parse().unwrap_or(0));
                n += 1;
            }
            _ => {}
        }
    }
    if n > 0 {
        if a.docs.len() > n && a.docs[0].path.is_none() && !a.docs[0].dirty && mtext(a.docs[0].sci).is_empty() {
            drop_tab(0);
            cur = cur.saturating_sub(0);
        }
        switch_to(cur.min(a.docs.len() - 1));
    }
    n > 0
}

fn confirm_lossy(name: &str) -> bool {
    let a = app();
    let d = gtk::MessageDialog::new(Some(&a.win), gtk::DialogFlags::MODAL, gtk::MessageType::Warning, gtk::ButtonsType::YesNo, &format!("Certains caractères n'existent pas en {name} et seront remplacés par « ? ». Enregistrer quand même ?"));
    let r = d.run();
    d.close();
    r == gtk::ResponseType::Yes
}
