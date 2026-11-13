//! Préférences d'édition partagées (Windows / Linux / macOS) : indentation par langage, autocomplétion des mots.
//! Enregistrées dans `prefs.ini` à côté des réglages.
use crate::sci::{send, HWND};
use std::cell::{Cell, RefCell};
use std::path::PathBuf;

const SETINDENT: u32 = 2122;
const SETTABINDENTS: u32 = 2260;
const SETBACKSPACEUNINDENTS: u32 = 2262;

/// Style d'indentation choisi par l'utilisateur.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Indent { Auto, Spaces(u8), Tabs }

thread_local! {
    static INDENT: Cell<Indent> = const { Cell::new(Indent::Auto) };
    static VIEW: Cell<(bool, bool)> = const { Cell::new((false, false)) };
    static AUTOCOMP: Cell<bool> = const { Cell::new(true) };
    static SMART: Cell<bool> = const { Cell::new(true) };
    static HIST: Cell<bool> = const { Cell::new(true) };
    static DIR: RefCell<PathBuf> = const { RefCell::new(PathBuf::new()) };
}

pub fn indent() -> Indent { INDENT.with(|c| c.get()) }
pub fn set_indent(i: Indent) { INDENT.with(|c| c.set(i)); save(); }
pub fn view() -> (bool, bool) { VIEW.with(|c| c.get()) }
pub fn set_view(g: bool, e: bool) { VIEW.with(|c| c.set((g, e))); save(); }
pub fn smart() -> bool { SMART.with(|c| c.get()) }
pub fn set_smart(b: bool) { SMART.with(|c| c.set(b)); save(); }
pub fn history() -> bool { HIST.with(|c| c.get()) }
pub fn set_history(b: bool) { HIST.with(|c| c.set(b)); save(); }
pub fn autocomp() -> bool { AUTOCOMP.with(|c| c.get()) }
pub fn set_autocomp(b: bool) { AUTOCOMP.with(|c| c.set(b)); save(); }

/// Dossier de réglages (favoris, mise en page, récupération), vide avant `init`.
pub fn dir() -> PathBuf { DIR.with(|d| d.borrow().clone()) }

/// Lit `prefs.ini` dans `dir` (à appeler au démarrage).
pub fn init(dir: PathBuf) {
    if let Ok(s) = std::fs::read_to_string(dir.join("prefs.ini")) {
        for l in s.lines() {
            match l.split_once('=') {
                Some(("guides", v)) => VIEW.with(|c| c.set((v == "1", c.get().1))),
                Some(("eol", v)) => VIEW.with(|c| c.set((c.get().0, v == "1"))),
                Some(("smart", v)) => SMART.with(|c| c.set(v == "1")),
                Some(("history", v)) => HIST.with(|c| c.set(v == "1")),
                Some(("autocomp", v)) => AUTOCOMP.with(|c| c.set(v == "1")),
                Some(("indent", v)) => INDENT.with(|c| c.set(match v {
                    "tabs" => Indent::Tabs,
                    x => x.strip_prefix("spaces").and_then(|n| n.parse().ok()).filter(|n| (1..=16).contains(n)).map_or(Indent::Auto, Indent::Spaces),
                })),
                _ => {}
            }
        }
    }
    DIR.with(|d| *d.borrow_mut() = dir);
}

fn save() {
    let dir = DIR.with(|d| d.borrow().clone());
    if dir.as_os_str().is_empty() {
        return;
    }
    let ind = match indent() { Indent::Auto => "auto".to_string(), Indent::Tabs => "tabs".into(), Indent::Spaces(n) => format!("spaces{n}") };
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join("prefs.ini"), format!("autocomp={}\nindent={ind}\nguides={}\neol={}\nsmart={}\nhistory={}\n", autocomp() as u8, view().0 as u8, view().1 as u8, smart() as u8, history() as u8));
}

/// Largeur et mode (tabulations ?) par défaut d'un langage.
pub fn lang_default(lexer: &str, name: &str) -> (usize, bool) {
    match (lexer, name) {
        (_, "JavaScript / TypeScript") => (2, false),
        ("makefile", _) | (_, "Go") => (4, true),
        (_, "Python") | (_, "Rust") | (_, "C#") | (_, "Java / Kotlin") | (_, "Swift / Objective-C") | (_, "Zig / Solidity / Protobuf") | ("gdscript", _) | ("nim", _) | ("cpp", _) | ("powershell", _) | ("sql", _) => (4, false),
        ("yaml", _) | ("json", _) | ("hypertext", _) | ("xml", _) | ("css", _) | ("ruby", _) | ("dart", _) | ("lua", _) | ("bash", _) | ("toml", _) | ("markdown", _) => (2, false),
        _ => (4, true),
    }
}

/// Largeur et mode réels pour un langage, selon le choix de l'utilisateur.
pub fn tab_style(lexer: &str, name: &str) -> (usize, bool) {
    let (w, t) = lang_default(lexer, name);
    match indent() {
        Indent::Auto => (w, t),
        Indent::Spaces(n) => (n as usize, false),
        Indent::Tabs => (w.max(4), true),
    }
}

/// Applique l'indentation à une vue Scintilla.
pub fn apply_indent(s: HWND, lang: usize) {
    let l = &crate::lang::LANGS[lang.min(crate::lang::LANGS.len() - 1)];
    let (w, tabs) = tab_style(l.lexer, l.name);
    send(s, crate::sci::SETTABWIDTH, w, 0);
    send(s, SETINDENT, if tabs { 0 } else { w }, 0);
    send(s, crate::sci::SETUSETABS, tabs as usize, 0);
    send(s, SETTABINDENTS, 1, 0);
    send(s, 2356, view().1 as usize, 0); // SETVIEWEOL
    send(s, SETBACKSPACEUNINDENTS, 1, 0);
    send(s, 2780, if history() { 3 } else { 0 }, 0); // SETCHANGEHISTORY : marqueurs de modification
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn styles() {
        assert_eq!(tab_style("python", "Python"), (4, false));
        assert_eq!(tab_style("yaml", "YAML"), (2, false));
        assert_eq!(tab_style("makefile", "Makefile"), (4, true));
        assert_eq!(tab_style("", "Texte brut"), (4, true));
        assert_eq!(tab_style("cpp", "JavaScript / TypeScript"), (2, false));
        INDENT.with(|c| c.set(Indent::Spaces(8)));
        assert_eq!(tab_style("makefile", "Makefile"), (8, false));
        INDENT.with(|c| c.set(Indent::Auto));
    }
}
