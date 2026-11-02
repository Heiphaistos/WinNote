//! Barre d'onglets (style Bloc-notes Windows 11) et barre d'état, dessinées en GDI.
use crate::theme::THEMES;
use crate::{app, lang::LANGS, sci, w};
use std::mem::zeroed;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

pub fn scale(v: i32) -> i32 {
    v * app().dpi as i32 / 96
}
pub fn tabs_height() -> i32 {
    scale(40)
}
pub fn status_height() -> i32 {
    scale(28)
}

fn blend(a: u32, b: u32, t: u32) -> u32 {
    let f = |s: u32| ((a >> s & 0xFF) * (100 - t) + (b >> s & 0xFF) * t) / 100;
    f(0) | f(8) << 8 | f(16) << 16
}

#[derive(Clone, Copy, PartialEq)]
enum Hit { None, Tab(usize), Close(usize), Plus }

static mut HOVER: Hit = Hit::None;

fn tab_rects(width: i32, n: usize) -> (Vec<RECT>, RECT) {
    let pad = scale(8);
    let avail = width - pad * 2 - scale(44);
    let tw = (avail / n.max(1) as i32).clamp(scale(110), scale(220));
    let rects = (0..n as i32)
        .map(|i| RECT { left: pad + i * tw, top: scale(6), right: pad + (i + 1) * tw - scale(2), bottom: tabs_height() })
        .collect();
    let l = pad + n as i32 * tw + scale(4);
    (rects, RECT { left: l, top: scale(8), right: l + scale(30), bottom: scale(38) })
}

fn close_rect(t: &RECT) -> RECT {
    let s = scale(22);
    let cy = (t.top + t.bottom) / 2;
    RECT { left: t.right - s - scale(6), top: cy - s / 2, right: t.right - scale(6), bottom: cy + s / 2 }
}

fn inside(r: &RECT, x: i32, y: i32) -> bool {
    x >= r.left && x < r.right && y >= r.top && y < r.bottom
}

fn hit_test(h: HWND, x: i32, y: i32) -> Hit {
    let mut rc: RECT = unsafe { zeroed() };
    unsafe { GetClientRect(h, &mut rc) };
    let (tabs, plus) = tab_rects(rc.right, app().docs.len());
    if inside(&plus, x, y) {
        return Hit::Plus;
    }
    for (i, t) in tabs.iter().enumerate() {
        if inside(t, x, y) {
            return if inside(&close_rect(t), x, y) { Hit::Close(i) } else { Hit::Tab(i) };
        }
    }
    Hit::None
}

fn text(dc: HDC, s: &str, r: &mut RECT, flags: u32) {
    let v: Vec<u16> = s.encode_utf16().collect();
    unsafe { DrawTextW(dc, v.as_ptr(), v.len() as i32, r, flags | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX) };
}

unsafe fn fill(dc: HDC, r: &RECT, color: u32) {
    let b = CreateSolidBrush(color);
    FillRect(dc, r, b);
    DeleteObject(b);
}

unsafe fn round_fill(dc: HDC, r: &RECT, color: u32, rad: i32) {
    let b = CreateSolidBrush(color);
    let old_b = SelectObject(dc, b);
    let old_p = SelectObject(dc, GetStockObject(NULL_PEN));
    RoundRect(dc, r.left, r.top, r.right + 1, r.bottom + 1, rad, rad);
    SelectObject(dc, old_p);
    SelectObject(dc, old_b);
    DeleteObject(b);
}

unsafe fn line(dc: HDC, color: u32, wd: i32, x1: i32, y1: i32, x2: i32, y2: i32) {
    let p = CreatePen(PS_SOLID, wd, color);
    let o = SelectObject(dc, p);
    MoveToEx(dc, x1, y1, std::ptr::null_mut());
    LineTo(dc, x2, y2);
    SelectObject(dc, o);
    DeleteObject(p);
}

unsafe fn paint_buffered(h: HWND, f: impl Fn(HDC, &RECT)) {
    let mut ps: PAINTSTRUCT = zeroed();
    let dc = BeginPaint(h, &mut ps);
    let mut rc: RECT = zeroed();
    GetClientRect(h, &mut rc);
    let mem = CreateCompatibleDC(dc);
    let bmp = CreateCompatibleBitmap(dc, rc.right, rc.bottom);
    let old = SelectObject(mem, bmp);
    SetBkMode(mem, TRANSPARENT as i32);
    SelectObject(mem, app().ui_font);
    f(mem, &rc);
    BitBlt(dc, 0, 0, rc.right, rc.bottom, mem, 0, 0, SRCCOPY);
    SelectObject(mem, old);
    DeleteObject(bmp);
    DeleteDC(mem);
    EndPaint(h, &ps);
}

