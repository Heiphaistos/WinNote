//! Barre de recherche / remplacement (regex, mot entier, casse, surlignage de toutes les occurrences).
use crate::edit::msg_box;
use crate::sci::{self, send};
use crate::theme::THEMES;
use crate::app;
use gtk::prelude::*;

const INDIC: usize = 8;

pub struct Bar {
    pub root: gtk::Box,
    pub find: gtk::Entry,
    pub repl: gtk::Entry,
    pub case: gtk::ToggleButton,
    pub word: gtk::ToggleButton,
    pub regex: gtk::ToggleButton,
    pub count: gtk::Label,
    pub repl_row: gtk::Box,
}

impl Bar {
    pub fn new() -> Bar {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 4);
        root.set_border_width(6);
        let row1 = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let find = gtk::Entry::new();
        find.set_placeholder_text(Some("Rechercher"));
        find.set_width_chars(34);
        let mk = |t: &str, tip: &str| {
            let b = gtk::ToggleButton::with_label(t);
            b.set_tooltip_text(Some(tip));
            b
        };
        let (case, word, regex) = (mk("Aa", "Respecter la casse"), mk("ab", "Mot entier"), mk(".*", "Expression régulière"));
        let prev = gtk::Button::with_label("↑");
        let next = gtk::Button::with_label("↓");
        let close = gtk::Button::from_icon_name(Some("window-close-symbolic"), gtk::IconSize::Menu);
        close.set_relief(gtk::ReliefStyle::None);
        let count = gtk::Label::new(None);
        for w in [find.upcast_ref::<gtk::Widget>(), case.upcast_ref(), word.upcast_ref(), regex.upcast_ref(), prev.upcast_ref(), next.upcast_ref(), count.upcast_ref()] {
            row1.pack_start(w, false, false, 0);
        }
        row1.pack_end(&close, false, false, 0);
        let repl_row = gtk::Box::new(gtk::Orientation::Horizontal, 4);
        let repl = gtk::Entry::new();
        repl.set_placeholder_text(Some("Remplacer par"));
        repl.set_width_chars(34);
        let one = gtk::Button::with_label("Remplacer");
        let all = gtk::Button::with_label("Tout");
        repl_row.pack_start(&repl, false, false, 0);
        repl_row.pack_start(&one, false, false, 0);
        repl_row.pack_start(&all, false, false, 0);
        root.pack_start(&row1, false, false, 0);
        root.pack_start(&repl_row, false, false, 0);

        find.connect_changed(|_| {
            find_from(true, true);
        });
        find.connect_activate(|_| {
            find_from(false, true);
        });
        repl.connect_activate(|_| replace_one());
        prev.connect_clicked(|_| {
            find_from(false, false);
        });
        next.connect_clicked(|_| {
            find_from(false, true);
        });
        one.connect_clicked(|_| replace_one());
        all.connect_clicked(|_| replace_all());
        close.connect_clicked(|_| close_bar());
        for t in [&case, &word, &regex] {
            t.connect_toggled(|_| {
                find_from(true, true);
            });
        }
        find.connect_key_press_event(|_, e| {
            if e.keyval() == gtk::gdk::keys::constants::Escape {
                close_bar();
                return gtk::glib::Propagation::Stop;
            }
            gtk::glib::Propagation::Proceed
        });
        Bar { root, find, repl, case, word, regex, count, repl_row }
    }

    pub fn hide(&self) {
        self.root.hide();
    }
}

fn flags() -> usize {
    let b = &app().find;
    (if b.case.is_active() { 4 } else { 0 }) | (if b.word.is_active() { 2 } else { 0 }) | (if b.regex.is_active() { 0x0020_0000 | 0x0080_0000 } else { 0 })
}

fn query() -> Vec<u8> {
    app().find.find.text().as_bytes().to_vec()
}

fn search(st: isize, en: isize, what: &[u8]) -> isize {
    let s = app().sci;
    send(s, sci::SETTARGETSTART, st as usize, 0);
    send(s, sci::SETTARGETEND, en as usize, 0);
    // le moteur regex C++11 lit un texte terminé par un zéro
    let mut z = what.to_vec();
    z.push(0);
    send(s, sci::SEARCHINTARGET, what.len(), z.as_ptr() as isize)
}

pub fn find_from(from_sel_start: bool, down: bool) -> bool {
    let what = query();
    let s = app().sci;
    if what.is_empty() {
        mark_all();
        return false;
    }
    let len = send(s, sci::GETLENGTH, 0, 0);
    send(s, sci::SETSEARCHFLAGS, flags(), 0);
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    let mut r = if down { search(if from_sel_start { ss } else { se }, len, &what) } else { search(ss, 0, &what) };
    if r < 0 {
        r = if down { search(0, len, &what) } else { search(len, 0, &what) };
    }
    if r >= 0 {
        let (ts, te) = (send(s, sci::GETTARGETSTART, 0, 0), send(s, sci::GETTARGETEND, 0, 0));
        send(s, sci::SETSEL, ts as usize, te);
    }
    mark_all();
    r >= 0
}

