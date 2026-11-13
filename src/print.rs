//! Impression (Ctrl+P) : mise en page par Scintilla sur le contexte de l'imprimante, noir sur blanc,
//! marges d'un pouce, en-tête (nom du fichier + date), pied de page « Page n / N », plage de pages et sélection.
use crate::sci::{self, send};
use crate::{app, w};
use windows_sys::Win32::{Graphics::Gdi::*, Storage::Xps::*, System::SystemInformation::GetLocalTime, UI::Controls::Dialogs::*};

#[repr(C)]
struct RangeToFormat {
    hdc: isize,
    hdc_target: isize,
    rc: [i32; 4],
    rc_page: [i32; 4],
    cp_min: isize,
    cp_max: isize,
}

const SETPRINTMAGNIFICATION: u32 = 2146;
const SETPRINTCOLOURMODE: u32 = 2148;
const SETPRINTWRAPMODE: u32 = 2406;
const FORMATRANGEFULL: u32 = 2777;

/// Positions de début de chaque page pour la zone [from, to) (passe de mesure, rien n'est dessiné).
fn paginate(hdc: HDC, rc: [i32; 4], rc_page: [i32; 4], from: isize, to: isize) -> Vec<isize> {
    let s = app().sci;
    let mut starts = Vec::new();
    let mut pos = from;
    while pos < to {
        starts.push(pos);
        let mut fr = RangeToFormat { hdc: hdc as isize, hdc_target: hdc as isize, rc, rc_page, cp_min: pos, cp_max: to };
        let next = send(s, FORMATRANGEFULL, 0, &mut fr as *mut _ as isize);
        if next <= pos {
            break;
        }
        pos = next;
    }
    send(s, FORMATRANGEFULL, 0, 0);
    starts
}

unsafe fn text_at(hdc: HDC, s: &str, x: i32, y: i32, align: u32) {
    let v: Vec<u16> = s.encode_utf16().collect();
    if v.is_empty() {
        return;
    }
    SetTextAlign(hdc, align);
    TextOutW(hdc, x, y, v.as_ptr(), v.len() as i32);
}

/// Imprime le document sur `hdc`. `pages` = plage demandée (1-based, incluse), `sel` = sélection à imprimer.
unsafe fn run(hdc: HDC, title: &str, output: Option<&str>, pages: Option<(u32, u32)>, sel: Option<(isize, isize)>) -> Result<u32, String> {
    let a = app();
    let s = a.sci;
    let cap = |i: u32| GetDeviceCaps(hdc, i as i32);
    let (pw, ph, dpi) = (cap(HORZRES), cap(VERTRES), cap(LOGPIXELSX).max(72));
    let dpi_y = cap(LOGPIXELSY).max(72);
    let (ox, oy) = (cap(PHYSICALOFFSETX), cap(PHYSICALOFFSETY));
    // marges de la mise en page (25 mm par défaut), relatives à la zone imprimable
    let layout = crate::extras::PrintLayout::load(&crate::prefs::dir());
    let mm = |d: i32| (layout.margin_mm as i32 * d * 10 / 254).max(0);
    let (ml, mt) = ((mm(dpi) - ox).max(0), (mm(dpi_y) - oy).max(0));
    let band = dpi_y / 3; // hauteur réservée à l'en-tête / au pied de page
    let rc_page = [0, 0, pw, ph];
    let rc = [ml, mt + band, pw - ml, ph - mt - band];
    send(s, SETPRINTCOLOURMODE, 2, 0); // noir sur blanc
    send(s, SETPRINTWRAPMODE, 1, 0); // retour à la ligne par mots
    send(s, SETPRINTMAGNIFICATION, 0, 0);
    let len = send(s, sci::GETLENGTH, 0, 0);
    let (from, to) = sel.unwrap_or((0, len));
    let starts = paginate(hdc, rc, rc_page, from, to);
    let total = starts.len().max(1) as u32;
    let wt = w(title);
    let out = output.map(w);
    let di = DOCINFOW { cbSize: std::mem::size_of::<DOCINFOW>() as i32, lpszDocName: wt.as_ptr(), lpszOutput: out.as_ref().map_or(std::ptr::null(), |o| o.as_ptr()), lpszDatatype: std::ptr::null(), fwType: 0 };
    if StartDocW(hdc, &di) <= 0 {
        return Err("StartDoc a échoué".into());
    }
    let face = w("Segoe UI");
    let font = CreateFontW(-(9 * dpi_y / 72), 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());
    let old = SelectObject(hdc, font);
    SetBkMode(hdc, TRANSPARENT as i32);
    SetTextColor(hdc, 0x555555);
    let (first, last) = pages.map_or((1, total), |(f, l)| (f.max(1), l.min(total).max(f.max(1))));
    let mut printed = 0;
    let mut st: windows_sys::Win32::Foundation::SYSTEMTIME = std::mem::zeroed();
    GetLocalTime(&mut st);
    let date = format!("{:02}/{:02}/{}", st.wDay, st.wMonth, st.wYear);
    for (i, &start) in starts.iter().enumerate() {
        let n = i as u32 + 1;
        if n < first || n > last {
            continue;
        }
        StartPage(hdc);
        let (hz, fz) = (crate::extras::zones(&layout.header, title, &date, n, total), crate::extras::zones(&layout.footer, title, &date, n, total));
        for (z, (x, al)) in [(ml, TA_LEFT), (pw / 2, TA_CENTER), (pw - ml, TA_RIGHT)].into_iter().enumerate() {
            text_at(hdc, &hz[z], x, mt, al | TA_TOP);
            text_at(hdc, &fz[z], x, ph - mt - band + band / 4, al | TA_TOP);
        }
        let mut fr = RangeToFormat { hdc: hdc as isize, hdc_target: hdc as isize, rc, rc_page, cp_min: start, cp_max: to };
        send(s, FORMATRANGEFULL, 1, &mut fr as *mut _ as isize);
        EndPage(hdc);
        printed += 1;
    }
    send(s, FORMATRANGEFULL, 0, 0);
    SelectObject(hdc, old);
    DeleteObject(font);
    EndDoc(hdc);
    Ok(printed)
}

