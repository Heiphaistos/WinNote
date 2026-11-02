//! Aperçu Markdown, vue scindée, orthographe différée et menu contextuel de l'éditeur.
use crate::mdrender::{render, BOLD, BOLDITAL, BULLET, CODE, CODEBLK, DIM, ITAL, LINK, QUOTE};
use crate::sci::{self, send, HWND};
use crate::theme::{blend, THEMES};
use crate::{app, cstr, edit, spell};
use gtk::prelude::*;
use std::cell::Cell;

thread_local! {
    static PENDING: Cell<bool> = const { Cell::new(false) };
}

/// Configure la vue d'aperçu (lecture seule, sans marges, sans curseur).
pub fn setup_preview(p: HWND) {
    send(p, sci::SETCODEPAGE, 65001, 0);
    send(p, sci::SETWRAP, 1, 0);
    send(p, sci::SETILEXER, 0, 0);
    for m in 0..3 {
        send(p, sci::SETMARGINW, m, 0);
    }
    send(p, sci::SETCARETLINEVIS, 0, 0);
    send(p, 2512, 0, 0); // SCI_SETCARETSTYLE = invisible
    send(p, sci::USEPOPUP, 0, 0);
}

pub fn restyle() {
    let a = app();
    let (p, t) = (a.prev, &THEMES[a.theme]);
    let font = cstr(edit::linux_font("Segoe UI"));
    let size = a.size.max(9);
    send(p, sci::STYLESETFONT, sci::STYLE_DEFAULT, font.as_ptr() as isize);
    send(p, sci::STYLESETSIZE, sci::STYLE_DEFAULT, size as isize);
    send(p, sci::STYLESETFORE, sci::STYLE_DEFAULT, t.fg as isize);
    send(p, sci::STYLESETBACK, sci::STYLE_DEFAULT, blend(t.bg, t.ui_bg, 18) as isize);
    send(p, sci::STYLECLEARALL, 0, 0);
    let set = |st: usize, fore: u32, bold: bool, ital: bool, sz: i32| {
        send(p, sci::STYLESETFORE, st, fore as isize);
        send(p, sci::STYLESETBOLD, st, bold as isize);
        send(p, sci::STYLESETITALIC, st, ital as isize);
        if sz != 0 {
            send(p, sci::STYLESETSIZE, st, sz as isize);
        }
    };
    for (i, m) in [200, 165, 140, 120, 108, 100].iter().enumerate() {
        set(i + 1, t.accent, true, false, size * m / 100);
    }
    set(BOLD as usize, t.fg, true, false, 0);
    set(ITAL as usize, t.fg, false, true, 0);
    set(BOLDITAL as usize, t.fg, true, true, 0);
    set(CODE as usize, t.c[1], false, false, 0);
    set(CODEBLK as usize, t.c[1], false, false, 0);
    set(QUOTE as usize, blend(t.fg, t.bg, 30), false, true, 0);
    set(LINK as usize, t.accent, false, false, 0);
    send(p, 2059, LINK as usize, 1); // SCI_STYLESETUNDERLINE
    set(DIM as usize, blend(t.fg, t.bg, 55), false, false, 0);
    set(BULLET as usize, t.accent, true, false, 0);
    for st in [CODE, CODEBLK] {
        send(p, sci::STYLESETFONT, st as usize, cstr(edit::linux_font("Consolas")).as_ptr() as isize);
        send(p, sci::STYLESETBACK, st as usize, blend(t.bg, t.fg, 8) as isize);
    }
    send(p, sci::SETMARGINLEFT, 0, 28);
    send(p, sci::SETMARGINRIGHT, 0, 28);
    // vue scindée : même style que l'éditeur
    if !a.docs.is_empty() {
        edit::style_view(a.sci2, a.docs[a.cur].lang);
    }
}

pub fn update_preview() {
    let a = app();
    if !a.preview || a.docs.is_empty() {
        return;
    }
    let md = crate::lang::LANGS[a.docs[a.cur].lang].lexer == "markdown";
    let r = if md { render(&String::from_utf8_lossy(&edit::text_of(a.cur))) } else { "Aperçu disponible pour les fichiers Markdown (.md).".chars().map(|c| (c, DIM)).collect() };
    let (mut text, mut styles) = (Vec::with_capacity(r.len()), Vec::with_capacity(r.len()));
    for (c, s) in r {
        let mut b = [0u8; 4];
        let e = c.encode_utf8(&mut b).as_bytes();
        text.extend_from_slice(e);
        styles.extend(std::iter::repeat(s).take(e.len()));
    }
    text.push(0);
    let p = a.prev;
    let first = send(p, sci::GETFIRSTVISIBLE, 0, 0);
    send(p, sci::SETREADONLY, 0, 0);
    send(p, sci::SETTEXT, 0, text.as_ptr() as isize);
    send(p, sci::STARTSTYLING, 0, 0);
    send(p, sci::SETSTYLINGEX, styles.len(), styles.as_ptr() as isize);
    send(p, sci::SETREADONLY, 1, 0);
    send(p, sci::SETFIRSTVISIBLE, first as usize, 0);
}

