//! Autotest intégré Linux : `winnote-gtk --selftest journal.txt` (à lancer sous Xvfb). Code de sortie = nombre d'échecs.
use crate::doc::Enc;
use crate::ids::*;
use crate::lang::LANGS;
use crate::sci::{self, send};
use crate::ui::on_command;
use crate::{app, cstr, edit, find};
use gtk::prelude::*;

struct T {
    log: Vec<String>,
    fails: u32,
}

impl T {
    fn check(&mut self, name: &str, got: &str, want: &str) {
        let ok = got == want;
        eprintln!("test: {name} -> {}", if ok { "ok" } else { "ECHEC" });
        self.fails += !ok as u32;
        self.log.push(format!("{} {name}{}", if ok { "OK  " } else { "FAIL" }, if ok { String::new() } else { format!(" : obtenu {got:?} attendu {want:?}") }));
    }
}

fn set(text: &str) {
    let s = app().sci;
    send(s, sci::SETEOLMODE, 2, 0);
    send(s, sci::SETTEXT, 0, cstr(text).as_ptr() as isize);
    send(s, sci::SETSEL, 0, 0);
}
fn get() -> String {
    String::from_utf8_lossy(&edit::text_of(app().cur)).into_owned()
}
fn sel_all() {
    send(app().sci, sci::SELECTALL, 0, 0);
}
fn lang(name: &str) {
    on_command(LANG + LANGS.iter().position(|l| l.name == name).expect("langage"));
}
fn goto_line(l: usize) {
    send(app().sci, sci::GOTOLINE, l, 0);
}
fn caret_line() -> String {
    let s = app().sci;
    send(s, sci::LINEFROMPOS, send(s, sci::GETCURRENTPOS, 0, 0) as usize, 0).to_string()
}

pub fn run(log_path: &str) -> i32 {
    let a = app();
    let mut t = T { log: Vec::new(), fails: 0 };
    lang("Texte brut");
    set("b\nA\nc");
    sel_all();
    on_command(SORTASC);
    t.check("trier A→Z", &get(), "A\nb\nc");
    on_command(SORTDESC);
    t.check("trier Z→A", &get(), "c\nb\nA");
    set("a\nb\na\nc\nb");
    on_command(DEDUPE);
    t.check("supprimer les doublons", &get(), "a\nb\nc");
    set("a  \nb\t \nc");
    on_command(TRIM);
    t.check("espaces en fin de ligne", &get(), "a\nb\nc");
    set("a\n b \n\nc");
    on_command(JOIN);
    t.check("joindre les lignes", &get(), "a b c");
    set("1\n2\n3");
    goto_line(1);
    on_command(MOVEUP);
    t.check("déplacer la ligne vers le haut", &get(), "2\n1\n3");
    set("a\nb");
    on_command(DUPLINE);
    t.check("dupliquer la ligne", &get(), "a\na\nb");
    on_command(DELLINE);
    t.check("supprimer la ligne", &get(), "a\nb");
    set("hello wORLD");
    sel_all();
    on_command(TITLECASE);
    t.check("Première Lettre En Majuscule", &get(), "Hello World");
    sel_all();
    on_command(UPPER);
    t.check("MAJUSCULES", &get(), "HELLO WORLD");
    set("\ta");
    on_command(TABS2SPACES);
    t.check("tabulations → espaces", &get(), "    a");
    lang("Rust");
    set("x\n  y");
    on_command(TOGGLECOMMENT);
    t.check("commenter (Rust)", &get(), "// x\n  // y");
    on_command(TOGGLECOMMENT);
    t.check("décommenter (Rust)", &get(), "x\n  y");
    lang("Texte brut");
    set("foo bar foo baz foo");
    send(a.sci, sci::SETSEL, 0, 3);
    on_command(SELNEXT);
    on_command(SELNEXT);
    t.check("multi-curseurs : 3 sélections", &send(a.sci, 2570, 0, 0).to_string(), "3");
    set("l0\nl1\nl2\nl3\nl4");
    goto_line(2);
    on_command(BOOKTOGGLE);
    goto_line(0);
    on_command(BOOKNEXT);
    t.check("signet suivant", &caret_line(), "2");
    lang("Rust");
    set("fn a() {\n    x;\n}\n");
    send(a.sci, sci::COLOURISE, 0, -1);
    on_command(FOLDALL);
    t.check("tout replier", &send(a.sci, 2228, 1, 0).to_string(), "0");
    on_command(UNFOLDALL);
    lang("Texte brut");
    // recherche / remplacement
    set("a1 b22");
    a.find.find.set_text("(\\d+)");
    a.find.repl.set_text("<\\1>");
    a.find.regex.set_active(true);
    find::replace_all();
    t.check("regex avec groupe de capture", &get(), "a<1> b<22>");
    a.find.regex.set_active(false);
    set("aAa");
    a.find.find.set_text("a");
    a.find.repl.set_text("z");
    a.find.case.set_active(true);
    find::replace_all();
    t.check("remplacer (sensible à la casse)", &get(), "zAz");
    // encodages
    let dir = std::env::temp_dir().join(format!("wn_selftest_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    for (i, enc) in Enc::ALL.iter().enumerate() {
        let p = dir.join(format!("enc{i}.txt")).to_string_lossy().to_string();
        edit::open_or_create(&p);
        let src = if *enc == Enc::Ansi { "héllo wörld\nligne 2" } else { "héllo wörld €\nligne 2" };
        set(src);
        a.docs[a.cur].enc = *enc;
        let idx = a.cur;
        edit::save_doc(idx, false);
        edit::close_tab(a.cur);
        edit::open_path(&p);
        t.check(&format!("enregistrer puis rouvrir en {}", enc.name()), &get().replace('\r', ""), src);
        t.check(&format!("encodage {} redétecté", enc.name()), &(a.docs[a.cur].enc == *enc).to_string(), "true");
    }
    // liens [[note]]
    set("voir [[note-x]] ici");
    send(a.sci, sci::SETSEL, 8, 8);
    a.docs[a.cur].path = Some(dir.join("base.md").to_string_lossy().to_string());
    let before = a.docs.len();
    on_command(OPENLINK);
    t.check("lien [[note-x]] : nouvel onglet", &(a.docs.len() - before).to_string(), "1");
    let _ = std::fs::remove_dir_all(&dir);
    t.log.push(format!("{} échec(s)", t.fails));
    let _ = std::fs::write(log_path, t.log.join("\n"));
    t.fails as i32
}
