//! Fenêtre, menus, thème (CSS), palette de commandes et répartition des commandes (Linux / GTK).
use crate::doc::Enc;
use crate::edit::{self, msg_box};
use crate::find::{self, Bar};
use crate::ids::*;
use crate::lang::LANGS;
use crate::sci::{self, send};
use crate::theme::{LAYOUTS, THEMES};
use crate::{app, cstr, textops, App};
use gtk::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

thread_local! {
    static CHECKS: RefCell<Vec<(usize, gtk::CheckMenuItem)>> = const { RefCell::new(Vec::new()) };
}

fn hex(c: u32) -> String {
    // COLORREF (BGR) -> #RRGGBB
    format!("#{:02x}{:02x}{:02x}", c & 0xFF, (c >> 8) & 0xFF, (c >> 16) & 0xFF)
}

pub fn apply_css() {
    let a = app();
    let t = &THEMES[a.theme];
    let (bg, fg, ui, uifg, acc) = (hex(t.bg), hex(t.fg), hex(t.ui_bg), hex(t.ui_fg), hex(t.accent));
    let css = format!(
        "window, .background {{ background-color: {ui}; color: {uifg}; }}
         menubar, menu, menuitem, menu menuitem {{ background-color: {ui}; color: {uifg}; background-image: none; }}
         menuitem:hover {{ background-color: {acc}; color: {bg}; }}
         notebook > header {{ background-color: {ui}; border: none; }}
         notebook > header > tabs > tab {{ background-color: {ui}; color: {uifg}; padding: 5px 10px; border: none; border-radius: 8px 8px 0 0; }}
         notebook > header > tabs > tab:checked {{ background-color: {bg}; color: {fg}; box-shadow: inset 0 -2px {acc}; }}
         entry {{ background-color: {bg}; color: {fg}; border-radius: 6px; }}
         button, togglebutton {{ background-image: none; background-color: {ui}; color: {uifg}; border: 1px solid {acc}; border-radius: 6px; }}
         togglebutton:checked {{ background-color: {acc}; color: {bg}; }}
         notebook button {{ border: none; background: transparent; padding: 0 2px; min-height: 0; min-width: 0; }}
         label {{ color: {uifg}; }}
         #statusbar {{ background-color: {ui}; padding: 3px 12px; }}
         list, row {{ background-color: {ui}; color: {uifg}; }}
         row:selected {{ background-color: {acc}; color: {bg}; }}"
    );
    let _ = a.css.load_from_data(css.as_bytes());
    if let Some(s) = gtk::Settings::default() {
        s.set_gtk_application_prefer_dark_theme(t.dark);
    }
}

pub fn apply_all() {
    apply_css();
    edit::restyle_all();
    sync_checks();
    edit::refresh();
}

/// set_active() d'une CheckMenuItem émet « activate » : on neutralise le gestionnaire pendant la synchronisation.
static mut SYNCING: bool = false;

fn sync_checks() {
    let a = app();
    unsafe { SYNCING = true };
    CHECKS.with(|c| {
        for (id, item) in c.borrow().iter() {
            let on = match *id {
                WRAP => a.wrap,
                LINES => a.lines,
                WS => a.ws,
                FOLDMARGIN => a.fold,
                _ => item.is_active(),
            };
            if item.is_active() != on {
                item.set_active(on);
            }
        }
    });
    unsafe { SYNCING = false };
}

pub fn update_status() {
    let a = app();
    if a.docs.is_empty() || a.cur >= a.docs.len() {
        return;
    }
    let s = a.sci;
    let pos = send(s, sci::GETCURRENTPOS, 0, 0);
    let (ln, col) = (send(s, sci::LINEFROMPOS, pos as usize, 0) + 1, send(s, sci::GETCOLUMN, pos as usize, 0) + 1);
    let len = send(s, sci::GETLENGTH, 0, 0);
    let chars = if len < 8_000_000 { send(s, 2633, 0, len) } else { len };
    let d = &a.docs[a.cur];
    let eol = match send(s, sci::GETEOLMODE, 0, 0) { 0 => "Windows (CRLF)", 1 => "Macintosh (CR)", _ => "Unix (LF)" };
    a.status.set_text(&format!("Ln {ln}, Col {col}      {chars} caractères      {eol}      {}      {}", d.enc.name(), LANGS[d.lang].name));
}

