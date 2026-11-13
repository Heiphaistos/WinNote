//! Logique « notes » partagée Windows / Linux : plan du document, liens entrants [[note]], parcours de dossier.
use std::path::{Path, PathBuf};

/// Lit un fichier texte d'au plus 4 Mio (les liens entrants / étiquettes parcourent des milliers de fichiers : pas de journal géant en mémoire).
pub fn read_small(p: &Path) -> Option<String> {
    (std::fs::metadata(p).ok()?.len() <= 4 << 20).then(|| std::fs::read_to_string(p).ok()).flatten()
}

pub fn walk(root: &Path, out: &mut Vec<PathBuf>, depth: u32) {
    if depth > 6 || out.len() > 5000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        // file_type() ne suit pas les liens : ni jonction bouclante, ni tube / périphérique dont la lecture bloquerait
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if !matches!(name.as_str(), ".git" | "node_modules" | "target" | ".logs" | "dist" | "build") {
                walk(&p, out, depth + 1);
            }
        } else if ft.is_file() {
            out.push(p);
        }
    }
}

/// Plan : (niveau, titre, ligne) — titres Markdown, ou définitions (fn / class / def...) pour le code.
pub fn outline(text: &str, md: bool) -> Vec<(usize, String, usize)> {
    let mut v = Vec::new();
    let mut in_code = false;
    for (i, l) in text.lines().enumerate() {
        let t = l.trim_start();
        if t.starts_with("```") {
            in_code = !in_code;
        }
        let (level, title) = if md && !in_code && t.starts_with('#') {
            let lv = t.chars().take_while(|&c| c == '#').count();
            (lv, t[lv..].trim().to_string())
        } else if !md && ["fn ", "pub fn ", "def ", "class ", "function ", "func ", "struct ", "impl ", "enum ", "trait ", "interface ", "async fn ", "pub struct ", "pub(crate) fn ", "pub async fn ", "pub enum ", "pub trait ", "export function ", "export class ", "export default function ", "export async function ", "sub ", "module ", "object "].iter().any(|k| t.starts_with(k)) {
            (1 + (l.len() - t.len()) / 4, t.trim_end_matches(['{', ':', ' ']).chars().take(80).collect())
        } else {
            continue;
        };
        if !title.is_empty() {
            v.push((level, title, i));
        }
    }
    v
}

/// Notes du dossier qui contiennent [[nom-de-la-note]] : (chemin, ligne, texte de la ligne).
pub fn backlinks(path: &str, root: Option<&str>) -> Vec<(String, usize, String)> {
    let p = Path::new(path);
    let stem = p.file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
    let base = root.map(PathBuf::from).or_else(|| p.parent().map(|d| d.to_path_buf())).unwrap_or_default();
    let mut files = Vec::new();
    walk(&base, &mut files, 0);
    let pats = [format!("[[{stem}]]"), format!("[[{stem}|"), format!("[[{stem}#")];
    let mut out = Vec::new();
    for f in files {
        let ext = f.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !matches!(ext.as_str(), "md" | "markdown" | "mdx" | "txt") || f == p {
            continue;
        }
        let Some(text) = read_small(&f) else { continue };
        if let Some((i, l)) = text.lines().enumerate().find(|(_, l)| {
            let ll = l.to_lowercase();
            pats.iter().any(|q| ll.contains(q.as_str()))
        }) {
            out.push((f.to_string_lossy().to_string(), i, l.trim().chars().take(70).collect()));
        }
    }
    out
}

/// Étiquettes #tag d'une ligne (pas les titres « # titre » ; lettres, chiffres, _ - /, au moins une lettre).
pub fn line_tags(l: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut prev = ' ';
    let mut it = l.char_indices().peekable();
    while let Some((i, c)) = it.next() {
        if c == '#' && (prev.is_whitespace() || prev == '(') {
            let t: String = l[i + 1..].chars().take_while(|x| x.is_alphanumeric() || matches!(x, '_' | '-' | '/')).collect();
            if t.chars().any(|x| x.is_alphabetic()) {
                out.push(t);
            }
        }
        prev = c;
    }
    out
}

/// Toutes les étiquettes des notes du dossier : (tag, chemin, ligne, texte). Ignore les blocs de code.
pub fn tag_hits(root: &str) -> Vec<(String, String, usize, String)> {
    let mut files = Vec::new();
    walk(Path::new(root), &mut files, 0);
    let mut out = Vec::new();
    for f in files {
        let ext = f.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
        if !matches!(ext.as_str(), "md" | "markdown" | "mdx" | "txt") {
            continue;
        }
        let Some(text) = read_small(&f) else { continue };
        let mut fence = false;
        for (i, l) in text.lines().enumerate() {
            if l.trim_start().starts_with("```") {
                fence = !fence;
            }
            if fence {
                continue;
            }
            for t in line_tags(l) {
                out.push((t, f.to_string_lossy().to_string(), i, l.trim().chars().take(70).collect()));
            }
        }
    }
    out.sort_by(|a, b| a.0.to_lowercase().cmp(&b.0.to_lowercase()).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn tag_hits_dir() {
        let d = std::env::temp_dir().join("wn_tags_test");
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("a.md"), "# T
voir #zeta et #alpha
```
#code
```
").unwrap();
        let h = super::tag_hits(d.to_str().unwrap());
        assert_eq!(h.iter().map(|x| x.0.as_str()).collect::<Vec<_>>(), vec!["alpha", "zeta"]);
        let _ = std::fs::remove_dir_all(&d);
    }
    #[test]
    fn tags() {
        assert_eq!(super::line_tags("# Titre #projet/a (#b-2) x#no #123 #é"), vec!["projet/a", "b-2", "é"]);
    }
    #[test]
    fn outline_md_and_code() {
        let o = super::outline("# A\n```\n# no\n```\n## B", true);
        assert_eq!(o, vec![(1, "A".into(), 0), (2, "B".into(), 4)]);
        assert_eq!(super::outline("fn x() {\n    fn y() {", false).len(), 2);
    }
}

/// Note quotidienne : (chemin, contenu initial). Le modèle est `modele-quotidien.md` du dossier (`{{date}}` remplacé), sinon un simple titre.
pub fn daily_note(root: &str, y: i32, m: u32, d: u32) -> (String, String) {
    let date = format!("{y:04}-{m:02}-{d:02}");
    let path = std::path::Path::new(root).join(format!("{date}.md"));
    let tpl = std::fs::read_to_string(std::path::Path::new(root).join("modele-quotidien.md")).unwrap_or_else(|_| "# {{date}}\n\n".to_string());
    (path.to_string_lossy().into_owned(), tpl.replace("{{date}}", &date))
}

#[cfg(test)]
mod daily_tests {
    #[test]
    fn daily() {
        let dir = std::env::temp_dir().join(format!("wn_daily_{}", std::process::id()));
        std::fs::create_dir_all(&dir).ok();
        let (p, c) = super::daily_note(&dir.to_string_lossy(), 2026, 3, 9);
        assert!(p.ends_with("2026-03-09.md") && c == "# 2026-03-09\n\n");
        std::fs::write(dir.join("modele-quotidien.md"), "J {{date}}").ok();
        assert_eq!(super::daily_note(&dir.to_string_lossy(), 2026, 3, 9).1, "J 2026-03-09");
        std::fs::remove_dir_all(&dir).ok();
    }
}