/// La vue scindée affiche le même document que l'onglet courant.
pub fn sync_split() {
    let a = app();
    if !a.split || a.docs.is_empty() {
        return;
    }
    let d = send(a.sci, 2357, 0, 0); // SCI_GETDOCPOINTER
    send(a.sci2, 2358, 0, d); // SCI_SETDOCPOINTER
    edit::style_view(a.sci2, a.docs[a.cur].lang);
}

/// Affiche / masque le volet de droite selon les modes aperçu et scindé.
pub fn relayout() {
    let a = app();
    let on = a.preview || a.split;
    let was_shown = a.rstack.is_visible();
    if on {
        a.rstack.show_all();
        a.rstack.set_visible_child_name(if a.split { "split" } else { "prev" });
        if !was_shown {
            // la moitié de la place, une fois la fenêtre redessinée
            gtk::glib::idle_add_local_once(|| {
                let a = app();
                a.psplit.set_position(a.psplit.allocated_width() / 2);
            });
        }
    } else {
        a.rstack.hide();
    }
    sync_split();
    update_preview();
    crate::ui::sync_checks();
}

pub fn toggle_preview() {
    let a = app();
    a.preview = !a.preview;
    if a.preview {
        a.split = false;
    }
    relayout();
}

pub fn toggle_split() {
    let a = app();
    a.split = !a.split;
    if a.split {
        a.preview = false;
    }
    relayout();
}

/// Changement d'onglet ou de document : remet panneaux et orthographe à jour.
pub fn on_tab_changed() {
    sync_split();
    update_preview();
    crate::side::refresh();
    spell::check_visible();
}

/// Mise à jour différée (aperçu, plan, orthographe) après une modification ou un défilement.
pub fn schedule() {
    let a = app();
    if !(a.preview || a.spell || (a.side.root.is_visible() && a.side.mode == 1)) || PENDING.with(|p| p.replace(true)) {
        return;
    }
    gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(400), || {
        PENDING.with(|p| p.set(false));
        update_preview();
        spell::check_visible();
        if app().side.mode == 1 {
            crate::side::refresh();
        }
    });
}

/// Menu contextuel de l'éditeur (suggestions d'orthographe, puis commandes d'édition).
pub fn context_menu(ev: &gtk::gdk::EventButton) {
    use crate::ids::*;
    let a = app();
    let (x, y) = ev.position();
    let pos = send(a.sci, 2023, x as usize, y as isize); // SCI_POSITIONFROMPOINTCLOSE
    let m = gtk::Menu::new();
    let sug = if pos >= 0 { spell::prepare_at(pos) } else { Vec::new() };
    if spell::has_pending() {
        for (i, s) in sug.iter().enumerate() {
            let it = gtk::MenuItem::with_label(s);
            it.connect_activate(move |_| spell::apply_fix(i));
            m.append(&it);
        }
        if sug.is_empty() {
            let it = gtk::MenuItem::with_label("(aucune suggestion)");
            it.set_sensitive(false);
            m.append(&it);
        }
        for (l, add) in [("Ajouter au dictionnaire", true), ("Ignorer", false)] {
            let it = gtk::MenuItem::with_label(l);
            it.connect_activate(move |_| spell::learn(add));
            m.append(&it);
        }
        m.append(&gtk::SeparatorMenuItem::new());
    }
    for (l, id) in [("Annuler", UNDO), ("Rétablir", REDO), ("", 0), ("Couper", CUT), ("Copier", COPY), ("Coller", PASTE), ("", 0), ("Tout sélectionner", SELALL), ("Ouvrir le lien", OPENLINK)] {
        if l.is_empty() {
            m.append(&gtk::SeparatorMenuItem::new());
            continue;
        }
        let it = gtk::MenuItem::with_label(l);
        it.connect_activate(move |_| crate::ui::on_command(id));
        m.append(&it);
    }
    m.show_all();
    m.popup_at_pointer(Some(ev));
}

/// Branche le menu contextuel sur un widget Scintilla d'onglet.
pub fn hook_tab(w: &gtk::Widget, s: HWND) {
    send(s, sci::USEPOPUP, 0, 0);
    w.connect_button_press_event(|_, e| {
        if e.button() == 3 {
            context_menu(e);
            return gtk::glib::Propagation::Stop;
        }
        gtk::glib::Propagation::Proceed
    });
}

pub fn toggle_readonly() {
    let a = app();
    let r = send(a.sci, sci::GETREADONLY, 0, 0) == 0;
    send(a.sci, sci::SETREADONLY, r as usize, 0);
    crate::ui::sync_checks();
}

pub fn toggle_topmost() {
    let a = app();
    a.topmost = !a.topmost;
    a.win.set_keep_above(a.topmost);
    crate::ui::sync_checks();
}

pub fn new_window() {
    let a = app();
    let d = &a.docs[a.cur];
    match d.path.clone() {
        Some(p) if !d.dirty => {
            if std::env::current_exe().ok().and_then(|e| std::process::Command::new(e).arg(&p).spawn().ok()).is_some() {
                edit::close_tab(a.cur);
            }
        }
        _ => edit::msg_box("Enregistrez d'abord le fichier pour l'ouvrir dans une nouvelle fenêtre."),
    }
}
