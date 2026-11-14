//! Outils de texte de parité Notepad++ / Obsidian, partagés Windows + Linux : crochet correspondant, sommaire Markdown,
//! renvoi à la ligne, Base64 et pourcentage d'URL sur la sélection, saut vers un titre `[[#Titre]]`.
use crate::cmds::msg_box;
use crate::sci::{self, send};
use crate::app;

const BRACEMATCH: u32 = 2353;
pub const REFLOW_WIDTH: usize = 80;

// ---------- fonctions pures (testées) ----------

/// Sommaire Markdown : une ligne `- [[#Titre]]` par titre, indentée selon le niveau relatif.
pub fn toc(text: &str) -> String {
    let items = crate::notes::outline(text, true);
    let min = items.iter().map(|i| i.0).min().unwrap_or(1);
    items.iter().map(|(lv, t, _)| format!("{}- [[#{}]]\n", "  ".repeat(lv - min), t)).collect()
}

/// Ligne (0 = première) du titre Markdown `heading` (sans tenir compte de la casse).
pub fn heading_line(text: &str, heading: &str) -> Option<usize> {
    let h = heading.trim().to_lowercase();
    crate::notes::outline(text, true).into_iter().find(|(_, t, _)| t.trim().to_lowercase() == h).map(|(_, _, l)| l)
}

/// Renvoie les paragraphes à `width` colonnes (l'indentation de la 1re ligne est reprise ; les blocs ``` sont laissés tels quels).
pub fn reflow(lines: Vec<String>, width: usize) -> Vec<String> {
    let (mut out, mut para, mut fence) = (Vec::new(), Vec::<String>::new(), false);
    let flush = |para: &mut Vec<String>, out: &mut Vec<String>| {
        if para.is_empty() {
            return;
        }
        let indent: String = para[0].chars().take_while(|c| c.is_whitespace()).collect();
        let mut cur = indent.clone();
        let mut empty = true;
        for word in para.iter().flat_map(|l| l.split_whitespace()) {
            if !empty && cur.chars().count() + 1 + word.chars().count() > width {
                out.push(std::mem::replace(&mut cur, indent.clone()));
                empty = true;
            }
            if !empty {
                cur.push(' ');
            }
            cur.push_str(word);
            empty = false;
        }
        out.push(cur);
        para.clear();
    };
    for l in lines {
        if l.trim_start().starts_with("```") {
            flush(&mut para, &mut out);
            fence = !fence;
            out.push(l);
        } else if fence || l.trim().is_empty() {
            flush(&mut para, &mut out);
            out.push(l);
        } else {
            para.push(l);
        }
    }
    flush(&mut para, &mut out);
    out
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(b: &[u8]) -> String {
    let mut o = String::with_capacity(b.len().div_ceil(3) * 4);
    for c in b.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        for i in 0..4 {
            if i <= c.len() {
                o.push(B64[(n >> (18 - 6 * i) & 63) as usize] as char);
            } else {
                o.push('=');
            }
        }
    }
    o
}

/// Décode du Base64 (espaces ignorés, `=` de fin facultatifs) ; None si un caractère n'est pas Base64.
pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let (mut o, mut acc, mut bits) = (Vec::new(), 0u32, 0);
    for c in s.bytes().filter(|c| !c.is_ascii_whitespace()).take_while(|&c| c != b'=') {
        acc = acc << 6 | B64.iter().position(|&x| x == c)? as u32;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            o.push((acc >> bits) as u8);
            acc &= (1 << bits) - 1;
        }
    }
    Some(o)
}

pub fn url_encode(s: &str) -> String {
    s.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect()
}

/// Décode les `%XX` ; None si une séquence est incomplète ou si le résultat n'est pas de l'UTF-8.
pub fn url_decode(s: &str) -> Option<String> {
    let (b, mut o, mut i) = (s.as_bytes(), Vec::new(), 0);
    while i < b.len() {
        if b[i] == b'%' {
            o.push(u8::from_str_radix(s.get(i + 1..i + 3)?, 16).ok()?);
            i += 3;
        } else {
            o.push(b[i]);
            i += 1;
        }
    }
    String::from_utf8(o).ok()
}

// ---------- commandes (éditeur) ----------

fn all_text(s: sci::HWND) -> String {
    crate::textops::range_text(s, 0, send(s, sci::GETLENGTH, 0, 0))
}

/// Position du crochet associé à celui sous / avant le curseur : (position du crochet, position de son assorti).
pub fn brace_pair(s: sci::HWND) -> Option<(isize, isize)> {
    let pos = send(s, sci::GETCURRENTPOS, 0, 0);
    [pos, pos - 1].into_iter().filter(|&p| p >= 0).find_map(|p| {
        let c = send(s, sci::GETCHARAT, p as usize, 0) as u8;
        let m = if b"()[]{}".contains(&c) && c != 0 { send(s, BRACEMATCH, p as usize, 0) } else { -1 };
        (m >= 0).then_some((p, m))
    })
}

