#![windows_subsystem = "windows"]
//! WinNote : éditeur de texte natif Windows 64 bits (Scintilla + Lexilla), façon Bloc-notes Windows 11 en mieux.
mod cmds;
mod doc;
mod findbar;
mod findx;
mod keymap;
mod macros;
mod prefs;
mod fileops;
mod ids;
mod lang;
mod mdpreview;
mod menu;
mod mdrender;
mod menubar;
mod notes;
mod palette;
mod assist;
mod mdedit;
mod mdlive;
mod mdstyle;
mod print;
mod selftest;
mod sci;
mod session;
mod settings;
mod shell;
mod sidebar;
mod spell;
mod textops;
mod theme;
mod ui;
mod updater;
mod vers;

use doc::Doc;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi}, Input::KeyboardAndMouse::SetFocus, Shell::*, WindowsAndMessaging::*},
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
    pub title: HWND,
    pub side: HWND,
    pub tree: HWND,
    pub find_bar: HWND,
    pub find_edit: HWND,
    pub repl_edit: HWND,
    pub prev: HWND,
    pub sci2: HWND,
    pub palette: HWND,
    pub menus: menu::Menus,
    pub accel: HACCEL,
    pub ui_font: HFONT,
    pub find_brush: HBRUSH,
    pub docs: Vec<Doc>,
    pub cur: usize,
    pub untitled: u32,
    pub theme: usize,
    pub layout: usize,
    pub wrap: bool,
    pub lines: bool,
    pub ws: bool,
    pub fold: bool,
    pub spacing: i32,
    pub status_vis: bool,
    pub font: String,
    pub size: i32,
    pub margin: i32,
    pub dpi: u32,
    pub chrome: u8,
    pub sidebar_vis: bool,
    pub side_mode: u8,
    pub preview: bool,
    pub find_vis: bool,
    pub find_replace: bool,
    pub f_case: bool,
    pub f_word: bool,
    pub f_regex: bool,
    pub find_count: u32,
    pub zen: bool,
    pub zen_style: isize,
    pub zen_rect: RECT,
    pub topmost: bool,
    pub auto_update: bool,
    pub root: Option<String>,
    pub recent: Vec<String>,
    pub closed: Vec<String>,
    pub quiet: bool,
    pub split: bool,
    pub spell: bool,
    pub spell_lang: String,
    pub dwrite: bool,
}

static mut APP: *mut App = null_mut();
pub fn app() -> &'static mut App {
    unsafe { &mut *APP }
}

pub fn relayout() {
    let a = app();
    if a.sci.is_null() {
        return;
    }
    unsafe {
        let mut rc: RECT = std::mem::zeroed();
        GetClientRect(a.hwnd, &mut rc);
        let z = a.zen;
        let th = if a.chrome == 1 && !z { ui::title_height() } else { 0 };
        let tabs_h = if z { 0 } else { ui::tabs_height() };
        let fh = if z { 0 } else { findbar::height() };
        let sh = if z || !a.status_vis { 0 } else { ui::status_height() };
        let (sw, pw) = if z { (0, 0) } else { (sidebar::width(), mdpreview::width()) };
        let y0 = th + tabs_h + fh;
        let h = (rc.bottom - y0 - sh).max(0);
        let show = |c: HWND, on: bool| {
            ShowWindow(c, if on { SW_SHOWNA } else { SW_HIDE });
        };
        MoveWindow(a.title, 0, 0, rc.right, th, 1);
        show(a.title, th > 0);
        MoveWindow(a.tabs, 0, th, rc.right, tabs_h, 1);
        show(a.tabs, tabs_h > 0);
        MoveWindow(a.find_bar, 0, th + tabs_h, rc.right, fh, 1);
        show(a.find_bar, fh > 0);
        MoveWindow(a.side, 0, y0, sw, h, 1);
        show(a.side, sw > 0);
        MoveWindow(a.sci, sw, y0, (rc.right - sw - pw).max(0), h, 1);
        MoveWindow(a.prev, rc.right - pw, y0, pw, h, 1);
        show(a.prev, pw > 0 && a.preview);
        MoveWindow(a.sci2, rc.right - pw, y0, pw, h, 1);
        show(a.sci2, pw > 0 && a.split);
        MoveWindow(a.status, 0, rc.bottom - sh, rc.right, sh, 1);
        show(a.status, sh > 0);
        findbar::layout_edits();
    }
}

