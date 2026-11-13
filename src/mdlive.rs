//! Markdown en direct dans l'éditeur Windows : minuteur de restyle + détection auto du Markdown (le style vient de mdstyle).
use crate::mdstyle;
use crate::theme::Theme;
use crate::app;
use windows_sys::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

pub fn attrs(p: windows_sys::Win32::Foundation::HWND, t: &Theme, size: i32, pct: [i32; 6]) {
    mdstyle::attrs(p, t, size, pct, "Consolas");
}

pub fn is_md() -> bool {
    let a = app();
    a.docs.get(a.cur).is_some_and(|d| crate::lang::LANGS[d.lang].lexer == "markdown")
}

/// Restyle tout le document courant (si Markdown).
pub fn style_doc() {
    if is_md() {
        mdstyle::apply(app().sci);
    }
}

pub fn schedule() {
    let a = app();
    if is_md() {
        unsafe { SetTimer(a.hwnd, 6, 60, None) };
    } else if a.docs.get(a.cur).is_some_and(|d| d.path.is_none() && d.lang == 0) {
        unsafe { SetTimer(a.hwnd, 6, 400, None) };
    }
}

pub fn on_timer() {
    let a = app();
    unsafe { KillTimer(a.hwnd, 6) };
    // Document sans nom en texte brut : dès qu'une ligne ressemble à un titre Markdown, on passe en Markdown.
    if a.docs.get(a.cur).is_some_and(|d| d.path.is_none() && d.lang == 0) {
        if mdstyle::text_of(a.sci, 200_000).is_some_and(|t| mdstyle::looks_markdown(&t)) {
            if let Some(i) = crate::lang::LANGS.iter().position(|l| l.lexer == "markdown") {
                crate::cmds::set_language(i);
            }
        }
        return;
    }
    style_doc();
}
