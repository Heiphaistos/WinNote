//! Boîte « Mise en page de l'impression » (Windows) : fenêtre modale avec marge et zones gauche / centre / droite de l'en-tête et du pied.
use crate::extras::PrintLayout;
use crate::ui::scale;
use crate::{app, w};
use std::cell::{Cell, RefCell};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::COLOR_BTNFACE,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::EnableWindow, WindowsAndMessaging::*},
};

const CLASS: &str = "WNPageSetup";
const ID_OK: isize = 1;
const ID_CANCEL: isize = 2;
const ID_MARGIN: isize = 100;
const ID_HEAD: isize = 101; // 101..=103
const ID_FOOT: isize = 104; // 104..=106
const ID_DEFAULT: isize = 110;

thread_local! {
    static START: RefCell<PrintLayout> = RefCell::new(PrintLayout::default());
    static RESULT: RefCell<Option<PrintLayout>> = const { RefCell::new(None) };
    static DONE: Cell<bool> = const { Cell::new(false) };
}

fn text_of(h: HWND, id: isize) -> String {
    let mut b = vec![0u16; 512];
    let n = unsafe { GetWindowTextW(GetDlgItem(h, id as i32), b.as_mut_ptr(), 512) } as usize;
    String::from_utf16_lossy(&b[..n])
}

fn set_text(h: HWND, id: isize, s: &str) {
    unsafe { SetWindowTextW(GetDlgItem(h, id as i32), w(s).as_ptr()) };
}

fn zones(tpl: &str) -> [String; 3] {
    let mut it = tpl.splitn(3, '|');
    [it.next().unwrap_or("").into(), it.next().unwrap_or("").into(), it.next().unwrap_or("").into()]
}

fn fill(h: HWND, l: &PrintLayout) {
    set_text(h, ID_MARGIN, &l.margin_mm.to_string());
    for (i, z) in zones(&l.header).iter().enumerate() {
        set_text(h, ID_HEAD + i as isize, z);
    }
    for (i, z) in zones(&l.footer).iter().enumerate() {
        set_text(h, ID_FOOT + i as isize, z);
    }
}

/// Mise en page lue dans les champs ; erreur lisible si la marge n'est pas un nombre de 5 à 60 mm.
fn read(h: HWND) -> Result<PrintLayout, String> {
    let m = text_of(h, ID_MARGIN);
    let margin_mm = m.trim().parse::<u32>().ok().filter(|v| (5..=60).contains(v)).ok_or_else(|| format!("Marge : « {} » n'est pas valide. Saisissez un nombre entier de 5 à 60 (millimètres).", m.trim()))?;
    let join = |first: isize| (0..3).map(|i| text_of(h, first + i).replace(['|', '\r', '\n'], "")).collect::<Vec<_>>().join("|");
    Ok(PrintLayout { margin_mm, header: join(ID_HEAD), footer: join(ID_FOOT) })
}

unsafe fn child(parent: HWND, class: &str, text: &str, style: u32, ex: u32, (x, y, cw, ch): (i32, i32, i32, i32), id: isize) {
    let c = CreateWindowExW(ex, w(class).as_ptr(), w(text).as_ptr(), WS_CHILD | WS_VISIBLE | style, scale(x), scale(y), scale(cw), scale(ch), parent, id as HMENU, GetModuleHandleW(null()), null());
    SendMessageW(c, WM_SETFONT, app().ui_font as usize, 1);
}

