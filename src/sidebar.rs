//! Panneau latéral : explorateur de dossier et plan du document (titres Markdown, fonctions, classes).
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
    UI::{Controls::*, WindowsAndMessaging::*},
};

const TV_ID: isize = 20;
const NM_DBLCLK_: u32 = -3i32 as u32;
const NM_RETURN_: u32 = -4i32 as u32;
const TVN_SELCHANGED_: u32 = -451i32 as u32;
const TVN_ITEMEXPANDING_: u32 = -454i32 as u32;
const NM_RCLICK_: u32 = -5i32 as u32;
const TVN_KEYDOWN_: u32 = -412i32 as u32;

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
    for (level, title, i) in crate::notes::outline(&text, md) {
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
    let hits = crate::notes::backlinks(&path, a.root.as_deref());
    let found = hits.len();
    for (f, i, l) in hits {
        let name = std::path::Path::new(&f).file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        nodes().push(Node { path: f, dir: false, loaded: true, line: i as isize });
        insert(0, &format!("{name} : {l}"), nodes().len() - 1, false);
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
                NM_RCLICK_ if app().side_mode == 0 => context_menu(),
                TVN_KEYDOWN_ if app().side_mode == 0 => {
                    match (*(lp as *const NMTVKEYDOWN)).wVKey {
                        0x71 => file_action(A_RENAME),
                        0x2E => file_action(A_DELETE),
                        _ => {}
                    }
                }
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

// ---------- opérations sur les fichiers (menu contextuel, F2, Suppr) ----------

const A_NEWFILE: usize = 1;
const A_NEWDIR: usize = 2;
const A_RENAME: usize = 3;
const A_DUP: usize = 4;
const A_DELETE: usize = 5;
const A_MOVE: usize = 6;
const A_REVEAL: usize = 7;
const A_COPYPATH: usize = 8;
const A_REFRESH: usize = 9;

fn context_menu() {
    let a = app();
    unsafe {
        let mut pt = zeroed();
        GetCursorPos(&mut pt);
        let mut sp = pt;
        windows_sys::Win32::Graphics::Gdi::ScreenToClient(a.tree, &mut sp);
        let hit = TVHITTESTINFO { pt: sp, flags: 0, hItem: 0 };
        let it = SendMessageW(a.tree, 0x1111, 0, &hit as *const _ as isize); // TVM_HITTEST
        if it != 0 {
            SendMessageW(a.tree, 0x110B, 9, it); // TVM_SELECTITEM(TVGN_CARET)
        }
        let on_item = it != 0;
        let m = CreatePopupMenu();
        let add = |id: usize, t: &str, on: bool| {
            AppendMenuW(m, MF_STRING | if on { 0 } else { MF_GRAYED }, id, w(t).as_ptr());
        };
        add(A_NEWFILE, "Nouveau fichier...", true);
        add(A_NEWDIR, "Nouveau dossier...", true);
        AppendMenuW(m, MF_SEPARATOR, 0, null());
        add(A_RENAME, "Renommer	F2", on_item);
        add(A_DUP, "Dupliquer", on_item);
        add(A_MOVE, "Déplacer vers...", on_item);
        add(A_DELETE, "Supprimer (corbeille)	Suppr", on_item);
        AppendMenuW(m, MF_SEPARATOR, 0, null());
        add(A_REVEAL, "Afficher dans l'explorateur", on_item);
        add(A_COPYPATH, "Copier le chemin", on_item);
        add(A_REFRESH, "Actualiser", true);
        let id = TrackPopupMenu(m, TPM_RETURNCMD | TPM_RIGHTBUTTON, pt.x, pt.y, 0, a.hwnd, null());
        DestroyMenu(m);
        if id > 0 {
            file_action(id as usize);
        }
    }
}

fn fail(e: String) {
    crate::cmds::msg_box(&e);
}

fn after_change(open: Option<String>) {
    show_files();
    if let Some(p) = open {
        crate::cmds::open_path(&p);
    }
}

/// Met à jour les onglets dont le fichier (ou son dossier) vient de changer de chemin.
fn retarget(old: &str, new: &str) {
    for d in app().docs.iter_mut() {
        if let Some(p) = &d.path {
            if p == old {
                d.path = Some(new.to_string());
            } else if let Some(rest) = p.strip_prefix(&format!("{old}\\")) {
                d.path = Some(format!("{new}\\{rest}"));
            }
        }
    }
    crate::cmds::refresh();
}

fn recycle(path: &str) -> bool {
    let mut from: Vec<u16> = path.encode_utf16().collect();
    from.extend([0, 0]);
    let mut op: windows_sys::Win32::UI::Shell::SHFILEOPSTRUCTW = unsafe { zeroed() };
    op.wFunc = 3; // FO_DELETE
    op.pFrom = from.as_ptr();
    op.fFlags = 0x40 | 0x10 | 0x4; // ALLOWUNDO | NOCONFIRMATION | SILENT
    unsafe { windows_sys::Win32::UI::Shell::SHFileOperationW(&mut op) == 0 && op.fAnyOperationsAborted == 0 }
}

fn file_action(act: usize) {
    let a = app();
    let sel = selected_node().map(|n| (nodes()[n].path.clone(), nodes()[n].dir));
    let root = a.root.clone().unwrap_or_default();
    let target_dir = match &sel {
        Some((p, true)) => p.clone(),
        Some((p, false)) => std::path::Path::new(p).parent().map(|d| d.to_string_lossy().into_owned()).unwrap_or(root.clone()),
        None => root.clone(),
    };
    let name_of = |p: &str| p.rsplit(['\\', '/']).next().unwrap_or(p).to_string();
    match (act, sel) {
        (A_NEWFILE, _) if !target_dir.is_empty() => crate::palette::input("Nom du nouveau fichier (Entrée pour créer)", "", Box::new(move |n| match crate::fileops::new_file(&target_dir, &n) {
            Ok(p) => after_change(Some(p)),
            Err(e) => fail(e),
        })),
        (A_NEWDIR, _) if !target_dir.is_empty() => crate::palette::input("Nom du nouveau dossier (Entrée pour créer)", "", Box::new(move |n| match crate::fileops::new_dir(&target_dir, &n) {
            Ok(_) => after_change(None),
            Err(e) => fail(e),
        })),
        (A_RENAME, Some((p, _))) => crate::palette::input("Nouveau nom (Entrée pour renommer)", &name_of(&p), Box::new(move |n| match crate::fileops::rename(&p, &n) {
            Ok(np) => {
                retarget(&p, &np);
                after_change(None)
            }
            Err(e) => fail(e),
        })),
        (A_DUP, Some((p, _))) => match crate::fileops::duplicate(&p) {
            Ok(np) => after_change(Some(np).filter(|x| !std::path::Path::new(x).is_dir())),
            Err(e) => fail(e),
        },
        (A_MOVE, Some((p, _))) => {
            if let Some(d) = crate::cmds::pick_folder() {
                match crate::fileops::move_to(&p, &d) {
                    Ok(np) => {
                        retarget(&p, &np);
                        after_change(None)
                    }
                    Err(e) => fail(e),
                }
            }
        }
        (A_DELETE, Some((p, _))) if p != root => {
            let ok = unsafe { MessageBoxW(a.hwnd, w(&format!("Mettre « {} » à la corbeille ?", name_of(&p))).as_ptr(), w("WinNote").as_ptr(), MB_YESNO | MB_ICONQUESTION) } == IDYES;
            if ok {
                if recycle(&p) {
                    after_change(None)
                } else {
                    fail("Impossible de mettre l'élément à la corbeille.".into())
                }
            }
        }
        (A_REVEAL, Some((p, _))) => {
            let _ = std::process::Command::new("explorer.exe").arg(format!("/select,{p}")).spawn();
        }
        (A_COPYPATH, Some((p, _))) => crate::cmds::copy_text(&p),
        (A_REFRESH, _) => show_files(),
        _ => {}
    }
}
