//! Logique partagée Windows / Linux : propriétés YAML (front matter), favoris, récupération après arrêt brutal, mise en page d'impression.
use std::path::{Path, PathBuf};

// ---------- Propriétés YAML (front matter) ----------

#[derive(Clone, Debug, PartialEq)]
pub struct Prop {
    pub key: String,
    /// Valeur en une ligne ; une liste « - a / - b » est affichée « a, b ».
    pub value: String,
}

fn eol_of(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

/// Lignes (sans fin de ligne) de l'en-tête `---` ... `---` : (lignes, octets de l'en-tête fermeture comprise). None si absent.
fn front_lines(text: &str) -> Option<(Vec<&str>, usize)> {
    let mut lines = Vec::new();
    let mut pos = 0;
    let mut it = text.split_inclusive('\n');
    let first = it.next()?;
    if first.trim_end() != "---" {
        return None;
    }
    pos += first.len();
    for l in it {
        pos += l.len();
        let t = l.trim_end();
        if t == "---" || t == "..." {
            return Some((lines, pos));
        }
        lines.push(t);
    }
    None
}

fn is_cont(l: &str) -> bool {
    l.starts_with(' ') || l.starts_with('\t') || l.starts_with("- ") || l == "-"
}

fn unquote(v: &str) -> &str {
    let v = v.trim();
    for q in ['"', '\''] {
        if v.len() >= 2 && v.starts_with(q) && v.ends_with(q) {
            return &v[1..v.len() - 1];
        }
    }
    v
}

fn key_of(l: &str) -> Option<(&str, &str)> {
    if is_cont(l) || l.starts_with('#') {
        return None;
    }
    let (k, v) = l.split_once(':')?;
    let k = k.trim();
    (!k.is_empty()).then_some((k, v.trim()))
}

/// Propriétés de l'en-tête YAML du document (vide s'il n'y en a pas).
pub fn props(text: &str) -> Vec<Prop> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let Some((lines, _)) = front_lines(text) else { return Vec::new() };
    let mut out: Vec<Prop> = Vec::new();
    for l in lines {
        if let Some((k, v)) = key_of(l) {
            out.push(Prop { key: k.to_string(), value: unquote(v).to_string() });
        } else if let (Some(p), true) = (out.last_mut(), is_cont(l)) {
            let item = unquote(l.trim().trim_start_matches('-'));
            if !item.is_empty() {
                p.value = if p.value.is_empty() { item.to_string() } else { format!("{}, {item}", p.value) };
            }
        }
    }
    out
}

/// « clé: valeur » -> (clé, valeur) ; None si la clé est vide ou contient des espaces / « : ».
pub fn parse_kv(s: &str) -> Option<(String, String)> {
    let (k, v) = s.split_once(':')?;
    let k = k.trim();
    (!k.is_empty() && !k.contains(char::is_whitespace)).then(|| (k.to_string(), v.trim().to_string()))
}

fn yaml_value(v: &str) -> String {
    let needs_quote = v.starts_with(['#', '&', '*', '!', '|', '>', '%', '@', '`', '"', '\'', '[', '{']) || v.contains(": ") || v.contains(" #") || v.ends_with(':');
    if needs_quote { format!("\"{}\"", v.replace('\\', "\\\\").replace('"', "\\\"")) } else { v.to_string() }
}

/// Texte avec la propriété `key` posée à `value` (créée si besoin, retirée si `value` est vide). Les autres lignes ne changent pas.
pub fn set_prop(text: &str, key: &str, value: &str) -> String {
    let eol = eol_of(text);
    let (bom, body) = match text.strip_prefix('\u{feff}') {
        Some(b) => ("\u{feff}", b),
        None => ("", text),
    };
    let value = value.trim();
    let line = format!("{key}: {}", yaml_value(value));
    let Some((lines, end)) = front_lines(body) else {
        if value.is_empty() {
            return text.to_string();
        }
        return format!("{bom}---{eol}{line}{eol}---{eol}{body}");
    };
    let mut out: Vec<String> = Vec::new();
    let mut found = false;
    let mut skipping = false;
    for l in lines {
        match key_of(l) {
            Some((k, _)) if k == key => {
                found = true;
                skipping = true;
                if !value.is_empty() {
                    out.push(line.clone());
                }
            }
            Some(_) => {
                skipping = false;
                out.push(l.to_string());
            }
            None if skipping && is_cont(l) => {}
            None => {
                skipping = false;
                out.push(l.to_string());
            }
        }
    }
    if !found && !value.is_empty() {
        out.push(line);
    }
    let mut head = format!("---{eol}");
    for l in &out {
        head.push_str(l);
        head.push_str(eol);
    }
    head.push_str(&format!("---{eol}"));
    if out.is_empty() {
        head.clear(); // plus aucune propriété : on retire l'en-tête
    }
    format!("{bom}{head}{}", &body[end..])
}

