//! Barre de recherche / remplacement intégrée (style Bloc-notes Windows 11), avec surlignage de toutes les occurrences.
use crate::sci::{self, send};
use crate::theme::THEMES;
use crate::ui::{blend, scale};
use crate::{app, w};
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    UI::Controls::EM_SETSEL,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

const ID_FIND: isize = 10;
const ID_REPL: isize = 11;
const EN_CHANGE: usize = 0x0300;
const INDIC: usize = 8;

#[derive(Clone, Copy, PartialEq)]
enum Btn { Case, Word, Regex, Prev, Next, Close, Replace, ReplaceAll }

pub fn height() -> i32 {
    let a = app();
    if !a.find_vis { 0 } else if a.find_replace { scale(80) } else { scale(44) }
}

fn buttons() -> Vec<(Btn, RECT)> {
    let (s, mut x) = (scale(30), scale(12) + scale(320) + scale(10));
    let top = scale(9);
    let mut v = Vec::new();
    for b in [Btn::Case, Btn::Word, Btn::Regex, Btn::Prev, Btn::Next] {
        v.push((b, RECT { left: x, top, right: x + s, bottom: top + scale(26) }));
        x += s + scale(2);
    }
    let a = app();
    let mut rc: RECT = unsafe { zeroed() };
    unsafe { GetClientRect(a.find_bar, &mut rc) };
    v.push((Btn::Close, RECT { left: rc.right - scale(40), top, right: rc.right - scale(10), bottom: top + scale(26) }));
    if a.find_replace {
        let top2 = scale(45);
        let x0 = scale(12) + scale(320) + scale(10);
        v.push((Btn::Replace, RECT { left: x0, top: top2, right: x0 + scale(96), bottom: top2 + scale(26) }));
        v.push((Btn::ReplaceAll, RECT { left: x0 + scale(100), top: top2, right: x0 + scale(100) + scale(70), bottom: top2 + scale(26) }));
    }
    v
}

pub fn layout_edits() {
    let a = app();
    unsafe {
        MoveWindow(a.find_edit, scale(20), scale(14), scale(300), scale(16) + 4, 1);
        MoveWindow(a.repl_edit, scale(20), scale(50), scale(300), scale(16) + 4, 1);
        ShowWindow(a.repl_edit, if a.find_replace { SW_SHOW } else { SW_HIDE });
    }
}

pub fn create(parent: HWND) {
    let a = app();
    unsafe {
        let inst = GetModuleHandleW(null());
        a.find_bar = CreateWindowExW(0, w("WNFind").as_ptr(), null(), WS_CHILD | WS_CLIPCHILDREN | WS_CLIPSIBLINGS, 0, 0, 0, 0, parent, 4 as HMENU, inst, null());
        let mk = |id: isize| {
            let e = CreateWindowExW(0, w("EDIT").as_ptr(), null(), WS_CHILD | WS_VISIBLE | ES_AUTOHSCROLL as u32, 0, 0, 0, 0, a.find_bar, id as HMENU, inst, null());
            SendMessageW(e, WM_SETFONT, a.ui_font as usize, 1);
            e
        };
        a.find_edit = mk(ID_FIND);
        a.repl_edit = mk(ID_REPL);
    }
}

fn text_of(e: HWND) -> String {
    let mut b = vec![0u16; 1024];
    let n = unsafe { GetWindowTextW(e, b.as_mut_ptr(), 1024) } as usize;
    String::from_utf16_lossy(&b[..n])
}

fn flags() -> usize {
    let a = app();
    (if a.f_case { 4 } else { 0 }) | (if a.f_word { 2 } else { 0 }) | (if a.f_regex { 0x0020_0000 | 0x0080_0000 } else { 0 })
}

fn search(st: isize, en: isize, what: &[u8]) -> isize {
    let s = app().sci;
    send(s, sci::SETTARGETSTART, st as usize, 0);
    send(s, sci::SETTARGETEND, en as usize, 0);
    // le moteur regex C++11 lit un texte terminé par un zéro : sans lui, résultat aléatoire
    let mut z = what.to_vec();
    z.push(0);
    send(s, sci::SEARCHINTARGET, what.len(), z.as_ptr() as isize)
}

