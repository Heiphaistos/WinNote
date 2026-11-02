//! Palette (Ctrl+Maj+P), ouverture rapide (Ctrl+P), aller à la ligne (Ctrl+G), recherche dans le dossier (Ctrl+Maj+F).
use crate::ids::PALETTE_CMDS;
use crate::sci::{self, send};
use crate::theme::THEMES;
use crate::ui::{blend, scale};
use crate::{app, w};
use std::mem::zeroed;
use std::ptr::null;
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::*, WindowsAndMessaging::*},
};

#[derive(Clone, Copy, PartialEq)]
pub enum Mode { Commands, Quick, Goto, InFiles }

#[derive(Clone)]
enum Act { Cmd(usize), Open(String), Tab(usize), Goto(isize), GotoIn(String, isize) }

struct Item { label: String, hint: String, act: Act }

struct Pal {
    mode: Mode,
    query: String,
    items: Vec<Item>,
    shown: Vec<usize>,
    sel: usize,
    searched: String,
    font: HFONT,
}

static mut PAL: Option<Pal> = None;

fn pal() -> &'static mut Pal {
    #[allow(static_mut_refs)]
    unsafe { PAL.as_mut().expect("palette fermée") }
}

const ROWS: usize = 9;

fn row_h() -> i32 { scale(34) }
fn head_h() -> i32 { scale(50) }

/// Score d'une correspondance « sous-séquence » (None = pas de correspondance).
fn fuzzy(q: &str, text: &str) -> Option<i32> {
    if q.is_empty() {
        return Some(0);
    }
    let (t, mut score, mut last) = (text.to_lowercase(), 0i32, -2i32);
    let mut ti = t.chars().enumerate().peekable();
    for qc in q.to_lowercase().chars() {
        loop {
            let (i, c) = ti.next()?;
            if c == qc {
                score += if i as i32 == last + 1 { 8 } else { 2 } - (i as i32 / 16);
                last = i as i32;
                break;
            }
        }
    }
    Some(score)
}

pub use crate::notes::walk;

fn collect(mode: Mode) -> Vec<Item> {
    let a = app();
    match mode {
        Mode::Commands => PALETTE_CMDS.iter().map(|&(id, n)| Item { label: n.to_string(), hint: String::new(), act: Act::Cmd(id) }).collect(),
        Mode::Quick => {
            let mut v: Vec<Item> = a.docs.iter().enumerate().map(|(i, d)| Item { label: d.title(), hint: d.path.clone().unwrap_or_else(|| "onglet ouvert".into()), act: Act::Tab(i) }).collect();
            let mut seen: std::collections::HashSet<String> = a.docs.iter().filter_map(|d| d.path.clone()).collect();
            for p in &a.recent {
                if seen.insert(p.clone()) {
                    v.push(Item { label: p.rsplit(['\\', '/']).next().unwrap_or(p).into(), hint: format!("récent · {p}"), act: Act::Open(p.clone()) });
                }
            }
            if let Some(r) = &a.root {
                let mut files = Vec::new();
                walk(std::path::Path::new(r), &mut files, 0);
                for f in files {
                    let s = f.to_string_lossy().to_string();
                    if seen.insert(s.clone()) {
                        let rel = s.strip_prefix(r.as_str()).unwrap_or(&s).trim_start_matches(['\\', '/']).to_string();
                        v.push(Item { label: f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), hint: rel, act: Act::Open(s) });
                    }
                }
            }
            v
        }
        _ => Vec::new(),
    }
}

fn search_in_files(q: &str) -> Vec<Item> {
    let Some(root) = app().root.clone() else { return vec![Item { label: "Ouvrez d'abord un dossier (Fichier > Ouvrir un dossier)".into(), hint: String::new(), act: Act::Goto(-1) }] };
    let mut files = Vec::new();
    walk(std::path::Path::new(&root), &mut files, 0);
    let ql = q.to_lowercase();
    let mut out = Vec::new();
    'f: for f in files {
        let Ok(meta) = std::fs::metadata(&f) else { continue };
        if meta.len() > 2_000_000 {
            continue;
        }
        let Ok(bytes) = std::fs::read(&f) else { continue };
        if bytes[..bytes.len().min(2048)].contains(&0) {
            continue;
        }
        let text = String::from_utf8_lossy(&bytes);
        for (i, l) in text.lines().enumerate() {
            if l.to_lowercase().contains(&ql) {
                let s = f.to_string_lossy().to_string();
                let rel = s.strip_prefix(root.as_str()).unwrap_or(&s).trim_start_matches(['\\', '/']).to_string();
                out.push(Item { label: l.trim().chars().take(120).collect(), hint: format!("{rel}:{}", i + 1), act: Act::GotoIn(s, i as isize) });
                if out.len() >= 300 {
                    break 'f;
                }
            }
        }
    }
    if out.is_empty() {
        out.push(Item { label: "Aucun résultat".into(), hint: String::new(), act: Act::Goto(-1) });
    }
    out
}