/// Impression interactive (boîte de dialogue Windows : imprimante, copies, pages, sélection).
pub fn print_doc() {
    let a = app();
    let title = a.docs[a.cur].title();
    unsafe {
        let (ss, se) = (send(a.sci, sci::GETSELSTART, 0, 0), send(a.sci, sci::GETSELEND, 0, 0));
        let mut pd: PRINTDLGW = std::mem::zeroed();
        pd.lStructSize = std::mem::size_of::<PRINTDLGW>() as u32;
        pd.hwndOwner = a.hwnd;
        pd.Flags = PD_RETURNDC | PD_USEDEVMODECOPIESANDCOLLATE | if se > ss { 0 } else { PD_NOSELECTION };
        pd.nCopies = 1;
        pd.nFromPage = 1;
        pd.nToPage = 1;
        pd.nMinPage = 1;
        pd.nMaxPage = 9999;
        if PrintDlgW(&mut pd) == 0 || pd.hDC.is_null() {
            return;
        }
        let pages = (pd.Flags & PD_PAGENUMS != 0).then_some((pd.nFromPage as u32, pd.nToPage as u32));
        let sel = (pd.Flags & PD_SELECTION != 0).then_some((ss, se));
        let r = run(pd.hDC, &title, None, pages, sel);
        DeleteDC(pd.hDC);
        if let Err(e) = r {
            crate::cmds::msg_box(&format!("Impression impossible : {e}"));
        }
    }
}

/// Impression sans boîte de dialogue vers une imprimante nommée (tests : « Microsoft Print to PDF » + fichier de sortie).
pub fn print_to(printer: &str, out_file: &str) -> Result<u32, String> {
    let a = app();
    let title = a.docs[a.cur].title();
    unsafe {
        let hdc = CreateDCW(w("WINSPOOL").as_ptr(), w(printer).as_ptr(), std::ptr::null(), std::ptr::null());
        if hdc.is_null() {
            return Err(format!("imprimante introuvable : {printer}"));
        }
        let r = run(hdc, &title, Some(out_file), None, None);
        DeleteDC(hdc);
        r
    }
}
