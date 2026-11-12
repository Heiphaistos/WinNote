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


// ---------- Style en direct dans l'éditeur (un style par octet, marqueurs atténués) ----------

fn find(t: &[u8], from: usize, pat: &[u8]) -> Option<usize> {
    (from..t.len().checked_sub(pat.len() - 1)?).find(|&j| &t[j..j + pat.len()] == pat)
}

fn inline_live(t: &[u8], out: &mut [u8], base: u8) {
    out.fill(base);
    let mut i = 0;
    while i < t.len() {
        let c = t[i];
        if c == 92 && i + 1 < t.len() {
            i += 2;
            continue;
        }
        if (c == b'*' || c == b'_') && t.get(i + 1) == Some(&c) {
            if let Some(e) = find(t, i + 2, &[c, c]).filter(|&e| e > i + 2) {
                out[i..i + 2].fill(DIM);
                out[e..e + 2].fill(DIM);
                inline_live(&t[i + 2..e], &mut out[i + 2..e], if base == ITAL { BOLDITAL } else { BOLD });
                i = e + 2;
                continue;
            }
        }
        if (c == b'*' || c == b'_') && t.get(i + 1).is_some_and(|x| !x.is_ascii_whitespace()) && !(c == b'_' && i > 0 && t[i - 1].is_ascii_alphanumeric()) {
            if let Some(e) = find(t, i + 1, &[c]).filter(|&e| e > i + 1 && !t[e - 1].is_ascii_whitespace()) {
                out[i] = DIM;
                out[e] = DIM;
                inline_live(&t[i + 1..e], &mut out[i + 1..e], if base == BOLD { BOLDITAL } else { ITAL });
                i = e + 1;
                continue;
            }
        }
        if c == b'~' && t.get(i + 1) == Some(&b'~') {
            if let Some(e) = find(t, i + 2, b"~~").filter(|&e| e > i + 2) {
                out[i..e + 2].fill(DIM);
                i = e + 2;
                continue;
            }
        }
        if c == b'`' {
            if let Some(e) = find(t, i + 1, b"`").filter(|&e| e > i + 1) {
                out[i] = DIM;
                out[e] = DIM;
                out[i + 1..e].fill(CODE);
                i = e + 1;
                continue;
            }
        }
        if c == b'[' || (c == b'!' && t.get(i + 1) == Some(&b'[')) {
            let st = if c == b'!' { i + 2 } else { i + 1 };
            if let Some(e) = find(t, st, b"]") {
                let end = if t.get(e + 1) == Some(&b'(') { find(t, e + 2, b")").map(|x| x + 1) } else if t.get(st) == Some(&b'[') && t.get(e + 1) == Some(&b']') { Some(e + 2) } else { None };
                if let Some(end) = end {
                    out[i..end].fill(LINK);
                    if t.get(e + 1) == Some(&b'(') {
                        out[e + 1..end].fill(DIM);
                    }
                    i = end;
                    continue;
                }
            }
        }
        if t[i..].starts_with(b"http://") || t[i..].starts_with(b"https://") {
            let e = t[i..].iter().position(|x| x.is_ascii_whitespace() || *x == b')' || *x == b'>').map_or(t.len(), |p| i + p);
            out[i..e].fill(LINK);
            i = e;
            continue;
        }
        i += 1;
    }
}

/// Markdown -> style par octet (mêmes ids que l'aperçu : 1..6 titres, BOLD, ITAL, CODE...). Les marqueurs (#, **, `) sont atténués (DIM).
pub fn live(src: &str) -> Vec<u8> {
    let mut st = vec![N; src.len()];
    let (mut pos, mut in_code) = (0, false);
    for line in src.split_inclusive(char::from(10)) {
        let l = line.trim_end_matches([char::from(10), char::from(13)]);
        let (b, s) = (l.as_bytes(), &mut st[pos..pos + l.len()]);
        let t = l.trim_start();
        let ind = l.len() - t.len();
        let tb = t.as_bytes();
        let hashes = tb.iter().take_while(|&&c| c == b'#').count();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_code = !in_code;
            s.fill(DIM);
        } else if in_code {
            s.fill(CODEBLK);
        } else if (1..=6).contains(&hashes) && (tb.len() == hashes || tb[hashes] == b' ') {
            s.fill(hashes as u8);
            s[ind..ind + hashes].fill(DIM);
        } else if let Some(r) = t.strip_prefix('>') {
            s[ind] = DIM;
            let from = ind + 1;
            inline_live(&b[from..], &mut s[from..], QUOTE);
            let _ = r;
        } else if t.len() >= 3 && t.chars().all(|c| matches!(c, '-' | '*' | '_' | ' ')) && t.chars().filter(|c| !c.is_whitespace()).count() >= 3 {
            s.fill(DIM);
        } else {
            let digits = tb.iter().take_while(|c| c.is_ascii_digit()).count();
            let mark = if matches!(tb.first(), Some(b'-' | b'*' | b'+')) && tb.get(1) == Some(&b' ') {
                1
            } else if digits > 0 && matches!(tb.get(digits), Some(b'.' | b')')) && tb.get(digits + 1) == Some(&b' ') {
                digits + 1
            } else {
                0
            };
            let mut from = 0;
            if mark > 0 {
                s[ind..ind + mark].fill(BULLET);
                from = ind + mark;
                let rest = t[mark..].trim_start();
                let off = from + (t.len() - mark - rest.len());
                if ["[ ]", "[x]", "[X]"].iter().any(|p| rest.starts_with(p)) {
                    s[off..off + 3].fill(BULLET);
                    from = off + 3;
                }
            }
            inline_live(&b[from..], &mut s[from..], N);
            if t.starts_with('|') {
                for (i, &c) in b.iter().enumerate() {
                    if c == b'|' {
                        s[i] = DIM;
                    }
                }
            }
        }
        pos += line.len();
    }
    st
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

    #[test]
    fn live_styles() {
        let src = "### Titre
Un **gras** et *it* `c` [l](u)
- a
> q
```
x
```
";
        let s = super::live(src);
        assert_eq!(s.len(), src.len());
        assert_eq!(s[0], super::DIM);
        assert_eq!(s[5], 3);
        let at = |w: &str| s[src.find(w).unwrap()];
        assert_eq!(at("gras"), super::BOLD);
        assert_eq!(at("it*"), super::ITAL);
        assert_eq!(at("c`"), super::CODE);
        assert_eq!(at("l]"), super::LINK);
        assert_eq!(at("- a"), super::BULLET);
        assert_eq!(at("x"), super::CODEBLK);
        assert_eq!(super::live("é **b**é").len(), "é **b**é".len());
    }
}
