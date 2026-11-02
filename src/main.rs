#![windows_subsystem = "windows"]
//! WinNote : éditeur de texte natif Windows 64 bits (Scintilla + Lexilla), façon Bloc-notes Windows 11 en mieux.
mod cmds;
mod doc;
mod lang;
mod menubar;
mod sci;
mod theme;
mod ui;

use cmds::*;
use doc::Doc;
use std::ptr::{null, null_mut};
use theme::{LAYOUTS, THEMES};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Controls::Dialogs::*, HiDpi::GetDpiForWindow, Input::KeyboardAndMouse::SetFocus, Shell::*, WindowsAndMessaging::*},
};

pub fn w(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn cstr(s: &str) -> Vec<u8> {
    s.bytes().chain(Some(0)).collect()
}

pub struct App {
    pub hwnd: HWND,
    pub sci: HWND,
    pub tabs: HWND,
    pub status: HWND,
    pub menu: HMENU,
    pub accel: HACCEL,
    pub ui_font: HFONT,
    pub docs: Vec<Doc>,
    pub cur: usize,
    pub untitled: u32,
    pub theme: usize,
    pub layout: usize,
    pub wrap: bool,
    pub lines: bool,
    pub ws: bool,
    pub status_vis: bool,
    pub font: String,
    pub size: i32,
    pub margin: i32,
    pub dpi: u32,
    pub find_dlg: HWND,
    pub fr: Box<FINDREPLACEW>,
    pub find_buf: Vec<u16>,
    pub repl_buf: Vec<u16>,
    pub find_msg: u32,
}

static mut APP: *mut App = null_mut();
pub fn app() -> &'static mut App {
    unsafe { &mut *APP }
}

fn settings_path() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join("WinNote").join("settings.ini")
}

fn load_settings(a: &mut App) {
    let Ok(s) = std::fs::read_to_string(settings_path()) else { return };
    for l in s.lines() {
        let Some((k, v)) = l.split_once('=') else { continue };
        match k {
            "theme" => a.theme = v.parse().ok().filter(|&x| x < THEMES.len()).unwrap_or(a.theme),
            "layout" => a.layout = v.parse().ok().filter(|&x| x < LAYOUTS.len()).unwrap_or(a.layout),
            "wrap" => a.wrap = v == "1",
            "lines" => a.lines = v == "1",
            "ws" => a.ws = v == "1",
            "status" => a.status_vis = v == "1",
            "font" => a.font = v.to_string(),
            "size" => a.size = v.parse().unwrap_or(a.size).clamp(6, 72),
            "margin" => a.margin = v.parse().unwrap_or(a.margin).clamp(0, 200),
            _ => {}
        }
    }
}

pub fn save_settings(a: &App) {
    let p = settings_path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let s = format!(
        "theme={}\nlayout={}\nwrap={}\nlines={}\nws={}\nstatus={}\nfont={}\nsize={}\nmargin={}\n",
        a.theme, a.layout, a.wrap as u8, a.lines as u8, a.ws as u8, a.status_vis as u8, a.font, a.size, a.margin
    );
    if let Err(e) = std::fs::write(&p, s) {
        msg_box(&format!("Réglages non enregistrés : {e}"));
    }
}

pub fn relayout() {
    let a = app();
    unsafe {
        let mut rc: RECT = std::mem::zeroed();
        GetClientRect(a.hwnd, &mut rc);
        let (th, sh) = (ui::tabs_height(), if a.status_vis { ui::status_height() } else { 0 });
        MoveWindow(a.tabs, 0, 0, rc.right, th, 1);
        MoveWindow(a.sci, 0, th, rc.right, (rc.bottom - th - sh).max(0), 1);
        ShowWindow(a.status, if a.status_vis { SW_SHOW } else { SW_HIDE });
        MoveWindow(a.status, 0, rc.bottom - sh, rc.right, sh, 1);
    }
}

