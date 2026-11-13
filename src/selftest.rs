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
    for (i, enc) in crate::doc::Enc::ALL.iter().enumerate().take(5) {
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

    // ----- correcteur orthographique -----
    {
        let a = app();
        lang("Texte brut");
        a.spell = true;
        a.spell_lang = "fr-FR".into();
        if crate::spell::init("fr-FR") {
            set("Bonjour le mondde et la maison");
            crate::spell::check_visible();
            let pos = get().find("mondde").unwrap() as usize;
            t.check("orthographe : mot inconnu souligné", &send(a.sci, 2507, 9, pos as isize + 2).to_string(), "1");
            t.check("orthographe : mot correct non souligné", &send(a.sci, 2507, 9, 2).to_string(), "0");
            let sug = crate::spell::prepare_at(pos as isize + 2);
            t.check_bool("orthographe : suggestion « monde »", sug.iter().any(|s| s == "monde"));
            if let Some(i) = sug.iter().position(|s| s == "monde") {
                crate::spell::apply_fix(i);
                t.check("orthographe : correction appliquée", &get(), "Bonjour le monde et la maison");
            }
        } else {
            t.log.push("SKIP orthographe : dictionnaire fr-FR non installé".into());
        }
    }

    // ----- vue scindée -----
    cmds::on_command(SPLIT);
    t.check("vue scindée : même document dans les deux vues", &(send(a.sci2, 2357, 0, 0) == send(a.sci, 2357, 0, 0)).to_string(), "true");
    cmds::on_command(SPLIT);
    t.check("vue scindée : désactivable", &a.split.to_string(), "false");

    // ----- liens entrants -----
    let bl = dir.join("liens");
    std::fs::create_dir_all(&bl).unwrap();
    std::fs::write(bl.join("cible.md"), "# Cible").unwrap();
    std::fs::write(bl.join("a.md"), "voir [[cible]] ici").unwrap();
    std::fs::write(bl.join("b.md"), "texte [[Cible|alias]]
autre").unwrap();
    std::fs::write(bl.join("c.md"), "rien").unwrap();
    cmds::open_path(&bl.join("cible.md").to_string_lossy());
    t.check("liens entrants trouvés (a.md et b.md)", &crate::sidebar::selftest_backlinks().to_string(), "2");

    // ----- indentation automatique -----
    cmds::new_doc(a);
    set("    abc");
    send(a.sci, sci::SETSEL, 7, 7);
    unsafe { SendMessageW(a.sci, WM_KEYDOWN, 0x0D, 0) };
    unsafe { SendMessageW(a.sci, WM_CHAR, 0x0D, 0) };
    send(a.sci, sci::SETSEL, send(a.sci, sci::GETCURRENTPOS, 0, 0) as usize, send(a.sci, sci::GETCURRENTPOS, 0, 0));
    t.check("indentation automatique après Entrée", &send(a.sci, 2127, 1, 0).to_string(), "4");

    // ----- Markdown en direct -----
    cmds::new_doc(a);
    lang("Markdown");
    set("# Titre\n**gras** et *it* et `c`\n- x");
    crate::mdlive::style_doc();
    let sty = |p: usize| send(a.sci, 2010, p, 0).to_string();
    t.check("Markdown direct : titre H1", &sty(3), "1");
    t.check("Markdown direct : gras", &sty(12), "7");
    t.check("Markdown direct : italique", &sty(21), "8");
    t.check("Markdown direct : code", &sty(29), "10");
    t.check("Markdown direct : puce", &sty(32), "15");
    cmds::new_doc(a);
    lang("Texte brut");
    set("bonjour\n\n## Un titre\n");
    crate::mdlive::on_timer();
    t.check("document sans nom : passe seul en Markdown", LANGS[a.docs[a.cur].lang].name, "Markdown");

    // ----- commandes de mise en forme Markdown -----
    let md = |id: usize, text: &str, s0: usize, s1: usize| {
        set(text);
        send(a.sci, sci::SETSEL, s0, s1 as isize);
        cmds::on_command(id);
        get()
    };
    t.check("Markdown : gras", &md(MD_BOLD, "un mot ici", 3, 6), "un **mot** ici");
    t.check("Markdown : gras retiré", &md(MD_BOLD, "un **mot** ici", 5, 8), "un mot ici");
    t.check("Markdown : italique sans sélection", &md(MD_ITALIC, "ab", 1, 1), "a**b");
    t.check("Markdown : titre 2", &md(MD_H1 + 1, "titre", 0, 0), "## titre");
    t.check("Markdown : liste à puces (2 lignes)", &md(MD_BULLET, "a\nb", 0, 3), "- a\n- b");
    t.check("Markdown : liste numérotée", &md(MD_NUMLIST, "a\nb", 0, 3), "1. a\n2. b");
    t.check("Markdown : lien", &md(MD_LINK, "site", 0, 4), "[site](adresse)");
    t.check("Markdown : bloc de code", &md(MD_CODEBLOCK, "x", 0, 1), "```\nx\n```");
    set("- a");
    send(a.sci, sci::SETSEL, 3, 3);
    unsafe { SendMessageW(a.sci, WM_KEYDOWN, 0x0D, 0) };
    unsafe { SendMessageW(a.sci, WM_CHAR, 0x0D, 0) };
    t.check("Markdown : Entrée continue la liste", &get().replace('\r', ""), "- a\n- ");
    unsafe { SendMessageW(a.sci, WM_KEYDOWN, 0x0D, 0) };
    unsafe { SendMessageW(a.sci, WM_CHAR, 0x0D, 0) };
    t.check("Markdown : Entrée sur puce vide termine la liste", &get().replace('\r', ""), "- a\n\n");

    // ----- paires automatiques + parenthèses assorties (en langage de code) -----
    lang("Rust");
    set("");
    let typ = |c: char| unsafe { SendMessageW(a.sci, WM_CHAR, c as usize, 0) };
    typ('(');
    t.check("paire auto : ( ferme )", &get(), "()");
    typ(')');
    t.check("paire auto : retaper ) avale le doublon", &get(), "()");
    set("");
    typ('"');
    t.check("paire auto : guillemet fermé", &get(), "\"\"");
    set("f(a)");
    send(a.sci, sci::SETSEL, 1, 1);
    crate::assist::brace_highlight(a.sci);
    t.check("parenthèses : assortie trouvée", &send(a.sci, 2353, 1, 0).to_string(), "3");
    lang("Texte brut");
    set("");
    typ('(');
    t.check("texte brut : pas de fermeture auto", &get(), "(");

    // ----- recherche : mode étendu, dans la sélection, tous les onglets, historique -----
    findbar::open(true);
    a.f_regex = false;
    a.f_case = false;
    set("a\nb a\nb");
    unsafe {
        SetWindowTextW(a.find_edit, w("a\\nb").as_ptr());
        SetWindowTextW(a.repl_edit, w("X").as_ptr());
    }
    crate::findx::set_ext(true);
    findbar::replace_all();
    t.check("mode étendu : \\n remplacé", &get(), "X X");
    crate::findx::set_ext(false);
    set("a a a a");
    send(a.sci, sci::SETSEL, 2, 5);
    crate::findx::set_sel(Some((2, 5)));
    unsafe {
        SetWindowTextW(a.find_edit, w("a").as_ptr());
        SetWindowTextW(a.repl_edit, w("bb").as_ptr());
    }
    findbar::replace_all();
    t.check("remplacer dans la sélection seulement", &get(), "a bb bb a");
    crate::findx::set_sel(None);
    cmds::new_doc(a);
    set("a");
    cmds::new_doc(a);
    set("a a");
    unsafe { SetWindowTextW(a.repl_edit, w("c").as_ptr()) };
    findbar::replace_all_tabs();
    t.check("tous les onglets : onglet courant", &get(), "c c");
    cmds::switch_to(a, a.cur - 1);
    t.check("tous les onglets : onglet précédent", &get(), "c");
    findbar::close();
    // ----- fichier modifié sur le disque : rechargement automatique + suivi de fin -----
    let ext = dir.join("externe.txt");
    std::fs::write(&ext, "un").ok();
    cmds::open_path(&ext.to_string_lossy());
    t.check("fichier ouvert", &get(), "un");
    std::thread::sleep(std::time::Duration::from_millis(30));
    std::fs::write(&ext, "un\ndeux").ok();
    cmds::check_external();
    t.check("rechargement automatique d'un fichier modifié sur le disque", &get(), "un\ndeux");
    cmds::on_command(FOLLOW);
    std::thread::sleep(std::time::Duration::from_millis(30));
    std::fs::write(&ext, "un\ndeux\ntrois").ok();
    cmds::check_external();
    t.check("suivi de fin : curseur en fin de fichier", &send(a.sci, sci::GETCURRENTPOS, 0, 0).to_string(), "13");
    cmds::on_command(FOLLOW);

    // ----- encodages : recharger en cyrillique, enregistrer sans perte -----
    let cyr = dir.join("cyr.txt");
    std::fs::write(&cyr, crate::doc::encode("Привет мир".as_bytes(), crate::doc::Enc::Win1251)).ok();
    cmds::open_path(&cyr.to_string_lossy());
    cmds::on_command(RENC + 6);
    t.check("recharger avec l'encodage Cyrillique (Windows-1251)", &get(), "Привет мир");
    t.check_bool("encodage mémorisé pour l'enregistrement", a.docs[a.cur].enc == crate::doc::Enc::Win1251);
    t.check_bool("caractères sans perte en Windows-1251", crate::doc::lossless(get().as_bytes(), crate::doc::Enc::Win1251));
    t.check_bool("perte détectée en ANSI (cyrillique)", !crate::doc::lossless(get().as_bytes(), crate::doc::Enc::Ansi));

    // ----- autocomplétion des mots + indentation par langage -----
    lang("Rust");
    set("foobar foobaz\n");
    send(a.sci, sci::GOTOPOS, 14, 0);
    for c in "foo".chars() {
        unsafe { SendMessageW(a.sci, WM_CHAR, c as usize, 0) };
    }
    t.check("autocomplétion : liste affichée après 3 lettres", &send(a.sci, 2102, 0, 0).to_string(), "1");
    send(a.sci, 2101, 0, 0); // AUTOCCANCEL
    lang("Python");
    t.check("indentation Python : 4 espaces", &format!("{} {}", send(a.sci, 2121, 0, 0), send(a.sci, 2125, 0, 0)), "4 0");
    lang("YAML");
    t.check("indentation YAML : 2 espaces", &format!("{} {}", send(a.sci, 2121, 0, 0), send(a.sci, 2125, 0, 0)), "2 0");
    lang("Makefile");
    t.check("indentation Makefile : tabulations", &send(a.sci, 2125, 0, 0).to_string(), "1");
    cmds::on_command(INDENT_AUTO + 3);
    t.check("indentation forcée : 8 espaces", &format!("{} {}", send(a.sci, 2121, 0, 0), send(a.sci, 2125, 0, 0)), "8 0");
    cmds::on_command(INDENT_AUTO);
    lang("Texte brut");

    // ----- saisie par la palette (renommer / nouveau fichier de l'explorateur) + opérations sur fichiers -----
    thread_local! { static GOT: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) }; }
    crate::palette::input("Nom ?", "ab", Box::new(|s| GOT.with(|g| *g.borrow_mut() = s)));
    unsafe {
        SendMessageW(a.palette, WM_CHAR, 'c' as usize, 0);
        SendMessageW(a.palette, WM_KEYDOWN, 0x0D, 0);
    }
    t.check("palette en saisie de texte : valeur validée", &GOT.with(|g| g.borrow().clone()), "abc");
    let fo = dir.join("fo");
    std::fs::create_dir_all(&fo).ok();
    let made = crate::fileops::new_file(&fo.to_string_lossy(), "n.md");
    t.check_bool("nouveau fichier créé", made.as_ref().is_ok_and(|p| std::path::Path::new(p).exists()));
    t.check_bool("nouveau fichier : nom déjà pris refusé", crate::fileops::new_file(&fo.to_string_lossy(), "n.md").is_err());
    let renamed = crate::fileops::rename(&made.unwrap_or_default(), "m.md");
    t.check_bool("renommer", renamed.as_ref().is_ok_and(|p| p.ends_with("m.md") && std::path::Path::new(p).exists()));
    t.check_bool("dupliquer", crate::fileops::duplicate(&renamed.unwrap_or_default()).is_ok_and(|p| p.ends_with("m copie.md")));

    // ----- macros, insertion en colonne, lignes supplémentaires, guides d'indentation -----
    lang("Texte brut");
    set("a\nb\nc");
    send(a.sci, sci::GOTOPOS, 0, 0);
    cmds::on_command(MACRO_REC);
    send(a.sci, 2170, 0, b"X\0".as_ptr() as isize); // REPLACESEL
    send(a.sci, 2300, 0, 0); // LINEDOWN
    send(a.sci, 2312, 0, 0); // HOME
    cmds::on_command(MACRO_REC);
    crate::macros::play(a.sci, 2);
    t.check("macro enregistrée puis rejouée 2 fois", &get(), "Xa\nXb\nXc");
    set("a\nb\nc");
    send(a.sci, sci::SETSEL, 0, 5);
    crate::textops::column_insert("{n}. ");
    t.check("insertion en colonne avec numéros", &get(), "1. a\n2. b\n3. c");
    set("ccc\na\nbb");
    cmds::on_command(LOP);
    t.check("trier par longueur", &get(), "a\nbb\nccc");
    set("a\n\nb\n");
    cmds::on_command(LOP + 4);
    t.check("supprimer les lignes vides", &get(), "a\nb");
    cmds::on_command(GUIDES);
    t.check("guides d'indentation activés", &send(a.sci, 2133, 0, 0).to_string(), "3");
    cmds::on_command(GUIDES);
    cmds::on_command(SHOWEOL);
    t.check("fins de ligne visibles", &send(a.sci, 2355, 0, 0).to_string(), "1");
    cmds::on_command(SHOWEOL);

    // ----- tous les langages : lexer chargé + au moins un octet coloré sur un échantillon -----
    for (i, l) in LANGS.iter().enumerate() {
        if l.lexer.is_empty() || l.lexer == "markdown" {
            continue;
        }
        cmds::new_doc(a);
        lang(l.name);
        let kw = l.kw.iter().flat_map(|k| k.split_whitespace()).next().unwrap_or("x");
        set(&format!("{kw} a = 42 \"s\" 'c' // c\n# c\n-- c\n/* c */ {kw}\n<a b=\"c\"> [s] x: 1\n\\section{{a}} % c\n"));
        send(a.sci, sci::COLOURISE, 0, -1);
        let n = send(a.sci, sci::GETLENGTH, 0, 0) as usize;
        let styled = (0..n).any(|p| send(a.sci, 2010, p, 0) != 0);
        t.check_bool(&format!("langage {i} « {} » : coloré", l.name), styled);
    }

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
