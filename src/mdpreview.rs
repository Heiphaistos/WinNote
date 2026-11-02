//! Aperçu Markdown natif (second contrôle Scintilla en lecture seule, stylé par un mini-rendu).
use crate::sci::{self, send};
use crate::theme::THEMES;
use crate::ui::{blend, scale};
use crate::{app, cstr, w};
use std::ptr::null;
use windows_sys::Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*};

const N: u8 = 0;
const BOLD: u8 = 7;
const ITAL: u8 = 8;
const BOLDITAL: u8 = 9;
const CODE: u8 = 10;
const CODEBLK: u8 = 11;
const QUOTE: u8 = 12;
const LINK: u8 = 13;
const DIM: u8 = 14;
const BULLET: u8 = 15;

pub fn width() -> i32 {
    let a = app();
    if a.preview {
        let mut rc: RECT = unsafe { std::mem::zeroed() };
        unsafe { GetClientRect(a.hwnd, &mut rc) };
        ((rc.right - crate::sidebar::width()) / 2).max(scale(200))
    } else {
        0
    }
}

pub fn create(parent: HWND) {
    let a = app();
    unsafe {
        a.prev = CreateWindowExW(0, w("Scintilla").as_ptr(), null(), WS_CHILD | WS_CLIPSIBLINGS, 0, 0, 0, 0, parent, 7 as HMENU, GetModuleHandleW(null()), null());
        let p = a.prev;
        send(p, sci::SETCODEPAGE, 65001, 0);
        send(p, sci::SETTECH, 1, 0);
        send(p, sci::SETBUFDRAW, 0, 0);
        send(p, sci::SETWRAP, 1, 0);
        send(p, sci::SETILEXER, 0, 0);
        send(p, sci::SETMARGINW, 0, 0);
        send(p, sci::SETMARGINW, 1, 0);
        send(p, sci::SETMARGINW, 2, 0);
        send(p, sci::SETCARETLINEVIS, 0, 0);
        send(p, 2512, 0, 0); // SCI_SETCARETSTYLE = invisible
        send(p, sci::USEPOPUP, 0, 0);
    }
}

fn inline(s: &str, base: u8, out: &mut Vec<(char, u8)>) {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let find = |from: usize, pat: &[char]| -> Option<usize> { (from..c.len().saturating_sub(pat.len() - 1)).find(|&j| c[j..j + pat.len()] == *pat) };
    while i < c.len() {
        let rest = &c[i..];
        if rest.starts_with(&['*', '*']) {
            if let Some(e) = find(i + 2, &['*', '*']) {
                let inner: String = c[i + 2..e].iter().collect();
                inline(&inner, if base == ITAL { BOLDITAL } else { BOLD }, out);
                i = e + 2;
                continue;
            }
        }
        if (c[i] == '*' || c[i] == '_') && i + 1 < c.len() && !c[i + 1].is_whitespace() {
            if let Some(e) = find(i + 1, &[c[i]]) {
                if e > i + 1 && !(c[i] == '_' && i > 0 && c[i - 1].is_alphanumeric()) {
                    let inner: String = c[i + 1..e].iter().collect();
                    inline(&inner, if base == BOLD { BOLDITAL } else { ITAL }, out);
                    i = e + 1;
                    continue;
                }
            }
        }
        if c[i] == '`' {
            if let Some(e) = find(i + 1, &['`']) {
                out.extend(c[i + 1..e].iter().map(|&ch| (ch, CODE)));
                i = e + 1;
                continue;
            }
        }
        if rest.starts_with(&['!', '[']) || c[i] == '[' {
            let img = c[i] == '!';
            let st = if img { i + 2 } else { i + 1 };
            if let Some(e) = find(st, &[']']) {
                let double = !img && c.get(st) == Some(&'[');
                if double {
                    if let Some(e2) = find(st + 1, &[']', ']']) {
                        out.extend(c[st + 1..e2].iter().map(|&ch| (ch, LINK)));
                        i = e2 + 2;
                        continue;
                    }
                } else if c.get(e + 1) == Some(&'(') {
                    if let Some(e2) = find(e + 2, &[')']) {
                        let label: String = c[st..e].iter().collect();
                        if img {
                            out.extend(format!("[image : {label}]").chars().map(|ch| (ch, DIM)));
                        } else {
                            out.extend(label.chars().map(|ch| (ch, LINK)));
                        }
                        i = e2 + 1;
                        continue;
                    }
                }
            }
        }
        out.push((c[i], base));
        i += 1;
    }
}

