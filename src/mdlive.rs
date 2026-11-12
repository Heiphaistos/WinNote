//! Markdown en direct dans l'éditeur : titres plus grands, gras, italique, code, liens, citations, listes (styles posés par mdrender::live).
use crate::mdrender::{live, BOLD, BOLDITAL, BULLET, CODE, CODEBLK, DIM, ITAL, LINK, QUOTE};
use crate::sci::{self, send};
use crate::theme::Theme;
use crate::ui::blend;
use crate::{app, cstr};
use windows_sys::Win32::UI::WindowsAndMessaging::{KillTimer, SetTimer};

const MAX_LIVE: usize = 4_000_000; // au-delà : pas de style en direct (fichiers énormes)

/// Attributs des styles Markdown sur un contrôle Scintilla (éditeur ou aperçu). `pct` = taille des titres H1..H6 en %.
pub fn attrs(p: windows_sys::Win32::Foundation::HWND, t: &Theme, size: i32, pct: [i32; 6]) {
    let set = |st: usize, fore: u32, bold: bool, ital: bool, sz: i32| {
        send(p, sci::STYLESETFORE, st, fore as isize);
        send(p, sci::STYLESETBOLD, st, bold as isize);
        send(p, sci::STYLESETITALIC, st, ital as isize);
        if sz != 0 {
            send(p, sci::STYLESETSIZE, st, sz as isize);
        }
    };
    for (i, m) in pct.iter().enumerate() {
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
        send(p, sci::STYLESETFONT, st as usize, cstr("Consolas").as_ptr() as isize);
        send(p, sci::STYLESETBACK, st as usize, blend(t.bg, t.fg, 8) as isize);
    }
}

pub fn is_md() -> bool {
    let a = app();
    a.docs.get(a.cur).is_some_and(|d| crate::lang::LANGS[d.lang].lexer == "markdown")
}

/// Restyle tout le document courant (si Markdown).
pub fn style_doc() {
    let a = app();
    if !is_md() {
        return;
    }
    let n = send(a.sci, sci::GETLENGTH, 0, 0) as usize;
    if n > MAX_LIVE {
        return;
    }
    let mut buf = vec![0u8; n + 1];
    send(a.sci, sci::GETTEXT, n + 1, buf.as_mut_ptr() as isize);
    let src = String::from_utf8_lossy(&buf[..n]);
    // from_utf8_lossy peut changer la longueur (octets invalides) : dans ce cas on ne style pas.
    if src.len() != n {
        return;
    }
    let st = live(&src);
    send(a.sci, sci::STARTSTYLING, 0, 0);
    send(a.sci, sci::SETSTYLINGEX, st.len(), st.as_ptr() as isize);
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
        let n = (send(a.sci, sci::GETLENGTH, 0, 0) as usize).min(200_000);
        let mut buf = vec![0u8; n + 1];
        send(a.sci, sci::GETTEXT, n + 1, buf.as_mut_ptr() as isize);
        let t = String::from_utf8_lossy(&buf[..n]);
        let looks = t.lines().any(|l| {
            let h = l.bytes().take_while(|&c| c == b'#').count();
            (1..=6).contains(&h) && l.as_bytes().get(h) == Some(&b' ') && l.len() > h + 1
        }) || t.contains("```\n") && t.matches("```").count() >= 2;
        if looks {
            if let Some(i) = crate::lang::LANGS.iter().position(|l| l.lexer == "markdown") {
                crate::cmds::set_language(i);
            }
        }
        return;
    }
    style_doc();
}