fn query() -> Vec<u8> {
    text_of(app().find_edit).into_bytes()
}

fn find_from(from_sel_start: bool, down: bool) -> bool {
    let what = query();
    let s = app().sci;
    if what.is_empty() {
        mark_all();
        return false;
    }
    let len = send(s, sci::GETLENGTH, 0, 0);
    send(s, sci::SETSEARCHFLAGS, flags(), 0);
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    let mut r = if down { search(if from_sel_start { ss } else { se }, len, &what) } else { search(ss, 0, &what) };
    if r < 0 {
        r = if down { search(0, len, &what) } else { search(len, 0, &what) };
    }
    if r >= 0 {
        let (ts, te) = (send(s, sci::GETTARGETSTART, 0, 0), send(s, sci::GETTARGETEND, 0, 0));
        send(s, sci::SETSEL, ts as usize, te);
    }
    mark_all();
    r >= 0
}

pub fn find_next(down: bool) {
    let a = app();
    if !a.find_vis || query().is_empty() {
        open(false);
        return;
    }
    find_from(false, down);
}

/// Surligne toutes les occurrences et met à jour le compteur.
pub fn mark_all() {
    let a = app();
    let s = a.sci;
    let len = send(s, sci::GETLENGTH, 0, 0);
    send(s, sci::SETINDICCURRENT, INDIC, 0);
    send(s, sci::INDICCLEAR, 0, len);
    a.find_count = 0;
    let what = query();
    if !what.is_empty() && a.find_vis {
        send(s, sci::SETSEARCHFLAGS, flags(), 0);
        let mut pos = 0;
        while a.find_count < 5000 {
            if search(pos, len, &what) < 0 {
                break;
            }
            let (ts, te) = (send(s, sci::GETTARGETSTART, 0, 0), send(s, sci::GETTARGETEND, 0, 0));
            send(s, sci::INDICFILL, ts as usize, te - ts);
            a.find_count += 1;
            pos = if te > ts { te } else { te + 1 };
            if pos >= len {
                break;
            }
        }
    }
    unsafe { InvalidateRect(a.find_bar, null(), 0) };
}

pub fn style_indicator() {
    let a = app();
    let t = &THEMES[a.theme];
    send(a.sci, sci::INDICSETSTYLE, INDIC, 8); // INDIC_STRAIGHTBOX
    send(a.sci, sci::INDICSETFORE, INDIC, t.accent as isize);
    send(a.sci, sci::INDICSETALPHA, INDIC, 70);
    send(a.sci, sci::INDICSETOUTLINEALPHA, INDIC, 160);
}

fn replace_target(rep: &[u8]) {
    let s = app().sci;
    let m = if app().f_regex { sci::REPLACETARGETRE } else { sci::REPLACETARGET };
    let mut z = rep.to_vec();
    z.push(0);
    send(s, m, rep.len(), z.as_ptr() as isize);
}

pub fn replace_one() {
    let a = app();
    let s = a.sci;
    let what = query();
    let rep = text_of(a.repl_edit).into_bytes();
    send(s, sci::SETSEARCHFLAGS, flags(), 0);
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    if !what.is_empty() && search(ss, se, &what) == ss && send(s, sci::GETTARGETEND, 0, 0) == se {
        replace_target(&rep);
        let e = send(s, sci::GETTARGETEND, 0, 0);
        send(s, sci::SETSEL, e as usize, e);
    }
    find_from(false, true);
}

pub fn replace_all() {
    let a = app();
    let s = a.sci;
    let what = query();
    if what.is_empty() {
        return;
    }
    let rep = text_of(a.repl_edit).into_bytes();
    send(s, sci::SETSEARCHFLAGS, flags(), 0);
    send(s, sci::BEGINUNDO, 0, 0);
    let (mut pos, mut n) = (0isize, 0);
    loop {
        let len = send(s, sci::GETLENGTH, 0, 0);
        if pos > len || search(pos, len, &what) < 0 {
            break;
        }
        replace_target(&rep);
        let e = send(s, sci::GETTARGETEND, 0, 0);
        pos = if e == send(s, sci::GETTARGETSTART, 0, 0) { e + 1 } else { e };
        n += 1;
    }
    send(s, sci::ENDUNDO, 0, 0);
    mark_all();
    crate::cmds::msg_box(&format!("{n} remplacement(s) effectué(s)."));
}

