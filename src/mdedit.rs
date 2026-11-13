//! Commandes de mise en forme Markdown (gras, italique, titres, listes, lien, tableau...), partagées Windows / Linux / macOS.
use crate::sci::{self, send, HWND};

// ---------- logique pure (testée) ----------

/// Retire un préfixe de liste / titre / citation existant ; renvoie (indentation, reste).
fn split_indent(l: &str) -> (&str, &str) {
    let t = l.trim_start();
    (&l[..l.len() - t.len()], t)
}

/// Longueur du marqueur de liste en tête de `t` (« - », « 1. », « - [ ] »...), 0 si aucun.
pub fn list_marker_len(t: &str) -> usize {
    let b = t.as_bytes();
    let mut n = if matches!(b.first(), Some(b'-' | b'*' | b'+')) && b.get(1) == Some(&b' ') {
        2
    } else {
        let d = b.iter().take_while(|c| c.is_ascii_digit()).count();
        if d > 0 && matches!(b.get(d), Some(b'.' | b')')) && b.get(d + 1) == Some(&b' ') { d + 2 } else { 0 }
    };
    if n > 0 && ["[ ] ", "[x] ", "[X] "].iter().any(|p| t[n..].starts_with(p)) {
        n += 4;
    }
    n
}

/// Ajoute (ou retire si toutes les lignes non vides l'ont déjà) un préfixe de ligne.
pub fn toggle_prefix(lines: Vec<String>, prefix: &str, numbered: bool) -> Vec<String> {
    let has = |l: &String| {
        let t = split_indent(l).1;
        if numbered { t.bytes().take_while(|c| c.is_ascii_digit()).count() > 0 && list_marker_len(t) > 0 } else { t.starts_with(prefix.trim_end()) && t[prefix.trim_end().len()..].starts_with(' ') || t == prefix.trim_end() }
    };
    let all = lines.iter().filter(|l| !l.trim().is_empty()).all(has) && lines.iter().any(|l| !l.trim().is_empty());
    let mut n = 0;
    lines
        .into_iter()
        .map(|l| {
            if l.trim().is_empty() {
                return l;
            }
            let (ind, t) = split_indent(&l);
            if all {
                let m = if numbered { list_marker_len(t) } else { prefix.trim_end().len() + t[prefix.trim_end().len()..].starts_with(' ') as usize };
                format!("{ind}{}", &t[m..])
            } else {
                n += 1;
                let rest = &t[list_marker_len(t).max(0)..];
                let rest = if numbered || prefix.starts_with('-') { rest } else { t };
                if numbered { format!("{ind}{n}. {rest}") } else { format!("{ind}{prefix}{rest}") }
            }
        })
        .collect()
}

/// Titre de niveau `n` (1..6) ; même niveau = retire le titre.
pub fn set_heading(line: &str, n: usize) -> String {
    let (ind, t) = split_indent(line);
    let h = t.bytes().take_while(|&c| c == b'#').count();
    let body = if h > 0 && t.as_bytes().get(h) == Some(&b' ') { &t[h + 1..] } else { t };
    if h == n && body.len() != t.len() { format!("{ind}{body}") } else { format!("{ind}{} {body}", "#".repeat(n)) }
}

/// Ligne à insérer après Entrée sur une ligne de liste `prev` (None = pas de liste ; Some("") = fin de liste, effacer le marqueur).
pub fn continue_list(prev: &str) -> Option<String> {
    let (ind, t) = split_indent(prev.trim_end_matches('\r'));
    let m = list_marker_len(t);
    if m == 0 {
        return None;
    }
    if t[m..].trim().is_empty() {
        return Some(String::new());
    }
    let mark = &t[..m];
    let digits = mark.bytes().take_while(|c| c.is_ascii_digit()).count();
    let next = if digits > 0 {
        format!("{}{}", mark[..digits].parse::<u64>().unwrap_or(0) + 1, &mark[digits..])
    } else {
        mark.replace("[x] ", "[ ] ").replace("[X] ", "[ ] ")
    };
    Some(format!("{ind}{next}"))
}

// ---------- glue Scintilla ----------

fn range_text(s: HWND, a: isize, b: isize) -> String {
    let mut buf = vec![0u8; (b - a).max(0) as usize + 1];
    let mut tr: [isize; 3] = [a, b, buf.as_mut_ptr() as isize];
    send(s, 2039, 0, tr.as_mut_ptr() as isize); // SCI_GETTEXTRANGEFULL
    buf.truncate((b - a).max(0) as usize);
    String::from_utf8_lossy(&buf).into_owned()
}