fn mk_child(parent: HWND, class: &str, style: u32, id: isize) -> HWND {
    unsafe { CreateWindowExW(0, w(class).as_ptr(), null(), WS_CHILD | WS_CLIPSIBLINGS | style, 0, 0, 0, 0, parent, id as HMENU, GetModuleHandleW(null()), null()) }
}

unsafe fn on_create(a: &mut App, h: HWND) {
    a.hwnd = h;
    a.dpi = GetDpiForWindow(h).max(96);
    a.ui_font = ui::make_ui_font(a.dpi);
    a.title = mk_child(h, "WNTitle", 0, 8);
    a.tabs = mk_child(h, "WNTabs", WS_VISIBLE, 2);
    a.status = mk_child(h, "WNStatus", WS_VISIBLE, 3);
    findbar::create(h);
    sidebar::create(h);
    a.sci = mk_child(h, "Scintilla", WS_CLIPCHILDREN | WS_VISIBLE, 1);
    mdpreview::create(h);
    a.sci2 = mk_child(h, "Scintilla", WS_CLIPCHILDREN, 10);
    sci::send(a.sci2, sci::SETCODEPAGE, 65001, 0);
    sci::send(a.sci2, sci::USEPOPUP, 0, 0);
    a.menus = menu::build();
    menu::rebuild_recent(a);
    a.accel = menu::build_accel();
    DragAcceptFiles(h, 1);
    cmds::new_doc(a);
    if a.chrome == 1 {
        cmds::apply_chrome(a);
    } else {
        SetMenu(h, a.menus.bar);
    }
    cmds::apply_all(a);
    findbar::recolor();
    relayout();
    if a.sidebar_vis {
        if a.side_mode == 1 || a.root.is_none() { sidebar::show_outline() } else { sidebar::show_files() }
    }
}

unsafe fn on_notify(a: &mut App, lp: LPARAM) {
    // NMHDR { hwndFrom, idFrom, code } ; SCNotification : modificationType à l'offset 40
    let hdr = lp as *const u8;
    if *(hdr as *const HWND) != a.sci {
        return;
    }
    let code = *(hdr.add(std::mem::size_of::<usize>() * 2) as *const u32);
    match code {
        sci::SCN_SAVEPOINTLEFT | sci::SCN_SAVEPOINTREACHED if !a.docs.is_empty() => {
            a.docs[a.cur].dirty = code == sci::SCN_SAVEPOINTLEFT;
            cmds::refresh();
        }
        sci::SCN_UPDATEUI => {
            InvalidateRect(a.status, null(), 0);
            spell::schedule();
            assist::brace_highlight(a.sci);
            assist::smart_highlight(a.sci);
        }
        2009 => macros::on_record(*(hdr.add(72) as *const i32) as u32, *(hdr.add(80) as *const usize), *(hdr.add(88) as *const isize)),
        2001 => {
            let lx = lang::LANGS[a.docs[a.cur].lang].lexer;
            assist::on_char(a.sci, *(hdr.add(32) as *const i32) as u8, !lx.is_empty() && lx != "markdown", lx.is_empty());
            // SCN_CHARADDED : indentation automatique (reprend celle de la ligne précédente)
            if *(hdr.add(32) as *const i32) == 10 {
                let ln = sci::send(a.sci, sci::LINEFROMPOS, sci::send(a.sci, sci::GETCURRENTPOS, 0, 0) as usize, 0);
                if ln > 0 {
                    let ind = sci::send(a.sci, 2127, (ln - 1) as usize, 0);
                    sci::send(a.sci, 2126, ln as usize, ind);
                    let p = sci::send(a.sci, 2128, ln as usize, 0);
                    sci::send(a.sci, sci::GOTOPOS, p as usize, 0);
                    if mdlive::is_md() {
                        mdedit::on_enter(a.sci);
                    }
                }
            }
        }
        sci::SCN_MODIFIED => {
            let mt = *(hdr.add(40) as *const i32) as u32;
            if mt & (sci::SC_MOD_INSERTTEXT | sci::SC_MOD_DELETETEXT) != 0 {
                mdpreview::schedule();
                mdlive::schedule();
                spell::schedule();
                if a.sidebar_vis && a.side_mode == 1 {
                    SetTimer(a.hwnd, 2, 900, None);
                }
            }
        }
        _ => {}
    }
}

