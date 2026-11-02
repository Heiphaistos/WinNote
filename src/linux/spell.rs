//! Correcteur orthographique via Enchant (libenchant-2 chargée à l'exécution : rien à lier, absent = correcteur inactif).
//! Mots inconnus soulignés en rouge (indicateur 9), suggestions dans le menu contextuel. Actif sur les textes, pas sur le code.
#![allow(static_mut_refs)]
use crate::sci::{self, send};
use crate::{app, cstr};
use std::ffi::{c_char, c_int, c_void, CStr, CString};

const INDIC: usize = 9;
pub const LANGS: [(&str, &str); 5] = [("fr_FR", "Français"), ("en_US", "English (US)"), ("es_ES", "Español"), ("de_DE", "Deutsch"), ("it_IT", "Italiano")];

extern "C" {
    fn dlopen(f: *const c_char, flags: c_int) -> *mut c_void;
    fn dlsym(h: *mut c_void, s: *const c_char) -> *mut c_void;
}

struct Api {
    broker: *mut c_void,
    request: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_void,
    check: unsafe extern "C" fn(*mut c_void, *const c_char, isize) -> c_int,
    suggest: unsafe extern "C" fn(*mut c_void, *const c_char, isize, *mut usize) -> *mut *mut c_char,
    free_list: unsafe extern "C" fn(*mut c_void, *mut *mut c_char),
    add: unsafe extern "C" fn(*mut c_void, *const c_char, isize),
    session: unsafe extern "C" fn(*mut c_void, *const c_char, isize),
}

static mut API: Option<Api> = None;
static mut TRIED: bool = false;
static mut DICT: *mut c_void = std::ptr::null_mut();
static mut LOADED: String = String::new();
static mut ERRORS: Vec<(isize, isize)> = Vec::new();
/// Mot sous le clic droit : (début, fin, suggestions, mot).
static mut PENDING: Option<(isize, isize, Vec<String>, String)> = None;

unsafe fn load_api() -> Option<&'static Api> {
    if !TRIED {
        TRIED = true;
        let names = ["libenchant-2.so.2", "libenchant-2.2.dylib", "libenchant-2.dylib", "/opt/homebrew/lib/libenchant-2.2.dylib", "/usr/local/lib/libenchant-2.2.dylib"];
        let h = names.iter().map(|n| dlopen(cstr(n).as_ptr() as *const c_char, 2)).find(|h| !h.is_null());
        if let Some(h) = h {
            let sym = |n: &str| dlsym(h, cstr(n).as_ptr() as *const c_char);
            let init = sym("enchant_broker_init");
            let (request, check, suggest, free_list, add, session) = (sym("enchant_broker_request_dict"), sym("enchant_dict_check"), sym("enchant_dict_suggest"), sym("enchant_dict_free_string_list"), sym("enchant_dict_add"), sym("enchant_dict_add_to_session"));
            if ![init, request, check, suggest, free_list, add, session].iter().any(|p| p.is_null()) {
                let broker = std::mem::transmute::<*mut c_void, unsafe extern "C" fn() -> *mut c_void>(init)();
                if !broker.is_null() {
                    API = Some(Api {
                        broker,
                        request: std::mem::transmute(request),
                        check: std::mem::transmute(check),
                        suggest: std::mem::transmute(suggest),
                        free_list: std::mem::transmute(free_list),
                        add: std::mem::transmute(add),
                        session: std::mem::transmute(session),
                    });
                }
            }
        }
    }
    API.as_ref()
}

/// Charge le dictionnaire de la langue. Renvoie false si Enchant ou le dictionnaire est absent.
pub fn init(lang: &str) -> bool {
    unsafe {
        if LOADED == lang && !DICT.is_null() {
            return true;
        }
        let Some(api) = load_api() else { return false };
        let d = (api.request)(api.broker, cstr(lang).as_ptr() as *const c_char);
        DICT = d;
        LOADED = if d.is_null() { String::new() } else { lang.to_string() };
        !d.is_null()
    }
}

pub fn style_indicator() {
    let a = app();
    for v in [a.sci, a.sci2] {
        send(v, sci::INDICSETSTYLE, INDIC, 1); // INDIC_SQUIGGLE
        send(v, sci::INDICSETFORE, INDIC, 0x3030E0);
    }
}

fn text_like() -> bool {
    let a = app();
    matches!(crate::lang::LANGS[a.docs[a.cur].lang].lexer, "" | "markdown" | "latex")
}

fn clear() {
    let a = app();
    send(a.sci, sci::SETINDICCURRENT, INDIC, 0);
    send(a.sci, sci::INDICCLEAR, 0, send(a.sci, sci::GETLENGTH, 0, 0));
    unsafe { ERRORS.clear() };
}

fn is_word(c: char) -> bool {
    c.is_alphabetic() || c == '\'' || c == '’'
}

