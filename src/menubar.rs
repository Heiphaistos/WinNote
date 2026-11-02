//! Barre de menus colorée selon le thème (messages UAH non documentés de Windows, utilisés par l'explorateur).
use crate::{app, theme::THEMES};
use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, UI::{Controls::{DRAWITEMSTRUCT, ODS_HOTLIGHT, ODS_SELECTED}, WindowsAndMessaging::*}};

pub const WM_UAHDRAWMENU: u32 = 0x91;
pub const WM_UAHDRAWMENUITEM: u32 = 0x92;

#[repr(C)]
struct UahMenu {
    hmenu: HMENU,
    hdc: HDC,
    flags: u32,
}
#[repr(C)]
struct UahItem {
    dis: DRAWITEMSTRUCT,
    um: UahMenu,
    pos: i32,
}

fn bar_rect(h: HWND) -> RECT {
    unsafe {
        let mut mbi: MENUBARINFO = std::mem::zeroed();
        mbi.cbSize = std::mem::size_of::<MENUBARINFO>() as u32;
        GetMenuBarInfo(h, OBJID_MENU, 0, &mut mbi);
        let mut wr: RECT = std::mem::zeroed();
        GetWindowRect(h, &mut wr);
        let mut r = mbi.rcBar;
        OffsetRect(&mut r, -wr.left, -wr.top);
        r
    }
}

fn mix(a: u32, b: u32, t: u32) -> u32 {
    let f = |s: u32| ((a >> s & 0xFF) * (100 - t) + (b >> s & 0xFF) * t) / 100;
    f(0) | f(8) << 8 | f(16) << 16
}

unsafe fn fill(dc: HDC, r: &RECT, c: u32) {
    let b = CreateSolidBrush(c);
    FillRect(dc, r, b);
    DeleteObject(b);
}

/// Traite les messages de dessin de la barre de menus. Renvoie true si géré.
pub fn handle(h: HWND, m: u32, lp: LPARAM) -> bool {
    let t = &THEMES[app().theme];
    unsafe {
        match m {
            WM_UAHDRAWMENU => {
                let um = &*(lp as *const UahMenu);
                fill(um.hdc, &bar_rect(h), t.ui_bg);
                true
            }
            WM_UAHDRAWMENUITEM => {
                let it = &*(lp as *const UahItem);
                let st = it.dis.itemState;
                let bg = if st & (ODS_HOTLIGHT | ODS_SELECTED) != 0 { mix(t.ui_bg, t.ui_fg, 14) } else { t.ui_bg };
                fill(it.um.hdc, &it.dis.rcItem, bg);
                let mut buf = [0u16; 64];
                let mut mii: MENUITEMINFOW = std::mem::zeroed();
                mii.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
                mii.fMask = MIIM_STRING;
                mii.dwTypeData = buf.as_mut_ptr();
                mii.cch = 63;
                GetMenuItemInfoW(it.um.hmenu, it.pos as u32, 1, &mut mii);
                SetBkMode(it.um.hdc, TRANSPARENT as i32);
                SetTextColor(it.um.hdc, t.ui_fg);
                let mut r = it.dis.rcItem;
                DrawTextW(it.um.hdc, buf.as_ptr(), mii.cch as i32, &mut r, DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_HIDEPREFIX);
                true
            }
            _ => false,
        }
    }
}

/// Efface le filet clair sous la barre de menus (après le dessin du cadre).
pub fn paint_line(h: HWND) {
    unsafe {
        let mut r = bar_rect(h);
        r.top = r.bottom;
        r.bottom += 1;
        let dc = GetWindowDC(h);
        fill(dc, &r, THEMES[app().theme].ui_bg);
        ReleaseDC(h, dc);
    }
}
