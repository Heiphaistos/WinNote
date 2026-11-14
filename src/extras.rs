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
    std::fs::create_dir_all(dir).map_err(|e| io_err("Dossier de réglages impossible à créer", dir, &e))?;
    let f = dir.join("favoris.txt");
    std::fs::write(&f, list.join("\n")).map_err(|e| io_err("Favoris impossibles à enregistrer", &f, &e))
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
// Une fenêtre (un processus) = un verrou `running-<pid>.lock` et un dossier `recovery/<pid>/`. Deux fenêtres ne se marchent donc pas dessus :
// la sortie de l'une ne retire que ses propres fichiers, et seules les traces d'un processus disparu sont reprises.

#[derive(Clone, Debug, PartialEq)]
pub struct Recovered {
    pub name: String,
    pub path: Option<String>,
    pub enc: usize,
    pub text: Vec<u8>,
}

/// Erreur lisible : l'action, le fichier concerné et la cause système.
pub fn io_err(action: &str, path: &Path, e: &std::io::Error) -> String {
    format!("{action} « {} » : {e}", path.display())
}

fn rdir(dir: &Path, pid: u32) -> PathBuf {
    dir.join("recovery").join(pid.to_string())
}

fn lock_of(dir: &Path, pid: u32) -> PathBuf {
    dir.join(format!("running-{pid}.lock"))
}

