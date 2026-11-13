//! Propriétés YAML, favoris, enregistrer une copie, mise en page, récupération après arrêt brutal, liste des fonctions (Linux / GTK).
use crate::doc::{self, Enc};
use crate::extras::{self, PrintLayout, Recovered};
use crate::sci::{self, send};
use crate::{app, edit, lang, prefs};
use gtk::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

fn cur_text() -> String {
    String::from_utf8_lossy(&edit::text_of(app().cur)).into_owned()
}

/// Remplace le texte du document courant en ne touchant que la zone modifiée (reste annulable, garde le curseur).
pub fn apply_new_text(new: &str) {
    let a = app();
    let (s, e, r) = extras::diff_range(&cur_text(), new);
    send(a.sci, sci::SETTARGETSTART, s, 0);
    send(a.sci, sci::SETTARGETEND, e, 0);
    send(a.sci, sci::REPLACETARGET, r.len(), r.as_ptr() as isize);
}

/// Pose « clé: valeur » dans l'en-tête YAML (valeur vide = retire la propriété).
pub fn set_property(kv: &str) {
    let Some((k, v)) = extras::parse_kv(kv) else {
        edit::msg_box("Saisie attendue : clé: valeur (la clé ne contient pas d'espace).");
        return;
    };
    apply_new_text(&extras::set_prop(&cur_text(), &k, &v));
}

/// (libellé, indication, saisie de départ de l'édition).
pub fn prop_items() -> Vec<(String, String, String)> {
    let mut v = vec![("+ Ajouter une propriété".to_string(), "clé: valeur".to_string(), String::new())];
    v.extend(extras::props(&cur_text()).into_iter().map(|p| (format!("{} : {}", p.key, p.value), "modifier".to_string(), format!("{}: {}", p.key, p.value))));
    v
}

/// (libellé, indication, ligne) des fonctions / titres du document courant.
pub fn symbols() -> Vec<(String, String, usize)> {
    let a = app();
    let md = lang::LANGS[a.docs[a.cur].lang].lexer == "markdown";
    crate::notes::outline(&cur_text(), md).into_iter().map(|(lv, t, l)| (format!("{}{t}", "  ".repeat(lv.saturating_sub(1).min(6))), format!("ligne {}", l + 1), l)).collect()
}

pub fn fav_toggle() {
    let a = app();
    let Some(p) = a.docs[a.cur].path.clone() else {
        edit::msg_box("Enregistrez d'abord le fichier pour le mettre en favori.");
        return;
    };
    let dir = prefs::dir();
    let mut l = extras::favs_load(&dir);
    let added = extras::favs_toggle(&mut l, &p);
    match extras::favs_save(&dir, &l) {
        Ok(()) => edit::msg_box(if added { "Fichier ajouté aux favoris." } else { "Fichier retiré des favoris." }),
        Err(e) => edit::msg_box(&format!("Favoris non enregistrés : {e}")),
    }
}

pub fn favs() -> Vec<String> {
    extras::favs_load(&prefs::dir())
}

/// Écrit le texte du document courant dans `path` avec son encodage, sans changer le nom ni l'état « modifié » de l'onglet.
pub fn write_copy(path: &str) -> Result<(), String> {
    let a = app();
    std::fs::write(path, doc::encode(&edit::text_of(a.cur), a.docs[a.cur].enc)).map_err(|e| extras::io_err("Copie impossible à écrire", std::path::Path::new(path), &e))
}

pub fn save_copy() {
    let a = app();
    let d = gtk::FileChooserNative::new(Some("Enregistrer une copie"), Some(&a.win), gtk::FileChooserAction::Save, Some("Enregistrer"), Some("Annuler"));
    d.set_do_overwrite_confirmation(true);
    d.set_current_name(&format!("{} - copie", edit::title_of(&a.docs[a.cur])));
    if d.run() != gtk::ResponseType::Accept {
        return;
    }
    let Some(p) = d.filename().map(|f| f.to_string_lossy().to_string()) else { return };
    match write_copy(&p) {
        Ok(()) => edit::msg_box(&format!("Copie enregistrée : {p}")),
        Err(e) => edit::msg_box(&e),
    }
}

/// Champs de la boîte de mise en page : marge, 3 zones d'en-tête, 3 zones de pied.
pub struct PageFields {
    pub margin: gtk::SpinButton,
    pub head: [gtk::Entry; 3],
    pub foot: [gtk::Entry; 3],
}

fn zones(tpl: &str) -> [String; 3] {
    let mut it = tpl.splitn(3, '|');
    [it.next().unwrap_or("").into(), it.next().unwrap_or("").into(), it.next().unwrap_or("").into()]
}

