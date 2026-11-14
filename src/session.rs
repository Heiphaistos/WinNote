//! Session : onglets restaurés à la réouverture (y compris les textes non enregistrés), fichiers récents.
use crate::doc::Enc;
use crate::{app, App};
use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(std::env::var("APPDATA").unwrap_or_default()).join("WinNote")
}

pub fn load_recent(a: &mut App) {
    if let Ok(s) = std::fs::read_to_string(dir().join("recent.txt")) {
        a.recent = s.lines().filter(|l| !l.is_empty()).map(String::from).take(15).collect();
    }
}

pub fn add_recent(p: &str) {
    let a = app();
    a.recent.retain(|r| r != p);
    a.recent.insert(0, p.to_string());
    a.recent.truncate(15);
    save_recent();
    crate::menu::rebuild_recent(a);
}

pub fn save_recent() {
    let a = app();
    let _ = std::fs::create_dir_all(dir());
    if let Err(e) = std::fs::write(dir().join("recent.txt"), a.recent.join("\n")) {
        crate::cmds::msg_box(&format!("Liste des fichiers récents non enregistrée : {e}"));
    }
}

fn enc_id(e: Enc) -> usize {
    Enc::ALL.iter().position(|&x| x == e).unwrap_or(0)
}

/// Écrit la session (appelée à la fermeture et toutes les 30 s).
pub fn save(a: &mut App) {
    let d = dir().join("session");
    let _ = std::fs::remove_dir_all(&d);
    if std::fs::create_dir_all(&d).is_err() {
        return;
    }
    crate::cmds::save_view_state_pub(a);
    let mut lines = vec![format!("cur\t{}", a.cur)];
    for i in 0..a.docs.len() {
        let doc = &a.docs[i];
        let keep = doc.dirty || doc.path.is_none();
        if doc.path.is_none() && !doc.dirty {
            continue; // onglet vide jamais touché : rien à restaurer
        }
        let (path, enc, caret, first, dirty, untitled) = (doc.path.clone().unwrap_or_default(), enc_id(doc.enc), doc.caret, doc.first, doc.dirty, doc.untitled);
        // langage mémorisé seulement s'il a été changé à la main (sinon l'extension le détermine à la réouverture)
        let auto = doc.path.as_deref().map_or(0, crate::lang::by_path);
        let lang = if doc.lang != auto { crate::lang::LANGS[doc.lang].name.to_string() } else { String::new() };
        let pinned = doc.pinned;
        let backup = if keep {
            let name = format!("{i}.txt");
            let text = crate::cmds::text_of_doc(a, i);
            if std::fs::write(d.join(&name), text).is_err() {
                continue;
            }
            name
        } else {
            String::new()
        };
        lines.push(crate::sessfmt::format(&crate::sessfmt::Rec { path, enc, caret, first, dirty, untitled, backup, pinned, lang }));
    }
    let _ = std::fs::write(d.join("session.txt"), lines.join("\n"));
}

/// Recrée les onglets de la dernière session. Renvoie true si au moins un onglet a été restauré.
pub fn restore(a: &mut App) -> bool {
    let d = dir().join("session");
    let Ok(s) = std::fs::read_to_string(d.join("session.txt")) else { return false };
    let mut cur = 0usize;
    let mut n = 0;
    for l in s.lines() {
        if let Some(c) = l.strip_prefix("cur\t") {
            cur = c.parse().unwrap_or(0);
            continue;
        }
        let Some(r) = crate::sessfmt::parse(l) else { continue };
        let enc = Enc::ALL.get(r.enc).copied().unwrap_or(Enc::Utf8);
        let path = (!r.path.is_empty()).then(|| r.path.clone());
        let text = if r.backup.is_empty() {
            match path.as_ref().and_then(|p| std::fs::read(p).ok()) {
                Some(b) => crate::doc::decode(&b).0,
                None => continue,
            }
        } else if crate::fileops::plain_name(&r.backup) {
            std::fs::read(d.join(&r.backup)).unwrap_or_default()
        } else {
            continue;
        };
        crate::cmds::push_doc_full(a, path, Some((text, enc)), r.dirty, r.untitled);
        if let Some(i) = crate::lang::LANGS.iter().position(|l| l.name == r.lang).filter(|_| !r.lang.is_empty()) {
            crate::cmds::set_language(i);
        }
        let last = a.docs.len() - 1;
        a.docs[last].caret = r.caret;
        a.docs[last].first = r.first;
        a.docs[last].pinned = r.pinned;
        // aussi dans la vue : switch_to() réenregistre l'état de la vue de l'onglet courant (sinon le dernier onglet perdrait son curseur)
        crate::sci::send(a.sci, crate::sci::SETSEL, r.caret.max(0) as usize, r.caret.max(0));
        crate::sci::send(a.sci, crate::sci::SETFIRSTVISIBLELINE, r.first.max(0) as usize, 0);
        n += 1;
    }
    if n > 0 {
        crate::cmds::switch_to(a, cur.min(a.docs.len() - 1));
    }
    n > 0
}
