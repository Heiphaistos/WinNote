//! Autotest intégré (`winnote.exe --selftest journal.txt`) : exécute les commandes d'édition dans une vraie fenêtre
//! et vérifie le texte obtenu. Code de sortie = nombre d'échecs.
use crate::ids::*;
use crate::lang::LANGS;
use crate::sci::{self, send};
use crate::{app, cmds, cstr, findbar, palette, w};
use windows_sys::Win32::UI::WindowsAndMessaging::{SendMessageW, SetWindowTextW, WM_CHAR, WM_KEYDOWN};

struct T {
    log: Vec<String>,
    fails: u32,
}

impl T {
    fn check(&mut self, name: &str, got: &str, want: &str) {
        let ok = got == want;
        self.fails += !ok as u32;
        self.log.push(format!("{} {name}{}", if ok { "OK  " } else { "FAIL" }, if ok { String::new() } else { format!(" : obtenu {got:?} attendu {want:?}") }));
    }
    fn check_bool(&mut self, name: &str, ok: bool) {
        self.check(name, &ok.to_string(), "true");
    }
}

fn set(text: &str) {
    let a = app();
    send(a.sci, sci::SETREADONLY, 0, 0);
    send(a.sci, sci::SETEOLMODE, 2, 0); // LF pour des attendus lisibles
    send(a.sci, sci::SETTEXT, 0, cstr(text).as_ptr() as isize);
    send(a.sci, sci::SETSEL, 0, 0);
}
fn get() -> String {
    let a = app();
    let n = send(a.sci, sci::GETLENGTH, 0, 0) as usize;
    let mut b = vec![0u8; n + 1];
    send(a.sci, sci::GETTEXT, n + 1, b.as_mut_ptr() as isize);
    b.truncate(n);
    String::from_utf8_lossy(&b).into_owned()
}
fn sel_all() {
    let a = app();
    send(a.sci, sci::SELECTALL, 0, 0);
}
fn lang(name: &str) {
    let i = LANGS.iter().position(|l| l.name == name).expect("langage inconnu");
    cmds::on_command(LANG + i);
}
fn caret_line() -> isize {
    let a = app();
    send(a.sci, sci::LINEFROMPOS, send(a.sci, sci::GETCURRENTPOS, 0, 0) as usize, 0)
}
fn goto_line(l: usize) {
    send(app().sci, sci::GOTOLINE, l, 0);
}