pub fn apply_layout(i: usize) {
    let a = app();
    let l = &LAYOUTS[i];
    (a.layout, a.font, a.size, a.wrap, a.lines, a.margin, a.fold, a.spacing) = (i, l.font.into(), l.size, l.wrap, l.lines, l.margin, l.fold, l.spacing);
    if let Some(t) = l.theme {
        a.theme = t;
    }
    for d in &a.docs {
        send(d.sci, sci::SETZOOM, 0, 0);
    }
    apply_all();
}

fn fuzzy(q: &str, text: &str) -> Option<i32> {
    if q.is_empty() {
        return Some(0);
    }
    let (t, mut score, mut last) = (text.to_lowercase(), 0i32, -2i32);
    let mut it = t.chars().enumerate();
    for qc in q.to_lowercase().chars() {
        loop {
            let (i, c) = it.next()?;
            if c == qc {
                score += if i as i32 == last + 1 { 8 } else { 2 } - (i as i32 / 16);
                last = i as i32;
                break;
            }
        }
    }
    Some(score)
}

#[derive(Clone)]
enum Act {
    Cmd(usize),
    Open(String),
    Tab(usize),
}

/// Commandes disponibles dans la version Linux (le reste de la table est propre à Windows).
fn supported(id: usize) -> bool {
    !matches!(id, REGISTER | UNREGISTER | AUTOUPDATE | CHROME_WIN | CHROME_MAC | SIDEBAR | OUTLINE | PREVIEW | SPLIT | SPELL | BACKLINKS | NEWWINDOW | TOPMOST | PRINT | READONLY | DWRITE)
}

fn palette(items: Vec<(String, String, Act)>, title: &str) {
    let a = app();
    let d = gtk::Dialog::with_buttons(Some(title), Some(&a.win), gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT, &[]);
    d.set_default_size(620, 420);
    let entry = gtk::SearchEntry::new();
    let list = gtk::ListBox::new();
    let sw = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
    sw.add(&list);
    sw.set_vexpand(true);
    let area = d.content_area();
    area.pack_start(&entry, false, false, 6);
    area.pack_start(&sw, true, true, 0);
    let items = Rc::new(items);
    let shown: Rc<RefCell<Vec<usize>>> = Rc::new(RefCell::new(Vec::new()));
    let chosen: Rc<RefCell<Option<Act>>> = Rc::new(RefCell::new(None));
    let refill = {
        let (list, items, shown, entry) = (list.clone(), items.clone(), shown.clone(), entry.clone());
        move || {
            for c in list.children() {
                list.remove(&c);
            }
            let q = entry.text().to_string();
            let mut sc: Vec<(i32, usize)> = items.iter().enumerate().filter_map(|(i, it)| fuzzy(&q, &it.0).or_else(|| fuzzy(&q, &it.1).map(|s| s - 20)).map(|s| (s, i))).collect();
            sc.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            *shown.borrow_mut() = sc.iter().map(|x| x.1).take(200).collect();
            for &i in shown.borrow().iter() {
                let row = gtk::Label::new(None);
                row.set_markup(&format!("{}   <small>{}</small>", gtk::glib::markup_escape_text(&items[i].0), gtk::glib::markup_escape_text(&items[i].1)));
                row.set_xalign(0.0);
                row.set_margin(6);
                list.add(&row);
            }
            list.show_all();
            if let Some(r) = list.row_at_index(0) {
                list.select_row(Some(&r));
            }
        }
    };
    refill();
    {
        let refill = refill.clone();
        entry.connect_search_changed(move |_| refill());
    }
    let pick = {
        let (items, shown, chosen, d) = (items.clone(), shown.clone(), chosen.clone(), d.clone());
        move |idx: i32| {
            if let Some(&i) = shown.borrow().get(idx as usize) {
                *chosen.borrow_mut() = Some(items[i].2.clone());
                d.response(gtk::ResponseType::Accept);
            }
        }
    };
    {
        let pick = pick.clone();
        list.connect_row_activated(move |_, r| pick(r.index()));
    }
    {
        let (pick, list) = (pick.clone(), list.clone());
        entry.connect_activate(move |_| pick(list.selected_row().map_or(0, |r| r.index())));
    }
    {
        let list = list.clone();
        entry.connect_key_press_event(move |_, e| {
            use gtk::gdk::keys::constants as k;
            let cur = list.selected_row().map_or(-1, |r| r.index());
            let target = if e.keyval() == k::Down { cur + 1 } else if e.keyval() == k::Up { cur - 1 } else { return gtk::glib::Propagation::Proceed };
            if let Some(r) = list.row_at_index(target.max(0)) {
                list.select_row(Some(&r));
            }
            gtk::glib::Propagation::Stop
        });
    }
    d.show_all();
    d.run();
    d.close();
    let act = chosen.borrow().clone();
    match act {
        Some(Act::Cmd(id)) => on_command(id),
        Some(Act::Open(p)) => edit::open_path(&p),
        Some(Act::Tab(i)) => edit::switch_to(i),
        None => {}
    }
}

