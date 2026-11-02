//! Opérations de texte : lignes, casse, commentaires, signets, repli, liens.
use crate::cmds::{msg_box, open_or_create};
use crate::lang::LANGS;
use crate::sci::{self, send};
use crate::{app, w};

fn eol_str(s: windows_sys::Win32::Foundation::HWND) -> &'static str {
    match send(s, sci::GETEOLMODE, 0, 0) { 0 => "\r\n", 1 => "\r", _ => "\n" }
}

fn range_text(s: windows_sys::Win32::Foundation::HWND, a: isize, b: isize) -> String {
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

fn open_url(u: &str) {
    use windows_sys::Win32::UI::{Shell::ShellExecuteW, WindowsAndMessaging::SW_SHOWNORMAL};
    unsafe { ShellExecuteW(std::ptr::null_mut(), w("open").as_ptr(), w(u).as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL) };
}

/// Ouvre le lien [[note]], l'URL ou le chemin sous le curseur.
pub fn open_link() {
    let a = app();
    let s = a.sci;
    let pos = send(s, sci::GETCURRENTPOS, 0, 0);
    let ln = send(s, sci::LINEFROMPOS, pos as usize, 0) as usize;
    let (p0, p1) = (send(s, sci::POSFROMLINE, ln, 0), send(s, sci::GETLINEENDPOS, ln, 0));
    let line = range_text(s, p0, p1);
    let col = (pos - p0) as usize;
    let find = |open: &str, close: &str| -> Option<String> {
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
        let base = a.docs[a.cur].path.as_ref().and_then(|p| std::path::Path::new(p).parent().map(|d| d.to_path_buf())).or_else(|| a.root.as_ref().map(std::path::PathBuf::from));
        let mut file = base.unwrap_or_default().join(&name);
        if file.extension().is_none() {
            file.set_extension("md");
        }
        open_or_create(&file.to_string_lossy());
        return;
    }
    let start = line[..col.min(line.len())].rfind(|c: char| c.is_whitespace() || "()<>\"'".contains(c)).map_or(0, |i| i + 1);
    let end = line[col.min(line.len())..].find(|c: char| c.is_whitespace() || "()<>\"'".contains(c)).map_or(line.len(), |i| i + col);
    let tok = &line[start..end.max(start)];
    if tok.starts_with("http://") || tok.starts_with("https://") || tok.starts_with("mailto:") {
        open_url(tok);
    } else if let Some(m) = find("](", ")") {
        open_url(&m);
    } else if std::path::Path::new(tok).exists() {
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
