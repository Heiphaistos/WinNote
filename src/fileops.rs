//! Opérations sur les fichiers de l'explorateur (partagées Windows / Linux / macOS) : créer, renommer, dupliquer, supprimer, déplacer.
use std::path::{Path, PathBuf};

/// Nom de fichier valide : non vide, sans séparateur ni caractère réservé, ni « . » / « .. ».
pub fn valid_name(n: &str) -> Result<(), String> {
    let n = n.trim();
    if n.is_empty() || n == "." || n == ".." {
        return Err("Nom vide ou invalide.".into());
    }
    if n.chars().any(|c| matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') || (c as u32) < 32) {
        return Err("Le nom ne peut pas contenir / \\ : * ? \" < > |".into());
    }
    if cfg!(windows) && (n.ends_with('.') || reserved_windows(n)) {
        return Err("Nom réservé par Windows (CON, NUL, COM1...) ou finissant par un point.".into());
    }
    if n.len() > 255 {
        return Err("Nom trop long (255 octets maximum).".into());
    }
    Ok(())
}

/// Noms de périphériques DOS (avec ou sans extension) : les ouvrir lit la console ou un port au lieu d'un fichier.
pub fn reserved_windows(n: &str) -> bool {
    let stem = n.trim().split('.').next().unwrap_or("").trim_end().to_ascii_uppercase();
    matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "CONIN$" | "CONOUT$")
        || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT")) && matches!(stem.as_bytes()[3], b'1'..=b'9'))
}

/// Nom de fichier simple (pas de séparateur ni « .. ») : un fichier de session retouché ne doit pas faire lire ailleurs.
pub fn plain_name(n: &str) -> bool {
    !n.is_empty() && n != ".." && n != "." && !n.contains(['/', '\\', ':', '\0'])
}

/// Plafond d'ouverture : au-delà, Scintilla et les conversions d'encodage (tailles en i32) ne sont plus fiables.
pub const MAX_OPEN: u64 = 1 << 30;

/// Vérifie qu'un chemin est un vrai fichier ouvrable : pas de périphérique (CON, tube : la lecture bloquerait l'interface), pas plus de 1 Gio.
pub fn check_openable(path: &str) -> Result<(), String> {
    let name = Path::new(path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    if cfg!(windows) && reserved_windows(&name) {
        return Err("c'est un nom de périphérique Windows, pas un fichier".into());
    }
    let m = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !m.is_file() {
        return Err("ce n'est pas un fichier ordinaire".into());
    }
    if m.len() > MAX_OPEN {
        return Err(format!("fichier trop gros ({} Mio, maximum {} Mio)", m.len() >> 20, MAX_OPEN >> 20));
    }
    Ok(())
}

/// Écrit `bytes` dans `path` sans jamais laisser un fichier tronqué : écriture complète dans un voisin temporaire, puis remplacement.
/// Un lien symbolique, un fichier en lecture seule ou inexistant est écrit directement (le remplacement casserait le lien / les droits).
pub fn write_atomic(path: &str, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    let p = Path::new(path);
    let direct = match std::fs::symlink_metadata(p) {
        Ok(m) => m.file_type().is_symlink() || m.permissions().readonly() || !m.is_file(),
        Err(_) => true,
    };
    if direct {
        return std::fs::write(p, bytes);
    }
    let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let tmp = p.with_file_name(format!(".{name}.wn-{}.tmp", std::process::id()));
    let res = (|| {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        std::fs::rename(&tmp, p)
    })();
    if res.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    res
}

fn join(dir: &Path, name: &str) -> Result<PathBuf, String> {
    valid_name(name)?;
    let p = dir.join(name.trim());
    if p.exists() { Err(format!("« {} » existe déjà.", name.trim())) } else { Ok(p) }
}

pub fn new_file(dir: &str, name: &str) -> Result<String, String> {
    let p = join(Path::new(dir), name)?;
    std::fs::write(&p, b"").map_err(|e| e.to_string())?;
    Ok(p.to_string_lossy().into_owned())
}

pub fn new_dir(dir: &str, name: &str) -> Result<String, String> {
    let p = join(Path::new(dir), name)?;
    std::fs::create_dir(&p).map_err(|e| e.to_string())?;
    Ok(p.to_string_lossy().into_owned())
}

pub fn rename(path: &str, name: &str) -> Result<String, String> {
    let from = Path::new(path);
    let dir = from.parent().unwrap_or(Path::new("."));
    let same_name = from.file_name().is_some_and(|n| n.to_string_lossy().to_lowercase() == name.trim().to_lowercase());
    let to = if same_name && valid_name(name).is_ok() { dir.join(name.trim()) } else { join(dir, name)? };
    std::fs::rename(from, &to).map_err(|e| e.to_string())?;
    Ok(to.to_string_lossy().into_owned())
}

/// Copie à côté de l'original : « note.md » -> « note copie.md », « note copie 2.md »...
pub fn duplicate(path: &str) -> Result<String, String> {
    let from = Path::new(path);
    let dir = from.parent().unwrap_or(Path::new("."));
    let stem = from.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ext = from.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    let mut to = dir.join(format!("{stem} copie{ext}"));
    let mut i = 2;
    while to.exists() {
        to = dir.join(format!("{stem} copie {i}{ext}"));
        i += 1;
    }
    if std::fs::symlink_metadata(from).is_ok_and(|m| m.is_dir()) {
        copy_dir(from, &to)?;
    } else {
        std::fs::copy(from, &to).map_err(|e| e.to_string())?;
    }
    Ok(to.to_string_lossy().into_owned())
}

fn copy_dir(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir(to).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())?.flatten() {
        let t = to.join(e.file_name());
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_symlink() {
            continue; // un lien (ou une jonction) peut boucler : jamais suivi
        }
        if ft.is_dir() { copy_dir(&e.path(), &t)? } else if ft.is_file() { std::fs::copy(e.path(), &t).map(|_| ()).map_err(|e| e.to_string())? }
    }
    Ok(())
}