fn replace(s: HWND, a: isize, b: isize, text: &str) {
    send(s, sci::SETTARGETRANGE, a as usize, b);
    send(s, sci::REPLACETARGET, text.len(), text.as_ptr() as isize);
}

/// Entoure la sélection de `open`/`close` (les retire si déjà présents) ; sans sélection, place le curseur entre les deux.
pub fn wrap(s: HWND, open: &str, close: &str) {
    let (a, b) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    let (ol, cl) = (open.len() as isize, close.len() as isize);
    send(s, sci::BEGINUNDO, 0, 0);
    if a != b && a >= ol && range_text(s, a - ol, a) == open && range_text(s, b, b + cl) == close {
        replace(s, b, b + cl, "");
        replace(s, a - ol, a, "");
        send(s, sci::SETSEL, (a - ol) as usize, b - ol);
    } else {
        let t = range_text(s, a, b);
        if t.starts_with(open) && t.ends_with(close) && t.len() as isize >= ol + cl && a != b {
            let inner = t[open.len()..t.len() - close.len()].to_string();
            replace(s, a, b, &inner);
            send(s, sci::SETSEL, a as usize, a + inner.len() as isize);
        } else {
            replace(s, a, b, &format!("{open}{t}{close}"));
            send(s, sci::SETSEL, (a + ol) as usize, a + ol + t.len() as isize);
        }
    }
    send(s, sci::ENDUNDO, 0, 0);
}

/// Applique `f` aux lignes touchées par la sélection (la ligne du curseur si pas de sélection).
pub fn edit_lines(s: HWND, f: impl FnOnce(Vec<String>) -> Vec<String>) {
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    let l0 = send(s, sci::LINEFROMPOS, ss as usize, 0);
    let mut l1 = send(s, sci::LINEFROMPOS, se as usize, 0);
    if se > ss && l1 > l0 && send(s, sci::POSFROMLINE, l1 as usize, 0) == se {
        l1 -= 1;
    }
    let (p0, p1) = (send(s, sci::POSFROMLINE, l0 as usize, 0), send(s, sci::GETLINEENDPOS, l1 as usize, 0));
    let eol = match send(s, sci::GETEOLMODE, 0, 0) { 0 => "\r\n", 1 => "\r", _ => "\n" };
    let lines: Vec<String> = range_text(s, p0, p1).split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
    let out = f(lines).join(eol);
    send(s, sci::BEGINUNDO, 0, 0);
    replace(s, p0, p1, &out);
    send(s, sci::SETSEL, p0 as usize, p0 + out.len() as isize);
    send(s, sci::ENDUNDO, 0, 0);
}

pub fn heading(s: HWND, n: usize) {
    edit_lines(s, |ls| ls.into_iter().map(|l| set_heading(&l, n)).collect());
}

pub fn insert(s: HWND, text: &str, caret_back: usize) {
    let a = send(s, sci::GETSELSTART, 0, 0);
    send(s, sci::BEGINUNDO, 0, 0);
    replace(s, a, send(s, sci::GETSELEND, 0, 0), text);
    let p = a + text.len() as isize - caret_back as isize;
    send(s, sci::SETSEL, p as usize, p);
    send(s, sci::ENDUNDO, 0, 0);
}

pub fn link(s: HWND) {
    let (a, b) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    let t = range_text(s, a, b);
    let url = if t.starts_with("http://") || t.starts_with("https://") { t.clone() } else { String::from("adresse") };
    let label = if url == t { String::new() } else { t };
    send(s, sci::BEGINUNDO, 0, 0);
    replace(s, a, b, &format!("[{label}]({url})"));
    let u0 = a + 1 + label.len() as isize + 2;
    send(s, sci::SETSEL, if label.is_empty() { (a + 1) as usize } else { u0 as usize }, if label.is_empty() { a + 1 } else { u0 + url.len() as isize });
    send(s, sci::ENDUNDO, 0, 0);
}

pub fn code_block(s: HWND) {
    let (a, b) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    let t = range_text(s, a, b);
    send(s, sci::BEGINUNDO, 0, 0);
    replace(s, a, b, &format!("```\n{t}\n```"));
    send(s, sci::SETSEL, (a + 4) as usize, a + 4 + t.len() as isize);
    send(s, sci::ENDUNDO, 0, 0);
}

pub fn table(s: HWND) {
    insert(s, "| Colonne 1 | Colonne 2 |\n| --- | --- |\n|  |  |\n", 0);
}

