//! Impression (Ctrl+P) : le moteur Scintilla met en page le texte sur le contexte de l'imprimante, noir sur blanc.
use crate::sci::{self, send};
use crate::{app, w};
use windows_sys::Win32::{Foundation::RECT, Graphics::Gdi::*, Storage::Xps::*, UI::Controls::Dialogs::*};

#[repr(C)]
struct RangeToFormat {
    hdc: isize,
    hdc_target: isize,
    rc: [i32; 4],
    rc_page: [i32; 4],
    cp_min: isize,
    cp_max: isize,
}

const SETPRINTCOLOURMODE: u32 = 2148;
const FORMATRANGEFULL: u32 = 2777;

pub fn print_doc() {
    let a = app();
    unsafe {
        let mut pd: PRINTDLGW = std::mem::zeroed();
        pd.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
        pd.hwndOwner = a.hwnd;
        pd.Flags = PD_RETURNDC | PD_NOSELECTION | PD_USEDEVMODECOPIESANDCOLLATE;
        pd.nCopies = 1;
        if PrintDlgW(&mut pd) == 0 || pd.hDC.is_null() {
            return;
        }
        let hdc = pd.hDC;
        let (pw, ph) = (GetDeviceCaps(hdc, HORZRES as i32), GetDeviceCaps(hdc, VERTRES as i32));
        let (ox, oy) = (GetDeviceCaps(hdc, PHYSICALOFFSETX as i32), GetDeviceCaps(hdc, PHYSICALOFFSETY as i32));
        let m = GetDeviceCaps(hdc, LOGPIXELSX as i32); // marge d'un pouce
        let rc_page = [0, 0, pw, ph];
        let rc = [(m - ox).max(0), (m - oy).max(0), pw - (m - ox).max(0), ph - (m - oy).max(0)];
        let title = w(&a.docs[a.cur].title());
        let di = DOCINFOW { cbSize: std::mem::size_of::<DOCINFOW>() as i32, lpszDocName: title.as_ptr(), lpszOutput: std::ptr::null(), lpszDatatype: std::ptr::null(), fwType: 0 };
        if StartDocW(hdc, &di) <= 0 {
            DeleteDC(hdc);
            crate::cmds::msg_box("Impression impossible.");
            return;
        }
        send(a.sci, SETPRINTCOLOURMODE, 2, 0); // SC_PRINT_BLACKONWHITE
        let len = send(a.sci, sci::GETLENGTH, 0, 0);
        let mut pos = 0isize;
        while pos < len {
            StartPage(hdc);
            let mut fr = RangeToFormat { hdc: hdc as isize, hdc_target: hdc as isize, rc, rc_page, cp_min: pos, cp_max: len };
            let next = send(a.sci, FORMATRANGEFULL, 1, &mut fr as *mut _ as isize);
            EndPage(hdc);
            if next <= pos {
                break;
            }
            pos = next;
        }
        send(a.sci, FORMATRANGEFULL, 0, 0); // libère le cache de mise en page
        EndDoc(hdc);
        DeleteDC(hdc);
    }
    let _ = RECT { left: 0, top: 0, right: 0, bottom: 0 };
}