pub fn run(log_path: &str) -> i32 {
    let a = app();
    a.quiet = true;
    let mut t = T { log: Vec::new(), fails: 0 };
    lang("Texte brut");

    // ----- lignes -----
    set("b\nA\nc");
    sel_all();
    cmds::on_command(SORTASC);
    t.check("trier A→Z", &get(), "A\nb\nc");
    cmds::on_command(SORTDESC);
    t.check("trier Z→A", &get(), "c\nb\nA");
    set("a\nb\na\nc\nb");
    cmds::on_command(DEDUPE);
    t.check("supprimer les doublons (sans sélection = tout)", &get(), "a\nb\nc");
    set("a  \nb\t \nc");
    cmds::on_command(TRIM);
    t.check("espaces en fin de ligne", &get(), "a\nb\nc");
    set("a\n b \n\nc");
    cmds::on_command(JOIN);
    t.check("joindre les lignes", &get(), "a b c");
    set("l1\nl2\nl3\nl4");
    send(a.sci, sci::SETSEL, 3, 8); // sélection l2..l3
    cmds::on_command(SORTDESC);
    t.check("trier seulement la sélection", &get(), "l1\nl3\nl2\nl4");
    set("a\r\nb\r\nc");
    send(a.sci, sci::SETEOLMODE, 0, 0);
    sel_all();
    cmds::on_command(SORTDESC);
    t.check("fins de ligne CRLF conservées", &get(), "c\r\nb\r\na");

    set("1\n2\n3");
    goto_line(1);
    cmds::on_command(MOVEUP);
    t.check("déplacer la ligne vers le haut", &get(), "2\n1\n3");
    goto_line(0);
    cmds::on_command(MOVEDOWN);
    t.check("déplacer la ligne vers le bas", &get(), "1\n2\n3");
    set("a\nb");
    cmds::on_command(DUPLINE);
    t.check("dupliquer la ligne", &get(), "a\na\nb");
    cmds::on_command(DELLINE);
    t.check("supprimer la ligne", &get(), "a\nb");

    // ----- casse et espaces -----
    set("hello wORLD");
    sel_all();
    cmds::on_command(TITLECASE);
    t.check("Première Lettre En Majuscule", &get(), "Hello World");
    sel_all();
    cmds::on_command(UPPER);
    t.check("MAJUSCULES", &get(), "HELLO WORLD");
    sel_all();
    cmds::on_command(LOWER);
    t.check("minuscules", &get(), "hello world");
    set("\ta\n\t\tb");
    cmds::on_command(TABS2SPACES);
    t.check("tabulations → espaces", &get(), "    a\n        b");
    cmds::on_command(SPACES2TABS);
    t.check("espaces → tabulations", &get(), "\ta\n\t\tb");

    // ----- commentaires -----
    lang("Rust");
    set("x\n  y");
    cmds::on_command(TOGGLECOMMENT);
    t.check("commenter (Rust)", &get(), "// x\n  // y");
    cmds::on_command(TOGGLECOMMENT);
    t.check("décommenter (Rust)", &get(), "x\n  y");
    lang("Python");
    set("a = 1");
    cmds::on_command(TOGGLECOMMENT);
    t.check("commenter (Python)", &get(), "# a = 1");
    lang("Texte brut");

    // ----- multi-curseurs -----
    set("foo bar foo baz foo");
    send(a.sci, sci::SETSEL, 0, 3);
    cmds::on_command(SELNEXT);
    cmds::on_command(SELNEXT);
    t.log.push(format!("debug multisel={} typing={} sel0={}..{}", send(a.sci, 2564, 0, 0), send(a.sci, 2566, 0, 0), send(a.sci, sci::GETSELSTART, 0, 0), send(a.sci, sci::GETSELEND, 0, 0)));
    t.check("multi-curseurs : 3 sélections", &send(a.sci, 2570, 0, 0).to_string(), "3");
    unsafe { SendMessageW(a.sci, WM_CHAR, 'X' as usize, 0) };
    t.check("multi-curseurs : saisie sur 3 occurrences", &get(), "X bar X baz X");

    // ----- signets -----
    set("l0\nl1\nl2\nl3\nl4");
    goto_line(2);
    cmds::on_command(BOOKTOGGLE);
    goto_line(0);
    cmds::on_command(BOOKNEXT);
    t.check("signet suivant", &caret_line().to_string(), "2");
    goto_line(4);
    cmds::on_command(BOOKPREV);
    t.check("signet précédent", &caret_line().to_string(), "2");
    cmds::on_command(BOOKCLEAR);
    t.check("effacer les signets", &send(a.sci, sci::MARKERGET, 2, 0).to_string(), "0");

    // ----- repli -----
    lang("Rust");
    set("fn a() {\n    x;\n}\n");
    send(a.sci, sci::COLOURISE, 0, -1);
    cmds::on_command(FOLDALL);
    t.check("tout replier : la ligne 2 devient invisible", &send(a.sci, 2228, 1, 0).to_string(), "0");
    cmds::on_command(UNFOLDALL);
    t.check("tout déplier", &send(a.sci, 2228, 1, 0).to_string(), "1");
    lang("Texte brut");

    // ----- recherche / remplacement -----
    set("aAa 12 b345");
    unsafe {
        SetWindowTextW(a.find_edit, w("a").as_ptr());
        SetWindowTextW(a.repl_edit, w("z").as_ptr());
    }
    a.f_case = false;
    findbar::open(true);
    t.check("compteur de résultats (insensible à la casse)", &a.find_count.to_string(), "3");
    findbar::replace_all();
    t.check("tout remplacer (insensible à la casse)", &get(), "zzz 12 b345");
    set("aAa");
    a.f_case = true;
    findbar::replace_all();
    t.check("tout remplacer (sensible à la casse)", &get(), "zAz");
    set("a1 b22");
    a.f_case = false;
    a.f_regex = true;
    unsafe {
        SetWindowTextW(a.find_edit, w("(\\d+)").as_ptr());
        SetWindowTextW(a.repl_edit, w("<\\1>").as_ptr());
    }
    t.log.push(format!("debug regex {}", findbar::debug_state()));
    findbar::replace_all();
    t.check("regex avec groupe de capture", &get(), "a<1> b<22>");
    a.f_regex = false;
    set("un deux un");
    unsafe {
        SetWindowTextW(a.find_edit, w("un").as_ptr());
        SetWindowTextW(a.repl_edit, w("1").as_ptr());
    }
    send(a.sci, sci::SETSEL, 0, 0);
    findbar::replace_one();
    t.check("remplacer une occurrence puis passer à la suivante", &get(), "un deux un");
    findbar::replace_one();
    t.check("remplacer l'occurrence sélectionnée", &get(), "1 deux un");
    findbar::close();

    // ----- recherche dans un dossier / ouverture rapide / liens -----
    let dir = std::env::temp_dir().join(format!("wn_selftest_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("sous")).unwrap();
    std::fs::write(dir.join("a.txt"), "bonjour monde\nsalut").unwrap();
    std::fs::write(dir.join("sous").join("b.rs"), "// Bonjour\nfn x() {}").unwrap();
    std::fs::write(dir.join("c.bin"), b"bonjour\0\0\0").unwrap();
    a.root = Some(dir.to_string_lossy().to_string());
    t.check("recherche dans le dossier (fichier binaire ignoré)", &palette::selftest_search("bonjour").to_string(), "2");
    let quick = palette::selftest_quick("b.rs");
    t.check_bool("ouverture rapide trouve sous/b.rs", quick.first().is_some_and(|f| f.ends_with("b.rs")));

    set("voir [[note-x]] ici");
    send(a.sci, sci::SETSEL, 8, 8);
    let before = a.docs.len();
    let base = dir.join("base.md");
    a.docs[a.cur].path = Some(base.to_string_lossy().to_string());
    cmds::on_command(OPENLINK);
    t.check("lien [[note-x]] : nouvel onglet créé", &(a.docs.len() - before).to_string(), "1");
    t.check_bool("lien [[note-x]] : chemin note-x.md à côté du fichier", a.docs[a.cur].path.as_deref().is_some_and(|p| p.ends_with("note-x.md")));

    // ----- enregistrement / réouverture avec encodages -----
    for (i, enc) in crate::doc::Enc::ALL.iter().enumerate() {
        let p = dir.join(format!("enc{i}.txt")).to_string_lossy().to_string();
        cmds::open_or_create(&p);
        let src = if *enc == crate::doc::Enc::Ansi { "héllo wörld\nligne 2" } else { "héllo wörld €\nligne 2" };
        set(src);
        a.docs[a.cur].enc = *enc;
        let idx = a.cur;
        cmds::save_doc(a, idx, false);
        cmds::cmd_close(a.cur);
        cmds::open_path(&p);
        let got = get();
        t.check(&format!("enregistrer puis rouvrir en {}", enc.name()), &got.replace('\r', ""), src);
        t.check_bool(&format!("encodage {} redétecté", enc.name()), a.docs[a.cur].enc == *enc || (*enc == crate::doc::Enc::Utf8 && a.docs[a.cur].enc == crate::doc::Enc::Utf8) || (*enc == crate::doc::Enc::Ansi && a.docs[a.cur].enc == crate::doc::Enc::Ansi));
    }

    // ----- indentation automatique -----
    cmds::new_doc(a);
    set("    abc");
    send(a.sci, sci::SETSEL, 7, 7);
    unsafe { SendMessageW(a.sci, WM_KEYDOWN, 0x0D, 0) };
    unsafe { SendMessageW(a.sci, WM_CHAR, 0x0D, 0) };
    send(a.sci, sci::SETSEL, send(a.sci, sci::GETCURRENTPOS, 0, 0) as usize, send(a.sci, sci::GETCURRENTPOS, 0, 0));
    t.check("indentation automatique après Entrée", &send(a.sci, 2127, 1, 0).to_string(), "4");

    // ----- fins de ligne -----
    set("a\r\nb");
    cmds::on_command(EOL + 2);
    t.check("convertir en LF", &get(), "a\nb");
    cmds::on_command(EOL);
    t.check("convertir en CRLF", &get(), "a\r\nb");

    let _ = std::fs::remove_dir_all(&dir);
    t.log.push(format!("{} échec(s)", t.fails));
    let _ = std::fs::write(log_path, t.log.join("\n"));
    t.fails as i32
}
