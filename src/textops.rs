//! Opérations de texte : lignes, casse, commentaires, signets, repli, liens.
use crate::cmds::{msg_box, open_or_create};
use crate::lang::LANGS;
use crate::sci::{self, send};
use crate::app;

fn eol_str(s: crate::sci::HWND) -> &'static str {
    match send(s, sci::GETEOLMODE, 0, 0) { 0 => "\r\n", 1 => "\r", _ => "\n" }
}

pub(crate) fn range_text(s: crate::sci::HWND, a: isize, b: isize) -> String {
    let mut buf = vec![0u8; (b - a) as usize + 1];
    // SCI_GETTEXTRANGEFULL (2039) avec Sci_TextRangeFull { Sci_CharacterRangeFull chrg{a,b}; char* lpstrText }
    let mut tr: [isize; 3] = [a, b, buf.as_mut_ptr() as isize];
    send(s, 2039, 0, tr.as_mut_ptr() as isize);
    buf.truncate((b - a) as usize);
    String::from_utf8_lossy(&buf).into_owned()
}

/// Applique f aux lignes de la sélection (document entier si pas de sélection).
pub fn transform_lines(f: impl FnOnce(Vec<String>) -> Vec<String>) {
    let a = app();
    let s = a.sci;
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    let n = send(s, sci::GETLINECOUNT, 0, 0);
    let (l0, mut l1) = if ss == se { (0, n - 1) } else { (send(s, sci::LINEFROMPOS, ss as usize, 0), send(s, sci::LINEFROMPOS, se as usize, 0)) };
    if ss != se && l1 > l0 && send(s, sci::POSFROMLINE, l1 as usize, 0) == se {
        l1 -= 1;
    }
    let (p0, p1) = (send(s, sci::POSFROMLINE, l0 as usize, 0), send(s, sci::GETLINEENDPOS, l1 as usize, 0));
    let text = range_text(s, p0, p1);
    let lines: Vec<String> = text.split('\n').map(|l| l.trim_end_matches('\r').to_string()).collect();
    let out = f(lines).join(eol_str(s));
    send(s, sci::BEGINUNDO, 0, 0);
    send(s, sci::SETTARGETRANGE, p0 as usize, p1);
    send(s, sci::REPLACETARGET, out.len(), out.as_ptr() as isize);
    send(s, sci::ENDUNDO, 0, 0);
}

pub fn sort_lines(desc: bool) {
    transform_lines(|mut v| {
        v.sort_by_key(|l| l.to_lowercase());
        if desc {
            v.reverse();
        }
        v
    });
}

/// Opérations de lignes supplémentaires (tris par longueur / numérique, inverser, mélanger, lignes vides...).
pub fn apply_line_op(kind: usize, mut v: Vec<String>) -> Vec<String> {
    let num = |l: &String| l.trim().split(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).next().and_then(|x| x.parse::<f64>().ok()).unwrap_or(f64::MAX);
    match kind {
        0 => v.sort_by_key(|l| l.chars().count()),
        1 => v.sort_by(|a, b| num(a).total_cmp(&num(b))),
        2 => v.reverse(),
        3 => {
            let mut x = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(7, |d| d.as_nanos() as u64) | 1;
            for i in (1..v.len()).rev() {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                v.swap(i, (x % (i as u64 + 1)) as usize);
            }
        }
        4 => v.retain(|l| !l.trim().is_empty()),
        5 => v = v.into_iter().map(|l| l.trim_start().to_string()).collect(),
        _ => v.dedup(),
    }
    v
}

pub fn line_op(kind: usize) {
    transform_lines(|v| apply_line_op(kind, v));
}

pub fn dedupe() {
    transform_lines(|v| {
        let mut seen = std::collections::HashSet::new();
        v.into_iter().filter(|l| seen.insert(l.clone())).collect()
    });
}

pub fn trim() {
    transform_lines(|v| v.into_iter().map(|l| l.trim_end().to_string()).collect());
}

pub fn join() {
    transform_lines(|v| vec![v.iter().map(|l| l.trim()).filter(|l| !l.is_empty()).collect::<Vec<_>>().join(" ")]);
}

pub fn tabs_to_spaces() {
    transform_lines(|v| v.into_iter().map(|l| l.replace('\t', "    ")).collect());
}