unsafe extern "system" fn wndproc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if APP.is_null() {
        return DefWindowProcW(h, m, wp, lp);
    }
    let a = app();
    if menubar::handle(h, m, lp) {
        return 0;
    }
    if m == WM_NCPAINT || m == WM_NCACTIVATE {
        let r = DefWindowProcW(h, m, wp, lp);
        menubar::paint_line(h);
        return r;
    }
    match m {
        WM_CREATE => {
            a.hwnd = h;
            a.dpi = GetDpiForWindow(h).max(96);
            a.ui_font = ui::make_ui_font(a.dpi);
            let inst = GetModuleHandleW(null());
            let mk = |class: &str, style: u32, id: isize| {
                CreateWindowExW(0, w(class).as_ptr(), null(), WS_CHILD | WS_VISIBLE | WS_CLIPSIBLINGS | style, 0, 0, 0, 0, h, id as HMENU, inst, null())
            };
            a.tabs = mk("WNTabs", 0, 2);
            a.status = mk("WNStatus", 0, 3);
            a.sci = mk("Scintilla", WS_CLIPCHILDREN, 1);
            a.menu = build_menu();
            SetMenu(h, a.menu);
            a.accel = build_accel();
            DragAcceptFiles(h, 1);
            new_doc(a);
            apply_all(a);
            relayout();
            0
        }
        WM_SIZE => {
            if !a.sci.is_null() {
                relayout();
            }
            0
        }
        WM_SETFOCUS => {
            SetFocus(a.sci);
            0
        }
        WM_DPICHANGED => {
            a.dpi = (wp & 0xFFFF) as u32;
            DeleteObject(a.ui_font);
            a.ui_font = ui::make_ui_font(a.dpi);
            let r = &*(lp as *const RECT);
            SetWindowPos(h, null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
            apply_all(a);
            relayout();
            0
        }
        WM_NOTIFY => {
            // NMHDR { hwndFrom, idFrom, code }
            let code = *((lp as *const u8).add(std::mem::size_of::<usize>() * 2) as *const u32);
            match code {
                sci::SCN_SAVEPOINTLEFT | sci::SCN_SAVEPOINTREACHED if !a.docs.is_empty() => {
                    a.docs[a.cur].dirty = code == sci::SCN_SAVEPOINTLEFT;
                    refresh();
                }
                sci::SCN_UPDATEUI => {
                    InvalidateRect(a.status, null(), 0);
                }
                _ => {}
            }
            0
        }
        WM_COMMAND => {
            on_command((wp & 0xFFFF) as usize);
            0
        }
        WM_DROPFILES => {
            let hd = wp as HDROP;
            for i in 0..DragQueryFileW(hd, 0xFFFF_FFFF, null_mut(), 0) {
                let mut buf = vec![0u16; 1024];
                let n = DragQueryFileW(hd, i, buf.as_mut_ptr(), 1024) as usize;
                open_path(&String::from_utf16_lossy(&buf[..n]));
            }
            DragFinish(hd);
            0
        }
        WM_CLOSE => {
            if confirm_close_all() {
                save_settings(a);
                DestroyWindow(h);
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ if m == a.find_msg => {
            on_find_msg(lp as *mut FINDREPLACEW);
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

fn register(name: &str, proc_: WNDPROC, style: u32) {
    unsafe {
        let c = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style,
            lpfnWndProc: proc_,
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: GetModuleHandleW(null()),
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: w(name).leak().as_ptr(),
            hIconSm: null_mut(),
        };
        RegisterClassExW(&c);
    }
}

fn main() {
    unsafe {
        sci::Scintilla_RegisterClasses(GetModuleHandleW(null()));
        register("WinNoteMain", Some(wndproc), CS_HREDRAW | CS_VREDRAW);
        register("WNTabs", Some(ui::tabs_proc), CS_DBLCLKS);
        register("WNStatus", Some(ui::status_proc), 0);

        let l = &LAYOUTS[0];
        let mut a = Box::new(App {
            hwnd: null_mut(), sci: null_mut(), tabs: null_mut(), status: null_mut(), menu: null_mut(), accel: null_mut(), ui_font: null_mut(),
            docs: Vec::new(), cur: 0, untitled: 0, theme: 0, layout: 0, wrap: l.wrap, lines: l.lines, ws: false, status_vis: true,
            font: l.font.into(), size: l.size, margin: l.margin, dpi: 96, find_dlg: null_mut(), fr: Box::new(std::mem::zeroed()),
            find_buf: vec![0; 512], repl_buf: vec![0; 512], find_msg: RegisterWindowMessageW(w("commdlg_FindReplace").as_ptr()),
        });
        load_settings(&mut a);
        APP = Box::into_raw(a);

        let dpi = windows_sys::Win32::UI::HiDpi::GetDpiForSystem().max(96) as i32;
        let hwnd = CreateWindowExW(
            0, w("WinNoteMain").as_ptr(), w("WinNote").as_ptr(), WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT, CW_USEDEFAULT, 1100 * dpi / 96, 720 * dpi / 96, null_mut(), null_mut(), GetModuleHandleW(null()), null(),
        );
        if hwnd.is_null() {
            return;
        }
        ShowWindow(hwnd, SW_SHOW);
        for p in std::env::args().skip(1) {
            open_path(&p);
        }
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            let a = app();
            if !a.find_dlg.is_null() && IsDialogMessageW(a.find_dlg, &msg) != 0 {
                continue;
            }
            if TranslateAcceleratorW(a.hwnd, a.accel, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
