//! Aperçu Markdown natif (second contrôle Scintilla en lecture seule, stylé par un mini-rendu).
use crate::sci::{self, send};
use crate::theme::THEMES;
use crate::ui::{blend, scale};
use crate::{app, cstr, w};
use std::ptr::null;
use windows_sys::Win32::{Foundation::*, System::LibraryLoader::GetModuleHandleW, UI::WindowsAndMessaging::*};

use crate::mdrender::{render, BOLD, BOLDITAL, BULLET, CODE, CODEBLK, DIM, ITAL, LINK, QUOTE};

pub fn width() -> i32 {
    let a = app();
    if a.preview || a.split {
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