pub fn spaces_to_tabs() {
    transform_lines(|v| {
        v.into_iter()
            .map(|l| {
                let lead = l.len() - l.trim_start_matches(' ').len();
                format!("{}{}{}", "\t".repeat(lead / 4), " ".repeat(lead % 4), &l[lead..])
            })
            .collect()
    });
}

pub fn title_case() {
    let s = app().sci;
    let (a, b) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    if b <= a {
        return;
    }
    let t = range_text(s, a, b);
    let mut up = true;
    let out: String = t
        .chars()
        .map(|c| {
            let r = if up { c.to_uppercase().next().unwrap_or(c) } else { c.to_lowercase().next().unwrap_or(c) };
            up = !c.is_alphanumeric() && c != '\'';
            r
        })
        .collect();
    send(s, sci::REPLACESEL, 0, crate::cstr(&out).as_ptr() as isize);
}

fn comment_prefix(lexer: &str) -> Option<&'static str> {
    Some(match lexer {
        "cpp" | "rust" | "json" => "//",
        "python" | "bash" | "yaml" | "toml" | "props" | "powershell" | "ruby" | "perl" | "makefile" | "cmake" => "#",
        "sql" | "lua" => "--",
        "batch" => "REM",
        _ => return None,
    })
}

pub fn toggle_comment() {
    let a = app();
    let Some(p) = comment_prefix(LANGS[a.docs[a.cur].lang].lexer) else {
        msg_box("Ce langage n'a pas de commentaire de ligne.");
        return;
    };
    transform_lines(|v| {
        let all = v.iter().filter(|l| !l.trim().is_empty()).all(|l| l.trim_start().starts_with(p));
        v.into_iter()
            .map(|l| {
                if l.trim().is_empty() {
                    return l;
                }
                let lead = l.len() - l.trim_start().len();
                if all {
                    let rest = &l[lead + p.len()..];
                    format!("{}{}", &l[..lead], rest.strip_prefix(' ').unwrap_or(rest))
                } else {
                    format!("{}{} {}", &l[..lead], p, &l[lead..])
                }
            })
            .collect()
    });
}

/// Entoure la sélection (ou la ligne) d'un commentaire de bloc, ou l'enlève s'il y est déjà.
pub fn block_comment() {
    let a = app();
    let s = a.sci;
    let (o, c) = match LANGS[a.docs[a.cur].lang].lexer {
        "cpp" | "rust" | "css" | "sql" => ("/*", "*/"),
        "hypertext" | "xml" | "markdown" => ("<!--", "-->"),
        _ => {
            msg_box("Ce langage n'a pas de commentaire de bloc.");
            return;
        }
    };
    let (mut b, mut e) = (send(s, 2143, 0, 0), send(s, 2145, 0, 0));
    if b == e {
        let ln = send(s, sci::LINEFROMPOS, b as usize, 0) as usize;
        b = send(s, 2167, ln, 0); // début de ligne
        e = send(s, 2136, ln, 0); // fin de ligne
        send(s, sci::SETSEL, b as usize, e);
    }
    let mut buf = vec![0u8; (e - b) as usize + 2];
    send(s, sci::GETSELTEXT, 0, buf.as_mut_ptr() as isize);
    buf.truncate((e - b) as usize);
    let t = String::from_utf8_lossy(&buf).to_string();
    let out = match t.trim().strip_prefix(o).and_then(|r| r.strip_suffix(c)) {
        Some(inner) => inner.trim_matches(' ').to_string(),
        None => format!("{o} {t} {c}"),
    };
    send(s, sci::REPLACESEL, 0, crate::cstr(&out).as_ptr() as isize);
}