/// Plus petite zone à remplacer pour passer de `old` à `new` : (début, fin dans `old`, texte de remplacement). Préserve l'annulation.
pub fn diff_range<'a>(old: &str, new: &'a str) -> (usize, usize, &'a str) {
    let pre = old.bytes().zip(new.bytes()).take_while(|(a, b)| a == b).count();
    let pre = (0..=pre).rev().find(|&i| old.is_char_boundary(i) && new.is_char_boundary(i)).unwrap_or(0);
    let max_suf = old.len().min(new.len()) - pre;
    let suf = old.bytes().rev().zip(new.bytes().rev()).take(max_suf).take_while(|(a, b)| a == b).count();
    let suf = (0..=suf).rev().find(|&i| old.is_char_boundary(old.len() - i) && new.is_char_boundary(new.len() - i)).unwrap_or(0);
    (pre, old.len() - suf, &new[pre..new.len() - suf])
}

// ---------- Favoris ----------

pub fn favs_load(dir: &Path) -> Vec<String> {
    std::fs::read_to_string(dir.join("favoris.txt")).map(|s| s.lines().filter(|l| !l.is_empty()).map(String::from).collect()).unwrap_or_default()
}

pub fn favs_save(dir: &Path, list: &[String]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(dir.join("favoris.txt"), list.join("\n")).map_err(|e| e.to_string())
}

/// Ajoute le chemin s'il n'y est pas, le retire sinon. Renvoie true si ajouté.
pub fn favs_toggle(list: &mut Vec<String>, path: &str) -> bool {
    if let Some(i) = list.iter().position(|p| p == path) {
        list.remove(i);
        false
    } else {
        list.insert(0, path.to_string());
        true
    }
}

// ---------- Récupération après arrêt brutal ----------

#[derive(Clone, Debug, PartialEq)]
pub struct Recovered {
    pub name: String,
    pub path: Option<String>,
    pub enc: usize,
    pub text: Vec<u8>,
}

fn rdir(dir: &Path) -> PathBuf {
    dir.join("recovery")
}

#[cfg(windows)]
fn alive(pid: u32) -> bool {
    use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::OpenProcess};
    unsafe {
        let h = OpenProcess(0x1000, 0, pid); // PROCESS_QUERY_LIMITED_INFORMATION
        if h.is_null() {
            return false;
        }
        CloseHandle(h);
        true
    }
}
#[cfg(target_os = "linux")]
fn alive(pid: u32) -> bool {
    Path::new(&format!("/proc/{pid}")).exists()
}
#[cfg(not(any(windows, target_os = "linux")))]
fn alive(_pid: u32) -> bool {
    false
}

/// Pose le verrou de session. Renvoie true si le verrou d'un lancement précédent traîne (processus disparu) : arrêt brutal.
pub fn recovery_begin(dir: &Path) -> bool {
    let lock = dir.join("running.lock");
    let me = std::process::id();
    let crashed = std::fs::read_to_string(&lock).ok().and_then(|s| s.trim().parse::<u32>().ok()).is_some_and(|pid| pid != me && !alive(pid));
    let _ = std::fs::create_dir_all(dir);
    let _ = std::fs::write(&lock, me.to_string());
    crashed
}

