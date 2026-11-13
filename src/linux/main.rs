//! WinNote pour Linux et macOS : interface GTK3 + moteur Scintilla/Lexilla (même cœur que la version Windows).
//! Compilation : cargo build --release --features gtk-ui --bin winnote-gtk  (nécessite libgtk-3-dev)
#![allow(dead_code, static_mut_refs)]

#[path = "../doc.rs"]
pub mod doc;
#[path = "../ids.rs"]
pub mod ids;
#[path = "../lang.rs"]
pub mod lang;
#[path = "../fileops.rs"]
pub mod fileops;
#[path = "../prefs.rs"]
pub mod prefs;
#[path = "../keymap.rs"]
pub mod keymap;
#[path = "../macros.rs"]
pub mod macros;
#[path = "../findx.rs"]
pub mod findx;
#[path = "../assist.rs"]
pub mod assist;
#[path = "../mdedit.rs"]
pub mod mdedit;
#[path = "../mdstyle.rs"]
pub mod mdstyle;
#[path = "../mdrender.rs"]
pub mod mdrender;
#[path = "../notes.rs"]
pub mod notes;
#[path = "../sci.rs"]
pub mod sci;
#[path = "../textops.rs"]
pub mod textops;
#[path = "../theme.rs"]
pub mod theme;
#[path = "../vers.rs"]
pub mod vers;
mod chrome;
mod edit;
mod find;
mod print;
mod selftest;
mod side;
mod spell;
mod ui;
mod update;
mod view;

use doc::Enc;
use gtk::prelude::*;
use sci::HWND;
use std::ptr::null_mut;

pub fn cstr(s: &str) -> Vec<u8> {
    s.bytes().chain(Some(0)).collect()
}

/// Fonctions attendues par textops.rs (partagé avec la version Windows).
pub mod cmds {
    pub use crate::edit::{msg_box, open_or_create, open_path};
}

pub struct Tab {
    pub sci: HWND,
    pub path: Option<String>,
    pub enc: Enc,
    pub lang: usize,
    pub dirty: bool,
    pub follow: bool,
    pub pinned: bool,
    pub mtime: Option<std::time::SystemTime>,
    pub untitled: u32,
    pub page: gtk::Widget,
    pub label: gtk::Label,
}

pub struct App {
    pub win: gtk::Window,
    pub nb: gtk::Notebook,
    pub status: gtk::Label,
    pub css: gtk::CssProvider,
    pub find: find::Bar,
    pub side: side::Side,
    pub vbox: gtk::Box,
    pub bar: chrome::Bar,
    /// barre de titre : 0 = système, 1 = style macOS (Linux seulement)
    pub chrome: u8,
    /// volet de gauche = éditeur, volet de droite = aperçu Markdown ou vue scindée
    pub psplit: gtk::Paned,
    pub rstack: gtk::Stack,
    pub prev: HWND,
    pub sci2: HWND,
    pub docs: Vec<Tab>,
    pub cur: usize,
    /// Vue Scintilla de l'onglet courant (utilisée par textops).
    pub sci: HWND,
    pub untitled: u32,
    pub theme: usize,
    pub layout: usize,
    pub wrap: bool,
    pub lines: bool,
    pub ws: bool,
    pub fold: bool,
    pub preview: bool,
    pub split: bool,
    pub spell: bool,
    pub spell_lang: String,
    pub topmost: bool,
    pub auto_update: bool,
    pub font: String,
    pub size: i32,
    pub margin: i32,
    pub spacing: i32,
    pub root: Option<String>,
    pub recent: Vec<String>,
    pub closed: Vec<String>,
    pub quiet: bool,
    /// lancé avec des fichiers en argument : ne touche pas à la session des autres fenêtres
    pub nosession: bool,
}

static mut APP: *mut App = null_mut();
pub fn app() -> &'static mut App {
    unsafe { &mut *APP }
}

fn arg_after(args: &[String], flag: &str) -> Option<usize> {
    args.iter().position(|a| a == flag)
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") {
        println!("winnote-gtk {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    // vérification de mise à jour en ligne de commande (sans interface) : écrit le résultat dans un fichier
    if let Some(i) = arg_after(&args, "--update-check") {
        let out = match update::fetch() {
            Ok(Some(f)) => format!("nouvelle version {} url={} sha={}", f.version, f.url, f.sha),
            Ok(None) => "a jour".to_string(),
            Err(e) => format!("erreur : {e}"),
        };
        let _ = std::fs::write(args.get(i + 1).map_or("update.txt", |s| s.as_str()), out);
        return;
    }
    gtk::init().expect("GTK n'a pas pu démarrer (pas d'affichage ?)");
    let test = arg_after(&args, "--selftest");
    let files: Vec<&String> = args.iter().filter(|x| !x.starts_with("--")).collect();
    let print_to = arg_after(&args, "--print-to");
    let mut a = ui::build_app(test.is_some() || print_to.is_some());
    a.nosession = !files.is_empty() || test.is_some();
    unsafe { APP = Box::into_raw(Box::new(a)) };
    let a = app();
    edit::load_settings();
    edit::new_tab(None, None, false, 0);
    ui::apply_all();
    if let Some(i) = test {
        let code = selftest::run(args.get(i + 1).map_or("selftest.log", |s| s.as_str()));
        std::process::exit(code);
    }
    // --print-to sortie.pdf fichier : impression de test dans un PDF
    if let Some(i) = print_to {
        if let (Some(out), Some(f)) = (args.get(i + 1), args.get(i + 2)) {
            edit::open_path(f);
            let r = print::export_pdf(out);
            println!("{}", match &r {
                Ok(n) => format!("{n} page(s)"),
                Err(e) => format!("erreur : {e}"),
            });
            std::process::exit(r.is_err() as i32);
        }
        std::process::exit(2);
    }
    if !a.nosession {
        edit::restore_session();
    }
    for p in &files {
        edit::open_path(p);
    }
    a.win.show_all();
    chrome::apply();
    a.find.hide();
    a.side.root.hide();
    a.rstack.hide();
    ui::sync_checks();
    view::relayout();
    if a.root.is_some() && !a.nosession {
        // le dossier ouvert la dernière fois reste visible dans le panneau
        side::set_root(&a.root.clone().unwrap_or_default());
    }
    if !a.nosession {
        gtk::glib::timeout_add_seconds_local(30, || {
            edit::save_session();
            gtk::glib::ControlFlow::Continue
        });
    }
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(1500), || {
        edit::check_external();
        gtk::glib::ControlFlow::Continue
    });
    update::start();
    gtk::main();
}