/// Ajoute la prochaine occurrence du texte sélectionné à la sélection (multi-curseurs) ; sans sélection, sélectionne le mot.
pub fn select_next() {
    let s = app().sci;
    let main = send(s, 2575, 0, 0) as usize; // SCI_GETMAINSELECTION
    let (ss, se) = (send(s, 2585, main, 0), send(s, 2587, main, 0));
    if ss == se {
        let p = send(s, sci::GETCURRENTPOS, 0, 0) as usize;
        let (ws, we) = (send(s, sci::WORDSTARTPOS, p, 1), send(s, sci::WORDENDPOS, p, 1));
        send(s, sci::SETSEL, ws as usize, we);
        return;
    }
    let what = range_text(s, ss, se).into_bytes();
    let len = send(s, sci::GETLENGTH, 0, 0);
    let find = |a: isize, b: isize| {
        send(s, sci::SETSEARCHFLAGS, 4, 0);
        send(s, sci::SETTARGETSTART, a as usize, 0);
        send(s, sci::SETTARGETEND, b as usize, 0);
        send(s, sci::SEARCHINTARGET, what.len(), what.as_ptr() as isize)
    };
    let mut r = find(se, len);
    if r < 0 {
        r = find(0, ss);
    }
    if r >= 0 {
        let (ts, te) = (send(s, sci::GETTARGETSTART, 0, 0), send(s, sci::GETTARGETEND, 0, 0));
        if (0..send(s, 2570, 0, 0)).any(|i| send(s, 2585, i as usize, 0) == ts) {
            return; // déjà sélectionnée : on a fait le tour
        }
        send(s, 2573, te as usize, ts); // SCI_ADDSELECTION(caret, anchor)
    }
}

pub fn bookmark_toggle() {
    let s = app().sci;
    let ln = send(s, sci::LINEFROMPOS, send(s, sci::GETCURRENTPOS, 0, 0) as usize, 0) as usize;
    if send(s, sci::MARKERGET, ln, 0) & 2 != 0 {
        send(s, sci::MARKERDELETE, ln, 1);
    } else {
        send(s, sci::MARKERADD, ln, 1);
    }
}

pub fn bookmark_step(next: bool) {
    let s = app().sci;
    let ln = send(s, sci::LINEFROMPOS, send(s, sci::GETCURRENTPOS, 0, 0) as usize, 0);
    let n = send(s, sci::GETLINECOUNT, 0, 0);
    let (m, wrap) = if next { (sci::MARKERNEXT, 0) } else { (sci::MARKERPREV, n - 1) };
    let mut r = send(s, m, (if next { ln + 1 } else { ln - 1 }).max(0) as usize, 2);
    if r < 0 {
        r = send(s, m, wrap as usize, 2);
    }
    if r >= 0 {
        send(s, sci::GOTOLINE, r as usize, 0);
    }
}

pub fn stats() {
    let a = app();
    let s = a.sci;
    let len = send(s, sci::GETLENGTH, 0, 0);
    let text = range_text(s, 0, len);
    let words = text.split_whitespace().count();
    let msg = format!(
        "Lignes : {}\nMots : {}\nCaractères : {}\nOctets (UTF-8) : {}\nTemps de lecture : environ {} min",
        send(s, sci::GETLINECOUNT, 0, 0), words, text.chars().count(), len, (words / 220).max(1)
    );
    msg_box(&msg);
}