fn open_palette() {
    let items = PALETTE_CMDS.iter().filter(|c| supported(c.0)).map(|&(id, n)| (n.to_string(), String::new(), Act::Cmd(id))).collect();
    palette(items, "Palette de commandes");
}

fn quick_open() {
    let a = app();
    let mut v: Vec<(String, String, Act)> = a.docs.iter().enumerate().map(|(i, t)| (edit::title_of(t), t.path.clone().unwrap_or_else(|| "onglet ouvert".into()), Act::Tab(i))).collect();
    let mut seen: std::collections::HashSet<String> = a.docs.iter().filter_map(|t| t.path.clone()).collect();
    for p in &a.recent {
        if seen.insert(p.clone()) {
            v.push((p.rsplit('/').next().unwrap_or(p).to_string(), format!("récent · {p}"), Act::Open(p.clone())));
        }
    }
    if let Some(r) = a.root.clone() {
        let mut files = Vec::new();
        walk(std::path::Path::new(&r), &mut files, 0);
        for f in files {
            let s = f.to_string_lossy().to_string();
            if seen.insert(s.clone()) {
                v.push((f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), s.strip_prefix(r.as_str()).unwrap_or(&s).trim_start_matches('/').to_string(), Act::Open(s)));
            }
        }
    }
    palette(v, "Ouverture rapide");
}

fn walk(root: &std::path::Path, out: &mut Vec<std::path::PathBuf>, depth: u32) {
    if depth > 6 || out.len() > 5000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let n = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if !matches!(n.as_str(), ".git" | "node_modules" | "target" | ".logs" | "dist" | "build") {
                walk(&p, out, depth + 1);
            }
        } else {
            out.push(p);
        }
    }
}

fn goto_line() {
    let a = app();
    let max = send(a.sci, sci::GETLINECOUNT, 0, 0) as f64;
    let d = gtk::Dialog::with_buttons(Some("Aller à la ligne"), Some(&a.win), gtk::DialogFlags::MODAL, &[("Annuler", gtk::ResponseType::Cancel), ("Aller", gtk::ResponseType::Accept)]);
    let spin = gtk::SpinButton::with_range(1.0, max, 1.0);
    spin.set_activates_default(true);
    d.set_default_response(gtk::ResponseType::Accept);
    d.content_area().pack_start(&spin, false, false, 10);
    d.show_all();
    if d.run() == gtk::ResponseType::Accept {
        send(a.sci, sci::GOTOLINE, spin.value() as usize - 1, 0);
    }
    d.close();
}