pub fn callout(s: HWND) {
    edit_lines(s, |ls| {
        let mut v = vec!["> [!note] Titre".to_string()];
        v.extend(ls.into_iter().map(|l| format!("> {l}")));
        v
    });
}

/// Après Entrée dans un document Markdown : continue (ou termine) la liste de la ligne précédente. À appeler après l'indentation auto.
pub fn on_enter(s: HWND) {
    let pos = send(s, sci::GETCURRENTPOS, 0, 0);
    let ln = send(s, sci::LINEFROMPOS, pos as usize, 0);
    if ln == 0 {
        return;
    }
    let (p0, p1) = (send(s, sci::POSFROMLINE, (ln - 1) as usize, 0), send(s, sci::GETLINEENDPOS, (ln - 1) as usize, 0));
    let prev = range_text(s, p0, p1);
    let Some(next) = continue_list(&prev) else { return };
    let (c0, c1) = (send(s, sci::POSFROMLINE, ln as usize, 0), send(s, sci::GETLINEENDPOS, ln as usize, 0));
    send(s, sci::BEGINUNDO, 0, 0);
    if next.is_empty() {
        // ligne précédente = marqueur seul : on la vide (fin de liste) et on repart sur une ligne vide
        replace(s, c0, c1, "");
        replace(s, p0, p1, "");
        let np = send(s, sci::POSFROMLINE, ln as usize, 0);
        send(s, sci::SETSEL, np as usize, np);
    } else {
        replace(s, c0, c1, &next);
        let np = c0 + next.len() as isize;
        send(s, sci::SETSEL, np as usize, np);
    }
    send(s, sci::ENDUNDO, 0, 0);
}

/// Exécute la commande Markdown `id` (ids MD_*) sur le contrôle `s`.
pub fn run(s: HWND, id: usize) {
    use crate::ids::*;
    match id {
        MD_BOLD => wrap(s, "**", "**"),
        MD_ITALIC => wrap(s, "*", "*"),
        MD_STRIKE => wrap(s, "~~", "~~"),
        MD_HILITE => wrap(s, "==", "=="),
        MD_CODE => wrap(s, "`", "`"),
        MD_QUOTE => edit_lines(s, |l| toggle_prefix(l, "> ", false)),
        MD_BULLET => edit_lines(s, |l| toggle_prefix(l, "- ", false)),
        MD_NUMLIST => edit_lines(s, |l| toggle_prefix(l, "1. ", true)),
        MD_TASK => edit_lines(s, |l| toggle_prefix(l, "- [ ] ", false)),
        MD_LINK => link(s),
        MD_WIKI => wrap(s, "[[", "]]"),
        MD_TABLE => table(s),
        MD_CODEBLOCK => code_block(s),
        MD_HR => insert(s, "\n---\n", 0),
        MD_CALLOUT => callout(s),
        x if (MD_H1..MD_H1 + 6).contains(&x) => heading(s, x - MD_H1 + 1),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn prefixes() {
        assert_eq!(toggle_prefix(v(&["a", "", "b"]), "- ", false), v(&["- a", "", "- b"]));
        assert_eq!(toggle_prefix(v(&["- a", "- b"]), "- ", false), v(&["a", "b"]));
        assert_eq!(toggle_prefix(v(&["a", "b"]), "1. ", true), v(&["1. a", "2. b"]));
        assert_eq!(toggle_prefix(v(&["1. a", "2. b"]), "1. ", true), v(&["a", "b"]));
        assert_eq!(toggle_prefix(v(&["x"]), "> ", false), v(&["> x"]));
        assert_eq!(toggle_prefix(v(&["> x"]), "> ", false), v(&["x"]));
        assert_eq!(toggle_prefix(v(&["- a"]), "- [ ] ", false), v(&["- [ ] a"]));
        assert_eq!(toggle_prefix(v(&["  a"]), "- ", false), v(&["  - a"]));
    }
    #[test]
    fn headings() {
        assert_eq!(set_heading("titre", 2), "## titre");
        assert_eq!(set_heading("# titre", 3), "### titre");
        assert_eq!(set_heading("## titre", 2), "titre");
    }
    #[test]
    fn lists() {
        assert_eq!(continue_list("- a"), Some("- ".into()));
        assert_eq!(continue_list("  3. a"), Some("  4. ".into()));
        assert_eq!(continue_list("- [x] fait"), Some("- [ ] ".into()));
        assert_eq!(continue_list("- "), Some(String::new()));
        assert_eq!(continue_list("texte"), None);
    }
}
