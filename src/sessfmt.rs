//! Format d'un onglet dans `session.txt` (partagé Windows + Linux). Lit aussi les anciens formats : Windows (8 champs) et Linux (6 champs).
//! doc \t chemin \t encodage \t curseur \t première ligne visible \t modifié \t n° sans titre \t copie \t épinglé \t langage (nom)

#[derive(Debug, Default, PartialEq)]
pub struct Rec {
    pub path: String,
    pub enc: usize,
    pub caret: isize,
    pub first: isize,
    pub dirty: bool,
    pub untitled: u32,
    pub backup: String,
    pub pinned: bool,
    /// Nom du langage choisi à la main ; vide = détecté par l'extension
    pub lang: String,
}

fn clean(s: &str) -> String {
    s.replace(['\t', '\n', '\r'], " ")
}

pub fn format(r: &Rec) -> String {
    format!(
        "doc\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        clean(&r.path), r.enc, r.caret, r.first, r.dirty as u8, r.untitled, clean(&r.backup), r.pinned as u8, clean(&r.lang)
    )
}

pub fn parse(line: &str) -> Option<Rec> {
    let f: Vec<&str> = line.split('\t').collect();
    let n = |s: &str| s.parse().unwrap_or(0);
    Some(match f.as_slice() {
        ["doc", path, enc, caret, first, dirty, untitled, backup, pinned, lang] => Rec { path: path.to_string(), enc: n(enc) as usize, caret: n(caret), first: n(first), dirty: *dirty == "1", untitled: n(untitled) as u32, backup: backup.to_string(), pinned: *pinned == "1", lang: lang.to_string() },
        ["doc", path, enc, caret, first, dirty, untitled, backup] => Rec { path: path.to_string(), enc: n(enc) as usize, caret: n(caret), first: n(first), dirty: *dirty == "1", untitled: n(untitled) as u32, backup: backup.to_string(), ..Rec::default() },
        ["doc", path, enc, dirty, untitled, backup] => Rec { path: path.to_string(), enc: n(enc) as usize, dirty: *dirty == "1", untitled: n(untitled) as u32, backup: backup.to_string(), ..Rec::default() },
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_old_formats() {
        let r = Rec { path: "C:\\n\\a.md".into(), enc: 2, caret: 40, first: 3, dirty: true, untitled: 0, backup: "1.txt".into(), pinned: true, lang: "Markdown".into() };
        assert_eq!(parse(&format(&r)), Some(r));
        let win = parse("doc\tx.rs\t1\t5\t2\t0\t0\t").unwrap();
        assert_eq!((win.caret, win.first, win.pinned, win.lang.as_str()), (5, 2, false, ""));
        let lin = parse("doc\tx.rs\t1\t1\t3\t0.txt").unwrap();
        assert_eq!((lin.enc, lin.dirty, lin.untitled, lin.backup.as_str(), lin.caret), (1, true, 3, "0.txt", 0));
        assert_eq!(parse("cur\t2"), None);
        assert_eq!(parse("doc\ta"), None);
        let hostile = Rec { path: "a\tb\nc".into(), ..Rec::default() };
        assert_eq!(parse(&format(&hostile)).unwrap().path, "a b c");
    }
}
