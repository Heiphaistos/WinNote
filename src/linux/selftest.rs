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
    set("x = 1");
    on_command(BLOCKCOMM);
    t.check("commentaire de bloc", &get(), "/* x = 1 */");
    on_command(BLOCKCOMM);
    t.check("retrait du commentaire de bloc", &get(), "x = 1");
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
    for (i, enc) in Enc::ALL.iter().enumerate().take(5) {
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
    // recherche : mode étendu, dans la sélection, tous les onglets
    set("a\nb a\nb");
    a.find.case.set_active(false);
    a.find.find.set_text("a\nb");
    a.find.repl.set_text("X");
    a.find.ext.set_active(true);
    find::replace_all();
    t.check("mode étendu : \n remplacé", &get(), "X X");
    a.find.ext.set_active(false);
    set("a a a a");
    a.find.find.set_text("a");
    a.find.repl.set_text("bb");
    send(a.sci, sci::SETSEL, 2, 5);
    crate::findx::set_pending(Some((2, 5)));
    a.find.sel.set_active(true);
    find::replace_all();
    t.check("remplacer dans la sélection seulement", &get(), "a bb bb a");
    a.find.sel.set_active(false);
    edit::new_tab(None, None, false, 0);
    set("a");
    edit::new_tab(None, None, false, 0);
    set("a a");
    a.find.repl.set_text("c");
    find::replace_all_tabs();
    t.check("tous les onglets : onglet courant", &get(), "c c");
    edit::switch_to(a.cur - 1);
    t.check("tous les onglets : onglet précédent", &get(), "c");
    // fichier modifié sur le disque + suivi de fin
    let extp = std::env::temp_dir().join(format!("wn_ext_{}.txt", std::process::id()));
    std::fs::write(&extp, "un").ok();
    edit::open_path(&extp.to_string_lossy());
    std::thread::sleep(std::time::Duration::from_millis(30));
    std::fs::write(&extp, "un\ndeux").ok();
    edit::check_external();
    t.check("rechargement automatique d'un fichier modifié", &get(), "un\ndeux");
    on_command(FOLLOW);
    std::thread::sleep(std::time::Duration::from_millis(30));
    std::fs::write(&extp, "un\ndeux\ntrois").ok();
    edit::check_external();
    t.check("suivi de fin : curseur en fin de fichier", &send(a.sci, sci::GETCURRENTPOS, 0, 0).to_string(), "13");
    on_command(FOLLOW);
    let _ = std::fs::remove_file(&extp);
    // recharger avec un encodage
    let cyr = std::env::temp_dir().join(format!("wn_cyr_{}.txt", std::process::id()));
    std::fs::write(&cyr, crate::doc::encode("Привет мир".as_bytes(), Enc::Win1251)).ok();
    edit::open_path(&cyr.to_string_lossy());
    on_command(RENC + 6);
    t.check("recharger avec l'encodage Cyrillique", &get(), "Привет мир");
    let _ = std::fs::remove_file(&cyr);
    // autocomplétion + indentation par langage
    lang("Rust");
    set("foobar foobaz\n");
    send(a.sci, sci::GOTOPOS, 14, 0);
    for ch in b"foo" {
        send(a.sci, sci::ADDTEXT, 1, ch as *const u8 as isize);
        crate::assist::on_char(a.sci, *ch, true, false);
    }
    t.check("autocomplétion : liste affichée après 3 lettres", &send(a.sci, 2102, 0, 0).to_string(), "1");
    send(a.sci, 2101, 0, 0);
    lang("Python");
    t.check("indentation Python : 4 espaces", &format!("{} {}", send(a.sci, 2121, 0, 0), send(a.sci, 2125, 0, 0)), "4 0");
    lang("YAML");
    t.check("indentation YAML : 2 espaces", &format!("{} {}", send(a.sci, 2121, 0, 0), send(a.sci, 2125, 0, 0)), "2 0");
    lang("Makefile");
    t.check("indentation Makefile : tabulations", &send(a.sci, 2125, 0, 0).to_string(), "1");
    on_command(INDENT_AUTO + 3);
    t.check("indentation forcée : 8 espaces", &format!("{} {}", send(a.sci, 2121, 0, 0), send(a.sci, 2125, 0, 0)), "8 0");
    on_command(INDENT_AUTO);
    lang("Texte brut");
    // macros, colonne, lignes, guides
    lang("Texte brut");
    set("a\nb\nc");
    send(a.sci, sci::GOTOPOS, 0, 0);
    on_command(MACRO_REC);
    send(a.sci, 2170, 0, b"X\0".as_ptr() as isize);
    send(a.sci, 2300, 0, 0);
    send(a.sci, 2312, 0, 0);
    on_command(MACRO_REC);
    crate::macros::play(a.sci, 2);
    t.check("macro enregistrée puis rejouée 2 fois", &get(), "Xa\nXb\nXc");
    set("a\nb\nc");
    send(a.sci, sci::SETSEL, 0, 5);
    crate::textops::column_insert("{n}. ");
    t.check("insertion en colonne avec numéros", &get(), "1. a\n2. b\n3. c");
    set("ccc\na\nbb");
    on_command(LOP);
    t.check("trier par longueur", &get(), "a\nbb\nccc");
    set("a\n\nb\n");
    on_command(LOP + 4);
    t.check("supprimer les lignes vides", &get(), "a\nb");
    on_command(GUIDES);
    t.check("guides d'indentation activés", &send(a.sci, 2133, 0, 0).to_string(), "3");
    on_command(GUIDES);
    set("foo bar foo baz foo");
    send(a.sci, sci::SETSEL, 0, 3);
    crate::assist::smart_highlight(a.sci);
    t.check("occurrences surlignées (hors sélection)", &format!("{}{}{}", send(a.sci, 2507, 10, 0), send(a.sci, 2507, 10, 8), send(a.sci, 2507, 10, 16)), "011");
    t.check("historique des modifications actif", &send(a.sci, 2781, 0, 0).to_string(), "3");
    let n1 = a.docs.len();
    on_command(PINTAB);
    t.check("épinglé : passe en tête", &format!("{}{}", app().docs[0].pinned, app().cur), "true0");
    edit::close_tab(0);
    t.check("épinglé : ne se ferme pas", &app().docs.len().to_string(), &n1.to_string());
    on_command(PINTAB);
    t.check("désépinglé", &app().docs[app().cur].pinned.to_string(), "false");
    // liens [[note]]
    set("voir [[note-x]] ici");
    send(a.sci, sci::SETSEL, 8, 8);
    a.docs[a.cur].path = Some(dir.join("base.md").to_string_lossy().to_string());
    let before = a.docs.len();
    on_command(OPENLINK);
    t.check("lien [[note-x]] : nouvel onglet", &(a.docs.len() - before).to_string(), "1");
    // ---- panneau latéral, aperçu, vue scindée, lecture seule
    lang("Markdown");
    set("# A
## B
texte
# C");
    crate::side::show(1);
    t.check("plan : 3 titres", &crate::side::row_count().to_string(), "3");
    std::fs::write(dir.join("cible.md"), "x").ok();
    std::fs::write(dir.join("autre.md"), "voir [[cible]] ici
").ok();
    std::fs::write(dir.join("rien.md"), "rien
").ok();
    edit::open_path(&dir.join("cible.md").to_string_lossy());
    a.root = Some(dir.to_string_lossy().to_string());
    t.check("liens entrants : 1 note", &crate::side::backlink_count().to_string(), "1");
    lang("Markdown");
    set("# Titre
**gras** et `code`
");
    a.preview = true;
    crate::view::update_preview();
    let n = send(a.prev, sci::GETLENGTH, 0, 0) as usize;
    let mut b = vec![0u8; n + 1];
    send(a.prev, sci::GETTEXT, n + 1, b.as_mut_ptr() as isize);
    t.check("aperçu Markdown rendu", String::from_utf8_lossy(&b[..n]).as_ref(), "Titre
gras et code
");
    a.preview = false;
    a.split = true;
    crate::view::sync_split();
    t.check("vue scindée : même document", &(send(a.sci2, 2357, 0, 0) == send(a.sci, 2357, 0, 0)).to_string(), "true");
    a.split = false;
    on_command(READONLY);
    t.check("lecture seule", &send(a.sci, sci::GETREADONLY, 0, 0).to_string(), "1");
    on_command(READONLY);
    // ---- orthographe (nécessite Enchant + dictionnaire fr_FR)
    lang("Texte brut");
    set("Bonjour le mondde");
    a.spell = true;
    if crate::spell::init("fr_FR") {
        a.spell_lang = "fr_FR".into();
        crate::spell::check_visible();
        t.check("orthographe : 1 mot inconnu", &crate::spell::error_count().to_string(), "1");
        let sug = crate::spell::prepare_at(14);
        t.check("orthographe : suggestion « monde »", &sug.iter().any(|s| s == "monde").to_string(), "true");
        crate::spell::apply_fix(sug.iter().position(|s| s == "monde").unwrap_or(0));
        t.check("orthographe : correction appliquée", &get(), "Bonjour le monde");
    } else {
        t.log.push("SKIP orthographe (Enchant ou dictionnaire fr_FR absent)".into());
    }
    a.spell = false;
    crate::spell::check_visible();
    // ---- propriétés YAML, liste des fonctions, favoris, enregistrer une copie, mise en page, récupération
    let cfg = dir.join("cfg");
    crate::prefs::init(cfg.clone());
    lang("Markdown");
    set("---
titre: Un
tags:
  - a
  - b
---
# Corps
");
    t.check("propriétés YAML : 2 lues (+ ligne d'ajout)", &crate::features::prop_items().len().to_string(), "3");
    crate::features::set_property("titre: Deux");
    t.check("propriété modifiée", &get(), "---
titre: Deux
tags:
  - a
  - b
---
# Corps
");
    crate::features::set_property("auteur: Momo");
    t.check("propriété ajoutée", &get().contains("auteur: Momo
---
# Corps").to_string(), "true");
    send(a.sci, sci::UNDO, 0, 0);
    send(a.sci, sci::UNDO, 0, 0);
    t.check("propriétés : annulables d'un seul geste chacune", &get(), "---
titre: Un
tags:
  - a
  - b
---
# Corps
");
    crate::features::set_property("titre:");
    t.check("propriété vide = retirée", &(!get().contains("titre:") && get().contains("tags:")).to_string(), "true");
    lang("Rust");
    set("fn a() {
}
struct S;
fn b() {
}
");
    let sy = crate::features::symbols();
    t.check("liste des fonctions : 3 symboles", &sy.len().to_string(), "3");
    t.check("liste des fonctions : « fn b » ligne 4", &format!("{}", sy.last().map_or(0, |s| s.2 + 1)), "4");
    let fav_file = dir.join("fav.txt");
    std::fs::write(&fav_file, "x").ok();
    edit::open_path(&fav_file.to_string_lossy());
    on_command(FAVTOGGLE);
    t.check("favori ajouté", &crate::features::favs().len().to_string(), "1");
    on_command(FAVTOGGLE);
    t.check("favori retiré", &crate::features::favs().len().to_string(), "0");
    edit::close_tab(app().cur);
    lang("Texte brut");
    set("contenu copie");
    app().docs[app().cur].dirty = true;
    let (p0, copy) = (app().docs[app().cur].path.clone(), dir.join("copie.txt"));
    t.check("enregistrer une copie : écrite", &(crate::features::write_copy(&copy.to_string_lossy()).is_ok() && std::fs::read_to_string(&copy).is_ok_and(|c| c == "contenu copie")).to_string(), "true");
    t.check("enregistrer une copie : onglet inchangé", &(app().docs[app().cur].path == p0 && app().docs[app().cur].dirty).to_string(), "true");
    let lay = crate::extras::PrintLayout { margin_mm: 15, header: "{fichier}".into(), footer: "||{page}".into() };
    t.check("mise en page : enregistrée puis relue", &(lay.save(&cfg).is_ok() && crate::extras::PrintLayout::load(&crate::prefs::dir()) == lay).to_string(), "true");
    {
        // boîte GTK de mise en page : champs remplis, lecture, séparateur de zone neutralisé
        let (d, f) = crate::features::page_dialog(&lay);
        t.check("mise en page : boîte remplie", &format!("{:?}", crate::features::page_read(&f)), "PrintLayout { margin_mm: 15, header: \"{fichier}||\", footer: \"||{page}\" }");
        f.margin.set_value(30.0);
        f.head[1].set_text("Titre; v2|x");
        f.foot[2].set_text("{page}");
        let l2 = crate::features::page_read(&f);
        t.check("mise en page : boîte lue", &format!("{l2:?}"), "PrintLayout { margin_mm: 30, header: \"{fichier}|Titre; v2x|\", footer: \"||{page}\" }");
        t.check("mise en page : « ; » relu depuis le fichier", &(l2.save(&cfg).is_ok() && crate::extras::PrintLayout::load(&cfg) == l2).to_string(), "true");
        d.close();
    }
    set("secret perdu");
    app().docs[app().cur].dirty = true;
    crate::features::recovery_snapshot();
    set("autre texte");
    // une autre fenêtre disparue (pid inexistant) a laissé verrou + copie ; notre propre verrou et nos copies ne comptent pas
    let dead = 4_294_967_000u32;
    let ddir = cfg.join("recovery").join(dead.to_string());
    std::fs::create_dir_all(&ddir).ok();
    std::fs::write(cfg.join(format!("running-{dead}.lock")), dead.to_string()).ok();
    std::fs::write(ddir.join("0.txt"), "perdu par l'autre fenêtre").ok();
    std::fs::write(ddir.join("index.txt"), "0\t0\t\tAutre fenêtre\n").ok();
    let nd = app().docs.len();
    crate::features::recovery_start(true);
    t.check("récupération après arrêt brutal : un onglet ajouté (nos copies exclues)", &(app().docs.len() - nd).to_string(), "1");
    t.check("récupération : texte de l'autre fenêtre retrouvé", &get(), "perdu par l'autre fenêtre");
    t.check("récupération : traces de l'autre fenêtre effacées après reprise", &(!cfg.join(format!("running-{dead}.lock")).exists() && !ddir.exists()).to_string(), "true");
    let nd = app().docs.len();
    on_command(RECOVER);
    t.check("commande « Restaurer » : rien à reprendre", &(app().docs.len() - nd).to_string(), "0");
    std::fs::create_dir_all(&ddir).ok();
    std::fs::write(ddir.join("0.txt"), "deuxième disparition").ok();
    std::fs::write(ddir.join("index.txt"), "0\t0\t\tAutre 2\n").ok();
    on_command(RECOVER);
    t.check("commande « Restaurer » : un onglet repris", &(app().docs.len() - nd).to_string(), "1");
    app().docs[app().cur].dirty = false;
    edit::close_tab(app().cur);
    app().docs[app().cur].dirty = false;
    crate::features::recovery_end();
    t.check("sortie normale : verrou et copies de CETTE fenêtre retirés", &(!cfg.join(format!("running-{}.lock", std::process::id())).exists() && !cfg.join("recovery").join(std::process::id().to_string()).exists()).to_string(), "true");
    lang("Texte brut");
    // ---- impression dans un PDF
    set(&(1..=300).map(|i| format!("ligne numero {i} avec un texte assez long pour la mise en page")).collect::<Vec<_>>().join("
"));
    let pdf = dir.join("sortie.pdf").to_string_lossy().to_string();
    let r = crate::print::export_pdf(&pdf);
    t.check("impression PDF : plusieurs pages", &r.as_ref().map_or(false, |&n| n >= 5).to_string(), "true");
    t.check("impression PDF : fichier valide", &std::fs::read(&pdf).map_or(false, |b| b.starts_with(b"%PDF")).to_string(), "true");
    // ---- mise à jour : API simulée (file://), empreinte, signature épinglée, remplacement (build test-hooks seulement)
    #[cfg(feature = "test-hooks")]
    {
        use crate::update;
        use ed25519_compact::{KeyPair, Seed};
        let kp = KeyPair::from_seed(Seed::new([9; 32]));
        let pubhex: String = kp.pk.iter().map(|b| format!("{b:02x}")).collect();
        std::env::set_var("WINNOTE_UPDATE_PUBKEY", &pubhex);
        let app_new = dir.join("nouvelle.AppImage");
        std::fs::write(&app_new, b"#!/bin/sh
echo v9
").ok();
        let sha = update::sha256_file(&app_new.to_string_lossy()).unwrap_or_default();
        let api = format!("{{\"tag_name\":\"v9.9.9\",\"assets\":[{{\"name\":\"WinNote-9.9.9-x86_64.AppImage\",\"digest\":\"sha256:{sha}\",\"browser_download_url\":\"file://{}\"}}]}}", app_new.display());
        let api_path = dir.join("api.json");
        std::fs::write(&api_path, api).ok();
        let sig_path = dir.join("nouvelle.AppImage.sig");
        let sign = |k: &KeyPair| std::fs::write(&sig_path, k.sk.sign(crate::vers::signed_msg("v9.9.9", "nouvelle.AppImage", &sha), None).as_ref()).ok();
        sign(&kp);
        std::env::set_var("WINNOTE_UPDATE_URL", format!("file://{}", api_path.display()));
        let found = update::fetch();
        t.check("mise à jour : nouvelle version détectée", &found.as_ref().map_or("erreur".into(), |f| f.as_ref().map_or("aucune".into(), |f| f.version.clone())), "v9.9.9");
        let target = dir.join("WinNote.AppImage");
        std::fs::write(&target, b"ancienne").ok();
        if let Ok(Some(f)) = &found {
            t.check("mise à jour : installation (signature valide)", &update::install(f, &target.to_string_lossy()).is_ok().to_string(), "true");
            t.check("mise à jour : fichier remplacé", &std::fs::read_to_string(&target).unwrap_or_default(), "#!/bin/sh
echo v9
");
            let bad = update::Found { sha: "0".repeat(64), ..f.clone() };
            std::fs::write(&target, b"ancienne").ok();
            t.check("mise à jour : mauvaise empreinte refusée", &update::install(&bad, &target.to_string_lossy()).is_err().to_string(), "true");
            t.check("mise à jour : ancien fichier intact", &std::fs::read_to_string(&target).unwrap_or_default(), "ancienne");
            sign(&KeyPair::from_seed(Seed::new([10; 32])));
            t.check("mise à jour : signature d'une autre clé refusée", &update::install(f, &target.to_string_lossy()).is_err().to_string(), "true");
            t.check("mise à jour : fichier intact après signature fausse", &std::fs::read_to_string(&target).unwrap_or_default(), "ancienne");
            std::fs::remove_file(&sig_path).ok();
            t.check("mise à jour : signature absente refusée", &update::install(f, &target.to_string_lossy()).is_err().to_string(), "true");
            t.check("mise à jour : fichier intact sans signature", &std::fs::read_to_string(&target).unwrap_or_default(), "ancienne");
        }
        std::env::remove_var("WINNOTE_UPDATE_URL");
        std::env::remove_var("WINNOTE_UPDATE_PUBKEY");
    }
    // Markdown en direct
    lang("Markdown");
    set("# Titre\n**gras** et *it* et `c`\n- x");
    crate::mdstyle::apply(app().sci);
    let sty = |p: usize| send(app().sci, 2010, p, 0).to_string();
    t.check("Markdown direct : titre H1", &sty(3), "1");
    t.check("Markdown direct : gras", &sty(12), "7");
    t.check("Markdown direct : italique", &sty(21), "8");
    t.check("Markdown direct : code", &sty(29), "10");
    t.check("Markdown direct : puce", &sty(32), "15");
    let _ = std::fs::remove_dir_all(&dir);
    t.log.push(format!("{} échec(s)", t.fails));
    let _ = std::fs::write(log_path, t.log.join("\n"));
    t.fails as i32
}