/// Mots d'un texte : (début, fin) en octets, apostrophes de bord exclues, hors mots contenant des chiffres ou collés à un `/` ou `.` (URL, noms de fichier).
pub fn words(text: &str) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut start: Option<usize> = None;
    let flush = |s: usize, e: usize, out: &mut Vec<(usize, usize)>| {
        let w = &text[s..e];
        let t = w.trim_matches(|c| c == '\'' || c == '’');
        let off = s + w.find(t).unwrap_or(0);
        let before = text[..off].chars().next_back();
        let after = text[off + t.len()..].chars().next();
        let glued = |c: Option<char>| matches!(c, Some('/' | '@' | '_' | '#'));
        if t.chars().count() >= 2 && !glued(before) && !glued(after) && !matches!(after, Some(c) if c.is_ascii_digit()) {
            out.push((off, off + t.len()));
        }
    };
    for (i, c) in text.char_indices() {
        if is_word(c) {
            start.get_or_insert(i);
        } else if let Some(s) = start.take() {
            flush(s, i, &mut out);
        }
    }
    if let Some(s) = start {
        flush(s, text.len(), &mut out);
    }
    out
}

fn range_text(p0: isize, p1: isize) -> String {
    let s = app().sci;
    let mut buf = vec![0u8; (p1 - p0) as usize + 1];
    let mut tr: [isize; 3] = [p0, p1, buf.as_mut_ptr() as isize];
    send(s, 2039, 0, tr.as_mut_ptr() as isize); // SCI_GETTEXTRANGEFULL
    buf.truncate((p1 - p0) as usize);
    String::from_utf8_lossy(&buf).to_string()
}

/// Vérifie les lignes visibles et souligne les mots inconnus.
pub fn check_visible() {
    let a = app();
    if a.docs.is_empty() {
        return;
    }
    clear();
    if !a.spell || !text_like() || !init(&a.spell_lang) {
        return;
    }
    let s = a.sci;
    let first = send(s, sci::GETFIRSTVISIBLE, 0, 0);
    let count = send(s, sci::GETLINECOUNT, 0, 0);
    let last = (first + send(s, 2370, 0, 0) + 2).min(count - 1); // SCI_LINESONSCREEN
    let (p0, p1) = (send(s, sci::POSFROMLINE, first as usize, 0), send(s, sci::GETLINEENDPOS, last as usize, 0));
    if p1 <= p0 {
        return;
    }
    let text = range_text(p0, p1);
    send(s, sci::SETINDICCURRENT, INDIC, 0);
    unsafe {
        let Some(api) = API.as_ref() else { return };
        for (b0, b1) in words(&text) {
            let w = &text[b0..b1];
            let Ok(cw) = CString::new(w) else { continue };
            if (api.check)(DICT, cw.as_ptr(), w.len() as isize) > 0 {
                send(s, sci::INDICFILL, (p0 + b0 as isize) as usize, (b1 - b0) as isize);
                ERRORS.push((p0 + b0 as isize, p0 + b1 as isize));
            }
        }
    }
}

pub fn error_count() -> usize {
    unsafe { ERRORS.len() }
}

pub fn has_pending() -> bool {
    unsafe { PENDING.is_some() }
}

/// Prépare les suggestions pour la position du clic droit (vide si le mot est correct).
pub fn prepare_at(pos: isize) -> Vec<String> {
    unsafe {
        PENDING = None;
        let hit = ERRORS.iter().find(|&&(a, b)| pos >= a && pos <= b).copied();
        let (Some((a, b)), Some(api)) = (hit, API.as_ref()) else { return Vec::new() };
        let word = range_text(a, b);
        let Ok(cw) = CString::new(word.as_str()) else { return Vec::new() };
        let mut n = 0usize;
        let list = (api.suggest)(DICT, cw.as_ptr(), word.len() as isize, &mut n);
        let mut out = Vec::new();
        if !list.is_null() {
            for i in 0..n.min(5) {
                out.push(CStr::from_ptr(*list.add(i)).to_string_lossy().to_string());
            }
            (api.free_list)(DICT, list);
        }
        PENDING = Some((a, b, out.clone(), word));
        out
    }
}

/// Applique la suggestion n au mot sous le clic droit.
pub fn apply_fix(n: usize) {
    unsafe {
        let Some((a, b, sug, _)) = PENDING.clone() else { return };
        let Some(t) = sug.get(n) else { return };
        let s = app().sci;
        send(s, sci::SETTARGETRANGE, a as usize, b);
        send(s, sci::REPLACETARGET, t.len(), cstr(t).as_ptr() as isize);
    }
    check_visible();
}

/// Ajoute le mot au dictionnaire personnel (add = true) ou l'ignore pour cette session.
pub fn learn(add: bool) {
    unsafe {
        let Some((_, _, _, word)) = PENDING.clone() else { return };
        if let (Some(api), Ok(cw)) = (API.as_ref(), CString::new(word.as_str())) {
            (if add { api.add } else { api.session })(DICT, cw.as_ptr(), word.len() as isize);
        }
    }
    check_visible();
}

#[cfg(test)]
mod tests {
    #[test]
    fn tokenizer() {
        let t = "Bonjour l'ami, voir http://x.org/a_b et 3e mondde.";
        let w: Vec<&str> = super::words(t).iter().map(|&(a, b)| &t[a..b]).collect();
        assert!(w.contains(&"Bonjour") && w.contains(&"l'ami") && w.contains(&"mondde"), "{w:?}");
        assert!(!w.contains(&"org") && !w.contains(&"x"), "{w:?}");
    }
}
