//! Propriétés YAML, favoris, enregistrer une copie, mise en page, récupération après arrêt brutal (Windows).
use crate::extras::{self, PrintLayout, Recovered};
use crate::sci::{self, send};
use crate::{app, cmds, doc, palette, prefs};

fn cur_text() -> String {
    let a = app();
    String::from_utf8_lossy(&cmds::text_of_doc(a, a.cur)).into_owned()
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
        cmds::msg_box("Saisie attendue : clé: valeur (la clé ne contient pas d'espace).");
        return;
    };
    apply_new_text(&extras::set_prop(&cur_text(), &k, &v));
}

/// Entrées de la palette des propriétés : (libellé, indication, saisie de départ de l'édition).
pub fn prop_items() -> Vec<(String, String, String)> {
    let mut v = vec![("+ Ajouter une propriété".to_string(), "clé: valeur".to_string(), String::new())];
    v.extend(extras::props(&cur_text()).into_iter().map(|p| (format!("{} : {}", p.key, p.value), "modifier".to_string(), format!("{}: {}", p.key, p.value))));
    v
}

pub fn fav_toggle() {
    let a = app();
    let Some(p) = a.docs[a.cur].path.clone() else {
        cmds::msg_box("Enregistrez d'abord le fichier pour le mettre en favori.");
        return;
    };
    let dir = prefs::dir();
    let mut l = extras::favs_load(&dir);
    let added = extras::favs_toggle(&mut l, &p);
    match extras::favs_save(&dir, &l) {
        Ok(()) => cmds::msg_box(if added { "Fichier ajouté aux favoris." } else { "Fichier retiré des favoris." }),
        Err(e) => cmds::msg_box(&format!("Favoris non enregistrés : {e}")),
    }
}

pub fn favs() -> Vec<String> {
    extras::favs_load(&prefs::dir())
}

/// Écrit le texte du document courant dans `path` avec son encodage, sans changer le nom ni l'état « modifié » de l'onglet.
pub fn write_copy(path: &str) -> Result<(), String> {
    let a = app();
    let text = cmds::text_of_doc(a, a.cur);
    std::fs::write(path, doc::encode(&text, a.docs[a.cur].enc)).map_err(|e| e.to_string())
}

/// « Enregistrer une copie... » : boîte d'enregistrement, puis écriture sans changer d'onglet.
pub fn save_copy() {
    let a = app();
    let Some(p) = cmds::file_dialog(true, &format!("{} - copie", a.docs[a.cur].title())).into_iter().next() else { return };
    match write_copy(&p) {
        Ok(()) => cmds::msg_box(&format!("Copie enregistrée : {p}")),
        Err(e) => cmds::msg_box(&format!("Échec de l'enregistrement de la copie : {e}")),
    }
}

pub fn page_setup() {
    let cur = PrintLayout::load(&prefs::dir()).to_line();
    palette::input(
        "Mise en page : marge (mm) ; en-tête ; pied — zones gauche|centre|droite, {fichier} {date} {page} {pages}",
        &cur,
        Box::new(|t| match PrintLayout::parse(&t) {
            Some(l) => {
                if let Err(e) = l.save(&prefs::dir()) {
                    cmds::msg_box(&format!("Mise en page non enregistrée : {e}"));
                }
            }
            None => cmds::msg_box("Format attendu : 25 ; {fichier}||{date} ; |Page {page} / {pages}|"),
        }),
    );
}

// ---------- Récupération ----------

/// Copies de secours des documents modifiés (appelée toutes les 10 s).
pub fn recovery_snapshot() {
    let a = app();
    let mut docs = Vec::new();
    for i in 0..a.docs.len() {
        if a.docs[i].dirty {
            let name = a.docs[i].title();
            let (path, enc) = (a.docs[i].path.clone(), doc::Enc::ALL.iter().position(|&e| e == a.docs[i].enc).unwrap_or(0));
            docs.push(Recovered { name, path, enc, text: cmds::text_of_doc(a, i) });
        }
    }
    extras::recovery_snapshot(&prefs::dir(), &docs);
}

/// Au démarrage, après la session : si le lancement précédent s'est arrêté brutalement, rouvre les copies de secours plus récentes que la session.
pub fn recovery_start() {
    let dir = prefs::dir();
    if !extras::recovery_begin(&dir) {
        return;
    }
    let a = app();
    let open: Vec<(Option<String>, Vec<u8>)> = (0..a.docs.len()).map(|i| (a.docs[i].path.clone(), cmds::text_of_doc(a, i))).collect();
    let pending = extras::recovery_pending(&dir, &open);
    for r in &pending {
        let enc = doc::Enc::ALL.get(r.enc).copied().unwrap_or(doc::Enc::Utf8);
        cmds::push_doc_full(a, None, Some((r.text.clone(), enc)), true, 0);
    }
    if !pending.is_empty() {
        let names: Vec<&str> = pending.iter().map(|r| r.name.as_str()).collect();
        cmds::msg_box(&format!("WinNote s'est arrêté brutalement : {} document(s) récupéré(s) dans de nouveaux onglets ({}).", pending.len(), names.join(", ")));
    }
}

pub fn recovery_end() {
    extras::recovery_end(&prefs::dir());
}
