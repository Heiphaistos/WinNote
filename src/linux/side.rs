//! Panneau latéral : explorateur de dossier, plan du document, liens entrants (comme la version Windows).
use crate::sci::{self, send};
use crate::{app, edit, notes};
use gtk::prelude::*;
use gtk::prelude::CellLayoutExt;
use std::cell::Cell;

thread_local! {
    static QUIET: Cell<bool> = const { Cell::new(false) };
}

pub struct Side {
    pub root: gtk::Box,
    pub tree: gtk::TreeView,
    pub store: gtk::TreeStore,
    pub btns: Vec<gtk::ToggleButton>,
    /// 0 = fichiers, 1 = plan, 2 = liens entrants
    pub mode: u8,
}

// colonnes : texte, chemin, ligne, dossier ?
const TXT: i32 = 0;
const PATH: i32 = 1;
const LINE: i32 = 2;
const DIR: i32 = 3;

impl Side {
    pub fn new() -> Side {
        let root = gtk::Box::new(gtk::Orientation::Vertical, 0);
        root.set_size_request(250, -1);
        let head = gtk::Box::new(gtk::Orientation::Horizontal, 0);
        head.set_homogeneous(true);
        let mut btns = Vec::new();
        for (i, t) in ["FICHIERS", "PLAN", "LIENS"].iter().enumerate() {
            let b = gtk::ToggleButton::with_label(t);
            b.set_relief(gtk::ReliefStyle::None);
            b.connect_toggled(move |b| {
                if b.is_active() && !QUIET.with(|q| q.get()) {
                    show(i as u8);
                }
            });
            head.pack_start(&b, true, true, 0);
            btns.push(b);
        }
        let store = gtk::TreeStore::new(&[String::static_type(), String::static_type(), i32::static_type(), bool::static_type()]);
        let tree = gtk::TreeView::with_model(&store);
        tree.set_headers_visible(false);
        let col = gtk::TreeViewColumn::new();
        let cell = gtk::CellRendererText::new();
        cell.set_ellipsize(gtk::pango::EllipsizeMode::End);
        CellLayoutExt::pack_start(&col, &cell, true);
        CellLayoutExt::add_attribute(&col, &cell, "text", TXT);
        tree.append_column(&col);
        let sw = gtk::ScrolledWindow::new(gtk::Adjustment::NONE, gtk::Adjustment::NONE);
        sw.add(&tree);
        root.pack_start(&head, false, false, 4);
        root.pack_start(&sw, true, true, 0);
        tree.connect_row_activated(|t, p, _| {
            if let Some(it) = t.model().and_then(|m| m.iter(p)) {
                activate(&it);
            }
        });
        tree.connect_test_expand_row(|t, it, _| {
            let st = &app().side.store;
            if !col_of::<bool>(st, it, DIR) {
                return gtk::glib::Propagation::Proceed;
            }
            if let Some(child) = st.iter_children(Some(it)) {
                if col_of::<String>(st, &child, PATH).is_empty() {
                    st.remove(&child);
                    let dir = col_of::<String>(st, it, PATH);
                    load_dir(Some(it), &dir);
                }
            }
            let _ = t;
            gtk::glib::Propagation::Proceed
        });
        tree.selection().connect_changed(|sel| {
            // dans le plan, un simple clic suffit pour naviguer
            if app().side.mode == 1 && !QUIET.with(|q| q.get()) {
                if let Some((m, it)) = sel.selected() {
                    let l = col_of::<i32>(&m, &it, LINE);
                    if l >= 0 {
                        send(app().sci, sci::GOTOLINE, l as usize, 0);
                    }
                }
            }
        });
        Side { root, tree, store, btns, mode: 0 }
    }
}

fn col_of<T: for<'a> gtk::glib::value::FromValue<'a> + Default>(m: &impl IsA<gtk::TreeModel>, it: &gtk::TreeIter, c: i32) -> T {
    m.value(it, c).get::<T>().unwrap_or_default()
}

fn row(parent: Option<&gtk::TreeIter>, text: &str, path: &str, line: i32, dir: bool) -> gtk::TreeIter {
    app().side.store.insert_with_values(parent, None, &[(TXT as u32, &text), (PATH as u32, &path), (LINE as u32, &line), (DIR as u32, &dir)])
}

fn load_dir(parent: Option<&gtk::TreeIter>, dir: &str) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut v: Vec<(bool, String, String)> = rd
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            (!matches!(n.as_str(), ".git" | "node_modules" | "target" | ".logs")).then(|| (e.path().is_dir(), n, e.path().to_string_lossy().to_string()))
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.to_lowercase().cmp(&b.1.to_lowercase())));
    for (d, n, p) in v {
        let it = row(parent, &n, &p, -1, d);
        if d {
            row(Some(&it), "", "", -1, false); // bouchon : le contenu est chargé à l'ouverture
        }
    }
}