/// Markdown -> (texte, style par caractère).
pub fn render(md: &str) -> Vec<(char, u8)> {
    let mut out: Vec<(char, u8)> = Vec::new();
    let mut in_code = false;
    for l in md.lines() {
        let t = l.trim_start();
        if t.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            out.extend(l.chars().map(|c| (c, CODEBLK)));
        } else if let Some(h) = (1..=6).rev().find(|&n| t.starts_with(&format!("{} ", "#".repeat(n)))) {
            inline(t[h + 1..].trim(), h as u8, &mut out);
        } else if t.starts_with("> ") {
            out.extend("▎ ".chars().map(|c| (c, DIM)));
            inline(&t[2..], QUOTE, &mut out);
        } else if ["- ", "* ", "+ "].iter().any(|p| t.starts_with(p)) {
            out.extend(std::iter::repeat((' ', N)).take((l.len() - t.len()) / 2 * 2));
            out.push(('•', BULLET));
            out.push((' ', N));
            let task = t[2..].trim_start();
            if let Some(r) = task.strip_prefix("[ ] ") {
                out.extend("☐ ".chars().map(|c| (c, DIM)));
                inline(r, N, &mut out);
            } else if let Some(r) = task.strip_prefix("[x] ").or_else(|| task.strip_prefix("[X] ")) {
                out.extend("☑ ".chars().map(|c| (c, BULLET)));
                inline(r, N, &mut out);
            } else {
                inline(&t[2..], N, &mut out);
            }
        } else if t.len() >= 3 && t.chars().all(|c| matches!(c, '-' | '*' | '_' | ' ')) && t.chars().filter(|c| !c.is_whitespace()).count() >= 3 {
            out.extend("────────────────────────".chars().map(|c| (c, DIM)));
        } else if t.starts_with('|') {
            if t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) {
                continue;
            }
            for cell in t.trim_matches('|').split('|') {
                out.extend(" │ ".chars().map(|c| (c, DIM)));
                inline(cell.trim(), N, &mut out);
            }
        } else {
            inline(l, N, &mut out);
        }
        out.push(('\n', N));
    }
    out
}

pub fn restyle() {
    let a = app();
    let (p, t) = (a.prev, &THEMES[a.theme]);
    if p.is_null() {
        return;
    }
    let font = cstr(&a.font);
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
        send(p, sci::STYLESETFONT, st as usize, cstr("Consolas").as_ptr() as isize);
        send(p, sci::STYLESETBACK, st as usize, blend(t.bg, t.fg, 8) as isize);
    }
    let m = scale(28) as isize;
    send(p, sci::SETMARGINLEFT, 0, m);
    send(p, sci::SETMARGINRIGHT, 0, m);
}

pub fn update() {
    let a = app();
    if !a.preview || a.prev.is_null() || a.docs.is_empty() {
        return;
    }
    let n = send(a.sci, sci::GETLENGTH, 0, 0) as usize;
    let mut buf = vec![0u8; n + 1];
    send(a.sci, sci::GETTEXT, n + 1, buf.as_mut_ptr() as isize);
    let r = if crate::lang::LANGS[a.docs[a.cur].lang].lexer == "markdown" {
        render(&String::from_utf8_lossy(&buf[..n]))
    } else {
        "Aperçu disponible pour les fichiers Markdown (.md).".chars().map(|c| (c, DIM)).collect()
    };
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

pub fn schedule() {
    let a = app();
    if a.preview {
        unsafe { SetTimer(a.hwnd, 1, 350, None) };
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn renders_styles() {
        let r = super::render("# Titre\n\nUn **gras** et `code`\n- item\n");
        let s: String = r.iter().map(|x| x.0).collect();
        assert!(s.contains("Titre") && s.contains("gras et code") && s.contains("• item"), "{s}");
        assert_eq!(r[0].1, 1);
        assert!(r.iter().any(|x| x.1 == super::BOLD) && r.iter().any(|x| x.1 == super::CODE));
    }
}