pub fn open(replace: bool) {
    let a = app();
    let s = a.sci;
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    if se > ss && se - ss < 200 {
        let mut b = vec![0u8; (se - ss) as usize + 1];
        send(s, sci::GETSELTEXT, 0, b.as_mut_ptr() as isize);
        let t = String::from_utf8_lossy(&b[..b.len() - 1]).to_string();
        if !t.contains('\n') && !t.contains('\r') {
            unsafe { SetWindowTextW(a.find_edit, w(&t).as_ptr()) };
        }
    }
    a.find_vis = true;
    a.find_replace = replace;
    layout_edits();
    crate::relayout();
    unsafe {
        ShowWindow(a.find_bar, SW_SHOW);
        SetFocus(a.find_edit);
        SendMessageW(a.find_edit, EM_SETSEL, 0, -1);
    }
    mark_all();
}

pub fn close() {
    let a = app();
    if !a.find_vis {
        return;
    }
    a.find_vis = false;
    send(a.sci, sci::SETINDICCURRENT, INDIC, 0);
    send(a.sci, sci::INDICCLEAR, 0, send(a.sci, sci::GETLENGTH, 0, 0));
    unsafe { ShowWindow(a.find_bar, SW_HIDE) };
    crate::relayout();
    unsafe { SetFocus(a.sci) };
}

/// Touches Entrée / Échap dans les champs de la barre. Renvoie true si traité.
pub fn pretranslate(m: &MSG) -> bool {
    let a = app();
    if m.message != WM_KEYDOWN || (m.hwnd != a.find_edit && m.hwnd != a.repl_edit) {
        return false;
    }
    let shift = unsafe { GetKeyState(VK_SHIFT as i32) } < 0;
    match m.wParam as u16 {
        VK_RETURN => {
            if m.hwnd == a.repl_edit {
                replace_one();
            } else {
                find_from(false, !shift);
            }
            true
        }
        VK_ESCAPE => {
            close();
            true
        }
        _ => false,
    }
}

fn toggled(b: Btn) -> bool {
    let a = app();
    match b { Btn::Case => a.f_case, Btn::Word => a.f_word, Btn::Regex => a.f_regex, _ => false }
}

fn paint(dc: HDC, rc: &RECT) {
    let a = app();
    let t = &THEMES[a.theme];
    unsafe {
        let fill = |r: &RECT, c: u32| {
            let b = CreateSolidBrush(c);
            FillRect(dc, r, b);
            DeleteObject(b);
        };
        fill(rc, t.ui_bg);
        let field = blend(t.ui_bg, t.bg, 80);
        let round = |r: &RECT, c: u32, border: u32| {
            let (b, p) = (CreateSolidBrush(c), CreatePen(PS_SOLID, 1, border));
            let (ob, op) = (SelectObject(dc, b), SelectObject(dc, p));
            RoundRect(dc, r.left, r.top, r.right, r.bottom, scale(8), scale(8));
            SelectObject(dc, ob);
            SelectObject(dc, op);
            DeleteObject(b);
            DeleteObject(p);
        };
        let border = blend(t.ui_bg, t.ui_fg, 18);
        round(&RECT { left: scale(12), top: scale(9), right: scale(332), bottom: scale(35) }, field, border);
        if a.find_replace {
            round(&RECT { left: scale(12), top: scale(45), right: scale(332), bottom: scale(71) }, field, border);
        }
        SetBkMode(dc, TRANSPARENT as i32);
        SelectObject(dc, a.ui_font);
        let hot = blend(t.ui_bg, t.accent, 35);
        for (b, r) in buttons() {
            let on = toggled(b);
            if on {
                round(&r, hot, t.accent);
            } else if matches!(b, Btn::Replace | Btn::ReplaceAll) {
                round(&r, blend(t.ui_bg, t.ui_fg, 8), border);
            }
            let label = match b { Btn::Case => "Aa", Btn::Word => "ab", Btn::Regex => ".*", Btn::Prev => "↑", Btn::Next => "↓", Btn::Close => "×", Btn::Replace => "Remplacer", Btn::ReplaceAll => "Tout" };
            SetTextColor(dc, t.ui_fg);
            let mut rr = r;
            let v: Vec<u16> = label.encode_utf16().collect();
            DrawTextW(dc, v.as_ptr(), v.len() as i32, &mut rr, DT_CENTER | DT_VCENTER | DT_SINGLELINE | DT_NOPREFIX);
        }
        let msg = if query().is_empty() { String::new() } else if a.find_count == 0 { "Aucun résultat".into() } else { format!("{} résultat(s)", a.find_count) };
        let x = buttons().iter().find(|b| b.0 == Btn::Next).map_or(0, |b| b.1.right + scale(12));
        let mut r = RECT { left: x, top: scale(9), right: rc.right - scale(44), bottom: scale(35) };
        SetTextColor(dc, blend(t.ui_fg, t.ui_bg, 35));
        let v: Vec<u16> = msg.encode_utf16().collect();
        if !v.is_empty() {
            DrawTextW(dc, v.as_ptr(), v.len() as i32, &mut r, DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS);
        }
        let sepc = blend(t.ui_bg, t.ui_fg, 12);
        fill(&RECT { left: 0, top: rc.bottom - 1, right: rc.right, bottom: rc.bottom }, sepc);
    }
}

