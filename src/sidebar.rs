//! Panneau latéral : explorateur de dossier et plan du document (titres Markdown, fonctions, classes).
use crate::sci::{self, send};
use crate::theme::THEMES;
use crate::ui::{blend, scale};
use crate::{app, w};
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::*,
    Graphics::Gdi::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Controls::*, WindowsAndMessaging::*},
};

const TV_ID: isize = 20;
const NM_DBLCLK_: u32 = -3i32 as u32;
const NM_RETURN_: u32 = -4i32 as u32;
const TVN_SELCHANGED_: u32 = -451i32 as u32;
const TVN_ITEMEXPANDING_: u32 = -454i32 as u32;

struct Node { path: String, dir: bool, loaded: bool, line: isize }
static mut NODES: Vec<Node> = Vec::new();

fn nodes() -> &'static mut Vec<Node> {
    #[allow(static_mut_refs)]
    unsafe { &mut NODES }
}

pub fn width() -> i32 {
    if app().sidebar_vis { scale(270) } else { 0 }
}
fn head_h() -> i32 {
    scale(34)
}

pub fn create(parent: HWND) {
    let a = app();
    unsafe {
        let inst = GetModuleHandleW(null());
        a.side = CreateWindowExW(0, w("WNSide").as_ptr(), null(), WS_CHILD | WS_CLIPCHILDREN | WS_CLIPSIBLINGS, 0, 0, 0, 0, parent, 6 as HMENU, inst, null());
        let style = WS_CHILD | WS_VISIBLE | (TVS_HASBUTTONS | TVS_SHOWSELALWAYS | TVS_FULLROWSELECT | TVS_NOHSCROLL | TVS_LINESATROOT) as u32;
        a.tree = CreateWindowExW(0, w("SysTreeView32").as_ptr(), null(), style, 0, 0, 0, 0, a.side, TV_ID as HMENU, inst, null());
        SendMessageW(a.tree, WM_SETFONT, a.ui_font as usize, 1);
    }
    recolor();
}

pub fn layout() {
    let a = app();
    let mut rc: RECT = unsafe { zeroed() };
    unsafe {
        GetClientRect(a.side, &mut rc);
        MoveWindow(a.tree, 0, head_h(), rc.right, (rc.bottom - head_h()).max(0), 1);
    }
}

pub fn recolor() {
    let a = app();
    if a.tree.is_null() {
        return;
    }
    let t = &THEMES[a.theme];
    unsafe {
        SendMessageW(a.tree, 0x111D, 0, t.ui_bg as isize); // TVM_SETBKCOLOR
        SendMessageW(a.tree, 0x111E, 0, t.ui_fg as isize); // TVM_SETTEXTCOLOR
        SendMessageW(a.tree, 0x111B, scale(26) as usize, 0); // TVM_SETITEMHEIGHT
        SetWindowTheme(a.tree, w(if t.dark { "DarkMode_Explorer" } else { "Explorer" }).as_ptr(), null());
        InvalidateRect(a.side, null(), 1);
    }
}

fn insert(parent: HTREEITEM, text: &str, node: usize, children: bool) -> HTREEITEM {
    let a = app();
    unsafe {
        let mut item: TVITEMW = zeroed();
        item.mask = TVIF_TEXT | TVIF_PARAM | TVIF_CHILDREN;
        let mut t = w(text);
        item.pszText = t.as_mut_ptr();
        item.lParam = node as isize;
        item.cChildren = children as i32;
        let ins = TVINSERTSTRUCTW { hParent: parent, hInsertAfter: -0xFFFE_isize, Anonymous: TVINSERTSTRUCTW_0 { item } };
        SendMessageW(a.tree, 0x1132, 0, &ins as *const _ as isize) as HTREEITEM // TVM_INSERTITEMW
    }
}

fn clear() {
    unsafe { SendMessageW(app().tree, 0x1101, 0, -0x10000) }; // TVM_DELETEITEM(TVI_ROOT)
    nodes().clear();
}

fn load_dir(parent: HTREEITEM, dir: &str) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut v: Vec<(bool, String, String)> = rd
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            (!matches!(n.as_str(), ".git" | "node_modules" | "target" | ".logs")).then(|| (e.path().is_dir(), n, e.path().to_string_lossy().to_string()))
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.to_lowercase().cmp(&b.1.to_lowercase())));
    for (d, n, p) in v {
        nodes().push(Node { path: p, dir: d, loaded: false, line: 0 });
        insert(parent, &n, nodes().len() - 1, d);
    }
}