/// Construit la boîte (sans la montrer) remplie avec `l`.
pub fn page_dialog(l: &PrintLayout) -> (gtk::Dialog, PageFields) {
    let a = app();
    let d = gtk::Dialog::with_buttons(Some("Mise en page de l'impression"), Some(&a.win), gtk::DialogFlags::MODAL, &[("Valeurs par défaut", gtk::ResponseType::Reject), ("Annuler", gtk::ResponseType::Cancel), ("Enregistrer", gtk::ResponseType::Ok)]);
    d.set_default_response(gtk::ResponseType::Ok);
    let g = gtk::Grid::new();
    g.set_row_spacing(8);
    g.set_column_spacing(8);
    g.set_border_width(12);
    let margin = gtk::SpinButton::with_range(5.0, 60.0, 1.0);
    margin.set_value(l.margin_mm as f64);
    g.attach(&gtk::Label::new(Some("Marge (mm)")), 0, 0, 1, 1);
    g.attach(&margin, 1, 0, 1, 1);
    for (i, n) in ["Gauche", "Centre", "Droite"].iter().enumerate() {
        g.attach(&gtk::Label::new(Some(n)), 1 + i as i32, 1, 1, 1);
    }
    let row = |label: &str, tpl: &str, y: i32| -> [gtk::Entry; 3] {
        g.attach(&gtk::Label::new(Some(label)), 0, y, 1, 1);
        let z = zones(tpl);
        std::array::from_fn(|i| {
            let e = gtk::Entry::new();
            e.set_text(&z[i]);
            e.set_width_chars(18);
            e.set_activates_default(true);
            g.attach(&e, 1 + i as i32, y, 1, 1);
            e
        })
    };
    let head = row("En-tête", &l.header, 2);
    let foot = row("Pied de page", &l.footer, 3);
    let help = gtk::Label::new(Some("Variables : {fichier} {date} {page} {pages}. Une zone peut rester vide."));
    help.set_xalign(0.0);
    g.attach(&help, 0, 4, 4, 1);
    d.content_area().pack_start(&g, true, true, 0);
    d.content_area().show_all();
    (d, PageFields { margin, head, foot })
}

/// Mise en page lue dans les champs (le « | » est réservé aux séparateurs de zones).
pub fn page_read(f: &PageFields) -> PrintLayout {
    let join = |e: &[gtk::Entry; 3]| e.iter().map(|x| x.text().replace('|', "")).collect::<Vec<_>>().join("|");
    PrintLayout { margin_mm: f.margin.value_as_int().clamp(5, 60) as u32, header: join(&f.head), footer: join(&f.foot) }
}

pub fn page_setup() {
    let dir = prefs::dir();
    let (d, f) = page_dialog(&PrintLayout::load(&dir));
    loop {
        match d.run() {
            gtk::ResponseType::Reject => {
                let def = PrintLayout::default();
                f.margin.set_value(def.margin_mm as f64);
                for (e, z) in f.head.iter().zip(zones(&def.header)).chain(f.foot.iter().zip(zones(&def.footer))) {
                    e.set_text(&z);
                }
            }
            gtk::ResponseType::Ok => {
                d.close();
                if let Err(e) = page_read(&f).save(&dir) {
                    edit::msg_box(&e);
                }
                return;
            }
            _ => {
                d.close();
                return;
            }
        }
    }
}

// ---------- Récupération ----------

static WARNED: AtomicBool = AtomicBool::new(false);

/// Signale une panne de récupération une seule fois (la copie tourne toutes les 10 s : pas de boîte à répétition).
fn warn_once(e: &str) {
    if !WARNED.swap(true, Ordering::Relaxed) {
        edit::msg_box(&format!("La récupération après arrêt brutal ne fonctionne pas :\n{e}"));
    }
}

/// Copies de secours des documents modifiés (appelée toutes les 10 s).
pub fn recovery_snapshot() {
    let a = app();
    let docs: Vec<Recovered> = a
        .docs
        .iter()
        .enumerate()
        .filter(|(_, t)| t.dirty)
        .map(|(i, t)| Recovered { name: edit::title_of(t), path: t.path.clone(), enc: Enc::ALL.iter().position(|&e| e == t.enc).unwrap_or(0), text: edit::text_of(i) })
        .collect();
    if let Err(e) = extras::recovery_snapshot(&prefs::dir(), &docs) {
        warn_once(&e);
    }
}

/// Rouvre, comme documents modifiés, les copies des fenêtres disparues sans sortie normale. Renvoie les noms rouverts.
fn recover() -> Vec<String> {
    let dir = prefs::dir();
    let a = app();
    let open: Vec<(Option<String>, Vec<u8>)> = (0..a.docs.len()).map(|i| (a.docs[i].path.clone(), edit::text_of(i))).collect();
    let pending = extras::recovery_pending(&dir, &open);
    for r in &pending {
        edit::new_tab(None, Some((r.text.clone(), Enc::ALL.get(r.enc).copied().unwrap_or(Enc::Utf8))), true, 0);
    }
    if !pending.is_empty() {
        recovery_snapshot(); // les copies sont à nous avant d'effacer les traces de la fenêtre disparue
    }
    extras::recovery_discard(&dir);
    pending.into_iter().map(|r| r.name).collect()
}

/// Au démarrage, après la session : pose le verrou de cette fenêtre ; avec `claim`, rouvre les copies des fenêtres disparues.
pub fn recovery_start(claim: bool) {
    if let Err(e) = extras::recovery_register(&prefs::dir()) {
        warn_once(&e);
    }
    if claim {
        let names = recover();
        if !names.is_empty() {
            edit::msg_box(&format!("WinNote s'est arrêté brutalement : {} document(s) récupéré(s) dans de nouveaux onglets ({}).", names.len(), names.join(", ")));
        }
    }
}

/// Commande « Restaurer les documents récupérés ».
pub fn recover_cmd() {
    let names = recover();
    edit::msg_box(&if names.is_empty() { "Aucun document à récupérer.".to_string() } else { format!("{} document(s) récupéré(s) dans de nouveaux onglets ({}).", names.len(), names.join(", ")) });
}

pub fn recovery_end() {
    extras::recovery_end(&prefs::dir());
}