#[cfg(windows)]
fn open_url(u: &str) {
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    let w = |s: &str| -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() };
    unsafe { ShellExecuteW(std::ptr::null_mut(), w("open").as_ptr(), w(u).as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
}

#[cfg(unix)]
pub fn open_url(u: &str) {
    let _ = std::process::Command::new(if cfg!(target_os = "macos") { "open" } else { "xdg-open" }).arg(u).spawn();
}

/// Seuls http, https et mailto partent au système : tout autre texte (chemin d'exécutable, file:, \\serveur\partage, ms-*:) serait lancé tel quel.
pub fn web_url(u: &str) -> bool {
    let l = u.trim().to_ascii_lowercase();
    (l.starts_with("http://") || l.starts_with("https://") || l.starts_with("mailto:")) && !l.chars().any(char::is_control)
}

/// Fichier visé par un lien relatif : toujours sous `base` (ni absolu, ni lecteur, ni UNC, ni « .. », ni flux NTFS « : »).
pub fn local_target(base: &std::path::Path, name: &str) -> Option<std::path::PathBuf> {
    let name = name.split('#').next().unwrap_or("").trim();
    if name.is_empty() || name.contains(':') || name.starts_with(['/', '\\']) || name.chars().any(char::is_control) {
        return None;
    }
    if name.split(['/', '\\']).any(|c| c == "..") {
        return None;
    }
    Some(base.join(name))
}

/// « Ouvrir dans le navigateur » : seulement les types qu'un navigateur affiche (jamais un .exe, .bat, .lnk...).
pub fn browser_safe(path: &str) -> bool {
    let ext = std::path::Path::new(path).extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    matches!(ext.as_str(), "html" | "htm" | "xhtml" | "mhtml" | "svg" | "pdf" | "txt" | "md" | "markdown")
}

/// Plus grand indice <= `i` tombant sur une frontière de caractère (un décalage d'octets venu de Scintilla peut couper un caractère remplacé).
fn floor_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// Ouvre le lien [[note]], l'URL ou le chemin sous le curseur.
pub fn open_link() {
    let a = app();
    let s = a.sci;
    let pos = send(s, sci::GETCURRENTPOS, 0, 0);
    let ln = send(s, sci::LINEFROMPOS, pos as usize, 0) as usize;
    let (p0, p1) = (send(s, sci::POSFROMLINE, ln, 0), send(s, sci::GETLINEENDPOS, ln, 0));
    let line = range_text(s, p0, p1);
    let col = floor_boundary(&line, (pos - p0).max(0) as usize);
    let find =|open: &str, close: &str| -> Option<String> {
        let mut from = 0;
        while let Some(i) = line[from..].find(open) {
            let st = from + i;
            let end = line[st + open.len()..].find(close)? + st + open.len();
            if col >= st && col <= end + close.len() {
                return Some(line[st + open.len()..end].to_string());
            }
            from = st + open.len();
        }
        None
    };
    if let Some(n) = find("[[", "]]") {
        let name = n.split('|').next().unwrap_or(&n).split('#').next().unwrap_or("").trim().to_string();
        if name.is_empty() {
            // [[#Titre]] : saut vers un titre du document courant
            let head = n.split('|').next().unwrap_or(&n).trim_start_matches('#');
            if !crate::textx::goto_heading(head) {
                msg_box("Titre introuvable dans ce document.");
            }
            return;
        }
        let base = a.docs[a.cur].path.as_ref().and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf())).or_else(|| a.root.as_ref().map(std::path::PathBuf::from));
        let Some(mut file) = base.and_then(|b| local_target(&b, &name)) else {
            msg_box("Lien refusé : une note doit rester dans le dossier du document (pas de chemin absolu, de « .. » ni de réseau).");
            return;
        };
        if file.extension().is_none() {
            file.set_extension("md");
        }
        open_or_create(&file.to_string_lossy());
        return;
    }
    let start = line[..col.min(line.len())].rfind(|c: char| c.is_whitespace() || "()<>\"'".contains(c)).map_or(0, |i| i + 1);
    let end = line[col.min(line.len())..].find(|c: char| c.is_whitespace() || "()<>\"'".contains(c)).map_or(line.len(), |i| i + col);
    let tok = &line[start..end.max(start)];
    if web_url(tok) {
        open_url(tok);
    } else if let Some(m) = find("](", ")") {
        let m = m.trim();
        let doc_dir = a.docs[a.cur].path.as_ref().and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf()));
        if web_url(m) {
            open_url(m);
        } else if let Some(f) = doc_dir.and_then(|d| local_target(&d, m)).filter(|f| f.is_file()) {
            crate::cmds::open_path(&f.to_string_lossy()); // ouvert dans WinNote, jamais exécuté
        } else {
            msg_box("Lien refusé : seuls http, https, mailto et les fichiers du dossier du document sont ouverts.");
        }
    } else if !tok.starts_with("\\\\") && std::path::Path::new(tok).is_file() {
        crate::cmds::open_path(tok);
    } else {
        msg_box("Aucun lien sous le curseur.");
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn prefixes() {
        assert_eq!(super::comment_prefix("python"), Some("#"));
        assert_eq!(super::comment_prefix("hypertext"), None);
    }
}

#[cfg(test)]
mod line_tests {
    use super::apply_line_op as op;
    fn v(x: &[&str]) -> Vec<String> { x.iter().map(|s| s.to_string()).collect() }
    #[test]
    fn line_ops() {
        assert_eq!(op(0, v(&["ccc", "a", "bb"])), v(&["a", "bb", "ccc"]));
        assert_eq!(op(1, v(&["10 b", "9 a", "100"])), v(&["9 a", "10 b", "100"]));
        assert_eq!(op(2, v(&["1", "2"])), v(&["2", "1"]));
        assert_eq!(op(4, v(&["a", "", "  ", "b"])), v(&["a", "b"]));
        assert_eq!(op(5, v(&["  a", " b"])), v(&["a", "b"]));
        assert_eq!(op(6, v(&["a", "a", "b", "a"])), v(&["a", "b", "a"]));
        let mut s = op(3, v(&["1", "2", "3", "4", "5"]));
        s.sort();
        assert_eq!(s, v(&["1", "2", "3", "4", "5"]));
    }
}

