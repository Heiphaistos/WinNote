//! Logique « notes » partagée Windows / Linux : plan du document, liens entrants [[note]], parcours de dossier.
use std::path::{Path, PathBuf};

pub fn walk(root: &Path, out: &mut Vec<PathBuf>, depth: u32) {
    if depth > 6 || out.len() > 5000 {
        return;
    }
    let Ok(rd) = std::fs::read_dir(root) else { return };
    for e in rd.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().to_string();
        if p.is_dir() {
            if !matches!(name.as_str(), ".git" | "node_modules" | "target" | ".logs" | "dist" | "build") {
                walk(&p, out, depth + 1);
            }
        } else {
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
        } else if !md && ["fn ", "pub fn ", "def ", "class ", "function ", "func ", "struct ", "impl ", "enum ", "trait ", "interface ", "async fn ", "pub struct "].iter().any(|k| t.starts_with(k)) {
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
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        if let Some((i, l)) = text.lines().enumerate().find(|(_, l)| {
            let ll = l.to_lowercase();
            pats.iter().any(|q| ll.contains(q.as_str()))
        }) {
            out.push((f.to_string_lossy().to_string(), i, l.trim().chars().take(70).collect()));
        }
    }
    out
}

#[cfg(test)]
mod tests {
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
