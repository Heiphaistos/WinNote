//! Aides d'édition partagées (Windows / Linux / macOS) : fermeture automatique des paires, surlignage des parenthèses assorties.
use crate::sci::{self, send, HWND};
use std::cell::RefCell;

const STYLE_BRACELIGHT: usize = 34;
const STYLE_BRACEBAD: usize = 35;
const BRACEHIGHLIGHT: u32 = 2351;
const BRACEBADLIGHT: u32 = 2352;
const BRACEMATCH: u32 = 2353;
const GETCHARAT: u32 = 2007;
const WORDSTARTPOSITION: u32 = 2266;
const AUTOCSHOW: u32 = 2100;
const AUTOCSETIGNORECASE: u32 = 2115;
const AUTOCSETMAXHEIGHT: u32 = 2210;

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
    if code && (ch.is_ascii_alphanumeric() || ch == b'_') {
        if crate::prefs::autocomp() {
            complete(s);
        }
        return;
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

/// Mots du texte qui commencent par `prefix` (sans tenir compte de la casse), distincts, triés, sans le mot en cours de frappe.
pub fn words_with_prefix(text: &str, prefix: &str, skip_at: usize) -> Vec<String> {
    let pl = prefix.to_lowercase();
    let mut out: Vec<String> = Vec::new();
    let (mut start, mut chars) = (None, text.char_indices().peekable());
    while let Some((i, c)) = chars.next() {
        let wc = c.is_alphanumeric() || c == '_';
        if wc && start.is_none() {
            start = Some(i);
        }
        let end = if !wc { Some(i) } else if chars.peek().is_none() { Some(text.len()) } else { None };
        if let (Some(e), Some(st)) = (end, start) {
            start = None;
            let w = &text[st..e];
            if st != skip_at && w.len() > prefix.len() && w.to_lowercase().starts_with(&pl) && !out.iter().any(|x| x == w) {
                out.push(w.to_string());
            }
        }
    }
    out.sort_by_key(|w| w.to_lowercase());
    out.truncate(40);
    out
}

/// Propose les mots du document qui complètent le mot en cours (à partir de 3 lettres).
pub fn complete(s: HWND) {
    let pos = send(s, sci::GETCURRENTPOS, 0, 0);
    let start = send(s, WORDSTARTPOSITION, pos as usize, 1);
    let len = (pos - start) as usize;
    let total = send(s, sci::GETLENGTH, 0, 0);
    if len < 3 || start < 0 || total > 2_000_000 {
        return;
    }
    let mut buf = vec![0u8; total as usize + 1];
    send(s, sci::GETTEXT, buf.len(), buf.as_mut_ptr() as isize);
    buf.truncate(total as usize);
    let text = String::from_utf8_lossy(&buf).into_owned();
    let Some(prefix) = text.get(start as usize..pos as usize) else { return };
    let words = words_with_prefix(&text, prefix, start as usize);
    if words.is_empty() {
        return;
    }
    let mut list = words.join(" ").into_bytes();
    list.push(0);
    send(s, AUTOCSETIGNORECASE, 1, 0);
    send(s, AUTOCSETMAXHEIGHT, 8, 0);
    send(s, AUTOCSHOW, len, list.as_ptr() as isize);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn words() {
        let t = "foo_bar fooBaz foo foobar x foo_bar";
        assert_eq!(words_with_prefix(t, "foo", 99), vec!["foo_bar", "foobar", "fooBaz"]);
        assert_eq!(words_with_prefix(t, "foo", 0), vec!["foo_bar", "foobar", "fooBaz"]);
        assert_eq!(words_with_prefix("été", "ét", 99), vec!["été"]);
    }
}