/// Affiche le dossier racine dans l'explorateur.
pub fn show_files() {
    let a = app();
    a.side_mode = 0;
    clear();
    if let Some(root) = a.root.clone() {
        nodes().push(Node { path: root.clone(), dir: true, loaded: true, line: 0 });
        let name = root.trim_end_matches(['\\', '/']).rsplit(['\\', '/']).next().unwrap_or(&root).to_string();
        let r = insert(0, &name, 0, true);
        load_dir(r, &root);
        unsafe { SendMessageW(a.tree, 0x1102, 2, r as isize) }; // TVM_EXPAND
    }
    unsafe { InvalidateRect(a.side, null(), 0) };
}

/// Plan : titres Markdown, ou définitions (fn / class / def / function...) pour le code.
pub fn show_outline() {
    let a = app();
    a.side_mode = 1;
    clear();
    let n = send(a.sci, sci::GETLENGTH, 0, 0) as usize;
    let mut buf = vec![0u8; n + 1];
    send(a.sci, sci::GETTEXT, n + 1, buf.as_mut_ptr() as isize);
    let text = String::from_utf8_lossy(&buf[..n]).to_string();
    let md = crate::lang::LANGS[a.docs[a.cur].lang].lexer == "markdown";
    let mut stack: Vec<(usize, HTREEITEM)> = Vec::new();
    let mut in_code = false;
    for (i, l) in text.lines().enumerate() {
        let t = l.trim_start();
        if t.starts_with("```") {
            in_code = !in_code;
        }
        let (level, title) = if md && !in_code && t.starts_with('#') {
            let lv = t.chars().take_while(|&c| c == '#').count();
            (lv, t[lv..].trim().to_string())
        } else if !md && ["fn ", "pub fn ", "def ", "class ", "function ", "func ", "struct ", "impl ", "enum ", "trait ", "interface ", "async fn ", "pub struct "].iter().any(|k| t.starts_with(k)) {
            (1 + (l.len() - t.len()) / 4, t.trim_end_matches(['{', ':', ' ']).chars().take(80).collect())
        } else {
            continue;
        };
        if title.is_empty() {
            continue;
        }
        while stack.last().is_some_and(|s| s.0 >= level) {
            stack.pop();
        }
        nodes().push(Node { path: String::new(), dir: false, loaded: true, line: i as isize });
        let parent = stack.last().map_or(0, |s| s.1);
        let it = insert(parent, &title, nodes().len() - 1, false);
        if parent != 0 {
            unsafe { SendMessageW(a.tree, 0x1102, 2, parent) };
        }
        stack.push((level, it));
    }
    unsafe {
        SendMessageW(a.tree, 0x1102, 2, 0);
        InvalidateRect(a.side, null(), 0);
    }
}