fn refilter() {
    let p = pal();
    match p.mode {
        Mode::Goto => {
            let max = send(app().sci, sci::GETLINECOUNT, 0, 0);
            p.items = vec![Item { label: format!("Aller à la ligne {} (1 – {})", p.query, max), hint: String::new(), act: Act::Goto(p.query.trim().parse::<isize>().map_or(-1, |n| (n - 1).clamp(0, max - 1))) }];
            p.shown = vec![0];
        }
        Mode::InFiles => {
            if p.query != p.searched {
                p.shown = (0..p.items.len()).collect();
            }
        }
        _ => {
            let mut sc: Vec<(i32, usize)> = p.items.iter().enumerate().filter_map(|(i, it)| fuzzy(&p.query, &it.label).or_else(|| fuzzy(&p.query, &it.hint).map(|s| s - 20)).map(|s| (s, i))).collect();
            sc.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
            p.shown = sc.into_iter().map(|x| x.1).collect();
        }
    }
    p.sel = 0;
}

fn place() {
    let a = app();
    let p = pal();
    let mut rc: RECT = unsafe { zeroed() };
    unsafe { GetClientRect(a.hwnd, &mut rc) };
    let wdt = (rc.right - scale(40)).min(scale(640));
    let rows = p.shown.len().clamp(1, ROWS) as i32;
    let h = head_h() + rows * row_h() + scale(8);
    unsafe { SetWindowPos(a.palette, HWND_TOP, (rc.right - wdt) / 2, crate::ui::title_height() + scale(50), wdt, h, SWP_SHOWWINDOW) };
}

pub fn open(mode: Mode) {
    let a = app();
    unsafe {
        if !a.palette.is_null() {
            close();
        }
        let face = w("Segoe UI");
        let font = CreateFontW(-(11 * a.dpi as i32 / 72 + 1), 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, face.as_ptr());
        PAL = Some(Pal { mode, query: String::new(), items: collect(mode), shown: Vec::new(), sel: 0, searched: "\u{0}".into(), font });
        a.palette = CreateWindowExW(0, w("WNPalette").as_ptr(), null(), WS_CHILD | WS_CLIPSIBLINGS, 0, 0, 10, 10, a.hwnd, 5 as HMENU, GetModuleHandleW(null()), null());
        refilter();
        place();
        SetFocus(a.palette);
    }
}

pub fn close() {
    let a = app();
    unsafe {
        if a.palette.is_null() {
            return;
        }
        let h = a.palette;
        a.palette = std::ptr::null_mut();
        #[allow(static_mut_refs)]
        if let Some(p) = PAL.take() {
            DeleteObject(p.font);
        }
        DestroyWindow(h);
        SetFocus(a.sci);
    }
}

fn run(i: usize) {
    let p = pal();
    let Some(&idx) = p.shown.get(i) else { return };
    if p.mode == Mode::InFiles && p.query != p.searched {
        p.searched = p.query.clone();
        p.items = search_in_files(&p.query);
        p.shown = (0..p.items.len()).collect();
        p.sel = 0;
        place();
        return;
    }
    let act = p.items[idx].act.clone();
    close();
    match act {
        Act::Cmd(id) => crate::cmds::on_command(id),
        Act::Open(f) => crate::cmds::open_path(&f),
        Act::Tab(t) => crate::cmds::cmd_select(t),
        Act::Goto(l) if l >= 0 => {
            send(app().sci, sci::GOTOLINE, l as usize, 0);
        }
        Act::GotoIn(f, l) => {
            crate::cmds::open_path(&f);
            send(app().sci, sci::GOTOLINE, l as usize, 0);
        }
        Act::Goto(_) => {}
    }
}