fn paint_tabs(dc: HDC, rc: &RECT) {
    let a = app();
    let t = &THEMES[a.theme];
    let hover = unsafe { HOVER };
    unsafe {
        fill(dc, rc, t.ui_bg);
        let (rects, plus) = tab_rects(rc.right, a.docs.len());
        let hover_bg = blend(t.ui_bg, t.bg, 45);
        let dim = blend(t.ui_fg, t.ui_bg, 40);
        for (i, r) in rects.iter().enumerate() {
            let sel = i == a.cur;
            let hov = matches!(hover, Hit::Tab(j) | Hit::Close(j) if j == i);
            if sel || hov {
                round_fill(dc, &RECT { bottom: r.bottom + scale(10), ..*r }, if sel { t.bg } else { hover_bg }, scale(10));
                fill(dc, &RECT { top: r.bottom - scale(2), ..*r }, if sel { t.bg } else { t.ui_bg });
            }
            SetTextColor(dc, if sel { t.ui_fg } else { dim });
            let mut tr = RECT { left: r.left + scale(14), right: close_rect(r).left, ..*r };
            text(dc, &a.docs[i].title(), &mut tr, DT_END_ELLIPSIS);
            let c = close_rect(r);
            let (cx, cy) = ((c.left + c.right) / 2, (c.top + c.bottom) / 2);
            let on_close = hover == Hit::Close(i);
            if on_close {
                round_fill(dc, &c, blend(t.ui_bg, t.ui_fg, 14), scale(6));
            }
            if a.docs[i].dirty && !on_close {
                let b = CreateSolidBrush(t.ui_fg);
                let o = SelectObject(dc, b);
                let rr = scale(3);
                Ellipse(dc, cx - rr, cy - rr, cx + rr + 1, cy + rr + 1);
                SelectObject(dc, o);
                DeleteObject(b);
            } else if sel || hov {
                let d = scale(4);
                let col = t.ui_fg;
                line(dc, col, 1, cx - d, cy - d, cx + d + 1, cy + d + 1);
                line(dc, col, 1, cx - d, cy + d, cx + d + 1, cy - d - 1);
            }
        }
        if hover == Hit::Plus {
            round_fill(dc, &plus, blend(t.ui_bg, t.ui_fg, 12), scale(8));
        }
        let (cx, cy) = ((plus.left + plus.right) / 2, (plus.top + plus.bottom) / 2);
        let d = scale(5);
        line(dc, t.ui_fg, 1, cx - d, cy, cx + d + 1, cy);
        line(dc, t.ui_fg, 1, cx, cy - d, cx, cy + d + 1);
    }
}

unsafe fn track_leave(h: HWND) {
    let mut t = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: h, dwHoverTime: 0 };
    TrackMouseEvent(&mut t);
}

pub unsafe extern "system" fn tabs_proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let (x, y) = ((lp & 0xFFFF) as i16 as i32, ((lp >> 16) & 0xFFFF) as i16 as i32);
    match m {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint_buffered(h, paint_tabs);
            0
        }
        WM_MOUSEMOVE => {
            let n = hit_test(h, x, y);
            if n != HOVER {
                HOVER = n;
                InvalidateRect(h, std::ptr::null(), 0);
            }
            track_leave(h);
            0
        }
        0x02A3 /* WM_MOUSELEAVE */ => {
            HOVER = Hit::None;
            InvalidateRect(h, std::ptr::null(), 0);
            0
        }
        WM_LBUTTONDOWN => {
            match hit_test(h, x, y) {
                Hit::Tab(i) => crate::cmds::cmd_select(i),
                Hit::Close(i) => crate::cmds::cmd_close(i),
                Hit::Plus => crate::cmds::cmd_new(),
                Hit::None => {}
            }
            0
        }
        WM_MBUTTONUP => {
            if let Hit::Tab(i) | Hit::Close(i) = hit_test(h, x, y) {
                crate::cmds::cmd_close(i);
            }
            0
        }
        WM_LBUTTONDBLCLK => {
            if hit_test(h, x, y) == Hit::None {
                crate::cmds::cmd_new();
            }
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

fn paint_status(dc: HDC, rc: &RECT) {
    let a = app();
    let t = &THEMES[a.theme];
    unsafe {
        fill(dc, rc, t.ui_bg);
        SetTextColor(dc, blend(t.ui_fg, t.ui_bg, 15));
        let s = a.sci;
        let pos = sci::send(s, sci::GETCURRENTPOS, 0, 0);
        let (ln, col) = (sci::send(s, sci::LINEFROMPOS, pos as usize, 0) + 1, sci::send(s, sci::GETCOLUMN, pos as usize, 0) + 1);
        let len = sci::send(s, sci::GETLENGTH, 0, 0);
        let chars = if len < 8_000_000 { sci::send(s, 2633, 0, len) } else { len };
        let d = &a.docs[a.cur];
        let eol = match sci::send(s, sci::GETEOLMODE, 0, 0) { 0 => "Windows (CRLF)", 1 => "Macintosh (CR)", _ => "Unix (LF)" };
        let zoom = sci::send(s, sci::GETZOOM, 0, 0) as i32;
        let pct = ((a.size + zoom).max(1) * 100) / a.size.max(1);
        let left = format!("Ln {}, Col {}", ln, col);
        let segs = [format!("{} caractères", chars), format!("{}%", pct), eol.to_string(), d.enc.name().to_string(), LANGS[d.lang].name.to_string()];
        let mut r = RECT { left: scale(16), ..*rc };
        text(dc, &left, &mut r, 0);
        let mut x = rc.right - scale(16);
        for sg in segs.iter().rev() {
            let v: Vec<u16> = sg.encode_utf16().collect();
            let mut sz: SIZE = zeroed();
            GetTextExtentPoint32W(dc, v.as_ptr(), v.len() as i32, &mut sz);
            let mut r = RECT { left: x - sz.cx, right: x, ..*rc };
            text(dc, sg, &mut r, DT_RIGHT);
            x -= sz.cx + scale(14);
            line(dc, blend(t.ui_bg, t.ui_fg, 25), 1, x + scale(7), rc.top + scale(8), x + scale(7), rc.bottom - scale(8));
        }
    }
}

pub unsafe extern "system" fn status_proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint_buffered(h, paint_status);
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

pub fn make_ui_font(dpi: u32) -> HFONT {
    let face = w("Segoe UI");
    unsafe { CreateFontW(-(9 * dpi as i32 / 72 + 1), 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr()) }
}