/// Texte à insérer sur la ligne `i` (0 = première) : `{n}` = compteur (départ 1), `{n:départ,pas}` = compteur réglé.
pub fn col_text(spec: &str, i: usize) -> String {
    let (mut out, mut rest) = (String::new(), spec);
    while let Some(p) = rest.find("{n") {
        out.push_str(&rest[..p]);
        let after = &rest[p + 2..];
        let Some(e) = after.find('}') else { out.push_str(&rest[p..]); return out };
        let args = after[..e].strip_prefix(':').unwrap_or("");
        let mut it = args.split(',').map(|x| x.trim().parse::<i64>());
        let (start, step) = (it.next().and_then(Result::ok).unwrap_or(1), it.next().and_then(Result::ok).unwrap_or(1));
        out.push_str(&(start + step * i as i64).to_string());
        rest = &after[e + 1..];
    }
    out.push_str(rest);
    out
}

/// Insère `spec` à la colonne du début de la sélection, sur chaque ligne sélectionnée (lignes courtes complétées par des espaces).
pub fn column_insert(spec: &str) {
    let s = app().sci;
    let (ss, se) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    let col = send(s, 2129, ss as usize, 0); // GETCOLUMN
    let l0 = send(s, sci::LINEFROMPOS, ss as usize, 0);
    let mut l1 = send(s, sci::LINEFROMPOS, se as usize, 0);
    if se > ss && l1 > l0 && send(s, sci::POSFROMLINE, l1 as usize, 0) == se {
        l1 -= 1;
    }
    send(s, sci::BEGINUNDO, 0, 0);
    for (i, l) in (l0..=l1).enumerate().collect::<Vec<_>>().into_iter().rev() {
        let end = send(s, sci::GETLINEENDPOS, l as usize, 0);
        let have = send(s, 2129, end as usize, 0);
        let mut t = " ".repeat((col - have).max(0) as usize);
        t.push_str(&col_text(spec, i));
        let pos = if have >= col { send(s, 2456, l as usize, col) } else { end }; // FINDCOLUMN
        let mut z = t.into_bytes();
        z.push(0);
        send(s, 2003, pos as usize, z.as_ptr() as isize); // INSERTTEXT
    }
    send(s, sci::ENDUNDO, 0, 0);
}

#[cfg(test)]
mod col_tests {
    use super::col_text;
    #[test]
    fn col() {
        assert_eq!(col_text("x", 3), "x");
        assert_eq!(col_text("a{n}b", 2), "a3b");
        assert_eq!(col_text("{n:10,5}", 2), "20");
        assert_eq!(col_text("{n", 0), "{n");
    }
}

#[cfg(test)]
mod link_tests {
    use super::*;
    use std::path::Path;
    #[test]
    fn web_only() {
        for u in ["http://a.b", "HTTPS://a.b/x?y=1", "mailto:a@b.c"] {
            assert!(web_url(u), "{u}");
        }
        for u in ["C:\\Windows\\System32\\calc.exe", "calc.exe", "file:///C:/x.exe", "\\\\srv\\share\\a.exe", "ms-msdt:/id", "javascript:alert(1)", "https://a\r\nb", "-oProxy"] {
            assert!(!web_url(u), "{u}");
        }
    }
    #[test]
    fn local_links_stay_inside() {
        let b = Path::new("notes");
        assert_eq!(local_target(b, "a.md#t"), Some(b.join("a.md")));
        assert_eq!(local_target(b, "sub/b"), Some(b.join("sub/b")));
        for n in ["..\\x", "a/../../x", "/etc/passwd", "\\\\srv\\s\\f", "C:\\x", "a.md:stream", "", "#t", "a\0b"] {
            assert_eq!(local_target(b, n), None, "{n:?}");
        }
    }
    #[test]
    fn browser_types() {
        assert!(browser_safe("a.HTML") && browser_safe("x.pdf") && browser_safe("n.md"));
        assert!(!browser_safe("a.exe") && !browser_safe("a.bat") && !browser_safe("a.lnk") && !browser_safe("noext"));
    }
    #[test]
    fn boundary() {
        assert_eq!(floor_boundary("aéb", 2), 1);
        assert_eq!(floor_boundary("abc", 99), 3);
    }
}
