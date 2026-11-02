//! Correcteur orthographique : utilise celui de Windows (ISpellChecker, dictionnaires installés), mots soulignés en rouge,
//! suggestions dans le menu contextuel. Actif sur les textes (texte brut, Markdown, LaTeX), pas sur le code.
#![allow(static_mut_refs)]
use crate::sci::{self, send};
use crate::{app, w};
use windows::core::{PCWSTR, PWSTR};
use windows::Win32::Globalization::{ISpellChecker, ISpellCheckerFactory, SpellCheckerFactory};
use windows::Win32::System::Com::{CoCreateInstance, CoTaskMemFree, CLSCTX_INPROC_SERVER};

const INDIC: usize = 9;
pub const LANGS: [(&str, &str); 5] = [("fr-FR", "Français"), ("en-US", "English (US)"), ("es-ES", "Español"), ("de-DE", "Deutsch"), ("it-IT", "Italiano")];

static mut CHECKER: Option<ISpellChecker> = None;
static mut LOADED: String = String::new();
static mut ERRORS: Vec<(isize, isize)> = Vec::new();
/// Mot sous le clic droit : (début, fin, suggestions, mot).
pub static mut PENDING: Option<(isize, isize, Vec<String>, String)> = None;

#[allow(static_mut_refs)]
fn checker() -> Option<&'static ISpellChecker> {
    unsafe { CHECKER.as_ref() }
}

/// (Re)crée le correcteur pour la langue demandée. Renvoie false si le dictionnaire n'est pas installé.
pub fn init(lang: &str) -> bool {
    unsafe {
        if LOADED == lang && CHECKER.is_some() {
            return true;
        }
        let made: windows::core::Result<ISpellChecker> = (|| {
            let f: ISpellCheckerFactory = CoCreateInstance(&SpellCheckerFactory, None, CLSCTX_INPROC_SERVER)?;
            let l = w(lang);
            if !f.IsSupported(PCWSTR(l.as_ptr()))?.as_bool() {
                return Err(windows::core::Error::from(windows::Win32::Foundation::E_NOTIMPL));
            }
            f.CreateSpellChecker(PCWSTR(l.as_ptr()))
        })();
        match made {
            Ok(c) => {
                CHECKER = Some(c);
                LOADED = lang.to_string();
                true
            }
            Err(_) => {
                CHECKER = None;
                LOADED.clear();
                false
            }
        }
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
    unsafe {
        ERRORS.clear();
    }
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
    let Some(c) = checker() else { return };
    let s = a.sci;
    let first = send(s, sci::GETFIRSTVISIBLE, 0, 0);
    let count = send(s, sci::GETLINECOUNT, 0, 0);
    let last = (first + send(s, 2370, 0, 0) + 2).min(count - 1); // SCI_LINESONSCREEN
    let (p0, p1) = (send(s, sci::POSFROMLINE, first as usize, 0), send(s, sci::GETLINEENDPOS, last as usize, 0));
    if p1 <= p0 {
        return;
    }
    let mut buf = vec![0u8; (p1 - p0) as usize + 1];
    let mut tr: [isize; 3] = [p0, p1, buf.as_mut_ptr() as isize];
    send(s, 2039, 0, tr.as_mut_ptr() as isize); // SCI_GETTEXTRANGEFULL
    buf.truncate((p1 - p0) as usize);
    let text = String::from_utf8_lossy(&buf).to_string();
    // décalage UTF-16 -> octets UTF-8
    let mut u16s: Vec<u16> = Vec::new();
    let mut byte_of: Vec<isize> = Vec::new();
    let mut off = 0isize;
    for ch in text.chars() {
        let mut t = [0u16; 2];
        for &u in ch.encode_utf16(&mut t).iter() {
            u16s.push(u);
            byte_of.push(off);
        }
        off += ch.len_utf8() as isize;
    }
    byte_of.push(off);
    u16s.push(0);
    unsafe {
        let Ok(errs) = c.Check(PCWSTR(u16s.as_ptr())) else { return };
        send(s, sci::SETINDICCURRENT, INDIC, 0);
        loop {
            let mut item: Option<windows::Win32::Globalization::ISpellingError> = None;
            if errs.Next(&mut item).0 != 0 {
                break;
            }
            let Some(e) = item else { break };
            let (Ok(st), Ok(len)) = (e.StartIndex(), e.Length()) else { continue };
            let (b0, b1) = (byte_of[(st as usize).min(byte_of.len() - 1)], byte_of[((st + len) as usize).min(byte_of.len() - 1)]);
            send(s, sci::INDICFILL, (p0 + b0) as usize, b1 - b0);
                ERRORS.push((p0 + b0, p0 + b1));
        }
    }
}

pub fn schedule() {
    let a = app();
    if a.spell {
        unsafe { windows_sys::Win32::UI::WindowsAndMessaging::SetTimer(a.hwnd, 9, 600, None) };
    }
}

/// Prépare les suggestions pour la position du clic droit. Renvoie les suggestions à proposer (vide si pas d'erreur).
pub fn prepare_at(pos: isize) -> Vec<String> {
    unsafe {
        let hit = ERRORS.iter().find(|&&(a, b)| pos >= a && pos <= b).copied();
        PENDING = None;
        let (Some((a, b)), Some(c)) = (hit, checker()) else { return Vec::new() };
        let s = app().sci;
        let mut buf = vec![0u8; (b - a) as usize + 1];
        let mut tr: [isize; 3] = [a, b, buf.as_mut_ptr() as isize];
        send(s, 2039, 0, tr.as_mut_ptr() as isize);
        buf.truncate((b - a) as usize);
        let word = String::from_utf8_lossy(&buf).to_string();
        let wv = w(&word);
        let mut out = Vec::new();
        if let Ok(en) = c.Suggest(PCWSTR(wv.as_ptr())) {
            let mut p = [PWSTR::null()];
            let mut n = 0u32;
            while out.len() < 5 && en.Next(&mut p, Some(&mut n)).is_ok() && n == 1 {
                out.push(p[0].to_string().unwrap_or_default());
                CoTaskMemFree(Some(p[0].0 as *const _));
            }
        }
        {
            PENDING = Some((a, b, out.clone(), word));
        }
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
        send(s, sci::REPLACETARGET, t.len(), crate::cstr(t).as_ptr() as isize);
    }
    check_visible();
}

/// Ajoute le mot au dictionnaire (add = true) ou l'ignore pour cette session.
pub fn learn(add: bool) {
    unsafe {
        let Some((_, _, _, word)) = PENDING.clone() else { return };
        if let Some(c) = checker() {
            let wv = w(&word);
            let _ = if add { c.Add(PCWSTR(wv.as_ptr())) } else { c.Ignore(PCWSTR(wv.as_ptr())) };
        }
    }
    check_visible();
}