/// Déplace vers un dossier (refuse d'écraser, et de déplacer un dossier dans lui-même).
pub fn move_to(path: &str, dir: &str) -> Result<String, String> {
    let from = Path::new(path);
    let name = from.file_name().ok_or("Chemin invalide.")?;
    let to = Path::new(dir).join(name);
    if to.exists() {
        return Err(format!("« {} » existe déjà dans le dossier cible.", name.to_string_lossy()));
    }
    if Path::new(dir).starts_with(from) {
        return Err("Impossible de déplacer un dossier dans lui-même.".into());
    }
    std::fs::rename(from, &to).map_err(|e| e.to_string())?;
    Ok(to.to_string_lossy().into_owned())
}

/// Suppression définitive (la version Windows passe par la corbeille avant d'arriver ici seulement en repli).
#[allow(dead_code)]
pub fn remove(path: &str) -> Result<(), String> {
    let p = Path::new(path);
    if p.is_dir() { std::fs::remove_dir_all(p) } else { std::fs::remove_file(p) }.map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ops() {
        let d = std::env::temp_dir().join(format!("wn_fileops_{}", std::process::id()));
        std::fs::create_dir_all(&d).ok();
        let ds = d.to_string_lossy().to_string();
        let f = new_file(&ds, "a.md").unwrap();
        assert!(new_file(&ds, "a.md").is_err());
        assert!(valid_name("x/y").is_err() && valid_name("  ").is_err() && valid_name("ok.txt").is_ok());
        let f2 = rename(&f, "b.md").unwrap();
        assert!(Path::new(&f2).exists() && !Path::new(&f).exists());
        let c = duplicate(&f2).unwrap();
        assert!(c.ends_with("b copie.md"));
        assert!(duplicate(&f2).unwrap().ends_with("b copie 2.md"));
        let sub = new_dir(&ds, "sub").unwrap();
        let m = move_to(&f2, &sub).unwrap();
        assert!(Path::new(&m).exists());
        assert!(move_to(&sub, &m).is_err());
        remove(&m).unwrap();
        assert!(!Path::new(&m).exists());
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn reserved_and_limits() {
        for n in ["CON", "nul.txt", "Com1", "lpt9.md", "aux ", "CONIN$"] {
            assert!(reserved_windows(n), "{n}");
            assert_eq!(valid_name(n).is_err(), cfg!(windows), "{n}");
        }
        for n in ["com10", "console.txt", "x.nul", "COM0"] {
            assert!(!reserved_windows(n) && valid_name(n).is_ok(), "{n}");
        }
        assert!(valid_name(&"a".repeat(256)).is_err());
        let d = std::env::temp_dir().join(format!("wn_fo2_{}", std::process::id()));
        std::fs::remove_dir_all(&d).ok();
        std::fs::create_dir_all(&d).unwrap();
        assert!(check_openable(&d.to_string_lossy()).is_err(), "un dossier n'est pas un fichier");
        let f = d.join("a.txt").to_string_lossy().to_string();
        write_atomic(&f, b"un").unwrap();
        write_atomic(&f, b"deux").unwrap();
        assert_eq!(std::fs::read(&f).unwrap(), b"deux");
        assert!(check_openable(&f).is_ok());
        assert_eq!(std::fs::read_dir(&d).unwrap().count(), 1, "aucun fichier temporaire ne reste");
        let g = rename(&f, "A.txt").unwrap();
        assert!(g.ends_with("A.txt") && std::fs::read_dir(&d).unwrap().next().unwrap().unwrap().file_name() == "A.txt");
        let sub = d.join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("x"), b"1").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&sub, sub.join("loop")).unwrap();
        let c = duplicate(&sub.to_string_lossy()).unwrap();
        assert!(Path::new(&c).join("x").exists() && !Path::new(&c).join("loop").exists());
        std::fs::remove_dir_all(&d).ok();
    }
}
