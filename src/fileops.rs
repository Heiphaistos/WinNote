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
    Ok(())
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
    let to = join(from.parent().unwrap_or(Path::new(".")), name)?;
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
    if from.is_dir() {
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
        if e.path().is_dir() { copy_dir(&e.path(), &t)? } else { std::fs::copy(e.path(), &t).map(|_| ()).map_err(|e| e.to_string())? }
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
}
