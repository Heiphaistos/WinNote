//! Fenêtre, menus, thème (CSS), palette de commandes et répartition des commandes (Linux / GTK).
use crate::doc::Enc;
use crate::edit::{self, msg_box};
use crate::find::{self, Bar};
use crate::ids::*;
use crate::lang::LANGS;
use crate::sci::{self, send};
use crate::theme::{LAYOUTS, THEMES};
use crate::{app, chrome, cstr, keymap, notes, prefs, side, spell, textops, view, App};
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
         #titlebar {{ background-color: {ui}; padding: 8px 0; }}
         #titlebar label {{ color: {uifg}; font-weight: bold; }}
         #traffic {{ min-width: 12px; min-height: 12px; padding: 0; border: none; border-radius: 50%; background-image: none; }}
         #traffic.tl-close {{ background-color: #ff5f57; }}
         #traffic.tl-min {{ background-color: #febc2e; }}
         #traffic.tl-max {{ background-color: #28c840; }}
         #statusbar {{ background-color: {ui}; padding: 3px 12px; }}
         list, row, treeview, treeview.view {{ background-color: {ui}; color: {uifg}; }}
         treeview.view:selected {{ background-color: {acc}; color: {bg}; }}
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

pub fn sync_checks() {
    let a = app();
    unsafe { SYNCING = true };
    CHECKS.with(|c| {
        for (id, item) in c.borrow().iter() {
            let on = match *id {
                WRAP => a.wrap,
                LINES => a.lines,
                WS => a.ws,
                FOLDMARGIN => a.fold,
                SIDEBAR => side::visible() && a.side.mode == 0,
                OUTLINE => side::visible() && a.side.mode == 1,
                BACKLINKS => side::visible() && a.side.mode == 2,
                PREVIEW => a.preview,
                SPLIT => a.split,
                SPELL => a.spell,
                TOPMOST => a.topmost,
                AUTOCOMP => prefs::autocomp(),
                GUIDES => prefs::view().0,
                SMARTHL => prefs::smart(),
                CHGHIST => prefs::history(),
                SHOWEOL => prefs::view().1,
                FOLLOW => a.docs.get(a.cur).is_some_and(|d| d.follow),
                x if (INDENT_AUTO..INDENT_AUTO + 5).contains(&x) => x - INDENT_AUTO == match prefs::indent() { prefs::Indent::Auto => 0, prefs::Indent::Spaces(2) => 1, prefs::Indent::Spaces(4) => 2, prefs::Indent::Spaces(8) => 3, _ => 4 },
                READONLY => !a.docs.is_empty() && send(a.sci, sci::GETREADONLY, 0, 0) != 0,
                AUTOUPDATE => a.auto_update,
                CHROME_WIN => a.chrome == 0,
                CHROME_MAC => a.chrome == 1,
                x if (SPELLLANG..SPELLLANG + spell::LANGS.len()).contains(&x) => spell::LANGS[x - SPELLLANG].0 == a.spell_lang,
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
    OpenAt(String, usize),
    Goto(usize),
    Prop(String),
}

/// Commandes disponibles dans la version Linux (le reste de la table est propre à Windows).
fn supported(id: usize) -> bool {
    !matches!(id, REGISTER | UNREGISTER | DWRITE) && !(cfg!(target_os = "macos") && matches!(id, CHROME_WIN | CHROME_MAC))
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
        Some(Act::OpenAt(p, l)) => {
            edit::open_path(&p);
            send(app().sci, sci::GOTOLINE, l, 0);
        }
        Some(Act::Goto(l)) => drop(send(app().sci, sci::GOTOLINE, l, 0)),
        Some(Act::Prop(init)) => {
            if let Some(t) = side::ask_name_pub("Propriété — clé: valeur (valeur vide = retirer)", &init) {
                crate::features::set_property(&t);
            }
        }
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
        notes::walk(std::path::Path::new(&r), &mut files, 0);
        for f in files {
            let s = f.to_string_lossy().to_string();
            if seen.insert(s.clone()) {
                v.push((f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), s.strip_prefix(r.as_str()).unwrap_or(&s).trim_start_matches('/').to_string(), Act::Open(s)));
            }
        }
    }
    palette(v, "Ouverture rapide");
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
    d.set_font(&format!("{} {}", edit::linux_font(&a.font), a.size));
    if d.run() == gtk::ResponseType::Ok {
        if let Some(desc) = d.font_desc() {
            a.font = desc.family().map(|f| f.to_string()).unwrap_or(a.font.clone());
            a.size = (desc.size() / gtk::pango::SCALE).max(6);
            apply_all();
        }
    }
    d.close();
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
                side::set_root(&p);
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
        CLOSELEFT => edit::close_side(doc_i, -1),
        CLOSERIGHT => edit::close_side(doc_i, 1),
        CLOSECLEAN => edit::close_side(doc_i, 0),
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
        FOLLOW => {
            edit::toggle_follow();
            sync_checks();
        }
        AUTOCOMP => {
            prefs::set_autocomp(!prefs::autocomp());
            sync_checks();
        }
        880..=884 => {
            use prefs::Indent;
            prefs::set_indent([Indent::Auto, Indent::Spaces(2), Indent::Spaces(4), Indent::Spaces(8), Indent::Tabs][id - INDENT_AUTO]);
            for t in &a.docs {
                edit::style_view(t.sci, t.lang);
            }
            sync_checks();
        }
        KEYS => {
            let p = keymap::file_path();
            if !p.exists() {
                let _ = std::fs::create_dir_all(p.parent().unwrap_or(std::path::Path::new(".")));
                let _ = std::fs::write(&p, keymap::template());
            }
            edit::open_path(&p.to_string_lossy());
        }
        REVEALFILE => {
            if let Some(p) = a.docs[doc_i].path.as_ref().and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf())) {
                textops::open_url(&p.to_string_lossy());
            }
        }
        COPYNAME | COPYDIR => {
            if let (Some(p), Some(d)) = (a.docs[doc_i].path.clone(), gtk::gdk::Display::default()) {
                let p = std::path::Path::new(&p);
                let r = if id == COPYNAME { p.file_name() } else { p.parent().map(|d| d.as_os_str()) };
                if let Some(r) = r {
                    gtk::Clipboard::default(&d).map(|c| c.set_text(&r.to_string_lossy()));
                }
            }
        }
        BLOCKCOMM => crate::textops::block_comment(),
        TAGS => {
            let Some(r) = a.root.clone() else { return };
            let v = notes::tag_hits(&r)
                .into_iter()
                .map(|(t, f, l, x)| (format!("#{t}  {x}"), format!("{}:{}", f.strip_prefix(r.as_str()).unwrap_or(&f).trim_start_matches('/'), l + 1), Act::OpenAt(f, l)))
                .collect();
            palette(v, "Étiquettes #tag");
        }
        PROPS => palette(crate::features::prop_items().into_iter().map(|(l, h, i)| (l, h, Act::Prop(i))).collect(), "Propriétés YAML"),
        FAVTOGGLE => crate::features::fav_toggle(),
        FAVS => palette(crate::features::favs().into_iter().map(|p| (p.rsplit('/').next().unwrap_or(&p).to_string(), p.clone(), Act::Open(p))).collect(), "Favoris"),
        SYMBOLS => palette(crate::features::symbols().into_iter().map(|(l, h, n)| (l, h, Act::Goto(n))).collect(), "Fonctions et titres"),
        SAVECOPY => crate::features::save_copy(),
        PAGESETUP => crate::features::page_setup(),
        RECOVER => crate::features::recover_cmd(),
        PINTAB => {
            let p = !a.docs[doc_i].pinned;
            a.docs[doc_i].pinned = p;
            let n = a.docs.iter().enumerate().filter(|(i, d)| d.pinned && *i != doc_i).count();
            let t = a.docs.remove(doc_i);
            a.nb.reorder_child(&t.page, Some(n as u32));
            a.docs.insert(n, t);
            a.cur = n;
            edit::refresh();
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
        DAILY => match a.root.clone() {
            Some(r) => {
                let t = gtk::glib::DateTime::now_local().ok();
                let (y, m, d) = t.map_or((1970, 1, 1), |t| (t.year(), t.month() as u32, t.day_of_month() as u32));
                let (p, c) = notes::daily_note(&r, y, m, d);
                if !std::path::Path::new(&p).exists() {
                    let _ = std::fs::write(&p, c);
                    side::show_files();
                }
                edit::open_path(&p);
            }
            None => msg_box("Ouvrez d'abord un dossier (Fichier > Ouvrir un dossier)."),
        },
        MACRO_REC => {
            crate::macros::toggle(a.sci);
        }
        MACRO_PLAY => crate::macros::play(a.sci, 1),
        MACRO_PLAYN => {
            if let Some(n) = side::ask_name_pub("Nombre de répétitions (0 = jusqu'à la fin du document)", "10") {
                crate::macros::play(a.sci, n.trim().parse().unwrap_or(1));
            }
        }
        COLINS => {
            if let Some(t) = side::ask_name_pub("Texte à insérer ({n} = numéro, {n:10,5} = départ,pas)", "{n}. ") {
                textops::column_insert(&t);
            }
        }
        886..=892 => textops::line_op(id - LOP),
        SMARTHL => {
            prefs::set_smart(!prefs::smart());
            for t in &a.docs {
                crate::assist::smart_highlight(t.sci);
            }
            sync_checks();
        }
        CHGHIST => {
            prefs::set_history(!prefs::history());
            for t in &a.docs {
                edit::style_view(t.sci, t.lang);
            }
            sync_checks();
        }
        GUIDES | SHOWEOL => {
            let (g, e) = prefs::view();
            prefs::set_view(if id == GUIDES { !g } else { g }, if id == SHOWEOL { !e } else { e });
            for t in &a.docs {
                edit::style_view(t.sci, t.lang);
            }
            sync_checks();
        }
        TRIM => textops::trim(),
        JOIN => textops::join(),
        TOGGLECOMMENT => textops::toggle_comment(),
        MD_BOLD..=MD_CALLOUT | MD_H1..=1025 => crate::mdedit::run(s, id),
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
        UPDATE => crate::update::check(true),
        AUTOUPDATE => {
            a.auto_update = !a.auto_update;
            sync_checks();
        }
        PRINT => crate::print::print_doc(),
        CHROME_WIN | CHROME_MAC => {
            a.chrome = (id - CHROME_WIN) as u8;
            chrome::apply();
            sync_checks();
        }
        NEWWINDOW => view::new_window(),
        SIDEBAR => side::toggle(0),
        OUTLINE => side::toggle(1),
        BACKLINKS => side::toggle(2),
        PREVIEW => view::toggle_preview(),
        SPLIT => view::toggle_split(),
        READONLY => view::toggle_readonly(),
        TOPMOST => view::toggle_topmost(),
        SPELL => {
            a.spell = !a.spell;
            spell::check_visible();
            if a.spell && !spell::init(&a.spell_lang) {
                msg_box("Le correcteur nécessite Enchant et un dictionnaire (ex. : paquets libenchant-2-2 et hunspell-fr, ou brew install enchant).");
            }
            sync_checks();
        }
        SPELLLANG..=864 => {
            a.spell_lang = spell::LANGS[id - SPELLLANG].0.to_string();
            if !spell::init(&a.spell_lang) {
                msg_box("Ce dictionnaire n'est pas installé sur le système.");
            }
            spell::check_visible();
            sync_checks();
        }
        ABOUT => msg_box(&format!("WinNote {}\nÉditeur de texte natif (Linux / GTK3).\nMoteur : Scintilla + Lexilla.", env!("CARGO_PKG_VERSION"))),
        EOL..=134 => {
            let m = id - EOL;
            send(s, sci::SETEOLMODE, m, 0);
            send(s, sci::CONVERTEOLS, m, 0);
            update_status();
        }
        200..=218 => {
            a.docs[doc_i].enc = Enc::ALL[id - ENC];
            edit::refresh();
        }
        230..=248 => edit::reload_with(Some(Enc::ALL[id - RENC])),
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
    crate::features::recovery_end();
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
    add(&f, ag, "Nouvel onglet", Some("<Primary>n"), NEW);
    add(&f, ag, "Ouvrir...", Some("<Primary>o"), OPEN);
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
    add(&f, ag, "Rouvrir l'onglet fermé", Some("<Primary><Shift>t"), REOPEN);
    f.append(&gtk::SeparatorMenuItem::new());
    add(&f, ag, "Enregistrer", Some("<Primary>s"), SAVE);
    add(&f, ag, "Enregistrer sous...", Some("<Primary><Shift>s"), SAVEAS);
    add(&f, ag, "Tout enregistrer", Some("<Primary><Alt>s"), SAVEALL);
    add(&f, ag, "Recharger depuis le disque", None, RELOAD);
    add(&f, ag, "Note du jour", None, DAILY);
    check(&f, ag, "Suivre la fin du fichier (tail -f)", None, FOLLOW);
    add(&f, ag, "Enregistrer une copie...", None, SAVECOPY);
    add(&f, ag, "Mise en page de l'impression...", None, PAGESETUP);
    add(&f, ag, "Restaurer les documents récupérés", None, RECOVER);
    add(&f, ag, "Imprimer...", Some("<Primary>p"), PRINT);
    add(&f, ag, "Ouvrir dans une nouvelle fenêtre", None, NEWWINDOW);
    f.append(&gtk::SeparatorMenuItem::new());
    add(&f, ag, "Afficher dans le gestionnaire de fichiers", None, REVEALFILE);
    add(&f, ag, "Copier le chemin", None, COPYPATH);
    add(&f, ag, "Copier le nom du fichier", None, COPYNAME);
    add(&f, ag, "Copier le dossier", None, COPYDIR);
    f.append(&gtk::SeparatorMenuItem::new());
    add(&f, ag, "Fermer l'onglet", Some("<Primary>w"), CLOSE);
    add(&f, ag, "Étiquettes #tag du dossier...", None, TAGS);
    add(&f, ag, "Aller à une fonction / un titre...", None, SYMBOLS);
    add(&f, ag, "Propriétés YAML du document...", None, PROPS);
    add(&f, ag, "Favoris : ajouter / retirer ce fichier", None, FAVTOGGLE);
    add(&f, ag, "Favoris...", None, FAVS);
    add(&f, ag, "Épingler / désépingler l'onglet", None, PINTAB);
    add(&f, ag, "Fermer les autres onglets", None, CLOSEOTHERS);
    add(&f, ag, "Fermer les onglets à gauche", None, CLOSELEFT);
    add(&f, ag, "Fermer les onglets à droite", None, CLOSERIGHT);
    add(&f, ag, "Fermer les onglets non modifiés", None, CLOSECLEAN);
    add(&f, ag, "Quitter", Some("<Primary>q"), EXIT);

    let e = sub(&bar, "Édition");
    for (l, i) in [("Annuler", UNDO), ("Rétablir", REDO), ("Couper", CUT), ("Copier", COPY), ("Coller", PASTE), ("Supprimer", DEL), ("Tout sélectionner", SELALL)] {
        add(&e, ag, l, None, i);
    }
    e.append(&gtk::SeparatorMenuItem::new());
    add(&e, ag, "Rechercher...", Some("<Primary>f"), FIND);
    add(&e, ag, "Suivant", Some("F3"), FINDNEXT);
    add(&e, ag, "Précédent", Some("<Shift>F3"), FINDPREV);
    add(&e, ag, "Remplacer...", Some("<Primary>h"), REPLACE);
    add(&e, ag, "Aller à la ligne...", Some("<Primary>g"), GOTO);
    e.append(&gtk::SeparatorMenuItem::new());
    add(&e, ag, "Insérer en colonne...", Some("<Alt>c"), COLINS);
    add(&e, ag, "Macro : démarrer / arrêter (F9)", Some("F9"), MACRO_REC);
    add(&e, ag, "Macro : rejouer (F10)", Some("F10"), MACRO_PLAY);
    add(&e, ag, "Macro : rejouer N fois...", None, MACRO_PLAYN);
    add(&e, ag, "Ajouter l'occurrence suivante", Some("<Alt>d"), SELNEXT);
    add(&e, ag, "Ouvrir le lien sous le curseur", Some("<Primary>Return"), OPENLINK);
    add(&e, ag, "Heure et date", Some("F5"), DATETIME);
    add(&e, ag, "Palette de commandes...", Some("<Primary><Shift>p"), PALETTE);
    add(&e, ag, "Ouverture rapide...", Some("<Primary>e"), QUICKOPEN);
    e.append(&gtk::SeparatorMenuItem::new());
    check(&e, ag, "Correcteur orthographique", None, SPELL);
    for (i, l) in spell::LANGS.iter().enumerate() {
        check(&e, ag, &format!("   Dictionnaire : {}", l.1), None, SPELLLANG + i);
    }

    let l = sub(&bar, "Lignes");
    add(&l, ag, "Déplacer vers le haut", Some("<Alt>Up"), MOVEUP);
    add(&l, ag, "Déplacer vers le bas", Some("<Alt>Down"), MOVEDOWN);
    add(&l, ag, "Dupliquer", Some("<Primary>d"), DUPLINE);
    add(&l, ag, "Supprimer", Some("<Primary><Shift>k"), DELLINE);
    add(&l, ag, "Joindre", Some("<Primary>j"), JOIN);
    l.append(&gtk::SeparatorMenuItem::new());
    for (t, i) in [("Trier par longueur", LOP), ("Trier par valeur numérique", LOP + 1), ("Inverser l'ordre", LOP + 2), ("Mélanger", LOP + 3), ("Supprimer les lignes vides", LOP + 4), ("Supprimer les espaces en début de ligne", LOP + 5), ("Supprimer les doublons consécutifs", LOP + 6)] {
        add(&l, ag, t, None, i);
    }
    for (t, i) in [("Trier A → Z", SORTASC), ("Trier Z → A", SORTDESC), ("Supprimer les doublons", DEDUPE), ("Supprimer les espaces en fin de ligne", TRIM)] {
        add(&l, ag, t, None, i);
    }
    l.append(&gtk::SeparatorMenuItem::new());
    add(&l, ag, "Commenter / décommenter", Some("<Primary>slash"), TOGGLECOMMENT);
    add(&l, ag, "Commenter en bloc", None, BLOCKCOMM);
    add(&l, ag, "EN MAJUSCULES", Some("<Primary><Shift>u"), UPPER);
    add(&l, ag, "en minuscules", Some("<Primary>u"), LOWER);
    add(&l, ag, "Première Lettre En Majuscule", None, TITLECASE);
    add(&l, ag, "Tabulations → espaces", None, TABS2SPACES);
    add(&l, ag, "Espaces → tabulations", None, SPACES2TABS);

    let fm = sub(&bar, "Format");
    check(&fm, ag, "Retour automatique à la ligne", None, WRAP);
    add(&fm, ag, "Police...", None, FONT);
    check(&fm, ag, "Autocomplétion des mots (code)", None, AUTOCOMP);
    let ind = gtk::Menu::new();
    let ind_item = gtk::MenuItem::with_label("Indentation");
    ind_item.set_submenu(Some(&ind));
    fm.append(&ind_item);
    for (i, t) in ["Selon le langage", "2 espaces", "4 espaces", "8 espaces", "Tabulations"].iter().enumerate() {
        check(&ind, ag, t, None, INDENT_AUTO + i);
    }
    add(&fm, ag, "Raccourcis clavier personnalisés...", None, KEYS);
    let eol = gtk::Menu::new();
    let eol_item = gtk::MenuItem::with_label("Fins de ligne");
    eol_item.set_submenu(Some(&eol));
    fm.append(&eol_item);
    for (i, t) in ["Windows (CRLF)", "Macintosh (CR)", "Unix (LF)"].iter().enumerate() {
        add(&eol, ag, t, None, EOL + i);
    }

    let md_item = gtk::MenuItem::with_label("Markdown");
    let md = gtk::Menu::new();
    md_item.set_submenu(Some(&md));
    fm.append(&md_item);
    add(&md, ag, "Gras", Some("<Primary>b"), MD_BOLD);
    add(&md, ag, "Italique", Some("<Primary>i"), MD_ITALIC);
    add(&md, ag, "Barré", None, MD_STRIKE);
    add(&md, ag, "Surligné", None, MD_HILITE);
    add(&md, ag, "Code en ligne", None, MD_CODE);
    md.append(&gtk::SeparatorMenuItem::new());
    add(&md, ag, "Titre 1", None, MD_H1);
    add(&md, ag, "Titre 2", None, MD_H1 + 1);
    add(&md, ag, "Titre 3", None, MD_H1 + 2);
    add(&md, ag, "Titre 4", None, MD_H1 + 3);
    add(&md, ag, "Titre 5", None, MD_H1 + 4);
    add(&md, ag, "Titre 6", None, MD_H1 + 5);
    md.append(&gtk::SeparatorMenuItem::new());
    add(&md, ag, "Citation", None, MD_QUOTE);
    add(&md, ag, "Liste à puces", None, MD_BULLET);
    add(&md, ag, "Liste numérotée", None, MD_NUMLIST);
    add(&md, ag, "Liste de tâches", None, MD_TASK);
    md.append(&gtk::SeparatorMenuItem::new());
    add(&md, ag, "Lien", Some("<Primary>k"), MD_LINK);
    add(&md, ag, "Lien [[note]]", None, MD_WIKI);
    add(&md, ag, "Tableau", None, MD_TABLE);
    add(&md, ag, "Bloc de code", None, MD_CODEBLOCK);
    add(&md, ag, "Ligne horizontale", None, MD_HR);
    add(&md, ag, "Encadré (callout)", None, MD_CALLOUT);

    let v = sub(&bar, "Affichage");
    check(&v, ag, "Guides d'indentation", None, GUIDES);
    check(&v, ag, "Surligner les occurrences du mot", None, SMARTHL);
    check(&v, ag, "Historique des modifications (marge)", None, CHGHIST);
    check(&v, ag, "Fins de ligne visibles", None, SHOWEOL);
    add(&v, ag, "Zoom avant", Some("<Primary>plus"), ZOOMIN);
    add(&v, ag, "Zoom avant (=)", Some("<Primary>equal"), ZOOMIN);
    add(&v, ag, "Zoom arrière", Some("<Primary>minus"), ZOOMOUT);
    add(&v, ag, "Zoom par défaut", Some("<Primary>0"), ZOOMRESET);
    v.append(&gtk::SeparatorMenuItem::new());
    check(&v, ag, "Numéros de ligne", None, LINES);
    check(&v, ag, "Marge de repli", None, FOLDMARGIN);
    check(&v, ag, "Espaces et tabulations visibles", None, WS);
    add(&v, ag, "Mode plein écran", Some("F11"), ZEN);
    v.append(&gtk::SeparatorMenuItem::new());
    check(&v, ag, "Explorateur de fichiers", Some("<Primary><Shift>e"), SIDEBAR);
    check(&v, ag, "Plan du document", Some("<Primary><Shift>o"), OUTLINE);
    check(&v, ag, "Liens entrants (backlinks)", None, BACKLINKS);
    check(&v, ag, "Aperçu Markdown", Some("<Primary><Shift>m"), PREVIEW);
    check(&v, ag, "Vue scindée", Some("<Primary>backslash"), SPLIT);
    check(&v, ag, "Toujours au premier plan", None, TOPMOST);
    if !cfg!(target_os = "macos") {
        check(&v, ag, "Barre de titre : système", None, CHROME_WIN);
        check(&v, ag, "Barre de titre : style macOS", None, CHROME_MAC);
    }
    check(&v, ag, "Lecture seule", None, READONLY);
    v.append(&gtk::SeparatorMenuItem::new());
    add(&v, ag, "Signet : basculer", Some("<Primary>F2"), BOOKTOGGLE);
    add(&v, ag, "Signet suivant", Some("F2"), BOOKNEXT);
    add(&v, ag, "Signet précédent", Some("<Shift>F2"), BOOKPREV);
    add(&v, ag, "Effacer les signets", None, BOOKCLEAR);
    add(&v, ag, "Tout replier", None, FOLDALL);
    add(&v, ag, "Tout déplier", None, UNFOLDALL);
    v.append(&gtk::SeparatorMenuItem::new());
    add(&v, ag, "Onglet suivant", Some("<Primary>Page_Down"), TABNEXT);
    add(&v, ag, "Onglet précédent", Some("<Primary>Page_Up"), TABPREV);

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
    for (i, e) in Enc::ALL.iter().enumerate().take(5) {
        add(&en, ag, e.name(), None, ENC + i);
    }
    for (title, base, skip) in [("Autres encodages", ENC, 5), ("Recharger avec l'encodage", RENC, 0)] {
        let m = gtk::Menu::new();
        for (i, e) in Enc::ALL.iter().enumerate().skip(skip) {
            add(&m, ag, e.name(), None, base + i);
        }
        let it = gtk::MenuItem::with_label(title);
        it.set_submenu(Some(&m));
        en.append(&it);
    }
    let h = sub(&bar, "?");
    add(&h, ag, "Statistiques du document", None, STATS);
    add(&h, ag, "Rechercher une mise à jour", None, UPDATE);
    check(&h, ag, "Mises à jour automatiques", None, AUTOUPDATE);
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
    user_accels(&ag);
    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 0);
    let bar = chrome::Bar::new();
    vbox.pack_start(&bar.root, false, false, 0);
    chrome::hook_resize(&win);
    vbox.pack_start(&build_menu(&ag), false, false, 0);
    let nb = gtk::Notebook::new();
    nb.set_scrollable(true);
    let find_bar = Bar::new();
    vbox.pack_start(&find_bar.root, false, false, 0);
    // aperçu Markdown et vue scindée partagent le volet de droite
    let (prev, prev_w) = edit::new_sci();
    view::setup_preview(prev);
    let (sci2, sci2_w) = edit::new_sci();
    let rstack = gtk::Stack::new();
    rstack.add_named(&prev_w, "prev");
    rstack.add_named(&sci2_w, "split");
    let psplit = gtk::Paned::new(gtk::Orientation::Horizontal);
    psplit.pack1(&nb, true, false);
    psplit.pack2(&rstack, true, false);
    let side = side::Side::new();
    let body = gtk::Paned::new(gtk::Orientation::Horizontal);
    body.pack1(&side.root, false, false);
    body.pack2(&psplit, true, false);
    vbox.pack_start(&body, true, true, 0);
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
            view::on_tab_changed();
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
        win, nb, status, css, find: find_bar, side, vbox, bar, chrome: 0, psplit, rstack, prev, sci2, preview: false, split: false, spell: false,
        spell_lang: "fr_FR".into(), topmost: false, auto_update: true, nosession: false, docs: Vec::new(), cur: 0, sci: std::ptr::null_mut(), untitled: 0, theme: 0, layout: 0,
        wrap: l.wrap, lines: l.lines, ws: false, fold: false, font: l.font.into(), size: l.size, margin: l.margin, spacing: 0,
        root: None, recent: Vec::new(), closed: Vec::new(), quiet: test,
    }
}

/// Raccourcis personnalisés (raccourcis.txt) : un élément de menu caché par combinaison.
fn user_accels(ag: &gtk::AccelGroup) {
    keymap::init(gtk::glib::user_config_dir().join("winnote")); // la fenêtre est construite avant les réglages
    // GTK n'active le raccourci d'un élément de menu que s'il a un menu parent : on les range dans un menu caché
    let hidden = gtk::Menu::new();
    for (chord, id) in keymap::user() {
        let it = gtk::MenuItem::new();
        it.show();
        accel(&it, ag, &keymap::gtk_spec(&chord));
        it.connect_activate(move |_| on_command(id));
        hidden.append(&it);
    }
    std::mem::forget(hidden);
}
