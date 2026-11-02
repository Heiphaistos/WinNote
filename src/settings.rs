//! Réglages enregistrés dans %APPDATA%\WinNote\settings.ini.
use crate::menu::Menus;
use crate::theme::{LAYOUTS, THEMES};
use crate::App;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::RECT;

fn path() -> std::path::PathBuf {
    std::path::PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join("WinNote").join("settings.ini")
}

pub fn defaults() -> App {
    let l = &LAYOUTS[0];
    App {
        hwnd: null_mut(), sci: null_mut(), tabs: null_mut(), status: null_mut(), title: null_mut(), side: null_mut(), tree: null_mut(),
        find_bar: null_mut(), find_edit: null_mut(), repl_edit: null_mut(), prev: null_mut(), palette: null_mut(),
        menus: Menus { bar: null_mut(), popup: null_mut(), recent: null_mut(), enc: null_mut(), lang: null_mut(), eol: null_mut() },
        accel: null_mut(), ui_font: null_mut(), find_brush: null_mut(), docs: Vec::new(), cur: 0, untitled: 0, theme: 0, layout: 0,
        wrap: l.wrap, lines: l.lines, ws: false, fold: false, spacing: 0, status_vis: true, font: l.font.into(), size: l.size, margin: l.margin, dpi: 96,
        chrome: 0, sidebar_vis: false, side_mode: 0, preview: false, find_vis: false, find_replace: false, f_case: false, f_word: false, f_regex: false,
        find_count: 0, zen: false, zen_style: 0, zen_rect: RECT { left: 0, top: 0, right: 0, bottom: 0 }, topmost: false, auto_update: true,
        root: None, recent: Vec::new(), closed: Vec::new(), quiet: false,
    }
}

pub fn load(a: &mut App) {
    let Ok(s) = std::fs::read_to_string(path()) else { return };
    for l in s.lines() {
        let Some((k, v)) = l.split_once('=') else { continue };
        let b = v == "1";
        match k {
            "theme" => a.theme = v.parse().ok().filter(|&x| x < THEMES.len()).unwrap_or(a.theme),
            "layout" => a.layout = v.parse().ok().filter(|&x| x < LAYOUTS.len()).unwrap_or(a.layout),
            "wrap" => a.wrap = b,
            "lines" => a.lines = b,
            "ws" => a.ws = b,
            "fold" => a.fold = b,
            "status" => a.status_vis = b,
            "chrome" => a.chrome = v.parse().ok().filter(|&x| x < 2).unwrap_or(0),
            "sidebar" => a.sidebar_vis = b,
            "sidemode" => a.side_mode = v.parse().ok().filter(|&x| x < 2).unwrap_or(0),
            "preview" => a.preview = b,
            "autoupdate" => a.auto_update = b,
            "font" => a.font = v.to_string(),
            "size" => a.size = v.parse().unwrap_or(a.size).clamp(6, 72),
            "margin" => a.margin = v.parse().unwrap_or(a.margin).clamp(0, 400),
            "spacing" => a.spacing = v.parse().unwrap_or(0).clamp(0, 30),
            "root" if !v.is_empty() && std::path::Path::new(v).is_dir() => a.root = Some(v.to_string()),
            _ => {}
        }
    }
}

pub fn save(a: &App) {
    let p = path();
    if let Some(d) = p.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    let s = format!(
        "theme={}\nlayout={}\nwrap={}\nlines={}\nws={}\nfold={}\nstatus={}\nchrome={}\nsidebar={}\nsidemode={}\npreview={}\nautoupdate={}\nfont={}\nsize={}\nmargin={}\nspacing={}\nroot={}\n",
        a.theme, a.layout, a.wrap as u8, a.lines as u8, a.ws as u8, a.fold as u8, a.status_vis as u8, a.chrome, a.sidebar_vis as u8, a.side_mode,
        a.preview as u8, a.auto_update as u8, a.font, a.size, a.margin, a.spacing, a.root.clone().unwrap_or_default()
    );
    if let Err(e) = std::fs::write(&p, s) {
        crate::cmds::msg_box(&format!("Réglages non enregistrés : {e}"));
    }
}