fn choose_font() {
    let a = app();
    let d = gtk::FontChooserDialog::new(Some("Police"), Some(&a.win));
    d.set_font(&format!("{} {}", linux_font(&a.font), a.size));
    if d.run() == gtk::ResponseType::Ok {
        if let Some(desc) = d.font_desc() {
            a.font = desc.family().map(|f| f.to_string()).unwrap_or(a.font.clone());
            a.size = (desc.size() / gtk::pango::SCALE).max(6);
            apply_all();
        }
    }
    d.close();
}

fn linux_font(f: &str) -> &str {
    match f {
        "Consolas" => "Monospace",
        "Segoe UI" => "Sans",
        "Georgia" => "Serif",
        o => o,
    }
}

fn stats() {
    let a = app();
    let s = a.sci;
    let len = send(s, sci::GETLENGTH, 0, 0);
    let text = String::from_utf8_lossy(&edit::text_of(a.cur)).to_string();
    let words = text.split_whitespace().count();
    msg_box(&format!(
        "Lignes : {}\nMots : {}\nCaractères : {}\nOctets (UTF-8) : {}\nTemps de lecture : environ {} min",
        send(s, sci::GETLINECOUNT, 0, 0), words, text.chars().count(), len, (words / 220).max(1)
    ));
}

fn bookmark(which: usize) {
    let a = app();
    if which == BOOKTOGGLE {
        send(a.sci, sci::SETMARGINW, 1, 14);
        textops::bookmark_toggle();
    } else {
        textops::bookmark_step(which == BOOKNEXT);
    }
}