unsafe fn on_timer(a: &mut App, id: usize) {
    match id {
        1 => {
            KillTimer(a.hwnd, 1);
            mdpreview::update();
        }
        2 => {
            KillTimer(a.hwnd, 2);
            sidebar::refresh();
        }
        6 => mdlive::on_timer(),
        7 => cmds::check_external(),
        3 => session::save(a),
        9 => {
            KillTimer(a.hwnd, 9);
            spell::check_visible();
        }
        4 | 5 => {
            if id == 5 {
                KillTimer(a.hwnd, 5);
                updater::cleanup();
            }
            if a.auto_update {
                updater::check(false);
            }
        }
        _ => {}
    }
}

static PROMPTING: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

unsafe fn on_update_msg(a: &mut App, m: u32, lp: LPARAM) {
    match m {
        updater::WM_UPD_FOUND => {
            let f = *Box::from_raw(lp as *mut updater::Found);
            if PROMPTING.swap(true, std::sync::atomic::Ordering::SeqCst) {
                return; // une proposition est deja affichee (verification auto + manuelle simultanees)
            }
            let q = format!("WinNote {} est disponible (version actuelle {}).\nTélécharger et installer maintenant ? WinNote redémarrera.", f.version, env!("CARGO_PKG_VERSION"));
            let yes = MessageBoxW(a.hwnd, w(&q).as_ptr(), w("Mise à jour").as_ptr(), MB_YESNO | MB_ICONINFORMATION) == IDYES;
            PROMPTING.store(false, std::sync::atomic::Ordering::SeqCst);
            if yes {
                updater::download(f);
            }
        }
        updater::WM_UPD_NONE => {
            let (manual, msg) = *Box::from_raw(lp as *mut (bool, String));
            if manual {
                cmds::msg_box(&msg);
            }
        }
        _ => match *Box::from_raw(lp as *mut Result<String, String>) {
            Ok(path) => {
                session::save(a);
                match updater::apply(&path) {
                    Ok(()) => save_all_and_quit(a),
                    Err(e) => cmds::msg_box(&format!("Mise à jour impossible : {e}")),
                }
            }
            Err(e) => cmds::msg_box(&format!("Mise à jour impossible : {e}")),
        },
    }
}

fn save_all_and_quit(a: &mut App) {
    session::save(a);
    settings::save(a);
    unsafe { DestroyWindow(a.hwnd) };
}