#[cfg(windows)]
fn alive(pid: u32) -> bool {
    use windows_sys::Win32::{
        Foundation::{CloseHandle, GetLastError},
        System::Threading::{GetExitCodeProcess, OpenProcess},
    };
    unsafe {
        let h = OpenProcess(0x1000, 0, pid); // PROCESS_QUERY_LIMITED_INFORMATION
        if h.is_null() {
            return GetLastError() == 5; // accès refusé : le processus existe (autre session)
        }
        // Un processus terminé reste « ouvrable » tant qu'un handle traîne quelque part : seul le code de sortie STILL_ACTIVE (259) prouve qu'il tourne.
        let mut code = 0u32;
        let running = GetExitCodeProcess(h, &mut code) == 0 || code == 259;
        CloseHandle(h);
        running
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

/// Pose le verrou de cette fenêtre (son pid).
pub fn recovery_register(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|e| io_err("Dossier de réglages impossible à créer", dir, &e))?;
    let lock = lock_of(dir, std::process::id());
    std::fs::write(&lock, std::process::id().to_string()).map_err(|e| io_err("Verrou de récupération impossible à écrire", &lock, &e))
}

/// Pids d'autres fenêtres disparues sans sortie normale : verrou ou dossier de copies resté, processus mort.
pub fn recovery_orphans(dir: &Path) -> Vec<u32> {
    let me = std::process::id();
    let mut out: Vec<u32> = Vec::new();
    let pid_of = |name: &str, pre: &str, post: &str| name.strip_prefix(pre).and_then(|n| n.strip_suffix(post)).and_then(|n| n.parse::<u32>().ok());
    if let Ok(rd) = std::fs::read_dir(dir) {
        out.extend(rd.flatten().filter_map(|e| pid_of(&e.file_name().to_string_lossy(), "running-", ".lock")));
    }
    if let Ok(rd) = std::fs::read_dir(dir.join("recovery")) {
        out.extend(rd.flatten().filter_map(|e| pid_of(&e.file_name().to_string_lossy(), "", "").or_else(|| pid_of(&e.file_name().to_string_lossy(), "", ".new"))));
    }
    out.sort_unstable();
    out.dedup();
    out.retain(|&p| p != me && !alive(p));
    out
}

/// Retire les traces des fenêtres disparues (à appeler une fois leurs copies rouvertes et re-sauvegardées).
pub fn recovery_discard(dir: &Path) {
    for p in recovery_orphans(dir) {
        let _ = std::fs::remove_file(lock_of(dir, p));
        let _ = std::fs::remove_dir_all(rdir(dir, p));
        let _ = std::fs::remove_dir_all(rdir(dir, p).with_extension("new"));
    }
}

/// Sortie normale : verrou et copies de secours de CETTE fenêtre retirés.
pub fn recovery_end(dir: &Path) {
    let me = std::process::id();
    let _ = std::fs::remove_file(lock_of(dir, me));
    let _ = std::fs::remove_dir_all(rdir(dir, me));
    let _ = std::fs::remove_dir_all(rdir(dir, me).with_extension("new"));
    let _ = std::fs::remove_dir(dir.join("recovery")); // seulement s'il est vide
}

/// Réécrit les copies de secours de cette fenêtre : une par document modifié (nom, chemin, encodage, texte UTF-8).
pub fn recovery_snapshot(dir: &Path, docs: &[Recovered]) -> Result<(), String> {
    let live = rdir(dir, std::process::id());
    if docs.is_empty() {
        let _ = std::fs::remove_dir_all(&live);
        return Ok(());
    }
    // Écrit dans un dossier voisin puis l'échange : un arrêt brutal pendant l'écriture laisse l'ancienne copie intacte.
    let d = live.with_file_name(format!("{}.new", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).map_err(|e| io_err("Dossier de copies de secours impossible à créer", &d, &e))?;
    let mut idx = String::new();
    for (i, r) in docs.iter().enumerate() {
        let f = d.join(format!("{i}.txt"));
        std::fs::write(&f, &r.text).map_err(|e| io_err("Copie de secours impossible à écrire", &f, &e))?;
        idx.push_str(&format!("{i}\t{}\t{}\t{}\n", r.enc, r.path.clone().unwrap_or_default(), r.name.replace(['\t', '\n'], " ")));
    }
    let f = d.join("index.txt");
    std::fs::write(&f, idx).map_err(|e| io_err("Index des copies de secours impossible à écrire", &f, &e))?;
    let _ = std::fs::remove_dir_all(&live);
    std::fs::rename(&d, &live).map_err(|e| io_err("Copies de secours impossibles à mettre en place", &live, &e))
}

/// Copies de secours des fenêtres disparues dont le texte n'est dans aucun document ouvert : à rouvrir comme documents modifiés.
pub fn recovery_pending(dir: &Path, open: &[(Option<String>, Vec<u8>)]) -> Vec<Recovered> {
    let mut out = Vec::new();
    for pid in recovery_orphans(dir) {
        let d = rdir(dir, pid);
        let Ok(idx) = std::fs::read_to_string(d.join("index.txt")) else { continue };
        for l in idx.lines() {
            let f: Vec<&str> = l.splitn(4, '\t').collect();
            let [n, enc, path, name] = f.as_slice() else { continue };
            let Ok(text) = std::fs::read(d.join(format!("{n}.txt"))) else { continue };
            if open.iter().any(|(_, t)| *t == text) || out.iter().any(|r: &Recovered| r.text == text) {
                continue;
            }
            out.push(Recovered { name: name.to_string(), path: (!path.is_empty()).then(|| path.to_string()), enc: enc.parse().unwrap_or(0), text });
        }
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
        // Fichier : 3 lignes (marge, en-tête, pied), les zones peuvent contenir « ; » ; ancien format et saisie : « marge ; en-tête ; pied ».
        let f: Vec<&str> = if s.contains('\n') { s.split('\n').collect() } else { s.split(';').collect() };
        let [m, h, p] = f.as_slice() else { return None };
        Some(PrintLayout { margin_mm: m.trim().parse::<u32>().ok()?.clamp(5, 60), header: h.trim().to_string(), footer: p.trim().to_string() })
    }
    /// Contenu du fichier de réglages : une valeur par ligne.
    pub fn to_file(&self) -> String {
        format!("{}\n{}\n{}", self.margin_mm, self.header.replace(['\r', '\n'], " "), self.footer.replace(['\r', '\n'], " "))
    }
    pub fn load(dir: &Path) -> PrintLayout {
        std::fs::read_to_string(dir.join("miseenpage.txt")).ok().and_then(|s| PrintLayout::parse(&s)).unwrap_or_default()
    }
    pub fn save(&self, dir: &Path) -> Result<(), String> {
        std::fs::create_dir_all(dir).map_err(|e| io_err("Dossier de réglages impossible à créer", dir, &e))?;
        let f = dir.join("miseenpage.txt");
        std::fs::write(&f, self.to_file()).map_err(|e| io_err("Mise en page impossible à enregistrer", &f, &e))
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
        recovery_register(&d).unwrap();
        assert!(recovery_orphans(&d).is_empty(), "notre propre verrou n'est pas un arrêt brutal");
        let docs = vec![
            Recovered { name: "a.txt".into(), path: Some("/x/a.txt".into()), enc: 0, text: b"un".to_vec() },
            Recovered { name: "Sans titre 1".into(), path: None, enc: 1, text: b"deux".to_vec() },
        ];
        recovery_snapshot(&d, &docs).unwrap();
        assert!(recovery_pending(&d, &[]).is_empty(), "nos copies ne sont pas reprises par nous");
        assert!(!rdir(&d, std::process::id()).with_extension("new").exists(), "le dossier d'écriture est échangé, pas laissé");
        recovery_snapshot(&d, &docs[..1]).unwrap();
        assert_eq!(std::fs::read_dir(rdir(&d, std::process::id())).unwrap().count(), 2, "0.txt + index.txt après réécriture");
        recovery_snapshot(&d, &docs).unwrap();
        // Une autre fenêtre morte (pid inexistant) laisse verrou + copies : reprises ; une autre fenêtre vivante (le parent de test n'est pas fiable) : simulée par un pid vivant.
        let dead = 4_294_967_000u32;
        std::fs::write(lock_of(&d, dead), dead.to_string()).unwrap();
        std::fs::create_dir_all(rdir(&d, dead)).unwrap();
        std::fs::write(rdir(&d, dead).join("0.txt"), b"un").unwrap();
        std::fs::write(rdir(&d, dead).join("1.txt"), b"deux").unwrap();
        std::fs::write(rdir(&d, dead).join("index.txt"), "0\t0\t/x/a.txt\ta.txt\n1\t1\t\tSans titre 1\n").unwrap();
        assert_eq!(recovery_orphans(&d), vec![dead]);
        assert_eq!(recovery_pending(&d, &[(None, b"un".to_vec())]), vec![docs[1].clone()]);
        assert_eq!(recovery_pending(&d, &[]).len(), 2);
        // Notre sortie normale ne touche pas aux traces de l'autre fenêtre ; elle retire les nôtres.
        recovery_end(&d);
        assert!(!lock_of(&d, std::process::id()).exists() && !rdir(&d, std::process::id()).exists());
        assert_eq!(recovery_orphans(&d), vec![dead]);
        recovery_discard(&d);
        assert!(recovery_orphans(&d).is_empty() && !lock_of(&d, dead).exists());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn two_live_windows() {
        // Deux fenêtres vivantes (nous + un processus enfant réel) : aucune n'est vue comme orpheline, et la mort de l'enfant la révèle.
        let d = std::env::temp_dir().join(format!("wn_two_{}", std::process::id()));
        std::fs::remove_dir_all(&d).ok();
        #[cfg(windows)]
        let mut child = std::process::Command::new("cmd").args(["/c", "ping -n 30 127.0.0.1 >nul"]).stdout(std::process::Stdio::null()).spawn().unwrap();
        #[cfg(not(windows))]
        let mut child = std::process::Command::new("sleep").arg("30").spawn().unwrap();
        let cp = child.id();
        std::fs::create_dir_all(rdir(&d, cp)).unwrap();
        std::fs::write(lock_of(&d, cp), cp.to_string()).unwrap();
        std::fs::write(rdir(&d, cp).join("index.txt"), "0\t0\t\ttexte\n").unwrap();
        std::fs::write(rdir(&d, cp).join("0.txt"), b"autre fenetre").unwrap();
        recovery_register(&d).unwrap();
        assert!(recovery_orphans(&d).is_empty(), "l'autre fenêtre est vivante");
        recovery_end(&d); // notre sortie
        assert!(lock_of(&d, cp).exists(), "le verrou de l'autre fenêtre reste");
        child.kill().unwrap();
        child.wait().unwrap();
        drop(child); // Windows : tant que le handle du processus est ouvert, il paraît vivant
        assert_eq!(recovery_orphans(&d), vec![cp], "morte sans sortie normale : reprise possible");
        assert_eq!(recovery_pending(&d, &[])[0].text, b"autre fenetre".to_vec());
        std::fs::remove_dir_all(&d).ok();
    }

    /// Entrées hostiles pseudo-aléatoires (UTF-8 multi-octets, marqueurs YAML / Markdown, BOM) : aucune ne doit paniquer (panic = abort en release).
    #[test]
    fn hostile_inputs_never_panic() {
        let alphabet = ["---", "\n", "\r\n", ":", " ", "- ", "#", "é", "日", "\u{feff}", "\"", "'", "[[", "]]", "](", "**", "`", "|", "\t", "...", "x", "\u{1f600}"];
        let mut seed = 0x2545F491u64;
        for _ in 0..3000 {
            let mut s = String::new();
            for _ in 0..(seed % 40) {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
                s.push_str(alphabet[(seed >> 33) as usize % alphabet.len()]);
            }
            let _ = props(&s);
            let _ = set_prop(&s, "k", "v: w");
            let _ = set_prop(&s, "k", "");
            let _ = crate::mdrender::render(&s);
            let _ = crate::mdrender::live(&s);
            let cut = |i: usize| (0..=i).rev().find(|&k| s.is_char_boundary(k)).unwrap_or(0);
            let (a, b) = (&s[..cut(s.len() / 2)], &s[cut(s.len() / 3)..]);
            let (x, y, z) = diff_range(a, b);
            assert_eq!(format!("{}{}{}", &a[..x], z, &a[y..]), b);
            let _ = PrintLayout::parse(&s);
            let _ = zones(&s, "f", "d", 1, 2);
            let raw: Vec<u8> = s.bytes().map(|c| c.wrapping_mul(seed as u8 | 1)).collect();
            let (t, _) = crate::doc::decode(&raw);
            let _ = crate::doc::encode(&t, crate::doc::Enc::Ansi);
        }
    }

    #[test]
    fn layout() {
        let l = PrintLayout::parse("99 ; {fichier}|x|{date} ; |p {page}/{pages}|").unwrap();
        assert_eq!(l.margin_mm, 60);
        assert_eq!(PrintLayout::parse(&l.to_file()), Some(l.clone()));
        let semi = PrintLayout { margin_mm: 20, header: "Rapport; v2||{date}".into(), footer: "".into() };
        assert_eq!(PrintLayout::parse(&semi.to_file()), Some(semi), "un point-virgule dans une zone survit au fichier");
        assert!(PrintLayout::parse("abc ; a ; b").is_none() && PrintLayout::parse("10").is_none());
        assert_eq!(zones(&l.footer, "f", "d", 2, 5), ["", "p 2/5", ""]);
        assert_eq!(zones("{fichier}||{date}", "f", "d", 1, 1), ["f", "", "d"]);
        assert_eq!(PrintLayout::default().margin_mm, 25);
    }
}