pub fn on_command(id: usize) {
    let a = app();
    let s = a.sci;
    let doc_i = a.cur;
    match id {
        NEW => edit::new_tab(None, None, false, 0),
        OPEN => {
            for p in edit::pick_files() {
                edit::open_path(&p);
            }
        }
        OPENFOLDER => {
            if let Some(p) = edit::pick_folder() {
                a.root = Some(p);
            }
        }
        SAVE => drop(edit::save_doc(doc_i, false)),
        SAVEAS => drop(edit::save_doc(doc_i, true)),
        SAVEALL => {
            for i in 0..a.docs.len() {
                if a.docs[i].dirty && !edit::save_doc(i, false) {
                    break;
                }
            }
        }
        CLOSE => edit::close_tab(doc_i),
        CLOSEOTHERS => edit::close_others(doc_i),
        CLOSEALL => {
            while !(a.docs.len() == 1 && a.docs[0].path.is_none() && !a.docs[0].dirty) {
                let before = (a.docs.len(), a.docs[0].sci);
                edit::close_tab(0);
                if (a.docs.len(), a.docs[0].sci) == before {
                    break; // fermeture annulée
                }
            }
        }
        REOPEN => edit::reopen_closed(),
        EXIT => quit(),
        RELOAD => edit::reload(),
        REVEALFILE => {
            if let Some(p) = a.docs[doc_i].path.as_ref().and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf())) {
                let _ = std::process::Command::new("xdg-open").arg(p).spawn();
            }
        }
        COPYPATH => {
            if let (Some(p), Some(d)) = (a.docs[doc_i].path.clone(), gtk::gdk::Display::default()) {
                gtk::Clipboard::default(&d).map(|c| c.set_text(&p));
            }
        }
        UNDO => drop(send(s, sci::UNDO, 0, 0)),
        REDO => drop(send(s, sci::REDO, 0, 0)),
        CUT => drop(send(s, sci::CUT, 0, 0)),
        COPY => drop(send(s, sci::COPY, 0, 0)),
        PASTE => drop(send(s, sci::PASTE, 0, 0)),
        DEL => drop(send(s, sci::CLEAR, 0, 0)),
        SELALL => drop(send(s, sci::SELECTALL, 0, 0)),
        FIND => find::open(false),
        REPLACE => find::open(true),
        FINDNEXT => find::find_next(true),
        FINDPREV => find::find_next(false),
        GOTO => goto_line(),
        PALETTE => open_palette(),
        QUICKOPEN => quick_open(),
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
        SELNEXT => textops::select_next(),
        OPENLINK => textops::open_link(),
        STATS => stats(),
        BOOKTOGGLE | BOOKNEXT | BOOKPREV => bookmark(id),
        BOOKCLEAR => drop(send(s, sci::MARKERDELETEALL, 1, 0)),
        FOLDALL => drop(send(s, sci::FOLDALL, 0, 0)),
        UNFOLDALL => drop(send(s, sci::FOLDALL, 1, 0)),
        FOLDMARGIN => {
            a.fold = !a.fold;
            apply_all();
        }
        DATETIME => {
            let t = gtk::glib::DateTime::now_local().ok().and_then(|d| d.format("%H:%M %d/%m/%Y").ok()).map(|s| s.to_string()).unwrap_or_default();
            send(s, sci::REPLACESEL, 0, cstr(&t).as_ptr() as isize);
        }
        WRAP => {
            a.wrap = !a.wrap;
            apply_all();
        }
        LINES => {
            a.lines = !a.lines;
            apply_all();
        }
        WS => {
            a.ws = !a.ws;
            apply_all();
        }
        FONT => choose_font(),
        ZOOMIN => drop(send(s, sci::ZOOMIN, 0, 0)),
        ZOOMOUT => drop(send(s, sci::ZOOMOUT, 0, 0)),
        ZOOMRESET => drop(send(s, sci::SETZOOM, 0, 0)),
        TABNEXT => edit::switch_to((a.cur + 1) % a.docs.len()),
        TABPREV => edit::switch_to((a.cur + a.docs.len() - 1) % a.docs.len()),
        ZEN => {
            let full = a.win.window().map_or(false, |w| w.state().contains(gtk::gdk::WindowState::FULLSCREEN));
            if full { a.win.unfullscreen() } else { a.win.fullscreen() }
        }
        UPDATE => {
            let _ = std::process::Command::new("xdg-open").arg("https://github.com/Heiphaistos/WinNote/releases/latest").spawn();
        }
        ABOUT => msg_box(&format!("WinNote {}\nÉditeur de texte natif (Linux / GTK3).\nMoteur : Scintilla + Lexilla.", env!("CARGO_PKG_VERSION"))),
        EOL..=134 => {
            let m = id - EOL;
            send(s, sci::SETEOLMODE, m, 0);
            send(s, sci::CONVERTEOLS, m, 0);
            update_status();
        }
        200..=204 => {
            a.docs[doc_i].enc = Enc::ALL[id - ENC];
            edit::refresh();
        }
        300..=399 if id - LANG < LANGS.len() => {
            a.docs[doc_i].lang = id - LANG;
            edit::set_lexer(s, id - LANG);
            edit::style_view(s, id - LANG);
            edit::refresh();
        }
        400..=499 if id - THEME < THEMES.len() => {
            a.theme = id - THEME;
            apply_all();
        }
        500..=599 if id - LAYOUT < LAYOUTS.len() => apply_layout(id - LAYOUT),
        700..=719 => {
            if let Some(p) = a.recent.get(id - RECENT).cloned() {
                edit::open_path(&p);
            }
        }
        _ => {}
    }
}

pub fn quit() {
    edit::save_session();
    edit::save_settings();
    gtk::main_quit();
}

// ---------- Construction ----------

fn accel(item: &gtk::MenuItem, ag: &gtk::AccelGroup, spec: &str) {
    let (key, mods) = gtk::accelerator_parse(spec);
    if key != 0 {
        item.add_accelerator("activate", ag, key, mods, gtk::AccelFlags::VISIBLE);
    }
}

fn add(menu: &gtk::Menu, ag: &gtk::AccelGroup, label: &str, spec: Option<&str>, id: usize) {
    let it = gtk::MenuItem::with_label(label);
    if let Some(s) = spec {
        accel(&it, ag, s);
    }
    it.connect_activate(move |_| on_command(id));
    menu.append(&it);
}