unsafe extern "system" fn wndproc(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if APP.is_null() {
        return DefWindowProcW(h, m, wp, lp);
    }
    let a = app();
    if a.chrome == 0 && !a.zen && !a.sci.is_null() {
        if menubar::handle(h, m, lp) {
            return 0;
        }
        if m == WM_NCPAINT || m == WM_NCACTIVATE {
            let r = DefWindowProcW(h, m, wp, lp);
            menubar::paint_line(h);
            return r;
        }
    }
    match m {
        WM_CREATE => {
            on_create(a, h);
            0
        }
        WM_NCCALCSIZE if a.chrome == 1 && !a.zen && wp != 0 => {
            if IsZoomed(h) != 0 {
                let p = &mut *(lp as *mut NCCALCSIZE_PARAMS);
                let f = GetSystemMetricsForDpi(SM_CXSIZEFRAME, a.dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, a.dpi);
                p.rgrc[0].left += f;
                p.rgrc[0].right -= f;
                p.rgrc[0].top += f;
                p.rgrc[0].bottom -= f;
            }
            0
        }
        WM_NCPAINT if a.chrome == 1 => 0,
        WM_NCACTIVATE if a.chrome == 1 => {
            InvalidateRect(a.title, null(), 0);
            DefWindowProcW(h, m, wp, -1)
        }
        WM_NCHITTEST if a.chrome == 1 && !a.zen => {
            let (x, y) = ((lp & 0xFFFF) as i16 as i32, ((lp >> 16) & 0xFFFF) as i16 as i32);
            let mut r: RECT = std::mem::zeroed();
            GetWindowRect(h, &mut r);
            let b = ui::scale(6);
            if IsZoomed(h) == 0 {
                let (l, rr, t, bt) = (x < r.left + b, x >= r.right - b, y < r.top + b, y >= r.bottom - b);
                let hit = match (l, rr, t, bt) {
                    (true, _, true, _) => HTTOPLEFT,
                    (_, true, true, _) => HTTOPRIGHT,
                    (true, _, _, true) => HTBOTTOMLEFT,
                    (_, true, _, true) => HTBOTTOMRIGHT,
                    (true, ..) => HTLEFT,
                    (_, true, ..) => HTRIGHT,
                    (_, _, true, _) => HTTOP,
                    (.., true) => HTBOTTOM,
                    _ => 0,
                };
                if hit != 0 {
                    return hit as LRESULT;
                }
            }
            if y < r.top + ui::title_height() { HTCAPTION as LRESULT } else { HTCLIENT as LRESULT }
        }
        WM_GETMINMAXINFO => {
            let mi = &mut *(lp as *mut MINMAXINFO);
            mi.ptMinTrackSize = POINT { x: ui::scale(520), y: ui::scale(360) };
            0
        }
        WM_SIZE => {
            relayout();
            0
        }
        WM_SETFOCUS => {
            if a.palette.is_null() {
                SetFocus(a.sci);
            }
            0
        }
        WM_ACTIVATE => {
            InvalidateRect(a.title, null(), 0);
            if (wp & 0xFFFF) != 0 && !a.docs.is_empty() && a.palette.is_null() {
                cmds::check_external();
            }
            DefWindowProcW(h, m, wp, lp)
        }
        WM_DPICHANGED => {
            a.dpi = (wp & 0xFFFF) as u32;
            DeleteObject(a.ui_font);
            a.ui_font = ui::make_ui_font(a.dpi);
            let r = &*(lp as *const RECT);
            SetWindowPos(h, null_mut(), r.left, r.top, r.right - r.left, r.bottom - r.top, SWP_NOZORDER | SWP_NOACTIVATE);
            cmds::apply_all(a);
            relayout();
            0
        }
        WM_NOTIFY => {
            on_notify(a, lp);
            0
        }
        WM_COMMAND => {
            if lp == 0 || (wp >> 16) == 0 {
                cmds::on_command((wp & 0xFFFF) as usize);
            }
            0
        }
        WM_CONTEXTMENU if wp as HWND == a.sci || wp as HWND == a.sci2 => {
            let (mut x, mut y) = ((lp & 0xFFFF) as i16 as i32, ((lp >> 16) & 0xFFFF) as i16 as i32);
            if lp == -1 {
                let mut p = POINT { x: 40, y: 40 };
                ClientToScreen(a.sci, &mut p);
                (x, y) = (p.x, p.y);
            }
            menu::editor_context(x, y);
            0
        }
        WM_TIMER => {
            on_timer(a, wp);
            0
        }
        WM_DROPFILES => {
            let hd = wp as HDROP;
            for i in 0..DragQueryFileW(hd, 0xFFFF_FFFF, null_mut(), 0) {
                let mut buf = vec![0u16; 1024];
                let n = DragQueryFileW(hd, i, buf.as_mut_ptr(), 1024) as usize;
                cmds::open_path(&String::from_utf16_lossy(&buf[..n]));
            }
            DragFinish(hd);
            0
        }
        WM_CLOSE => {
            // sortie « à chaud » : les onglets non enregistrés sont gardés dans la session et restaurés au prochain lancement
            save_all_and_quit(a);
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        updater::WM_UPD_FOUND | updater::WM_UPD_NONE | updater::WM_UPD_READY => {
            on_update_msg(a, m, lp);
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
            hIcon: LoadIconW(GetModuleHandleW(null()), 1 as windows_sys::core::PCWSTR),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: null_mut(),
            lpszMenuName: null(),
            lpszClassName: w(name).leak().as_ptr(),
            hIconSm: null_mut(),
        };
        RegisterClassExW(&c);
    }
}

fn sci_empty(a: &mut App, i: usize) -> bool {
    cmds::text_of_doc(a, i).is_empty()
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--register") {
        std::process::exit(!shell::register() as i32);
    }
    if args.iter().any(|a| a == "--unregister") {
        std::process::exit(!shell::unregister() as i32);
    }
    std::panic::set_hook(Box::new(|i| {
        let p = std::path::PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join("WinNote").join("crash.log");
        let _ = std::fs::write(p, format!("{i}"));
    }));
    unsafe extern "system" fn crash_filter(info: *const windows_sys::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS) -> i32 {
        let r = &*(*info).ExceptionRecord;
        let base = GetModuleHandleW(null()) as usize;
        let p = std::path::PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join("WinNote").join("crash.log");
        let _ = std::fs::write(p, format!("exception {:#x} à rva {:#x}", r.ExceptionCode, r.ExceptionAddress as usize - base));
        0
    }
    unsafe { windows_sys::Win32::System::Diagnostics::Debug::SetUnhandledExceptionFilter(Some(crash_filter)) };
    if let Some(i) = args.iter().position(|a| a == "--apply-update") {
        // mode sans fenêtre : remplace l'exe par le fichier indiqué puis relance (utilisé par la mise à jour et ses tests)
        std::process::exit(args.get(i + 1).map_or(2, |f| updater::apply(f).is_err() as i32));
    }
    if let Some(i) = args.iter().position(|a| a == "--update-check") {
        // mode sans fenetre (diagnostic) : ecrit le resultat de la verification dans le fichier indique
        let r = match updater::fetch() {
            Ok(Some(f)) => format!("nouvelle version {} url={} sha={}", f.version, f.url, f.sha),
            Ok(None) => "a jour".to_string(),
            Err(e) => format!("erreur : {e}"),
        };
        let _ = std::fs::write(args.get(i + 1).map_or("update-check.txt", |s| s.as_str()), r);
        std::process::exit(0);
    }
    updater::cleanup();
    unsafe {
        windows_sys::Win32::System::Ole::OleInitialize(null());
        sci::Scintilla_RegisterClasses(GetModuleHandleW(null()));
        register("WinNoteMain", Some(wndproc), CS_HREDRAW | CS_VREDRAW);
        register("WNTabs", Some(ui::tabs_proc), CS_DBLCLKS);
        register("WNStatus", Some(ui::status_proc), 0);
        register("WNTitle", Some(ui::title_proc), 0);
        register("WNFind", Some(findbar::proc_), 0);
        register("WNSide", Some(sidebar::proc_), 0);
        register("WNPalette", Some(palette::proc_), 0);

        let mut a = Box::new(settings::defaults());
        settings::load(&mut a);
        let cfg = std::path::PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join("WinNote");
        prefs::init(cfg.clone());
        keymap::init(cfg);
        session::load_recent(&mut a);
        APP = Box::into_raw(a);

        let dpi = windows_sys::Win32::UI::HiDpi::GetDpiForSystem().max(96) as i32;
        let hwnd = CreateWindowExW(
            0, w("WinNoteMain").as_ptr(), w("WinNote").as_ptr(), WS_OVERLAPPEDWINDOW | WS_CLIPCHILDREN,
            CW_USEDEFAULT, CW_USEDEFAULT, 1100 * dpi / 96, 720 * dpi / 96, null_mut(), null_mut(), GetModuleHandleW(null()), null(),
        );
        if hwnd.is_null() {
            return;
        }
        let test = args.iter().any(|x| x == "--selftest" || x == "--print-to");
        if !test {
            ShowWindow(hwnd, SW_SHOW);
        }
        let a = app();
        if let Some(i) = args.iter().position(|x| x == "--selftest") {
            std::process::exit(selftest::run(args.get(i + 1).map_or("selftest.log", |s| s.as_str())));
        }
        if let Some(i) = args.iter().position(|x| x == "--print-to") {
            // --print-to <imprimante> <sortie.pdf> <fichier> : impression sans boîte de dialogue (tests)
            a.quiet = true;
            cmds::open_path(&args[i + 3]);
            let r = print::print_to(&args[i + 1], &args[i + 2]);
            let _ = std::fs::write(format!("{}.txt", args[i + 2]), format!("{r:?}"));
            std::process::exit(r.is_err() as i32);
        }
        if session::restore(a) && a.docs.len() > 1 && a.docs[0].path.is_none() && !a.docs[0].dirty && sci_empty(a, 0) {
            cmds::cmd_close(0);
        }
        for p in args.iter().filter(|x| !x.starts_with("--")) {
            cmds::open_path(p);
        }
        SetTimer(hwnd, 3, 30_000, None);
        SetTimer(hwnd, 7, 1_500, None);
        SetTimer(hwnd, 4, 6 * 3600 * 1000, None);
        SetTimer(hwnd, 5, 8_000, None);
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            let a = app();
            if findbar::pretranslate(&msg) {
                continue;
            }
            if a.palette.is_null() && TranslateAcceleratorW(a.hwnd, a.accel, &msg) != 0 {
                continue;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }
}
