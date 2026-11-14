//! Propriétés YAML, favoris, enregistrer une copie, mise en page, récupération après arrêt brutal (Windows).
use crate::extras::{self, PrintLayout, Recovered};
use crate::sci::{self, send};
use crate::{app, cmds, doc, prefs};
use std::sync::atomic::{AtomicBool, Ordering};

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
    crate::fileops::write_atomic(path, &doc::encode(&text, a.docs[a.cur].enc)).map_err(|e| extras::io_err("Copie impossible à écrire", std::path::Path::new(path), &e))
}

/// « Enregistrer une copie... » : boîte d'enregistrement, puis écriture sans changer d'onglet.
pub fn save_copy() {
    let a = app();
    let Some(p) = cmds::file_dialog(true, &format!("{} - copie", a.docs[a.cur].title())).into_iter().next() else { return };
    match write_copy(&p) {
        Ok(()) => cmds::msg_box(&format!("Copie enregistrée : {p}")),
        Err(e) => cmds::msg_box(&e),
    }
}

pub fn page_setup() {
    let dir = prefs::dir();
    if let Some(l) = crate::pagedlg::show(&PrintLayout::load(&dir)) {
        if let Err(e) = l.save(&dir) {
            cmds::msg_box(&e);
        }
    }
}

// ---------- Récupération ----------

static WARNED: AtomicBool = AtomicBool::new(false);

/// Signale une panne de récupération une seule fois (la copie tourne toutes les 10 s : pas de boîte à répétition).
fn warn_once(e: &str) {
    if !WARNED.swap(true, Ordering::Relaxed) {
        cmds::msg_box(&format!("La récupération après arrêt brutal ne fonctionne pas :\n{e}"));
    }
}

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
    if let Err(e) = extras::recovery_snapshot(&prefs::dir(), &docs) {
        warn_once(&e);
    }
}

/// Rouvre, comme documents modifiés, les copies des fenêtres disparues sans sortie normale. Renvoie les noms rouverts.
fn recover() -> Vec<String> {
    let dir = prefs::dir();
    let a = app();
    let open: Vec<(Option<String>, Vec<u8>)> = (0..a.docs.len()).map(|i| (a.docs[i].path.clone(), cmds::text_of_doc(a, i))).collect();
    let pending = extras::recovery_pending(&dir, &open);
    // l'onglet vide du démarrage ne reste pas à côté des documents récupérés
    let blank = a.docs.len() == 1 && a.docs[0].path.is_none() && !a.docs[0].dirty && open[0].1.is_empty() && !pending.is_empty();
    for r in &pending {
        let enc = doc::Enc::ALL.get(r.enc).copied().unwrap_or(doc::Enc::Utf8);
        cmds::push_doc_full(a, None, Some((r.text.clone(), enc)), true, 0);
    }
    if blank {
        cmds::cmd_close(0);
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
            cmds::msg_box(&format!("WinNote s'est arrêté brutalement : {} document(s) récupéré(s) dans de nouveaux onglets ({}).", names.len(), names.join(", ")));
        }
    }
}

/// Commande « Restaurer les documents récupérés ».
pub fn recover_cmd() {
    let names = recover();
    cmds::msg_box(&if names.is_empty() { "Aucun document à récupérer.".to_string() } else { format!("{} document(s) récupéré(s) dans de nouveaux onglets ({}).", names.len(), names.join(", ")) });
}

pub fn recovery_end() {
    extras::recovery_end(&prefs::dir());
}
