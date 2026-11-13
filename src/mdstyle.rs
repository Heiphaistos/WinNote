//! Style Markdown en direct, partagé Windows / Linux / macOS : attributs des styles + pose des styles sur un contrôle Scintilla.
use crate::mdrender::{live, BOLD, BOLDITAL, BULLET, CODE, CODEBLK, DIM, ITAL, LINK, QUOTE};
use crate::sci::{self, send, HWND};
use crate::theme::{blend, Theme};

const MAX_LIVE: usize = 4_000_000; // au-delà : pas de style en direct (fichiers énormes)

/// Attributs des styles Markdown sur un contrôle Scintilla (éditeur ou aperçu). `pct` = taille des titres H1..H6 en %, `mono` = police du code.
pub fn attrs(p: HWND, t: &Theme, size: i32, pct: [i32; 6], mono: &str) {
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
        send(p, sci::STYLESETFONT, st as usize, crate::cstr(mono).as_ptr() as isize);
        send(p, sci::STYLESETBACK, st as usize, blend(t.bg, t.fg, 8) as isize);
    }
}

/// Texte complet d'un contrôle (UTF-8), ou None s'il est trop gros / invalide.
pub fn text_of(p: HWND, max: usize) -> Option<String> {
    let n = send(p, sci::GETLENGTH, 0, 0) as usize;
    if n > max {
        return None;
    }
    let mut buf = vec![0u8; n + 1];
    send(p, sci::GETTEXT, n + 1, buf.as_mut_ptr() as isize);
    buf.truncate(n);
    String::from_utf8(buf).ok()
}

/// Restyle tout le document du contrôle (contrôle sans lexer Lexilla).
pub fn apply(p: HWND) {
    let Some(src) = text_of(p, MAX_LIVE) else { return };
    let st = live(&src);
    send(p, sci::STARTSTYLING, 0, 0);
    send(p, sci::SETSTYLINGEX, st.len(), st.as_ptr() as isize);
}

/// Un texte sans nom ressemble-t-il à du Markdown ? (ligne « # titre » ou bloc de code)
pub fn looks_markdown(t: &str) -> bool {
    t.lines().any(|l| {
        let h = l.bytes().take_while(|&c| c == b'#').count();
        (1..=6).contains(&h) && l.as_bytes().get(h) == Some(&b' ') && l.len() > h + 1
    }) || t.matches("```").count() >= 2
}
