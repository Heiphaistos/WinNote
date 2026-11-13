//! Raccourcis clavier personnalisés (Windows / Linux / macOS) : fichier `raccourcis.txt`, une ligne `Ctrl+Maj+X = nom de la commande`.
use crate::ids::PALETTE_CMDS;
use std::cell::RefCell;
use std::path::PathBuf;

#[derive(Clone, Debug, PartialEq)]
pub struct Chord { pub ctrl: bool, pub shift: bool, pub alt: bool, pub key: String }

thread_local! {
    static USER: RefCell<Vec<(Chord, usize)>> = const { RefCell::new(Vec::new()) };
    static PATH: RefCell<PathBuf> = const { RefCell::new(PathBuf::new()) };
}

pub fn user() -> Vec<(Chord, usize)> { USER.with(|u| u.borrow().clone()) }
pub fn file_path() -> PathBuf { PATH.with(|p| p.borrow().clone()) }

/// Fixe l'emplacement du fichier et le charge (à appeler au démarrage).
pub fn init(dir: PathBuf) {
    PATH.with(|p| *p.borrow_mut() = dir.join("raccourcis.txt"));
    reload();
}

pub fn reload() {
    let text = std::fs::read_to_string(file_path()).unwrap_or_default();
    USER.with(|u| *u.borrow_mut() = parse_file(&text));
}

/// « Ctrl+Maj+K » -> combinaison. Accepte Ctrl/Maj/Shift/Alt et une touche (lettre, chiffre, F1..F24, Entrée, Tab, Suppr...).
pub fn parse_chord(s: &str) -> Option<Chord> {
    let mut c = Chord { ctrl: false, shift: false, alt: false, key: String::new() };
    for t in s.split('+').map(str::trim).filter(|t| !t.is_empty()) {
        match t.to_lowercase().as_str() {
            "ctrl" | "ctl" | "control" => c.ctrl = true,
            "maj" | "shift" => c.shift = true,
            "alt" => c.alt = true,
            k => c.key = k.to_uppercase(),
        }
    }
    (!c.key.is_empty() && vk(&c.key).is_some() && (c.ctrl || c.alt || c.key.starts_with('F') && c.key.len() > 1)).then_some(c)
}

/// Lignes `combinaison = commande` ; `#` commence un commentaire. La commande est le libellé de la palette (ou un numéro).
pub fn parse_file(text: &str) -> Vec<(Chord, usize)> {
    let mut v = Vec::new();
    for l in text.lines() {
        let l = l.trim();
        if l.starts_with('#') {
            continue;
        }
        let Some((k, name)) = l.split_once('=') else { continue };
        let name = name.trim();
        let id = PALETTE_CMDS.iter().find(|c| c.1.eq_ignore_ascii_case(name) || c.1.to_lowercase() == name.to_lowercase()).map(|c| c.0).or_else(|| name.parse().ok());
        if let (Some(c), Some(id)) = (parse_chord(k), id) {
            v.push((c, id));
        }
    }
    v
}

/// Code de touche virtuelle Windows.
pub fn vk(key: &str) -> Option<u16> {
    let b = key.as_bytes();
    if b.len() == 1 && (b[0].is_ascii_uppercase() || b[0].is_ascii_digit()) {
        return Some(b[0] as u16);
    }
    if let Some(n) = key.strip_prefix('F').and_then(|n| n.parse::<u16>().ok()).filter(|n| (1..=24).contains(n)) {
        return Some(0x6F + n);
    }
    Some(match key {
        "ENTRÉE" | "ENTREE" | "ENTER" => 0x0D,
        "TAB" => 0x09,
        "ESPACE" | "SPACE" => 0x20,
        "SUPPR" | "DELETE" => 0x2E,
        "INSER" | "INSERT" => 0x2D,
        "HAUT" | "UP" => 0x26,
        "BAS" | "DOWN" => 0x28,
        "GAUCHE" | "LEFT" => 0x25,
        "DROITE" | "RIGHT" => 0x27,
        "DÉBUT" | "DEBUT" | "HOME" => 0x24,
        "FIN" | "END" => 0x23,
        _ => return None,
    })
}

/// Spécification d'accélérateur GTK (« <Primary><Shift>k »).
#[allow(dead_code)]
pub fn gtk_spec(c: &Chord) -> String {
    let key = match c.key.as_str() {
        "ENTRÉE" | "ENTREE" | "ENTER" => "Return".to_string(),
        "ESPACE" | "SPACE" => "space".into(),
        "SUPPR" | "DELETE" => "Delete".into(),
        "INSER" | "INSERT" => "Insert".into(),
        "HAUT" | "UP" => "Up".into(),
        "BAS" | "DOWN" => "Down".into(),
        "GAUCHE" | "LEFT" => "Left".into(),
        "DROITE" | "RIGHT" => "Right".into(),
        "DÉBUT" | "DEBUT" | "HOME" => "Home".into(),
        "FIN" | "END" => "End".into(),
        "TAB" => "Tab".into(),
        k if k.len() == 1 => k.to_lowercase(),
        k => k.to_string(),
    };
    format!("{}{}{}{key}", if c.ctrl { "<Primary>" } else { "" }, if c.shift { "<Shift>" } else { "" }, if c.alt { "<Alt>" } else { "" })
}

/// Contenu proposé à la première ouverture du fichier : explications + liste des commandes.
pub fn template() -> String {
    let mut s = String::from("# Raccourcis clavier de WinNote\n# Une ligne par raccourci :  combinaison = commande\n# Exemple :  Ctrl+Alt+B = Markdown : gras\n# Modificateurs : Ctrl, Maj, Alt. Touches : A-Z, 0-9, F1-F24, Entrée, Tab, Espace, Suppr, Haut, Bas, Gauche, Droite, Début, Fin.\n# Enregistrez ce fichier : les raccourcis sont pris en compte tout de suite (Windows) ou au prochain lancement.\n# Sous Linux et macOS, choisissez une combinaison libre (elle ne remplace pas un raccourci du menu).\n#\n# Commandes disponibles :\n");
    for (_, n) in PALETTE_CMDS {
        s.push_str(&format!("#   {n}\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse() {
        let c = parse_chord("Ctrl+Maj+k").unwrap();
        assert!(c.ctrl && c.shift && !c.alt && c.key == "K");
        assert!(parse_chord("K").is_none(), "une lettre seule est refusée");
        assert!(parse_chord("F5").is_some());
        assert_eq!(gtk_spec(&c), "<Primary><Shift>k");
        let v = parse_file("# x\nCtrl+Alt+B = Markdown : gras\nCtrl+9 = 102\nbad\n");
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].1, crate::ids::MD_BOLD);
        assert_eq!(vk("A"), Some(65));
        assert_eq!(vk("F2"), Some(0x71));
    }
}