pub fn find_next(down: bool) {
    if !app().find.root.is_visible() || query().is_empty() {
        open(false);
        return;
    }
    find_from(false, down);
}

pub fn mark_all() {
    let a = app();
    let s = a.sci;
    let len = send(s, sci::GETLENGTH, 0, 0);
    send(s, sci::SETINDICCURRENT, INDIC, 0);
    send(s, sci::INDICCLEAR, 0, len);
    let mut n = 0;
    let what = query();
    if !what.is_empty() && a.find.root.is_visible() {
        send(s, sci::SETSEARCHFLAGS, flags(), 0);
        let mut pos = 0;
        while n < 5000 {
            if search(pos, len, &what) < 0 {
                break;
            }
            let (ts, te) = (send(s, sci::GETTARGETSTART, 0, 0), send(s, sci::GETTARGETEND, 0, 0));
            send(s, sci::INDICFILL, ts as usize, te - ts);
            n += 1;
            pos = if te > ts { te } else { te + 1 };
            if pos >= len {
                break;
            }
        }
    }
    a.find.count.set_text(&if what.is_empty() { String::new() } else if n == 0 { "Aucun résultat".into() } else { format!("{n} résultat(s)") });
}

pub fn style_indicator() {
    let a = app();
    let t = &THEMES[a.theme];
    for d in &a.docs {
        send(d.sci, sci::INDICSETSTYLE, INDIC, 8);
        send(d.sci, sci::INDICSETFORE, INDIC, t.accent as isize);
        send(d.sci, sci::INDICSETALPHA, INDIC, 70);
        send(d.sci, sci::INDICSETOUTLINEALPHA, INDIC, 160);
    }
}

fn replace_target(rep: &[u8]) {
    let s = app().sci;
    let m = if app().find.regex.is_active() { sci::REPLACETARGETRE } else { sci::REPLACETARGET };
    let mut z = rep.to_vec();
    z.push(0);
    send(s, m, rep.len(), z.as_ptr() as isize);
}

pub fn replace_one() {
    let a = app();
    let s = a.sci;
    let what = query();
    let rep = a.find.repl.text().as_bytes().to_vec();
    send(s, sci::SETSEARCHFLAGS, flags(), 0);
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    if !what.is_empty() && search(ss, se, &what) == ss && send(s, sci::GETTARGETEND, 0, 0) == se {
        replace_target(&rep);
        let e = send(s, sci::GETTARGETEND, 0, 0);
        send(s, sci::SETSEL, e as usize, e);
    }
    find_from(false, true);
}

pub fn replace_all() {
    let a = app();
    let s = a.sci;
    let what = query();
    if what.is_empty() {
        return;
    }
    let rep = a.find.repl.text().as_bytes().to_vec();
    send(s, sci::SETSEARCHFLAGS, flags(), 0);
    send(s, sci::BEGINUNDO, 0, 0);
    let (mut pos, mut n) = (0isize, 0);
    loop {
        let len = send(s, sci::GETLENGTH, 0, 0);
        if pos > len || search(pos, len, &what) < 0 {
            break;
        }
        replace_target(&rep);
        let e = send(s, sci::GETTARGETEND, 0, 0);
        pos = if e == send(s, sci::GETTARGETSTART, 0, 0) { e + 1 } else { e };
        n += 1;
    }
    send(s, sci::ENDUNDO, 0, 0);
    mark_all();
    msg_box(&format!("{n} remplacement(s) effectué(s)."));
}

pub fn open(replace: bool) {
    let a = app();
    let s = a.sci;
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    if se > ss && se - ss < 200 {
        let mut b = vec![0u8; (se - ss) as usize + 1];
        send(s, sci::GETSELTEXT, 0, b.as_mut_ptr() as isize);
        let t = String::from_utf8_lossy(&b[..b.len() - 1]).to_string();
        if !t.contains('\n') && !t.contains('\r') {
            a.find.find.set_text(&t);
        }
    }
    a.find.root.show();
    if replace { a.find.repl_row.show() } else { a.find.repl_row.hide() }
    a.find.find.grab_focus();
    a.find.find.select_region(0, -1);
    mark_all();
}

pub fn close_bar() {
    let a = app();
    a.find.root.hide();
    for d in &a.docs {
        send(d.sci, sci::SETINDICCURRENT, INDIC, 0);
        send(d.sci, sci::INDICCLEAR, 0, send(d.sci, sci::GETLENGTH, 0, 0));
    }
    unsafe { gtk::ffi::gtk_widget_grab_focus(a.sci as *mut _) };
}