/// Sortie normale : verrou et copies de secours retirés (seulement si le verrou est le nôtre).
pub fn recovery_end(dir: &Path) {
    let lock = dir.join("running.lock");
    if std::fs::read_to_string(&lock).is_ok_and(|s| s.trim() == std::process::id().to_string()) {
        let _ = std::fs::remove_file(&lock);
        let _ = std::fs::remove_dir_all(rdir(dir));
    }
}

/// Réécrit les copies de secours : une par document modifié (nom, chemin, encodage, texte UTF-8).
pub fn recovery_snapshot(dir: &Path, docs: &[Recovered]) {
    let d = rdir(dir);
    let _ = std::fs::remove_dir_all(&d);
    if docs.is_empty() || std::fs::create_dir_all(&d).is_err() {
        return;
    }
    let mut idx = String::new();
    for (i, r) in docs.iter().enumerate() {
        if std::fs::write(d.join(format!("{i}.txt")), &r.text).is_ok() {
            idx.push_str(&format!("{i}\t{}\t{}\t{}\n", r.enc, r.path.clone().unwrap_or_default(), r.name.replace(['\t', '\n'], " ")));
        }
    }
    let _ = std::fs::write(d.join("index.txt"), idx);
}

/// Copies de secours dont le texte n'est dans aucun document ouvert (chemin, texte) : à rouvrir comme documents modifiés.
pub fn recovery_pending(dir: &Path, open: &[(Option<String>, Vec<u8>)]) -> Vec<Recovered> {
    let d = rdir(dir);
    let Ok(idx) = std::fs::read_to_string(d.join("index.txt")) else { return Vec::new() };
    let mut out = Vec::new();
    for l in idx.lines() {
        let f: Vec<&str> = l.splitn(4, '\t').collect();
        let [n, enc, path, name] = f.as_slice() else { continue };
        let Ok(text) = std::fs::read(d.join(format!("{n}.txt"))) else { continue };
        if open.iter().any(|(_, t)| *t == text) {
            continue;
        }
        out.push(Recovered { name: name.to_string(), path: (!path.is_empty()).then(|| path.to_string()), enc: enc.parse().unwrap_or(0), text });
    }
    out
}

// ---------- Mise en page d'impression ----------

#[derive(Clone, Debug, PartialEq)]
pub struct PrintLayout {
    pub margin_mm: u32,
    /// Trois zones « gauche|centre|droite » ; {fichier} {date} {page} {pages}.
    pub header: String,
    pub footer: String,
}

impl Default for PrintLayout {
    fn default() -> Self {
        PrintLayout { margin_mm: 25, header: "{fichier}||{date}".into(), footer: "|Page {page} / {pages}|".into() }
    }
}

impl PrintLayout {
    /// « marge ; en-tête ; pied » (la marge est bornée à 5..60 mm).
    pub fn parse(s: &str) -> Option<PrintLayout> {
        let f: Vec<&str> = s.split(';').collect();
        let [m, h, p] = f.as_slice() else { return None };
        Some(PrintLayout { margin_mm: m.trim().parse::<u32>().ok()?.clamp(5, 60), header: h.trim().to_string(), footer: p.trim().to_string() })
    }
    pub fn to_line(&self) -> String {
        format!("{} ; {} ; {}", self.margin_mm, self.header, self.footer)
    }
    pub fn load(dir: &Path) -> PrintLayout {
        std::fs::read_to_string(dir.join("miseenpage.txt")).ok().and_then(|s| PrintLayout::parse(s.trim())).unwrap_or_default()
    }
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("miseenpage.txt"), self.to_line()).map_err(|e| e.to_string())
    }
}

