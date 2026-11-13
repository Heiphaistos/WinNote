//! Aides d'édition partagées (Windows / Linux / macOS) : fermeture automatique des paires, surlignage des parenthèses assorties.
use crate::sci::{self, send, HWND};
use std::cell::RefCell;

const STYLE_BRACELIGHT: usize = 34;
const STYLE_BRACEBAD: usize = 35;
const BRACEHIGHLIGHT: u32 = 2351;
const BRACEBADLIGHT: u32 = 2352;
const BRACEMATCH: u32 = 2353;
const GETCHARAT: u32 = 2007;

thread_local! {
    /// Positions (fermant) des paires insérées automatiquement, pour les « retaper par-dessus ».
    static AUTO: RefCell<Vec<(usize, isize, u8)>> = const { RefCell::new(Vec::new()) };
}

pub fn style(s: HWND, accent: u32, sel: u32, fg: u32) {
    send(s, sci::STYLESETFORE, STYLE_BRACELIGHT, accent as isize);
    send(s, sci::STYLESETBACK, STYLE_BRACELIGHT, sel as isize);
    send(s, sci::STYLESETBOLD, STYLE_BRACELIGHT, 1);
    send(s, sci::STYLESETFORE, STYLE_BRACEBAD, 0x0000FF);
    send(s, sci::STYLESETBACK, STYLE_BRACEBAD, fg as isize);
}

fn char_at(s: HWND, p: isize) -> u8 {
    if p < 0 { 0 } else { send(s, GETCHARAT, p as usize, 0) as u8 }
}

/// À appeler sur SCN_UPDATEUI : surligne la parenthèse / accolade / crochet sous le curseur et son assortie.
pub fn brace_highlight(s: HWND) {
    let pos = send(s, sci::GETCURRENTPOS, 0, 0);
    let is_brace = |c: u8| b"()[]{}".contains(&c) && c != 0;
    let at = [pos - 1, pos].into_iter().find(|&p| p >= 0 && is_brace(char_at(s, p)));
    match at {
        Some(p) => {
            let m = send(s, BRACEMATCH, p as usize, 0);
            if m >= 0 {
                send(s, BRACEHIGHLIGHT, p as usize, m);
            } else {
                send(s, BRACEBADLIGHT, p as usize, 0);
            }
        }
        None => {
            send(s, BRACEHIGHLIGHT, usize::MAX, -1);
        }
    }
}

/// À appeler sur SCN_CHARADDED. `code` = langage de programmation (guillemets et accents graves fermés seulement dans le code), `text_only` = texte brut (rien).
pub fn on_char(s: HWND, ch: u8, code: bool, text_only: bool) {
    if text_only || send(s, 2570, 0, 0) > 1 {
        return; // pas de fermeture auto en texte brut ni avec plusieurs curseurs
    }
    let pos = send(s, sci::GETCURRENTPOS, 0, 0); // curseur juste après le caractère tapé
    let key = s as usize;
    // retaper un fermant déjà posé : on avale le doublon
    if matches!(ch, b')' | b']' | b'}' | b'"' | b'`') {
        let hit = AUTO.with(|a| {
            let mut a = a.borrow_mut();
            let i = a.iter().position(|&(k, p, c)| k == key && p == pos - 1 && c == ch);
            i.map(|i| a.remove(i)).is_some()
        });
        if hit && char_at(s, pos) == ch {
            send(s, sci::BEGINUNDO, 0, 0);
            send(s, sci::SETTARGETRANGE, (pos - 1) as usize, pos);
            send(s, sci::REPLACETARGET, 0, b"".as_ptr() as isize);
            send(s, sci::SETSEL, pos as usize, pos);
            send(s, sci::ENDUNDO, 0, 0);
            return;
        }
    }
    let close = match ch {
        b'(' => b')',
        b'[' => b']',
        b'{' => b'}',
        b'"' if code => b'"',
        b'`' if code => b'`',
        _ => return,
    };
    let next = char_at(s, pos);
    if !(next == 0 || next.is_ascii_whitespace() || b")]}".contains(&next)) {
        return;
    }
    if (ch == b'"' || ch == b'`') && pos >= 2 && char_at(s, pos - 2).is_ascii_alphanumeric() {
        return; // fermeture d'une chaîne, pas ouverture
    }
    send(s, sci::BEGINUNDO, 0, 0);
    send(s, sci::INSERTTEXT, pos as usize, [close, 0].as_ptr() as isize);
    send(s, sci::ENDUNDO, 0, 0);
    AUTO.with(|a| {
        let mut a = a.borrow_mut();
        a.retain(|&(k, p, _)| k != key || p < pos);
        a.push((key, pos, close));
    });
}