/// Liens entrants : notes du dossier qui contiennent [[nom-de-la-note-courante]].
pub fn show_backlinks() {
    let a = app();
    a.side_mode = 2;
    clear();
    let cur = a.docs[a.cur].path.clone();
    let Some(path) = cur else {
        insert(0, "Enregistrez la note pour voir ses liens entrants", 0, false);
        nodes().push(Node { path: String::new(), dir: false, loaded: true, line: -1 });
        return;
    };
    let p = std::path::Path::new(&path);
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let base = a.root.clone().map(std::path::PathBuf::from).or_else(|| p.parent().map(|d| d.to_path_buf())).unwrap_or_default();
    let mut files = Vec::new();
    crate::palette::walk(&base, &mut files, 0);
    let pats = [format!("[[{stem}]]"), format!("[[{stem}|"), format!("[[{stem}#")];
    let mut found = 0;
    for f in files {
        let ext = f.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !matches!(ext.as_str(), "md" | "markdown" | "mdx" | "txt") || f == p {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        if let Some((i, l)) = text.lines().enumerate().find(|(_, l)| {
            let ll = l.to_lowercase();
            pats.iter().any(|q| ll.contains(q.as_str()))
        }) {
            let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            nodes().push(Node { path: f.to_string_lossy().to_string(), dir: false, loaded: true, line: i as isize });
            insert(0, &format!("{name} : {}", l.trim().chars().take(70).collect::<String>()), nodes().len() - 1, false);
            found += 1;
        }
    }
    if found == 0 {
        nodes().push(Node { path: String::new(), dir: false, loaded: true, line: -1 });
        insert(0, "Aucun lien entrant", nodes().len() - 1, false);
    }
    unsafe { InvalidateRect(a.side, null(), 0) };
}

pub fn refresh() {
    let a = app();
    if a.sidebar_vis && a.side_mode == 1 {
        show_outline();
    } else if a.sidebar_vis && a.side_mode == 2 {
        show_backlinks();
    }
}

pub fn set_root(p: &str) {
    let a = app();
    a.root = Some(p.to_string());
    a.sidebar_vis = true;
    crate::relayout();
    show_files();
}

fn selected_node() -> Option<usize> {
    let a = app();
    unsafe {
        let it = SendMessageW(a.tree, 0x110A, 9, 0) as HTREEITEM; // TVM_GETNEXTITEM(TVGN_CARET)
        if it == 0 {
            return None;
        }
        let mut i: TVITEMW = zeroed();
        i.mask = TVIF_PARAM;
        i.hItem = it;
        SendMessageW(a.tree, 0x113E, 0, &mut i as *mut _ as isize); // TVM_GETITEMW
        Some(i.lParam as usize)
    }
}

fn activate() {
    let Some(n) = selected_node() else { return };
    let node = &nodes()[n];
    if app().side_mode == 2 {
        if !node.path.is_empty() {
            let (p, l) = (node.path.clone(), node.line);
            crate::cmds::open_path(&p);
            send(app().sci, sci::GOTOLINE, l as usize, 0);
        }
    } else if app().side_mode == 1 {
        send(app().sci, sci::GOTOLINE, node.line as usize, 0);
        unsafe { windows_sys::Win32::UI::Input::KeyboardAndMouse::SetFocus(app().sci) };
    } else if !node.dir {
        let p = node.path.clone();
        crate::cmds::open_path(&p);
    }
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
        SetBkMode(dc, TRANSPARENT as i32);
        SelectObject(dc, a.ui_font);
        let half = rc.right / 3;
        for (i, label) in ["FICHIERS", "PLAN", "LIENS"].iter().enumerate() {
            let mut r = RECT { left: i as i32 * half, top: 0, right: (i as i32 + 1) * half, bottom: head_h() };
            SetTextColor(dc, if a.side_mode as usize == i { t.ui_fg } else { blend(t.ui_fg, t.ui_bg, 50) });
            let v: Vec<u16> = label.encode_utf16().collect();
            DrawTextW(dc, v.as_ptr(), v.len() as i32, &mut r, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
            if a.side_mode as usize == i {
                fill(&RECT { left: r.left + scale(24), top: head_h() - scale(3), right: r.right - scale(24), bottom: head_h() - scale(1) }, t.accent);
            }
        }
        fill(&RECT { left: rc.right - 1, top: 0, right: rc.right, bottom: rc.bottom }, blend(t.ui_bg, t.ui_fg, 14));
    }
}

pub unsafe extern "system" fn proc_(h: HWND, m: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match m {
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            layout();
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let dc = BeginPaint(h, &mut ps);
            let mut rc: RECT = zeroed();
            GetClientRect(h, &mut rc);
            paint(dc, &RECT { bottom: head_h(), ..rc });
            EndPaint(h, &ps);
            0
        }
        WM_LBUTTONDOWN => {
            let x = (lp & 0xFFFF) as i16 as i32;
            let mut rc: RECT = zeroed();
            GetClientRect(h, &mut rc);
            match x * 3 / rc.right.max(1) { 0 => show_files(), 1 => show_outline(), _ => show_backlinks() }
            0
        }
        WM_NOTIFY => {
            let hdr = &*(lp as *const NMHDR);
            match hdr.code {
                NM_DBLCLK_ | NM_RETURN_ => activate(),
                TVN_SELCHANGED_ if app().side_mode == 1 => activate_outline_quiet(),
                TVN_ITEMEXPANDING_ => {
                    let nm = &*(lp as *const NMTREEVIEWW);
                    let n = nm.itemNew.lParam as usize;
                    if nm.action == 2 && n < nodes().len() && nodes()[n].dir && !nodes()[n].loaded {
                        nodes()[n].loaded = true;
                        let p = nodes()[n].path.clone();
                        load_dir(nm.itemNew.hItem, &p);
                    }
                }
                _ => {}
            }
            0
        }
        _ => DefWindowProcW(h, m, wp, lp),
    }
}

fn activate_outline_quiet() {
    if let Some(n) = selected_node() {
        send(app().sci, sci::GOTOLINE, nodes()[n].line as usize, 0);
    }
}

/// Pour l'autotest : nombre de liens entrants trouvés.
pub fn selftest_backlinks() -> usize {
    show_backlinks();
    nodes().iter().filter(|n| !n.path.is_empty()).count()
}