pub unsafe extern "system" fn proc_(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let a = app();
    match m {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let dc = BeginPaint(h, &mut ps);
            let mut rc: RECT = zeroed();
            GetClientRect(h, &mut rc);
            let (mem, bmp) = (CreateCompatibleDC(dc), CreateCompatibleBitmap(dc, rc.right, rc.bottom));
            let old = SelectObject(mem, bmp);
            paint(mem, &rc);
            BitBlt(dc, 0, 0, rc.right, rc.bottom, mem, 0, 0, SRCCOPY);
            SelectObject(mem, old);
            DeleteObject(bmp);
            DeleteDC(mem);
            EndPaint(h, &ps);
            0
        }
        0x0133 /* WM_CTLCOLOREDIT */ => {
            let t = &THEMES[a.theme];
            SetTextColor(wp as HDC, t.ui_fg);
            SetBkColor(wp as HDC, blend(t.ui_bg, t.bg, 80));
            a.find_brush as LRESULT
        }
        WM_COMMAND => {
            if (wp >> 16) == EN_CHANGE && (wp & 0xFFFF) as isize == ID_FIND {
                find_from(true, true);
            }
            0
        }
        WM_LBUTTONDOWN => {
            let (x, y) = ((lp & 0xFFFF) as i16 as i32, ((lp >> 16) & 0xFFFF) as i16 as i32);
            for (b, r) in buttons() {
                if x >= r.left && x < r.right && y >= r.top && y < r.bottom {
                    match b {
                        Btn::Case => a.f_case = !a.f_case,
                        Btn::Word => a.f_word = !a.f_word,
                        Btn::Regex => a.f_regex = !a.f_regex,
                        Btn::Prev => drop(find_from(false, false)),
                        Btn::Next => drop(find_from(false, true)),
                        Btn::Close => close(),
                        Btn::Replace => replace_one(),
                        Btn::ReplaceAll => replace_all(),
                    }
                    if matches!(b, Btn::Case | Btn::Word | Btn::Regex) {
                        find_from(true, true);
                    }
                    InvalidateRect(h, null(), 0);
                    break;
                }
            }
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

pub fn recolor() {
    let a = app();
    unsafe {
        if !a.find_brush.is_null() {
            DeleteObject(a.find_brush);
        }
        let t = &THEMES[a.theme];
        a.find_brush = CreateSolidBrush(blend(t.ui_bg, t.bg, 80));
        InvalidateRect(a.find_bar, null(), 0);
    }
    style_indicator();
    let _ = null_mut::<u8>();
}

/// Pour l'autotest : requête courante et drapeaux de recherche.
pub fn debug_state() -> String {
    format!("q={:?} flags={:#x}", String::from_utf8_lossy(&query()), flags())
}
