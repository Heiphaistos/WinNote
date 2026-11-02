//! WinNote pour Linux : interface GTK3 + moteur Scintilla/Lexilla (même cœur que la version Windows).
//! Compilation : cargo build --release --features gtk-ui --bin winnote-gtk  (nécessite libgtk-3-dev)
#![allow(dead_code, static_mut_refs)]

#[path = "../doc.rs"]
pub mod doc;
#[path = "../ids.rs"]
pub mod ids;
#[path = "../lang.rs"]
pub mod lang;
#[path = "../sci.rs"]
pub mod sci;
#[path = "../textops.rs"]
pub mod textops;
#[path = "../theme.rs"]
pub mod theme;
mod edit;
mod find;
mod selftest;
mod ui;

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
    pub font: String,
    pub size: i32,
    pub margin: i32,
    pub spacing: i32,
    pub root: Option<String>,
    pub recent: Vec<String>,
    pub closed: Vec<String>,
    pub quiet: bool,
}

static mut APP: *mut App = null_mut();
pub fn app() -> &'static mut App {
    unsafe { &mut *APP }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--version") {
        println!("winnote-gtk {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    eprintln!("A init");
    gtk::init().expect("GTK n'a pas pu démarrer (pas d'affichage ?)");
    let test = args.iter().position(|a| a == "--selftest");
    eprintln!("B build");
    let a = Box::new(ui::build_app(test.is_some()));
    unsafe { APP = Box::into_raw(a) };
    let a = app();
    eprintln!("C settings");
    edit::load_settings();
    eprintln!("D tab");
    edit::new_tab(None, None, false, 0);
    eprintln!("E apply");
    ui::apply_all();
    eprintln!("F done");
    if let Some(i) = test {
        let code = selftest::run(args.get(i + 1).map_or("selftest.log", |s| s.as_str()));
        std::process::exit(code);
    }
    eprintln!("G restore");
    if !edit::restore_session() {
        // rien à restaurer : l'onglet vide créé au démarrage suffit
    }
    eprintln!("H open args");
    for p in args.iter().filter(|x| !x.starts_with("--")) {
        edit::open_path(p);
    }
    eprintln!("I show");
    a.win.show_all();
    eprintln!("J shown");
    a.find.hide();
    gtk::glib::timeout_add_seconds_local(30, || {
        edit::save_session();
        gtk::glib::ControlFlow::Continue
    });
    gtk::main();
}