unsafe extern "system" fn proc_(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_CREATE => {
            let (lbl, edit, tab) = (0u32, ES_AUTOHSCROLL as u32, WS_TABSTOP);
            child(h, "STATIC", "Marge (mm)", lbl, 0, (16, 18, 110, 18), -1);
            child(h, "EDIT", "", edit | ES_NUMBER as u32 | tab, WS_EX_CLIENTEDGE, (130, 14, 60, 24), ID_MARGIN);
            for (i, n) in ["Gauche", "Centre", "Droite"].iter().enumerate() {
                child(h, "STATIC", n, lbl, 0, (130 + 115 * i as i32, 48, 100, 16), -1);
            }
            for (row, (name, first)) in [("En-tête", ID_HEAD), ("Pied de page", ID_FOOT)].iter().enumerate() {
                let y = 68 + 34 * row as i32;
                child(h, "STATIC", name, lbl, 0, (16, y + 4, 110, 18), -1);
                for i in 0..3 {
                    child(h, "EDIT", "", edit | tab, WS_EX_CLIENTEDGE, (130 + 115 * i, y, 108, 24), first + i as isize);
                }
            }
            child(h, "STATIC", "Variables : {fichier} {date} {page} {pages}. Une zone peut rester vide.", lbl, 0, (16, 140, 470, 34), -1);
            child(h, "BUTTON", "Valeurs par défaut", BS_PUSHBUTTON as u32 | tab, 0, (16, 182, 130, 28), ID_DEFAULT);
            child(h, "BUTTON", "Enregistrer", BS_DEFPUSHBUTTON as u32 | tab, 0, (286, 182, 96, 28), ID_OK);
            child(h, "BUTTON", "Annuler", BS_PUSHBUTTON as u32 | tab, 0, (390, 182, 96, 28), ID_CANCEL);
            START.with(|s| fill(h, &s.borrow()));
            0
        }
        WM_COMMAND => {
            match (wp & 0xffff) as isize {
                ID_OK => match read(h) {
                    Ok(l) => {
                        RESULT.with(|r| *r.borrow_mut() = Some(l));
                        DestroyWindow(h);
                    }
                    Err(e) => crate::cmds::msg_box(&e),
                },
                ID_CANCEL => {
                    DestroyWindow(h);
                }
                ID_DEFAULT => fill(h, &PrintLayout::default()),
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(h);
            0
        }
        WM_DESTROY => {
            DONE.with(|d| d.set(true));
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

/// Crée la fenêtre (non modale tant que `run` n'est pas appelé).
pub fn build(owner: HWND, start: &PrintLayout) -> HWND {
    START.with(|s| *s.borrow_mut() = start.clone());
    RESULT.with(|r| *r.borrow_mut() = None);
    DONE.with(|d| d.set(false));
    unsafe {
        let inst = GetModuleHandleW(null());
        let c = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: 0,
            lpfnWndProc: Some(proc_),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: inst,
            hIcon: null_mut(),
            hCursor: LoadCursorW(null_mut(), IDC_ARROW),
            hbrBackground: (COLOR_BTNFACE + 1) as usize as _,
            lpszMenuName: null(),
            lpszClassName: w(CLASS).leak().as_ptr(),
            hIconSm: null_mut(),
        };
        RegisterClassExW(&c); // déjà enregistrée : sans effet
        let style = WS_POPUP | WS_CAPTION | WS_SYSMENU;
        let mut rc = RECT { left: 0, top: 0, right: scale(502), bottom: scale(224) };
        AdjustWindowRectEx(&mut rc, style, 0, WS_EX_DLGMODALFRAME);
        let (ww, wh) = (rc.right - rc.left, rc.bottom - rc.top);
        let mut or = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let anchor = if owner.is_null() { GetDesktopWindow() } else { owner };
        GetWindowRect(anchor, &mut or);
        let (x, y) = (or.left + ((or.right - or.left) - ww) / 2, or.top + ((or.bottom - or.top) - wh) / 2);
        CreateWindowExW(WS_EX_DLGMODALFRAME, w(CLASS).as_ptr(), w("Mise en page de l'impression").as_ptr(), style | WS_VISIBLE, x.max(0), y.max(0), ww, wh, owner, null_mut(), inst, null())
    }
}

/// Boucle modale : la fenêtre principale est désactivée jusqu'à la fermeture. Renvoie la mise en page validée.
pub fn run(dlg: HWND, owner: HWND) -> Option<PrintLayout> {
    unsafe {
        if !owner.is_null() {
            EnableWindow(owner, 0);
        }
        let mut msg: MSG = std::mem::zeroed();
        while !DONE.with(|d| d.get()) && GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            if IsDialogMessageW(dlg, &msg) == 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
        if !owner.is_null() {
            EnableWindow(owner, 1);
            SetForegroundWindow(owner);
        }
    }
    RESULT.with(|r| r.borrow_mut().take())
}

/// Affiche la boîte et renvoie la mise en page validée (None si annulée).
pub fn show(start: &PrintLayout) -> Option<PrintLayout> {
    let owner = app().hwnd;
    let dlg = build(owner, start);
    run(dlg, owner)
}

/// Aides de l'autotest.
pub fn selftest_set(h: HWND, id: isize, s: &str) {
    set_text(h, id, s);
}
pub fn selftest_ids() -> (isize, isize, isize) {
    (ID_MARGIN, ID_HEAD, ID_FOOT)
}
pub fn selftest_ok(h: HWND) {
    unsafe { PostMessageW(h, WM_COMMAND, ID_OK as usize, 0) };
}
pub fn selftest_cancel(h: HWND) {
    unsafe { PostMessageW(h, WM_COMMAND, ID_CANCEL as usize, 0) };
}