fn paint(dc: HDC, rc: &RECT) {
    let a = app();
    let t = &THEMES[a.theme];
    let p = pal();
    unsafe {
        let panel = blend(t.ui_bg, t.bg, 55);
        let border = blend(panel, t.ui_fg, 22);
        let brush = CreateSolidBrush(panel);
        let pen = CreatePen(PS_SOLID, 1, border);
        let (ob, op) = (SelectObject(dc, brush), SelectObject(dc, pen));
        RoundRect(dc, 0, 0, rc.right, rc.bottom, scale(14), scale(14));
        SelectObject(dc, ob);
        SelectObject(dc, op);
        DeleteObject(brush);
        DeleteObject(pen);
        SetBkMode(dc, TRANSPARENT as i32);
        SelectObject(dc, p.font);
        let draw = |s: &str, r: &mut RECT, flags: u32| {
            let v: Vec<u16> = s.encode_utf16().collect();
            if v.is_empty() {
                return;
            }
            DrawTextW(dc, v.as_ptr(), v.len() as i32, r, flags | DT_SINGLELINE | DT_VCENTER | DT_NOPREFIX | DT_END_ELLIPSIS);
        };
        let hint = match p.mode {
            Mode::Commands => "Tapez une commande...",
            Mode::Quick => "Ouvrir un fichier (onglets, récents, dossier)...",
            Mode::Goto => "Numéro de ligne...",
            Mode::InFiles => "Texte à chercher dans le dossier, puis Entrée...",
        };
        let mut hr = RECT { left: scale(20), top: 0, right: rc.right - scale(20), bottom: head_h() };
        if p.query.is_empty() {
            SetTextColor(dc, blend(t.ui_fg, panel, 60));
            draw(hint, &mut hr, 0);
        } else {
            SetTextColor(dc, t.ui_fg);
            draw(&p.query, &mut hr, 0);
        }
        let sepb = CreateSolidBrush(border);
        FillRect(dc, &RECT { left: 0, top: head_h() - 1, right: rc.right, bottom: head_h() }, sepb);
        DeleteObject(sepb);
        if p.shown.is_empty() {
            let mut r = RECT { left: scale(20), top: head_h(), right: rc.right - scale(20), bottom: head_h() + row_h() + scale(8) };
            SetTextColor(dc, blend(t.ui_fg, panel, 60));
            draw("Aucun résultat", &mut r, 0);
        }
        let first = p.sel.saturating_sub(ROWS - 1);
        for (row, &idx) in p.shown.iter().skip(first).take(ROWS).enumerate() {
            let top = head_h() + row as i32 * row_h() + scale(4);
            let r = RECT { left: scale(8), top, right: rc.right - scale(8), bottom: top + row_h() };
            let selected = first + row == p.sel;
            if selected {
                let b = CreateSolidBrush(blend(panel, t.accent, 30));
                let pe = CreatePen(PS_SOLID, 1, blend(panel, t.accent, 30));
                let (ob, op) = (SelectObject(dc, b), SelectObject(dc, pe));
                RoundRect(dc, r.left, r.top, r.right, r.bottom, scale(8), scale(8));
                SelectObject(dc, ob);
                SelectObject(dc, op);
                DeleteObject(b);
                DeleteObject(pe);
            }
            let it = &p.items[idx];
            let mut lr = RECT { left: r.left + scale(12), right: r.right - scale(12), ..r };
            SetTextColor(dc, t.ui_fg);
            if it.hint.is_empty() {
                draw(&it.label, &mut lr, 0);
            } else {
                let mut hr2 = lr;
                hr2.left = lr.left + (lr.right - lr.left) * 11 / 20;
                lr.right = hr2.left - scale(8);
                draw(&it.label, &mut lr, 0);
                SetTextColor(dc, blend(t.ui_fg, panel, 55));
                draw(&it.hint, &mut hr2, DT_RIGHT);
            }
        }
    }
}

pub unsafe extern "system" fn proc_(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if app().palette != h {
        return DefWindowProcW(h, m, wp, lp);
    }
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
        WM_GETDLGCODE => 0x0004 | 0x0001 | 0x0080, // DLGC_WANTALLKEYS
        WM_CHAR => {
            let p = pal();
            let c = char::from_u32(wp as u32).unwrap_or('\0');
            if wp == 8 {
                p.query.pop();
            } else if !c.is_control() {
                p.query.push(c);
            } else {
                return 0;
            }
            refilter();
            place();
            InvalidateRect(h, null(), 0);
            0
        }
        WM_KEYDOWN => {
            let p = pal();
            let n = p.shown.len();
            match wp as u16 {
                VK_ESCAPE => close(),
                VK_RETURN => run(p.sel),
                VK_DOWN if n > 0 => p.sel = (p.sel + 1) % n,
                VK_UP if n > 0 => p.sel = (p.sel + n - 1) % n,
                VK_NEXT if n > 0 => p.sel = (p.sel + ROWS).min(n - 1),
                VK_PRIOR => p.sel = p.sel.saturating_sub(ROWS),
                _ => {}
            }
            if app().palette == h {
                InvalidateRect(h, null(), 0);
            }
            0
        }
        WM_LBUTTONDOWN => {
            let y = ((lp >> 16) & 0xFFFF) as i16 as i32;
            let p = pal();
            let row = (y - head_h() - scale(4)) / row_h();
            if y > head_h() && row >= 0 {
                let first = p.sel.saturating_sub(ROWS - 1);
                run(first + row as usize);
            }
            0
        }
        WM_MOUSEWHEEL => {
            let p = pal();
            let d = ((wp >> 16) as i16) as i32;
            if d > 0 { p.sel = p.sel.saturating_sub(1) } else if p.sel + 1 < p.shown.len() { p.sel += 1 }
            InvalidateRect(h, null(), 0);
            0
        }
        WM_KILLFOCUS => {
            close();
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

/// Aides pour l'autotest : nombre de résultats de la recherche dans le dossier, et chemins trouvés par l'ouverture rapide.
pub fn selftest_search(q: &str) -> usize {
    search_in_files(q).iter().filter(|i| matches!(i.act, Act::GotoIn(..))).count()
}

pub fn selftest_quick(q: &str) -> Vec<String> {
    let items = collect(Mode::Quick);
    let mut sc: Vec<(i32, usize)> = items.iter().enumerate().filter_map(|(i, it)| fuzzy(q, &it.label).or_else(|| fuzzy(q, &it.hint).map(|s| s - 20)).map(|s| (s, i))).collect();
    sc.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
    sc.into_iter().filter_map(|(_, i)| if let Act::Open(p) = &items[i].act { Some(p.clone()) } else { None }).collect()
}
