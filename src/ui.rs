//! Barre d'onglets (style Bloc-notes Windows 11) et barre d'état, dessinées en GDI.
use crate::theme::THEMES;
use crate::{app, lang::LANGS, sci, w};
use std::mem::zeroed;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

/// DPI effectif d'une fenêtre ; le build `test-hooks` (jamais publié) accepte `WINNOTE_TEST_DPI` (96..384) pour simuler 100..400 % sans toucher à l'écran.
pub fn dpi_of(h: HWND) -> u32 {
    #[cfg(feature = "test-hooks")]
    if let Some(d) = std::env::var("WINNOTE_TEST_DPI").ok().and_then(|v| v.parse::<u32>().ok()).filter(|d| (96..=384).contains(d)) {
        return d;
    }
    unsafe { windows_sys::Win32::UI::HiDpi::GetDpiForWindow(h).max(96) }
}
pub fn scale(v: i32) -> i32 {
    v * app().dpi as i32 / 96
}
pub fn tabs_height() -> i32 {
    scale(40)
}
pub fn status_height() -> i32 {
    scale(28)
}

pub use crate::theme::blend;

#[derive(Clone, Copy, PartialEq)]
enum Hit { None, Tab(usize), Close(usize), Plus }

static mut HOVER: Hit = Hit::None;
static mut DRAG: Option<usize> = None;

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
    if v.is_empty() {
        return;
    }
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
    let p = CreatePen(PS_SOLID, scale(wd).max(1), color); // épaisseur proportionnelle au DPI (croix, plus)
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
            text(dc, &format!("{}{}", if a.docs[i].pinned { "◆ " } else { "" }, a.docs[i].title()), &mut tr, DT_END_ELLIPSIS);
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
        WM_LBUTTONUP => {
            DRAG = None;
            ReleaseCapture();
            0
        }
        WM_MOUSEMOVE => {
            if let (Some(d), true) = (DRAG, wp & 1 != 0) {
                if let Hit::Tab(j) | Hit::Close(j) = hit_test(h, x, y) {
                    if j != d {
                        crate::cmds::move_tab(d, j);
                        DRAG = Some(j);
                    }
                }
                return 0;
            }
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
                Hit::Tab(i) => {
                    crate::cmds::cmd_select(i);
                    DRAG = Some(i);
                    SetCapture(h);
                }
                Hit::Close(i) => crate::cmds::cmd_close(i),
                Hit::Plus => crate::cmds::cmd_new(),
                Hit::None => {}
            }
            0
        }
        WM_RBUTTONUP => {
            if let Hit::Tab(i) | Hit::Close(i) = hit_test(h, x, y) {
                crate::cmds::cmd_select(i);
                let mut p = POINT { x, y };
                ClientToScreen(h, &mut p);
                crate::menu::tab_context(p.x, p.y);
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
        WM_LBUTTONUP => {
            status_click(h, (lp & 0xFFFF) as i16 as i32);
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

pub fn make_ui_font(dpi: u32) -> HFONT {
    let face = w("Segoe UI");
    unsafe { CreateFontW(-(9 * dpi as i32 / 72 + 1), 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr()) }
}

// ---------- Barre de titre style macOS (feux tricolores à gauche) ----------

pub fn title_height() -> i32 {
    if app().chrome == 1 { scale(40) } else { 0 }
}

fn light_rects() -> [RECT; 3] {
    let d = scale(13);
    let cy = title_height() / 2;
    let mk = |i: i32| RECT { left: scale(16) + i * scale(22), top: cy - d / 2, right: scale(16) + i * scale(22) + d, bottom: cy - d / 2 + d };
    [mk(0), mk(1), mk(2)]
}

fn menu_btn_rect(width: i32) -> RECT {
    let h = title_height();
    RECT { left: width - scale(46), top: scale(7), right: width - scale(12), bottom: h - scale(7) }
}

static mut TITLE_HOVER: i32 = -1; // 0..2 = feux, 3 = menu

fn title_hit(h: HWND, x: i32, y: i32) -> i32 {
    let mut rc: RECT = unsafe { zeroed() };
    unsafe { GetClientRect(h, &mut rc) };
    for (i, r) in light_rects().iter().enumerate() {
        let pad = scale(5);
        if inside(&RECT { left: r.left - pad, top: r.top - pad, right: r.right + pad, bottom: r.bottom + pad }, x, y) {
            return i as i32;
        }
    }
    if inside(&menu_btn_rect(rc.right), x, y) { 3 } else { -1 }
}

fn paint_title(dc: HDC, rc: &RECT) {
    let a = app();
    let t = &THEMES[a.theme];
    let hover = unsafe { TITLE_HOVER };
    let active = unsafe { GetForegroundWindow() } == a.hwnd;
    unsafe {
        fill(dc, rc, t.ui_bg);
        let cols = [0x5F57FFu32, 0x2EBCFE, 0x40C828]; // BGR de #FF5F57, #FEBC2E, #28C840
        let grey = blend(t.ui_bg, t.ui_fg, 22);
        for (i, r) in light_rects().iter().enumerate() {
            let col = if active || (0..3).contains(&hover) { cols[i] } else { grey };
            let b = CreateSolidBrush(col);
            let p = CreatePen(PS_SOLID, 1, blend(col, 0, 18));
            let (ob, op) = (SelectObject(dc, b), SelectObject(dc, p));
            Ellipse(dc, r.left, r.top, r.right + 1, r.bottom + 1);
            SelectObject(dc, ob);
            SelectObject(dc, op);
            DeleteObject(b);
            DeleteObject(p);
            if (0..3).contains(&hover) {
                let (cx, cy, d) = ((r.left + r.right) / 2, (r.top + r.bottom) / 2, scale(3));
                let dark = 0x303030u32;
                match i {
                    0 => {
                        line(dc, dark, 1, cx - d, cy - d, cx + d + 1, cy + d + 1);
                        line(dc, dark, 1, cx - d, cy + d, cx + d + 1, cy - d - 1);
                    }
                    1 => line(dc, dark, 1, cx - d, cy, cx + d + 1, cy),
                    _ => {
                        line(dc, dark, 1, cx - d, cy, cx + d + 1, cy);
                        line(dc, dark, 1, cx, cy - d, cx, cy + d + 1);
                    }
                }
            }
        }
        let d = &a.docs[a.cur];
        let name = format!("{}{}", if d.dirty { "• " } else { "" }, d.title());
        SetTextColor(dc, blend(t.ui_fg, t.ui_bg, 20));
        let mut r = RECT { left: scale(110), right: rc.right - scale(60), ..*rc };
        text(dc, &name, &mut r, DT_CENTER | DT_END_ELLIPSIS);
        let m = menu_btn_rect(rc.right);
        if hover == 3 {
            round_fill(dc, &m, blend(t.ui_bg, t.ui_fg, 12), scale(8));
        }
        SetTextColor(dc, t.ui_fg);
        let mut mr = m;
        text(dc, "☰", &mut mr, DT_CENTER);
        line(dc, blend(t.ui_bg, t.ui_fg, 12), 1, 0, rc.bottom - 1, rc.right, rc.bottom - 1);
    }
}

pub unsafe extern "system" fn title_proc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    let (x, y) = ((lp & 0xFFFF) as i16 as i32, ((lp >> 16) & 0xFFFF) as i16 as i32);
    match m {
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint_buffered(h, paint_title);
            0
        }
        WM_NCHITTEST => {
            // hors boutons : la fenêtre principale gère (déplacement par glisser)
            let mut p = POINT { x: (lp & 0xFFFF) as i16 as i32, y: ((lp >> 16) & 0xFFFF) as i16 as i32 };
            ScreenToClient(h, &mut p);
            if title_hit(h, p.x, p.y) >= 0 { HTCLIENT as LRESULT } else { HTTRANSPARENT as LRESULT }
        }
        WM_MOUSEMOVE => {
            let n = title_hit(h, x, y);
            if n != TITLE_HOVER {
                TITLE_HOVER = n;
                InvalidateRect(h, std::ptr::null(), 0);
            }
            track_leave(h);
            0
        }
        0x02A3 => {
            TITLE_HOVER = -1;
            InvalidateRect(h, std::ptr::null(), 0);
            0
        }
        WM_LBUTTONUP => {
            let main = app().hwnd;
            match title_hit(h, x, y) {
                0 => drop(PostMessageW(main, WM_CLOSE, 0, 0)),
                1 => drop(ShowWindow(main, SW_MINIMIZE)),
                2 => drop(ShowWindow(main, if IsZoomed(main) != 0 { SW_RESTORE } else { SW_MAXIMIZE })),
                3 => {
                    let mut rc: RECT = zeroed();
                    GetClientRect(h, &mut rc);
                    let mut p = POINT { x: rc.right - scale(200), y: rc.bottom };
                    ClientToScreen(h, &mut p);
                    crate::menu::popup_at(app().menus.popup, p.x, p.y);
                }
                _ => {}
            }
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

/// Rectangles des segments de droite (caractères, zoom, eol, encodage, langage).
fn status_segments(h: HWND) -> Vec<RECT> {
    let a = app();
    let mut rc: RECT = unsafe { zeroed() };
    unsafe { GetClientRect(h, &mut rc) };
    let d = &a.docs[a.cur];
    let eol = match crate::sci::send(a.sci, crate::sci::GETEOLMODE, 0, 0) { 0 => "Windows (CRLF)", 1 => "Macintosh (CR)", _ => "Unix (LF)" };
    let labels = ["999 999 caractères".to_string(), "100%".to_string(), eol.to_string(), d.enc.name().to_string(), LANGS[d.lang].name.to_string()];
    let dc = unsafe { GetDC(h) };
    let old = unsafe { SelectObject(dc, a.ui_font) };
    let mut x = rc.right - scale(16);
    let mut out = vec![RECT { left: 0, top: 0, right: 0, bottom: 0 }; 5];
    for (i, l) in labels.iter().enumerate().rev() {
        let v: Vec<u16> = l.encode_utf16().collect();
        let mut sz: SIZE = unsafe { zeroed() };
        unsafe { GetTextExtentPoint32W(dc, v.as_ptr(), v.len() as i32, &mut sz) };
        let wdt = if i == 0 { 0 } else { sz.cx };
        out[i] = RECT { left: x - wdt - scale(6), top: 0, right: x + scale(6), bottom: rc.bottom };
        x -= wdt + scale(14);
    }
    unsafe {
        SelectObject(dc, old);
        ReleaseDC(h, dc);
    }
    out
}

/// Clic sur la barre d'état : ouvre le menu du segment (fins de ligne, encodage, langage).
pub fn status_click(h: HWND, x: i32) {
    let a = app();
    for (i, r) in status_segments(h).iter().enumerate() {
        if x >= r.left && x < r.right {
            let menu = match i {
                2 => a.menus.eol,
                3 => a.menus.enc,
                4 => a.menus.lang,
                _ => return,
            };
            let mut p = POINT { x: r.left, y: 0 };
            unsafe {
                ClientToScreen(h, &mut p);
                let mut wr: RECT = zeroed();
                GetWindowRect(h, &mut wr);
                crate::menu::popup_at(menu, p.x, wr.top - scale(20));
            }
            return;
        }
    }
}
