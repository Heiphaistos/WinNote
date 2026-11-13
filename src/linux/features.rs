//! Propriétés YAML, favoris, enregistrer une copie, mise en page, récupération après arrêt brutal, liste des fonctions (Linux / GTK).
use crate::doc::{self, Enc};
use crate::extras::{self, PrintLayout, Recovered};
use crate::sci::{self, send};
use crate::{app, edit, lang, prefs, side};
use gtk::prelude::*;

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
    std::fs::write(path, doc::encode(&edit::text_of(a.cur), a.docs[a.cur].enc)).map_err(|e| e.to_string())
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
        Err(e) => edit::msg_box(&format!("Échec de l'enregistrement de la copie : {e}")),
    }
}

pub fn page_setup() {
    let cur = PrintLayout::load(&prefs::dir()).to_line();
    let Some(t) = side::ask_name_pub("Mise en page : marge (mm) ; en-tête ; pied — zones gauche|centre|droite, {fichier} {date} {page} {pages}", &cur) else { return };
    match PrintLayout::parse(&t) {
        Some(l) => {
            if let Err(e) = l.save(&prefs::dir()) {
                edit::msg_box(&format!("Mise en page non enregistrée : {e}"));
            }
        }
        None => edit::msg_box("Format attendu : 25 ; {fichier}||{date} ; |Page {page} / {pages}|"),
    }
}

// ---------- Récupération ----------

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
    extras::recovery_snapshot(&prefs::dir(), &docs);
}

/// Au démarrage, après la session : si le lancement précédent s'est arrêté brutalement, rouvre les copies de secours plus récentes que la session.
pub fn recovery_start() {
    let dir = prefs::dir();
    if !extras::recovery_begin(&dir) {
        return;
    }
    let a = app();
    let open: Vec<(Option<String>, Vec<u8>)> = (0..a.docs.len()).map(|i| (a.docs[i].path.clone(), edit::text_of(i))).collect();
    let pending = extras::recovery_pending(&dir, &open);
    for r in &pending {
        edit::new_tab(None, Some((r.text.clone(), Enc::ALL.get(r.enc).copied().unwrap_or(Enc::Utf8))), true, 0);
    }
    if !pending.is_empty() {
        let names: Vec<&str> = pending.iter().map(|r| r.name.as_str()).collect();
        edit::msg_box(&format!("WinNote s'est arrêté brutalement : {} document(s) récupéré(s) dans de nouveaux onglets ({}).", pending.len(), names.join(", ")));
    }
}

pub fn recovery_end() {
    extras::recovery_end(&prefs::dir());
}