fn quiet<R>(f: impl FnOnce() -> R) -> R {
    QUIET.with(|q| q.set(true));
    let r = f();
    QUIET.with(|q| q.set(false));
    r
}

pub fn show_files() {
    let a = app();
    quiet(|| {
        a.side.store.clear();
        if let Some(root) = a.root.clone() {
            let name = root.trim_end_matches('/').rsplit('/').next().unwrap_or(&root).to_string();
            let r = row(None, &name, &root, -1, true);
            load_dir(Some(&r), &root);
            if let Some(p) = a.side.store.path(&r) {
                a.side.tree.expand_row(&p, false);
            }
        }
    });
}

pub fn show_outline() {
    let a = app();
    let text = String::from_utf8_lossy(&edit::text_of(a.cur)).to_string();
    let md = crate::lang::LANGS[a.docs[a.cur].lang].lexer == "markdown";
    quiet(|| {
        a.side.store.clear();
        let mut stack: Vec<(usize, gtk::TreeIter)> = Vec::new();
        for (level, title, line) in notes::outline(&text, md) {
            while stack.last().is_some_and(|s| s.0 >= level) {
                stack.pop();
            }
            let it = row(stack.last().map(|s| &s.1), &title, "", line as i32, false);
            stack.push((level, it));
        }
        a.side.tree.expand_all();
    });
}

pub fn show_backlinks() {
    let a = app();
    quiet(|| {
        a.side.store.clear();
        let Some(path) = a.docs[a.cur].path.clone() else {
            row(None, "Enregistrez la note pour voir ses liens entrants", "", -1, false);
            return;
        };
        let hits = notes::backlinks(&path, a.root.as_deref());
        if hits.is_empty() {
            row(None, "Aucun lien entrant", "", -1, false);
        }
        for (f, i, l) in hits {
            let name = f.rsplit('/').next().unwrap_or(&f).to_string();
            row(None, &format!("{name} : {l}"), &f, i as i32, false);
        }
    });
}

/// Affiche l'onglet demandé du panneau (0/1/2) et met les boutons en phase.
pub fn show(mode: u8) {
    let a = app();
    a.side.mode = mode;
    quiet(|| {
        for (i, b) in a.side.btns.iter().enumerate() {
            b.set_active(i as u8 == mode);
        }
    });
    match mode {
        1 => show_outline(),
        2 => show_backlinks(),
        _ => show_files(),
    }
}

/// Commande de menu : bascule le panneau sur ce mode (le referme si déjà affiché).
pub fn toggle(mode: u8) {
    let a = app();
    let vis = a.side.root.is_visible() && a.side.mode == mode;
    if vis {
        a.side.root.hide();
    } else {
        a.side.root.show_all();
        show(mode);
    }
    crate::ui::sync_checks();
}

pub fn visible() -> bool {
    app().side.root.is_visible()
}

pub fn refresh() {
    let a = app();
    if a.side.root.is_visible() && a.side.mode != 0 {
        let m = a.side.mode;
        show(m);
    }
}

pub fn set_root(p: &str) {
    let a = app();
    a.root = Some(p.to_string());
    a.side.root.show_all();
    show(0);
    crate::ui::sync_checks();
}

fn activate(it: &gtk::TreeIter) {
    let a = app();
    let st = &a.side.store;
    let (path, line, dir) = (col_of::<String>(st, it, PATH), col_of::<i32>(st, it, LINE), col_of::<bool>(st, it, DIR));
    match a.side.mode {
        2 if !path.is_empty() => {
            edit::open_path(&path);
            send(app().sci, sci::GOTOLINE, line.max(0) as usize, 0);
        }
        1 if line >= 0 => {
            send(a.sci, sci::GOTOLINE, line as usize, 0);
            unsafe { gtk::ffi::gtk_widget_grab_focus(a.sci as *mut _) };
        }
        0 if !dir && !path.is_empty() => edit::open_path(&path),
        _ => {}
    }
}

/// Nombre de lignes du modèle dont `keep` (texte, chemin) est vrai, enfants compris (autotest).
fn count(parent: Option<&gtk::TreeIter>, keep: &dyn Fn(&str) -> bool) -> usize {
    let st = &app().side.store;
    let mut n = 0;
    if let Some(it) = st.iter_children(parent) {
        loop {
            n += keep(&col_of::<String>(st, &it, PATH)) as usize + count(Some(&it), keep);
            if !st.iter_next(&it) {
                break;
            }
        }
    }
    n
}

/// Pour l'autotest : nombre de liens entrants trouvés.
pub fn backlink_count() -> usize {
    show(2);
    count(None, &|p| !p.is_empty())
}

pub fn row_count() -> usize {
    count(None, &|_| true)
}
