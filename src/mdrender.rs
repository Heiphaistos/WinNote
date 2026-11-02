//! Rendu Markdown simplifié -> (caractère, style) ; partagé par l'aperçu Windows et Linux.

pub const N: u8 = 0;
pub const BOLD: u8 = 7;
pub const ITAL: u8 = 8;
pub const BOLDITAL: u8 = 9;
pub const CODE: u8 = 10;
pub const CODEBLK: u8 = 11;
pub const QUOTE: u8 = 12;
pub const LINK: u8 = 13;
pub const DIM: u8 = 14;
pub const BULLET: u8 = 15;

pub fn inline(s: &str, base: u8, out: &mut Vec<(char, u8)>) {
    let c: Vec<char> = s.chars().collect();
    let mut i = 0;
    let find = |from: usize, pat: &[char]| -> Option<usize> { (from..c.len().saturating_sub(pat.len() - 1)).find(|&j| c[j..j + pat.len()] == *pat) };
    while i < c.len() {
        let rest = &c[i..];
        if rest.starts_with(&['*', '*']) {
            if let Some(e) = find(i + 2, &['*', '*']) {
                let inner: String = c[i + 2..e].iter().collect();
                inline(&inner, if base == ITAL { BOLDITAL } else { BOLD }, out);
                i = e + 2;
                continue;
            }
        }
        if (c[i] == '*' || c[i] == '_') && i + 1 < c.len() && !c[i + 1].is_whitespace() {
            if let Some(e) = find(i + 1, &[c[i]]) {
                if e > i + 1 && !(c[i] == '_' && i > 0 && c[i - 1].is_alphanumeric()) {
                    let inner: String = c[i + 1..e].iter().collect();
                    inline(&inner, if base == BOLD { BOLDITAL } else { ITAL }, out);
                    i = e + 1;
                    continue;
                }
            }
        }
        if c[i] == '`' {
            if let Some(e) = find(i + 1, &['`']) {
                out.extend(c[i + 1..e].iter().map(|&ch| (ch, CODE)));
                i = e + 1;
                continue;
            }
        }
        if rest.starts_with(&['!', '[']) || c[i] == '[' {
            let img = c[i] == '!';
            let st = if img { i + 2 } else { i + 1 };
            if let Some(e) = find(st, &[']']) {
                let double = !img && c.get(st) == Some(&'[');
                if double {
                    if let Some(e2) = find(st + 1, &[']', ']']) {
                        out.extend(c[st + 1..e2].iter().map(|&ch| (ch, LINK)));
                        i = e2 + 2;
                        continue;
                    }
                } else if c.get(e + 1) == Some(&'(') {
                    if let Some(e2) = find(e + 2, &[')']) {
                        let label: String = c[st..e].iter().collect();
                        if img {
                            out.extend(format!("[image : {label}]").chars().map(|ch| (ch, DIM)));
                        } else {
                            out.extend(label.chars().map(|ch| (ch, LINK)));
                        }
                        i = e2 + 1;
                        continue;
                    }
                }
            }
        }
        out.push((c[i], base));
        i += 1;
    }
}

/// Markdown -> (texte, style par caractère).
pub fn render(md: &str) -> Vec<(char, u8)> {
    let mut out: Vec<(char, u8)> = Vec::new();
    let mut in_code = false;
    for l in md.lines() {
        let t = l.trim_start();
        if t.starts_with("```") {
            in_code = !in_code;
            continue;
        }
        if in_code {
            out.extend(l.chars().map(|c| (c, CODEBLK)));
        } else if let Some(h) = (1..=6).rev().find(|&n| t.starts_with(&format!("{} ", "#".repeat(n)))) {
            inline(t[h + 1..].trim(), h as u8, &mut out);
        } else if t.starts_with("> ") {
            out.extend("▎ ".chars().map(|c| (c, DIM)));
            inline(&t[2..], QUOTE, &mut out);
        } else if ["- ", "* ", "+ "].iter().any(|p| t.starts_with(p)) {
            out.extend(std::iter::repeat((' ', N)).take((l.len() - t.len()) / 2 * 2));
            out.push(('•', BULLET));
            out.push((' ', N));
            let task = t[2..].trim_start();
            if let Some(r) = task.strip_prefix("[ ] ") {
                out.extend("☐ ".chars().map(|c| (c, DIM)));
                inline(r, N, &mut out);
            } else if let Some(r) = task.strip_prefix("[x] ").or_else(|| task.strip_prefix("[X] ")) {
                out.extend("☑ ".chars().map(|c| (c, BULLET)));
                inline(r, N, &mut out);
            } else {
                inline(&t[2..], N, &mut out);
            }
        } else if t.len() >= 3 && t.chars().all(|c| matches!(c, '-' | '*' | '_' | ' ')) && t.chars().filter(|c| !c.is_whitespace()).count() >= 3 {
            out.extend("────────────────────────".chars().map(|c| (c, DIM)));
        } else if t.starts_with('|') {
            if t.chars().all(|c| matches!(c, '|' | '-' | ':' | ' ')) {
                continue;
            }
            for cell in t.trim_matches('|').split('|') {
                out.extend(" │ ".chars().map(|c| (c, DIM)));
                inline(cell.trim(), N, &mut out);
            }
        } else {
            inline(l, N, &mut out);
        }
        out.push(('\n', N));
    }
    out
}


#[cfg(test)]
mod tests {
    #[test]
    fn renders_styles() {
        let r = super::render("# Titre\n\nUn **gras** et `code`\n- item\n");
        let s: String = r.iter().map(|x| x.0).collect();
        assert!(s.contains("Titre") && s.contains("gras et code") && s.contains("• item"), "{s}");
        assert_eq!(r[0].1, 1);
        assert!(r.iter().any(|x| x.1 == super::BOLD) && r.iter().any(|x| x.1 == super::CODE));
    }
}