fn check(menu: &gtk::Menu, ag: &gtk::AccelGroup, label: &str, spec: Option<&str>, id: usize) {
    let it = gtk::CheckMenuItem::with_label(label);
    if let Some(s) = spec {
        accel(it.upcast_ref(), ag, s);
    }
    it.connect_activate(move |_| {
        if unsafe { !SYNCING } {
            on_command(id)
        }
    });
    menu.append(&it);
    CHECKS.with(|c| c.borrow_mut().push((id, it)));
}

fn sub(bar: &gtk::MenuBar, label: &str) -> gtk::Menu {
    let m = gtk::Menu::new();
    let it = gtk::MenuItem::with_label(label);
    it.set_submenu(Some(&m));
    bar.append(&it);
    m
}

fn build_menu(ag: &gtk::AccelGroup) -> gtk::MenuBar {
    let bar = gtk::MenuBar::new();
    let f = sub(&bar, "Fichier");
    add(&f, ag, "Nouvel onglet", Some("<Control>n"), NEW);
    add(&f, ag, "Ouvrir...", Some("<Control>o"), OPEN);
    add(&f, ag, "Ouvrir un dossier...", None, OPENFOLDER);
    let rec = gtk::Menu::new();
    let rec_item = gtk::MenuItem::with_label("Fichiers récents");
    rec_item.set_submenu(Some(&rec));
    f.append(&rec_item);
    {
        let rec = rec.clone();
        rec_item.connect_activate(move |_| {
            for c in rec.children() {
                rec.remove(&c);
            }
            let a = app();
            if a.recent.is_empty() {
                let e = gtk::MenuItem::with_label("(aucun)");
                e.set_sensitive(false);
                rec.append(&e);
            }
            for (i, p) in a.recent.iter().enumerate().take(15) {
                let it = gtk::MenuItem::with_label(p);
                it.connect_activate(move |_| on_command(RECENT + i));
                rec.append(&it);
            }
            rec.show_all();
        });
    }
    add(&f, ag, "Rouvrir l'onglet fermé", Some("<Control><Shift>t"), REOPEN);
    f.append(&gtk::SeparatorMenuItem::new());
    add(&f, ag, "Enregistrer", Some("<Control>s"), SAVE);
    add(&f, ag, "Enregistrer sous...", Some("<Control><Shift>s"), SAVEAS);
    add(&f, ag, "Tout enregistrer", Some("<Control><Alt>s"), SAVEALL);
    add(&f, ag, "Recharger depuis le disque", None, RELOAD);
    f.append(&gtk::SeparatorMenuItem::new());
    add(&f, ag, "Afficher dans le gestionnaire de fichiers", None, REVEALFILE);
    add(&f, ag, "Copier le chemin", None, COPYPATH);
    f.append(&gtk::SeparatorMenuItem::new());
    add(&f, ag, "Fermer l'onglet", Some("<Control>w"), CLOSE);
    add(&f, ag, "Fermer les autres onglets", None, CLOSEOTHERS);
    add(&f, ag, "Quitter", Some("<Control>q"), EXIT);

    let e = sub(&bar, "Édition");
    for (l, i) in [("Annuler", UNDO), ("Rétablir", REDO), ("Couper", CUT), ("Copier", COPY), ("Coller", PASTE), ("Supprimer", DEL), ("Tout sélectionner", SELALL)] {
        add(&e, ag, l, None, i);
    }
    e.append(&gtk::SeparatorMenuItem::new());
    add(&e, ag, "Rechercher...", Some("<Control>f"), FIND);
    add(&e, ag, "Suivant", Some("F3"), FINDNEXT);
    add(&e, ag, "Précédent", Some("<Shift>F3"), FINDPREV);
    add(&e, ag, "Remplacer...", Some("<Control>h"), REPLACE);
    add(&e, ag, "Aller à la ligne...", Some("<Control>g"), GOTO);
    e.append(&gtk::SeparatorMenuItem::new());
    add(&e, ag, "Ajouter l'occurrence suivante", Some("<Alt>d"), SELNEXT);
    add(&e, ag, "Ouvrir le lien sous le curseur", Some("<Control>Return"), OPENLINK);
    add(&e, ag, "Heure et date", Some("F5"), DATETIME);
    add(&e, ag, "Palette de commandes...", Some("<Control><Shift>p"), PALETTE);
    add(&e, ag, "Ouverture rapide...", Some("<Control>e"), QUICKOPEN);

    let l = sub(&bar, "Lignes");
    add(&l, ag, "Déplacer vers le haut", Some("<Alt>Up"), MOVEUP);
    add(&l, ag, "Déplacer vers le bas", Some("<Alt>Down"), MOVEDOWN);
    add(&l, ag, "Dupliquer", Some("<Control>d"), DUPLINE);
    add(&l, ag, "Supprimer", Some("<Control><Shift>k"), DELLINE);
    add(&l, ag, "Joindre", Some("<Control>j"), JOIN);
    l.append(&gtk::SeparatorMenuItem::new());
    for (t, i) in [("Trier A → Z", SORTASC), ("Trier Z → A", SORTDESC), ("Supprimer les doublons", DEDUPE), ("Supprimer les espaces en fin de ligne", TRIM)] {
        add(&l, ag, t, None, i);
    }
    l.append(&gtk::SeparatorMenuItem::new());
    add(&l, ag, "Commenter / décommenter", Some("<Control>slash"), TOGGLECOMMENT);
    add(&l, ag, "EN MAJUSCULES", Some("<Control><Shift>u"), UPPER);
    add(&l, ag, "en minuscules", Some("<Control>u"), LOWER);
    add(&l, ag, "Première Lettre En Majuscule", None, TITLECASE);
    add(&l, ag, "Tabulations → espaces", None, TABS2SPACES);
    add(&l, ag, "Espaces → tabulations", None, SPACES2TABS);

    let fm = sub(&bar, "Format");
    check(&fm, ag, "Retour automatique à la ligne", None, WRAP);
    add(&fm, ag, "Police...", None, FONT);
    let eol = gtk::Menu::new();
    let eol_item = gtk::MenuItem::with_label("Fins de ligne");
    eol_item.set_submenu(Some(&eol));
    fm.append(&eol_item);
    for (i, t) in ["Windows (CRLF)", "Macintosh (CR)", "Unix (LF)"].iter().enumerate() {
        add(&eol, ag, t, None, EOL + i);
    }

    let v = sub(&bar, "Affichage");
    add(&v, ag, "Zoom avant", Some("<Control>plus"), ZOOMIN);
    add(&v, ag, "Zoom avant (=)", Some("<Control>equal"), ZOOMIN);
    add(&v, ag, "Zoom arrière", Some("<Control>minus"), ZOOMOUT);
    add(&v, ag, "Zoom par défaut", Some("<Control>0"), ZOOMRESET);
    v.append(&gtk::SeparatorMenuItem::new());
    check(&v, ag, "Numéros de ligne", None, LINES);
    check(&v, ag, "Marge de repli", None, FOLDMARGIN);
    check(&v, ag, "Espaces et tabulations visibles", None, WS);
    add(&v, ag, "Mode plein écran", Some("F11"), ZEN);
    v.append(&gtk::SeparatorMenuItem::new());
    add(&v, ag, "Signet : basculer", Some("<Control>F2"), BOOKTOGGLE);
    add(&v, ag, "Signet suivant", Some("F2"), BOOKNEXT);
    add(&v, ag, "Signet précédent", Some("<Shift>F2"), BOOKPREV);
    add(&v, ag, "Effacer les signets", None, BOOKCLEAR);
    add(&v, ag, "Tout replier", None, FOLDALL);
    add(&v, ag, "Tout déplier", None, UNFOLDALL);
    v.append(&gtk::SeparatorMenuItem::new());
    add(&v, ag, "Onglet suivant", Some("<Control>Page_Down"), TABNEXT);
    add(&v, ag, "Onglet précédent", Some("<Control>Page_Up"), TABPREV);

    let th = sub(&bar, "Thème");
    for (i, t) in THEMES.iter().enumerate() {
        add(&th, ag, t.name, None, THEME + i);
    }
    let ds = sub(&bar, "Disposition");
    for (i, l) in LAYOUTS.iter().enumerate() {
        add(&ds, ag, l.name, None, LAYOUT + i);
    }
    let lg = sub(&bar, "Langage");
    for (i, la) in LANGS.iter().enumerate() {
        add(&lg, ag, la.name, None, LANG + i);
    }
    let en = sub(&bar, "Encodage");
    for (i, e) in Enc::ALL.iter().enumerate() {
        add(&en, ag, e.name(), None, ENC + i);
    }
    let h = sub(&bar, "?");
    add(&h, ag, "Statistiques du document", None, STATS);
    add(&h, ag, "Rechercher une mise à jour (page des versions)", None, UPDATE);
    add(&h, ag, "À propos de WinNote", None, ABOUT);
    bar
}