/// Aller au crochet correspondant (`select` : sélectionne aussi tout ce qui est entre les deux, crochets compris).
pub fn goto_brace(select: bool) {
    let s = app().sci;
    let Some((p, m)) = brace_pair(s) else {
        msg_box("Aucun crochet ( ) [ ] { } à côté du curseur, ou sans correspondant.");
        return;
    };
    if select {
        let (a, b) = (p.min(m), p.max(m) + 1);
        send(s, sci::SETSEL, a as usize, b);
    } else {
        send(s, sci::GOTOPOS, m as usize, 0);
    }
    send(s, sci::ENSUREVISIBLE, send(s, sci::LINEFROMPOS, m as usize, 0) as usize, 0);
}

/// Insère le sommaire du document Markdown à la position du curseur.
pub fn insert_toc() {
    let s = app().sci;
    let t = toc(&all_text(s));
    if t.is_empty() {
        msg_box("Aucun titre Markdown (# Titre) dans ce document.");
        return;
    }
    send(s, sci::REPLACESEL, 0, crate::cstr(&t).as_ptr() as isize);
}

pub fn reflow_cmd() {
    crate::textops::transform_lines(|v| reflow(v, REFLOW_WIDTH));
}

/// Va au titre `heading` du document courant ; false si absent.
pub fn goto_heading(heading: &str) -> bool {
    let s = app().sci;
    let Some(l) = heading_line(&all_text(s), heading) else { return false };
    send(s, sci::GOTOLINE, l, 0);
    true
}

/// Remplace la sélection par `f(texte)` ; `f` renvoie Err(message) si le texte ne convient pas.
fn map_selection(f: impl Fn(&str) -> Result<String, &'static str>) {
    let s = app().sci;
    let (a, b) = (send(s, sci::GETSELSTART, 0, 0), send(s, sci::GETSELEND, 0, 0));
    if b <= a {
        msg_box("Sélectionnez d'abord le texte à convertir.");
        return;
    }
    match f(&crate::textops::range_text(s, a, b)) {
        Ok(o) => {
            send(s, sci::REPLACESEL, 0, crate::cstr(&o).as_ptr() as isize);
        }
        Err(e) => msg_box(e),
    }
}

/// kind : 0 = Base64 encoder, 1 = Base64 décoder, 2 = URL encoder, 3 = URL décoder.
pub fn convert(kind: usize) {
    map_selection(|t| match kind {
        0 => Ok(base64_encode(t.as_bytes())),
        1 => base64_decode(t).and_then(|b| String::from_utf8(b).ok()).ok_or("Ce texte n'est pas du Base64 valide (ou ne contient pas de texte UTF-8)."),
        2 => Ok(url_encode(t)),
        _ => url_decode(t).ok_or("Ce texte n'est pas une adresse encodée valide (séquence %XX incomplète ou non UTF-8)."),
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    fn v(x: &[&str]) -> Vec<String> {
        x.iter().map(|s| s.to_string()).collect()
    }
    #[test]
    fn toc_and_heading() {
        let md = "# A\ntexte\n## B\n### C\n```\n# pas un titre\n```\n## D\n";
        assert_eq!(toc(md), "- [[#A]]\n  - [[#B]]\n    - [[#C]]\n  - [[#D]]\n");
        assert_eq!(toc("pas de titre"), "");
        assert_eq!(toc("## x\n### y\n"), "- [[#x]]\n  - [[#y]]\n");
        assert_eq!(heading_line(md, "c"), Some(3));
        assert_eq!(heading_line(md, "pas un titre"), None);
    }
    #[test]
    fn reflow_paragraphs() {
        assert_eq!(reflow(v(&["aaa bbb ccc ddd"]), 8), v(&["aaa bbb", "ccc ddd"]));
        assert_eq!(reflow(v(&["aaa", "bbb", "", "ccc"]), 80), v(&["aaa bbb", "", "ccc"]));
        assert_eq!(reflow(v(&["  un deux trois"]), 9), v(&["  un deux", "  trois"]));
        assert_eq!(reflow(v(&["```", "a    b", "```", "x  y"]), 80), v(&["```", "a    b", "```", "x y"]));
        assert_eq!(reflow(v(&["extraordinairement long"]), 5), v(&["extraordinairement", "long"]));
    }
    #[test]
    fn base64_roundtrip() {
        for (t, e) in [("", ""), ("f", "Zg=="), ("fo", "Zm8="), ("foo", "Zm9v"), ("été ✓", "w6l0w6kg4pyT")] {
            assert_eq!(base64_encode(t.as_bytes()), e);
            assert_eq!(base64_decode(e).as_deref(), Some(t.as_bytes()));
        }
        assert_eq!(base64_decode("Zm9v\nYmFy").as_deref(), Some(&b"foobar"[..]));
        assert_eq!(base64_decode("Zm9v!"), None);
    }
    #[test]
    fn url_roundtrip() {
        assert_eq!(url_encode("a b&é"), "a%20b%26%C3%A9");
        assert_eq!(url_decode("a%20b%26%C3%A9").as_deref(), Some("a b&é"));
        assert_eq!(url_decode("100%"), None);
        assert_eq!(url_decode("%ZZ"), None);
        assert_eq!(url_decode("%FF"), None);
    }
}
