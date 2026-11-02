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
        let (path, enc, caret, first, dirty, untitled) = (doc.path.clone().unwrap_or_default(), enc_id(doc.enc), doc.caret, doc.first, doc.dirty as u8, doc.untitled);
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
        lines.push(format!("doc\t{path}\t{enc}\t{caret}\t{first}\t{dirty}\t{untitled}\t{backup}"));
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
        let f: Vec<&str> = l.split('\t').collect();
        match f.as_slice() {
            ["cur", c] => cur = c.parse().unwrap_or(0),
            ["doc", path, enc, caret, first, dirty, untitled, backup] => {
                let enc = Enc::ALL.get(enc.parse().unwrap_or(0)).copied().unwrap_or(Enc::Utf8);
                let path = (!path.is_empty()).then(|| path.to_string());
                let text = if backup.is_empty() {
                    match path.as_ref().and_then(|p| std::fs::read(p).ok()) {
                        Some(b) => crate::doc::decode(&b).0,
                        None => continue,
                    }
                } else {
                    std::fs::read(d.join(backup)).unwrap_or_default()
                };
                let dirty = *dirty == "1";
                crate::cmds::push_doc_full(a, path, Some((text, enc)), dirty, untitled.parse().unwrap_or(0));
                let last = a.docs.len() - 1;
                a.docs[last].caret = caret.parse().unwrap_or(0);
                a.docs[last].first = first.parse().unwrap_or(0);
                n += 1;
            }
            _ => {}
        }
    }
    if n > 0 {
        crate::cmds::switch_to(a, cur.min(a.docs.len() - 1));
    }
    n > 0
}