pub fn build_app(test: bool) -> App {
    let win = gtk::Window::new(gtk::WindowType::Toplevel);
    win.set_title("WinNote");
    win.set_default_size(1100, 720);
    win.set_icon_name(Some("winnote"));
    let ag = gtk::AccelGroup::new();
    win.add_accel_group(&ag);
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    vbox.pack_start(&build_menu(&ag), false, false, 0);
    let nb = gtk::Notebook::new();
    nb.set_scrollable(true);
    let find_bar = Bar::new();
    vbox.pack_start(&find_bar.root, false, false, 0);
    vbox.pack_start(&nb, true, true, 0);
    let status = gtk::Label::new(None);
    status.set_xalign(0.0);
    status.set_widget_name("statusbar");
    vbox.pack_end(&status, false, false, 0);
    win.add(&vbox);
    let css = gtk::CssProvider::new();
    if let Some(scr) = gtk::gdk::Screen::default() {
        gtk::StyleContext::add_provider_for_screen(&scr, &css, 600);
    }
    nb.connect_switch_page(|_, _, idx| {
        let a = app();
        if (idx as usize) < a.docs.len() {
            a.cur = idx as usize;
            a.sci = a.docs[a.cur].sci;
            find::mark_all();
            edit::refresh();
        }
    });
    nb.connect_page_reordered(|_, child, idx| {
        let a = app();
        if let Some(old) = a.docs.iter().position(|t| &t.page == child) {
            let t = a.docs.remove(old);
            a.docs.insert((idx as usize).min(a.docs.len()), t);
            a.cur = a.nb.current_page().unwrap_or(0) as usize;
        }
    });
    win.connect_delete_event(|_, _| {
        quit();
        gtk::glib::Propagation::Stop
    });
    // glisser-déposer de fichiers
    win.drag_dest_set(gtk::DestDefaults::ALL, &[gtk::TargetEntry::new("text/uri-list", gtk::TargetFlags::OTHER_APP, 0)], gtk::gdk::DragAction::COPY);
    win.connect_drag_data_received(|_, _, _, _, sel, _, _| {
        for u in sel.uris() {
            if let Some(p) = gtk::gio::File::for_uri(&u).path() {
                edit::open_path(&p.to_string_lossy());
            }
        }
    });
    let l = &LAYOUTS[0];
    App {
        win, nb, status, css, find: find_bar, docs: Vec::new(), cur: 0, sci: std::ptr::null_mut(), untitled: 0, theme: 0, layout: 0,
        wrap: l.wrap, lines: l.lines, ws: false, fold: false, font: l.font.into(), size: l.size, margin: l.margin, spacing: 0,
        root: None, recent: Vec::new(), closed: Vec::new(), quiet: test,
    }
}