/// Remplace les variables puis coupe en trois zones (gauche, centre, droite).
pub fn zones(tpl: &str, file: &str, date: &str, page: u32, pages: u32) -> [String; 3] {
    let s = tpl.replace("{fichier}", file).replace("{date}", date).replace("{page}", &page.to_string()).replace("{pages}", &pages.to_string());
    let mut it = s.splitn(3, '|');
    [it.next().unwrap_or("").to_string(), it.next().unwrap_or("").to_string(), it.next().unwrap_or("").to_string()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn yaml_props() {
        let t = "---\ntitre: Mon \"idée\"\ntags:\n  - a\n  - b\nok: oui\n---\ncorps\n";
        assert_eq!(props(t), vec![Prop { key: "titre".into(), value: "Mon \"idée\"".into() }, Prop { key: "tags".into(), value: "a, b".into() }, Prop { key: "ok".into(), value: "oui".into() }]);
        assert!(props("pas d'entête\n---\nx: 1\n---\n").is_empty());
        let s = set_prop(t, "ok", "non");
        assert!(s.contains("ok: non\n---\ncorps"));
        let s = set_prop(&s, "tags", "z");
        assert!(s.contains("tags: z\n") && !s.contains("  - a"));
        assert_eq!(props(&set_prop(&s, "nouveau", "x: y"))[3].value, "x: y");
        assert!(!set_prop(&s, "ok", "").contains("ok:"));
        assert_eq!(set_prop("texte\n", "a", "1"), "---\na: 1\n---\ntexte\n");
        assert_eq!(set_prop("---\na: 1\n---\nt", "a", ""), "t");
        assert_eq!(set_prop("t\r\n", "a", "1"), "---\r\na: 1\r\n---\r\nt\r\n");
        assert_eq!(parse_kv("a: b c"), Some(("a".into(), "b c".into())));
        assert_eq!(parse_kv("a b: c"), None);
    }

    #[test]
    fn diff() {
        let (a, b) = ("---\nx: 1\n---\nété", "---\nx: 1\ny: é\n---\nété");
        let (s, e, r) = diff_range(a, b);
        assert_eq!(format!("{}{}{}", &a[..s], r, &a[e..]), b);
        assert!(r.len() <= 6);
        let (s, e, r) = diff_range("aé", "aè");
        assert_eq!(format!("{}{}{}", &"aé"[..s], r, &"aé"[e..]), "aè");
    }

    #[test]
    fn favs() {
        let d = std::env::temp_dir().join(format!("wn_favs_{}", std::process::id()));
        let mut l = favs_load(&d);
        assert!(l.is_empty() && favs_toggle(&mut l, "/a") && favs_toggle(&mut l, "/b"));
        favs_save(&d, &l).unwrap();
        assert_eq!(favs_load(&d), vec!["/b", "/a"]);
        assert!(!favs_toggle(&mut l, "/b"));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn recovery() {
        let d = std::env::temp_dir().join(format!("wn_recov_{}", std::process::id()));
        std::fs::remove_dir_all(&d).ok();
        assert!(!recovery_begin(&d));
        let docs = vec![
            Recovered { name: "a.txt".into(), path: Some("/x/a.txt".into()), enc: 0, text: b"un".to_vec() },
            Recovered { name: "Sans titre 1".into(), path: None, enc: 1, text: b"deux".to_vec() },
        ];
        recovery_snapshot(&d, &docs);
        assert!(!recovery_begin(&d), "notre propre verrou n'est pas un arrêt brutal");
        std::fs::write(d.join("running.lock"), "4294967000").unwrap();
        assert!(recovery_begin(&d), "verrou d'un processus disparu = arrêt brutal");
        let p = recovery_pending(&d, &[(None, b"un".to_vec())]);
        assert_eq!(p, vec![docs[1].clone()]);
        assert_eq!(recovery_pending(&d, &[]).len(), 2);
        recovery_end(&d);
        assert!(!recovery_begin(&d) && recovery_pending(&d, &[]).is_empty());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn layout() {
        let l = PrintLayout::parse("99 ; {fichier}|x|{date} ; |p {page}/{pages}|").unwrap();
        assert_eq!(l.margin_mm, 60);
        assert_eq!(PrintLayout::parse(&l.to_line()), Some(l.clone()));
        assert!(PrintLayout::parse("abc ; a ; b").is_none() && PrintLayout::parse("10").is_none());
        assert_eq!(zones(&l.footer, "f", "d", 2, 5), ["", "p 2/5", ""]);
        assert_eq!(zones("{fichier}||{date}", "f", "d", 1, 1), ["f", "", "d"]);
        assert_eq!(PrintLayout::default().margin_mm, 25);
    }
}
